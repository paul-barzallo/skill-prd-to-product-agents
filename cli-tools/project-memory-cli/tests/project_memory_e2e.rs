use serde_json::Value;
use rusqlite::Connection;
use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;
use tempfile::tempdir;

fn write_file(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent directories");
    }
    fs::write(path, content).expect("write fixture file");
}

fn run_cli(project_root: &Path, args: &[&str]) -> (std::process::ExitStatus, Value) {
    run_cli_with_global_args(project_root, &[], args)
}

fn run_cli_raw_with_env(
    project_root: &Path,
    global_args: &[&str],
    args: &[&str],
    envs: &[(&str, &str)],
) -> (std::process::ExitStatus, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_project-memory-cli"))
        .args(["--project-root", project_root.to_str().expect("project root utf-8")])
        .args(global_args)
        .args(args)
        .envs(envs.iter().copied())
        .output()
        .expect("run project-memory-cli raw");

    (
        output.status,
        String::from_utf8(output.stdout).expect("stdout utf-8"),
        String::from_utf8(output.stderr).expect("stderr utf-8"),
    )
}

fn run_cli_with_env(
    project_root: &Path,
    global_args: &[&str],
    args: &[&str],
    envs: &[(&str, &str)],
) -> (std::process::ExitStatus, Value) {
    let (status, stdout, _stderr) = run_cli_raw_with_env(project_root, global_args, args, envs);
    let json = serde_json::from_str::<Value>(&stdout).expect("valid JSON output");
    (status, json)
}

fn run_cli_with_global_args(
    project_root: &Path,
    global_args: &[&str],
    args: &[&str],
) -> (std::process::ExitStatus, Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_project-memory-cli"))
        .args(["--project-root", project_root.to_str().expect("project root utf-8")])
        .args(global_args)
        .args(args)
        .output()
        .expect("run project-memory-cli");

    let stdout = String::from_utf8(output.stdout).expect("stdout utf-8");
    let json = serde_json::from_str::<Value>(&stdout).expect("valid JSON output");
    (output.status, json)
}

fn start_embedding_stub(expected_requests: usize) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind embedding stub");
    let address = listener.local_addr().expect("embedding stub address");
    let handle = thread::spawn(move || {
        for _ in 0..expected_requests {
            let (mut stream, _) = listener.accept().expect("accept embedding request");
            let mut buffer = Vec::new();
            let mut chunk = [0_u8; 4096];

            loop {
                let read = stream.read(&mut chunk).expect("read embedding request");
                if read == 0 {
                    break;
                }
                buffer.extend_from_slice(&chunk[..read]);
                if let Some(header_end) = find_header_end(&buffer) {
                    let content_length = parse_content_length(&buffer[..header_end]);
                    let body_end = header_end + content_length;
                    if buffer.len() >= body_end {
                        let body = &buffer[header_end..body_end];
                        let request: Value = serde_json::from_slice(body).expect("parse embedding request");
                        let response = build_embedding_response(&request);
                        let response_body = serde_json::to_vec(&response).expect("serialize embedding response");
                        let headers = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            response_body.len()
                        );
                        stream.write_all(headers.as_bytes()).expect("write embedding headers");
                        stream.write_all(&response_body).expect("write embedding body");
                        break;
                    }
                }
            }
        }
    });

    (format!("http://{address}/v1/embeddings"), handle)
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
}

fn parse_content_length(headers: &[u8]) -> usize {
    let text = String::from_utf8_lossy(headers);
    text.lines()
        .find_map(|line| {
            line.split_once(':').and_then(|(name, value)| {
                if name.eq_ignore_ascii_case("content-length") {
                    value.trim().parse().ok()
                } else {
                    None
                }
            })
        })
        .unwrap_or(0)
}

fn build_embedding_response(request: &Value) -> Value {
    let data = request["inputs"]
        .as_array()
        .expect("embedding inputs array")
        .iter()
        .map(|input| {
            let id = input["id"].as_str().expect("embedding input id");
            let text = input["text"].as_str().expect("embedding input text");
            let token_like = text.split_whitespace().count() as f32;
            let incident_bias = if text.to_ascii_lowercase().contains("incident") {
                5.0
            } else {
                1.0
            };

            serde_json::json!({
                "id": id,
                "embedding": [text.len() as f32, token_like, incident_bias],
            })
        })
        .collect::<Vec<_>>();

    serde_json::json!({ "data": data })
}

fn start_openai_embedding_stub(
    expected_requests: usize,
    expected_modes: &[&str],
) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind openai embedding stub");
    let address = listener.local_addr().expect("openai embedding stub address");
    let modes = expected_modes.iter().map(|value| value.to_string()).collect::<Vec<_>>();

    let handle = thread::spawn(move || {
        for index in 0..expected_requests {
            let (mut stream, _) = listener.accept().expect("accept openai embedding request");
            let mut buffer = Vec::new();
            let mut chunk = [0_u8; 4096];

            loop {
                let read = stream.read(&mut chunk).expect("read openai embedding request");
                if read == 0 {
                    break;
                }
                buffer.extend_from_slice(&chunk[..read]);
                if let Some(header_end) = find_header_end(&buffer) {
                    let content_length = parse_content_length(&buffer[..header_end]);
                    let body_end = header_end + content_length;
                    if buffer.len() >= body_end {
                        let headers = String::from_utf8_lossy(&buffer[..header_end]).to_string();
                        let request_line = headers.lines().next().expect("request line");
                        let mode = modes.get(index).expect("expected openai mode");
                        let body = &buffer[header_end..body_end];
                        let request: Value = serde_json::from_slice(body).expect("parse openai request");
                        assert_openai_request(mode, request_line, &headers, &request);
                        let response = build_openai_embedding_response(&request);
                        let response_body = serde_json::to_vec(&response).expect("serialize openai response");
                        let response_headers = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            response_body.len()
                        );
                        stream.write_all(response_headers.as_bytes()).expect("write openai headers");
                        stream.write_all(&response_body).expect("write openai body");
                        break;
                    }
                }
            }
        }
    });

    (format!("http://{address}"), handle)
}

fn assert_openai_request(mode: &str, request_line: &str, headers: &str, request: &Value) {
    let header_map = parse_headers(headers);
    match mode {
        "generic" => {
            assert!(request_line.starts_with("POST /v1/embeddings "), "generic provider should call /v1/embeddings");
            let auth = header_map
                .get("authorization")
                .expect("generic authorization header");
            assert!(auth.starts_with("Bearer "));
            assert!(request["model"].is_string(), "generic provider should send model");
            assert!(request["input"].is_array(), "generic provider should send input array");
        }
        "azure" => {
            assert!(
                request_line.starts_with("POST /openai/deployments/team-embed/embeddings?api-version=2024-06-01-preview "),
                "azure provider should call deployment-scoped embeddings path"
            );
            assert_eq!(
                header_map.get("api-key").map(String::as_str),
                Some("azure-secret"),
                "azure provider should send api-key header"
            );
            assert!(request["input"].is_array(), "azure provider should send input array");
        }
        other => panic!("unexpected openai mode {other}"),
    }
}

fn parse_headers(headers: &str) -> BTreeMap<String, String> {
    let mut parsed = BTreeMap::new();
    for line in headers.lines().skip(1) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some((name, value)) = trimmed.split_once(':') {
            parsed.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }
    parsed
}

fn build_openai_embedding_response(request: &Value) -> Value {
    let data = request["input"]
        .as_array()
        .expect("openai input array")
        .iter()
        .enumerate()
        .map(|(index, input)| {
            let text = input.as_str().expect("openai input text");
            serde_json::json!({
                "index": index,
                "embedding": [text.len() as f32, text.split_whitespace().count() as f32, (index + 1) as f32],
            })
        })
        .collect::<Vec<_>>();

    serde_json::json!({ "data": data })
}

fn build_fixture_project() -> tempfile::TempDir {
    let temp = tempdir().expect("create temp dir");
    write_file(
        temp.path(),
        ".gitignore",
        ".project-memory/\nignored.log\ncli-tools/project-memory-cli/target/\n",
    );
    write_file(
        temp.path(),
        "docs/prd.md",
        "# Checkout PRD\n\nREQ-001 Checkout must validate cart totals before payment.\nSee src/checkout.rs for the pricing flow.\n\nREQ-002 Payment audit trail must be recorded.\n\nREQ-003 Alerts must be emitted through src/missing.rs.\n",
    );
    write_file(
        temp.path(),
        "docs/spec.md",
        "# Supporting Spec\n\nREQ-001 uses docs/prd.md as the source requirement.\n",
    );
    write_file(
        temp.path(),
        "src/checkout.rs",
        "use crate::pricing::calculate_total;\n\npub fn checkout_total() -> i64 {\n    // REQ-001\n    calculate_total()\n}\n",
    );
    write_file(
        temp.path(),
        "src/pricing.rs",
        "pub fn calculate_total() -> i64 {\n    42\n}\n",
    );
    write_file(temp.path(), "ignored.log", "ignore me\n");
    temp
}

#[test]
fn ingest_query_trace_impact_and_validate_round_trip() {
    let project = build_fixture_project();

    let (status, ingest_json) = run_cli(project.path(), &["ingest"]);
    assert!(status.success(), "ingest should succeed");
    assert_eq!(ingest_json["command"], "ingest");
    assert_eq!(ingest_json["data"]["files_indexed"], 5);
    assert_eq!(ingest_json["data"]["changed_files"], 5);

    let (status, query_json) = run_cli(project.path(), &["query", "--text", "validate cart"]);
    assert!(status.success(), "query should succeed");
    assert_eq!(query_json["data"]["total_matches"], 1);
    assert_eq!(query_json["data"]["results"][0]["path"], "docs/prd.md");
    assert_eq!(query_json["data"]["results"][0]["chunk_kind"], "section");
    assert!(query_json["data"]["results"][0]["chunk_id"].is_string());
    assert_eq!(query_json["data"]["results"][0]["start_line"], 1);

    let (status, trace_json) = run_cli(project.path(), &["trace", "--requirement", "REQ-001"]);
    assert!(status.success(), "trace should succeed");
    let edges = trace_json["data"]["edges"].as_array().expect("trace edges array");
    assert!(
        edges.iter().any(|edge| edge["target"]["id"] == "src/checkout.rs"),
        "REQ-001 should link to src/checkout.rs"
    );

    let (status, impact_json) = run_cli(project.path(), &["impact", "--node", "src/checkout.rs"]);
    assert!(status.success(), "impact should succeed");
    let impacted = impact_json["data"]["impacted_nodes"].as_array().expect("impacted nodes array");
    assert!(
        impacted.iter().any(|node| node["id"] == "REQ-001"),
        "src/checkout.rs should point back to REQ-001"
    );

    let (status, validate_json) = run_cli(project.path(), &["validate"]);
    assert_eq!(status.code(), Some(1), "validate should fail on findings");
    assert!(
        validate_json["data"]["summary"]["errors"]
            .as_u64()
            .expect("error count")
            >= 1
    );

    let findings = validate_json["data"]["findings"]
        .as_array()
        .expect("validation findings array");
    assert!(
        findings.iter().any(|finding| {
            finding["rule"] == "requirement_coverage"
                && finding["message"]
                    .as_str()
                    .expect("coverage message")
                    .contains("REQ-002")
        }),
        "validate should flag REQ-002 as uncovered"
    );
    assert!(
        findings.iter().any(|finding| {
            finding["rule"] == "broken_reference"
                && finding["message"]
                    .as_str()
                    .expect("broken reference message")
                    .contains("src/missing.rs")
        }),
        "validate should flag src/missing.rs as a broken reference"
    );
}

#[test]
fn repeated_ingest_reuses_snapshot_entries_and_refreshes_changed_files() {
    let project = build_fixture_project();

    let (status, first_ingest) = run_cli(project.path(), &["ingest"]);
    assert!(status.success(), "first ingest should succeed");
    assert_eq!(first_ingest["data"]["changed_files"], 5);

    let (status, second_ingest) = run_cli(project.path(), &["ingest"]);
    assert!(status.success(), "second ingest should succeed");
    assert_eq!(second_ingest["data"]["changed_files"], 0);
    assert_eq!(second_ingest["data"]["reused_files"], 5);

    write_file(
        project.path(),
        "src/checkout.rs",
        "pub fn checkout_total() -> i64 {\n    // REQ-001\n    // updated logic\n    84\n}\n",
    );

    let (status, third_ingest) = run_cli(project.path(), &["ingest"]);
    assert!(status.success(), "third ingest should succeed");
    assert_eq!(third_ingest["data"]["changed_files"], 1);
    assert_eq!(third_ingest["data"]["reused_files"], 4);
}

#[test]
fn ingest_creates_sqlite_store_with_file_rows() {
    let project = build_fixture_project();

    let (status, ingest_json) = run_cli(project.path(), &["ingest"]);
    assert!(status.success(), "ingest should succeed");
    assert_eq!(ingest_json["data"]["files_indexed"], 5);

    let database = project
        .path()
        .join(".project-memory")
        .join("project-memory.db");
    assert!(database.is_file(), "ingest should create a SQLite mirror store");

    let connection = Connection::open(&database).expect("open SQLite store");
    let file_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM files", [], |row| row.get(0))
        .expect("count files");
    let requirement_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM file_requirements", [], |row| row.get(0))
        .expect("count requirements");
    let chunk_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM chunks", [], |row| row.get(0))
        .expect("count chunks");
    let embedding_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM chunk_embeddings", [], |row| row.get(0))
        .expect("count chunk embeddings");

    assert_eq!(file_count, 5, "SQLite store should mirror indexed files");
    assert!(requirement_count >= 3, "SQLite store should mirror requirement links");
    assert!(chunk_count >= file_count, "SQLite store should persist chunk rows for retrieval");
    assert_eq!(embedding_count, chunk_count, "SQLite store should persist one embedding per chunk");
}

#[test]
fn ingest_and_retrieve_support_local_microservice_provider_from_config() {
    let project = build_fixture_project();
    let (endpoint, server) = start_embedding_stub(2);
    write_file(
        project.path(),
        ".project-memory/config.toml",
        &format!(
            "[embedding]\nprovider = \"local_microservice\"\nendpoint = \"{endpoint}\"\ntimeout_ms = 1500\n"
        ),
    );

    let (status, ingest_json) = run_cli(project.path(), &["ingest"]);
    assert!(status.success(), "ingest should succeed with local microservice provider");
    assert_eq!(ingest_json["data"]["embedding_provider"], "local_microservice");

    let connection = Connection::open(project.path().join(".project-memory").join("project-memory.db"))
        .expect("open SQLite store");
    let persisted_provider: String = connection
        .query_row(
            "SELECT value FROM snapshot_metadata WHERE key = 'embedding_provider'",
            [],
            |row| row.get(0),
        )
        .expect("read embedding provider metadata");
    assert_eq!(persisted_provider, "local_microservice");

    let (status, retrieve_json) = run_cli(project.path(), &["retrieve", "--text", "incident triage"]);
    assert!(status.success(), "retrieve should succeed with local microservice provider");
    assert_eq!(retrieve_json["data"]["embedding_provider"], "local_microservice");
    assert!(retrieve_json["data"]["results"][0]["semantic_score"].is_number());

    server.join().expect("join embedding stub");
}

#[test]
fn cli_embedding_flags_override_configured_provider() {
    let project = build_fixture_project();
    let (endpoint, server) = start_embedding_stub(1);
    write_file(
        project.path(),
        ".project-memory/config.toml",
        "[embedding]\nprovider = \"local_hashed_v1\"\n",
    );

    let (status, ingest_json) = run_cli_with_global_args(
        project.path(),
        &[
            "--embedding-provider",
            "local_microservice",
            "--embedding-endpoint",
            endpoint.as_str(),
        ],
        &["ingest"],
    );
    assert!(status.success(), "ingest should honor CLI provider override");
    assert_eq!(ingest_json["data"]["embedding_provider"], "local_microservice");

    server.join().expect("join override embedding stub");
}

#[test]
fn retrieve_recomputes_embeddings_when_provider_changes() {
    let project = build_fixture_project();
    let (endpoint, server) = start_embedding_stub(1);
    write_file(
        project.path(),
        ".project-memory/config.toml",
        &format!(
            "[embedding]\nprovider = \"local_microservice\"\nendpoint = \"{endpoint}\"\ntimeout_ms = 1500\n"
        ),
    );

    let (status, ingest_json) = run_cli(project.path(), &["ingest"]);
    assert!(status.success(), "ingest should succeed with local microservice provider");
    assert_eq!(ingest_json["data"]["embedding_provider"], "local_microservice");
    server.join().expect("join initial embedding stub");

    write_file(
        project.path(),
        ".project-memory/config.toml",
        "[embedding]\nprovider = \"local_hashed_v1\"\n",
    );

    let (status, retrieve_json) = run_cli(project.path(), &["retrieve", "--text", "incident triage"]);
    assert!(status.success(), "retrieve should succeed after provider change");
    assert_eq!(retrieve_json["data"]["configured_embedding_provider"], "local_hashed_v1");
    assert_eq!(retrieve_json["data"]["embedding_provider"], "local_hashed_v1");
    assert_eq!(retrieve_json["data"]["cache_status"], "mismatch_recomputed");

    let (status, second_retrieve_json) = run_cli(project.path(), &["retrieve", "--text", "incident triage"]);
    assert!(status.success(), "second retrieve should succeed after refreshing the cache");
    assert_eq!(second_retrieve_json["data"]["cache_status"], "hit");
}

#[test]
fn ingest_falls_back_to_local_hashed_when_microservice_fails() {
    let project = build_fixture_project();
    write_file(
        project.path(),
        ".project-memory/config.toml",
        "[embedding]\nprovider = \"local_microservice\"\nendpoint = \"http://127.0.0.1:9/v1/embeddings\"\nfallback_provider = \"local_hashed_v1\"\n",
    );

    let (status, ingest_json) = run_cli(project.path(), &["ingest"]);
    assert!(status.success(), "ingest should fall back to local_hashed_v1");
    assert_eq!(ingest_json["data"]["embedding_provider"], "local_hashed_v1");
    assert!(ingest_json["warnings"]
        .as_array()
        .expect("warnings array")
        .iter()
        .any(|warning| warning.as_str().expect("warning text").contains("fell back")));
}

#[test]
fn retrieve_reports_fallback_diagnostics_for_remote_failure() {
    let project = build_fixture_project();
    write_file(
        project.path(),
        ".project-memory/config.toml",
        "[embedding]\nprovider = \"local_hashed_v1\"\n",
    );
    let (status, _ingest_json) = run_cli(project.path(), &["ingest"]);
    assert!(status.success(), "seed ingest should succeed");

    write_file(
        project.path(),
        ".project-memory/config.toml",
        "[embedding]\nprovider = \"openai_compatible\"\nbase_url = \"http://127.0.0.1:9/v1\"\nmodel = \"text-embedding-3-small\"\napi_key_env = \"PMEM_TEST_OPENAI_KEY\"\nremote_enabled = true\nmax_requests_per_run = 2\nfallback_provider = \"local_hashed_v1\"\n",
    );

    let envs = [("PMEM_TEST_OPENAI_KEY", "generic-secret")];
    let (status, retrieve_json) = run_cli_with_env(project.path(), &[], &["retrieve", "--text", "incident triage"], &envs);
    assert!(status.success(), "retrieve should succeed by falling back to local_hashed_v1");
    assert_eq!(retrieve_json["data"]["configured_embedding_provider"], "openai_compatible");
    assert_eq!(retrieve_json["data"]["embedding_provider"], "local_hashed_v1");
    assert_eq!(retrieve_json["data"]["fallback_used"], true);
    assert_eq!(retrieve_json["data"]["remote_access"], false);
    assert_eq!(retrieve_json["data"]["cost_risk"], "none");
    assert!(retrieve_json["data"]["fallback_reason"]
        .as_str()
        .expect("fallback reason")
        .contains("requesting embeddings"));
}

#[test]
fn openai_compatible_provider_supports_generic_openai_shape() {
    let project = build_fixture_project();
    let (base_url, server) = start_openai_embedding_stub(2, &["generic", "generic"]);
    write_file(
        project.path(),
        ".project-memory/config.toml",
        &format!(
            "[embedding]\nprovider = \"openai_compatible\"\nbase_url = \"{base_url}/v1\"\nmodel = \"text-embedding-3-small\"\napi_key_env = \"PMEM_TEST_OPENAI_KEY\"\nremote_enabled = true\nmax_requests_per_run = 1\n"
        ),
    );

    let envs = [("PMEM_TEST_OPENAI_KEY", "generic-secret")];
    let (status, ingest_json) = run_cli_with_env(project.path(), &[], &["ingest"], &envs);
    assert!(status.success(), "ingest should succeed with openai_compatible provider");
    assert_eq!(ingest_json["data"]["embedding_provider"], "openai_compatible");
    assert_eq!(ingest_json["data"]["embedding_model"], "text-embedding-3-small");
    assert!(ingest_json["warnings"]
        .as_array()
        .expect("warnings array")
        .iter()
        .any(|warning| warning.as_str().expect("warning text").contains("external embedding provider")));

    let connection = Connection::open(project.path().join(".project-memory").join("project-memory.db"))
        .expect("open SQLite store");
    let persisted_model: String = connection
        .query_row(
            "SELECT value FROM snapshot_metadata WHERE key = 'embedding_model'",
            [],
            |row| row.get(0),
        )
        .expect("read embedding model metadata");
    assert_eq!(persisted_model, "text-embedding-3-small");

    let (status, retrieve_json) = run_cli_with_env(project.path(), &[], &["retrieve", "--text", "incident triage"], &envs);
    assert!(status.success(), "retrieve should succeed with openai_compatible provider");
    assert_eq!(retrieve_json["data"]["embedding_provider"], "openai_compatible");
    assert_eq!(retrieve_json["data"]["embedding_model"], "text-embedding-3-small");
    assert_eq!(retrieve_json["data"]["remote_access"], true);

    server.join().expect("join generic openai stub");
}

#[test]
fn openai_compatible_provider_supports_azure_shaping() {
    let project = build_fixture_project();
    let (base_url, server) = start_openai_embedding_stub(1, &["azure"]);
    write_file(
        project.path(),
        ".project-memory/config.toml",
        &format!(
            "[embedding]\nprovider = \"openai_compatible\"\nbase_url = \"{base_url}\"\ndeployment = \"team-embed\"\napi_version = \"2024-06-01-preview\"\napi_key_env = \"PMEM_TEST_AZURE_KEY\"\nmodel = \"text-embedding-3-large\"\nremote_enabled = true\nmax_requests_per_run = 1\n"
        ),
    );

    let envs = [("PMEM_TEST_AZURE_KEY", "azure-secret")];
    let (status, ingest_json) = run_cli_with_env(project.path(), &[], &["ingest"], &envs);
    assert!(status.success(), "ingest should succeed with azure-compatible openai provider");
    assert_eq!(ingest_json["data"]["embedding_provider"], "openai_compatible");
    assert_eq!(ingest_json["data"]["embedding_model"], "text-embedding-3-large");

    server.join().expect("join azure openai stub");
}

#[test]
fn openai_compatible_provider_requires_explicit_remote_enablement() {
    let project = build_fixture_project();
    write_file(
        project.path(),
        ".project-memory/config.toml",
        "[embedding]\nprovider = \"openai_compatible\"\nbase_url = \"https://example.invalid/v1\"\nmodel = \"text-embedding-3-small\"\napi_key_env = \"PMEM_TEST_OPENAI_KEY\"\n",
    );

    let (status, _stdout, stderr) = run_cli_raw_with_env(
        project.path(),
        &[],
        &["ingest"],
        &[("PMEM_TEST_OPENAI_KEY", "secret")],
    );
    assert!(!status.success(), "ingest should fail when remote provider is not explicitly enabled");
    assert!(stderr.contains("explicit remote enablement"));
}

#[test]
fn openai_compatible_provider_rejects_secrets_in_config() {
    let project = build_fixture_project();
    write_file(
        project.path(),
        ".project-memory/config.toml",
        "[embedding]\nprovider = \"openai_compatible\"\nbase_url = \"https://example.invalid/v1\"\nmodel = \"text-embedding-3-small\"\nremote_enabled = true\napi_key = \"hardcoded-secret\"\n",
    );

    let (status, _stdout, stderr) = run_cli_raw_with_env(project.path(), &[], &["ingest"], &[]);
    assert!(!status.success(), "ingest should fail when a secret is stored directly in config");
    assert!(stderr.contains("must not store embedding secrets directly"));
}

#[test]
fn ingest_persists_deterministic_chunks_in_snapshot_and_sqlite() {
    let project = tempdir().expect("create temp dir");
    write_file(project.path(), ".gitignore", ".project-memory/\n");
    write_file(
        project.path(),
        "docs/architecture.md",
        "# Architecture\n\n## Overview\n\nThis section explains the overall architecture.\n\n## Storage\n\nSQLite stores files and chunks.\n\n## Retrieval\n\nHybrid retrieval combines FTS and vectors.\n",
    );
    write_file(
        project.path(),
        "src/module.rs",
        &(1..=90)
            .map(|index| format!("pub fn handler_{index}() {{}}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );

    let (status, _ingest_json) = run_cli(project.path(), &["ingest"]);
    assert!(status.success(), "ingest should succeed");

    let snapshot = fs::read_to_string(project.path().join(".project-memory").join("snapshot.json"))
        .expect("read snapshot");
    let snapshot_json: Value = serde_json::from_str(&snapshot).expect("parse snapshot JSON");
    let files = snapshot_json["files"].as_array().expect("files array");
    let architecture = files
        .iter()
        .find(|file| file["path"] == "docs/architecture.md")
        .expect("architecture file present");
    let module = files
        .iter()
        .find(|file| file["path"] == "src/module.rs")
        .expect("module file present");

    assert!(architecture["chunks"].as_array().expect("architecture chunks").len() >= 3);
    assert!(module["chunks"].as_array().expect("module chunks").len() >= 3);

    let connection = Connection::open(project.path().join(".project-memory").join("project-memory.db"))
        .expect("open SQLite store");
    let markdown_chunks: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM chunks WHERE file_path = 'docs/architecture.md'",
            [],
            |row| row.get(0),
        )
        .expect("count markdown chunks");
    let code_chunks: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM chunks WHERE file_path = 'src/module.rs'",
            [],
            |row| row.get(0),
        )
        .expect("count code chunks");

    assert!(markdown_chunks >= 3, "markdown headings should produce section chunks");
    assert!(code_chunks >= 3, "long code files should produce window chunks");
}

#[test]
fn ingest_indexes_hidden_workflow_files_when_not_ignored() {
    let project = tempdir().expect("create temp dir");
    write_file(project.path(), ".gitignore", ".project-memory/\nignored.log\n");
    write_file(
        project.path(),
        ".github/workflows/build-skill-binaries.yml",
        "name: Build Scoped CLI Binaries\non: [push]\njobs:\n  validate:\n    runs-on: ubuntu-latest\n",
    );
    write_file(
        project.path(),
        "docs/guide.md",
        "# Guide\n\nSee .github/workflows/build-skill-binaries.yml.\n",
    );

    let (status, ingest_json) = run_cli(project.path(), &["ingest"]);
    assert!(status.success(), "ingest should succeed");
    assert_eq!(ingest_json["data"]["files_indexed"], 2);

    let snapshot = fs::read_to_string(project.path().join(".project-memory").join("snapshot.json"))
        .expect("read snapshot");
    let snapshot_json: Value = serde_json::from_str(&snapshot).expect("parse snapshot JSON");
    let files = snapshot_json["files"].as_array().expect("files array");
    assert!(
        files.iter().any(|file| file["path"] == ".github/workflows/build-skill-binaries.yml"),
        "hidden workflow file should be present in the snapshot"
    );

    let (status, query_json) = run_cli(project.path(), &["query", "--path-contains", ".github/workflows"]);
    assert!(status.success(), "query should succeed");
    assert_eq!(query_json["data"]["total_matches"], 1);
    assert_eq!(query_json["data"]["results"][0]["path"], ".github/workflows/build-skill-binaries.yml");
}

#[test]
fn ingest_skips_hidden_root_secrets_while_allowlisting_workflows() {
    let project = tempdir().expect("create temp dir");
    write_file(project.path(), ".gitignore", ".project-memory/\n");
    write_file(project.path(), ".env", "PMEM_OPENAI_API_KEY=secret\n");
    write_file(
        project.path(),
        ".github/workflows/build.yml",
        "name: Build\non: [push]\njobs:\n  test:\n    runs-on: ubuntu-latest\n",
    );
    write_file(project.path(), "docs/guide.md", "# Guide\n\nSee workflow.\n");

    let (status, ingest_json) = run_cli(project.path(), &["ingest"]);
    assert!(status.success(), "ingest should succeed");
    assert_eq!(ingest_json["data"]["files_indexed"], 2);

    let snapshot = fs::read_to_string(project.path().join(".project-memory").join("snapshot.json"))
        .expect("read snapshot");
    let snapshot_json: Value = serde_json::from_str(&snapshot).expect("parse snapshot JSON");
    let files = snapshot_json["files"].as_array().expect("files array");

    assert!(
        !files.iter().any(|file| file["path"] == ".env"),
        "hidden root secrets should stay out of the snapshot"
    );
    assert!(
        files.iter().any(|file| file["path"] == ".github/workflows/build.yml"),
        "workflow allowlist should still include hidden workflow files"
    );
}

#[test]
fn ingest_builds_structured_yaml_chunks_for_workflows() {
    let project = tempdir().expect("create temp dir");
    write_file(project.path(), ".gitignore", ".project-memory/\n");
    write_file(
        project.path(),
        ".github/workflows/build.yml",
        "name: Build Scoped CLI Binaries\n\non: [push]\n\njobs:\n  release-gate:\n    runs-on: ubuntu-latest\n    steps:\n      - name: Enforce executable bits\n        run: chmod +x collected/skill-dev-cli-linux-x64\n      - uses: actions/download-artifact@v4\n        with:\n          path: collected\n  publish:\n    needs: [release-gate]\n    steps:\n      - run: sha256sum skill-dev-cli-linux-x64 > checksums.sha256\n",
    );

    let (status, ingest_json) = run_cli(project.path(), &["ingest"]);
    assert!(status.success(), "ingest should succeed");
    assert_eq!(ingest_json["data"]["files_indexed"], 1);

    let snapshot = fs::read_to_string(project.path().join(".project-memory").join("snapshot.json"))
        .expect("read snapshot");
    let snapshot_json: Value = serde_json::from_str(&snapshot).expect("parse snapshot JSON");
    let workflow = snapshot_json["files"]
        .as_array()
        .expect("files array")
        .iter()
        .find(|file| file["path"] == ".github/workflows/build.yml")
        .expect("workflow present");
    let chunk_titles = workflow["chunks"]
        .as_array()
        .expect("workflow chunks")
        .iter()
        .filter_map(|chunk| chunk["title"].as_str())
        .collect::<Vec<_>>();

    assert!(
        chunk_titles.iter().any(|title| *title == "job release-gate > step Enforce executable bits"),
        "workflow chunks should capture named steps"
    );
    assert!(
        chunk_titles
            .iter()
            .any(|title| *title == "job release-gate > step uses actions/download-artifact@v4"),
        "workflow chunks should capture uses steps"
    );
    assert!(
        chunk_titles.iter().any(|title| *title == "job publish > step run sha256sum skill-dev-cli-linux-x64 > checksums.sha256"),
        "workflow chunks should capture run steps"
    );

    let (status, query_json) = run_cli(project.path(), &["query", "--text", "checksums.sha256", "--path-contains", ".github/workflows"]);
    assert!(status.success(), "query should succeed");
    assert_eq!(query_json["data"]["results"][0]["chunk_title"], "job publish > step run sha256sum skill-dev-cli-linux-x64 > checksums.sha256");
}

#[test]
fn watch_refreshes_snapshot_after_a_file_change() {
    let project = build_fixture_project();

    let (status, ingest_json) = run_cli(project.path(), &["ingest"]);
    assert!(status.success(), "seed ingest should succeed");
    assert_eq!(ingest_json["data"]["files_indexed"], 5);

    let project_root = project.path().to_path_buf();
    let child = Command::new(env!("CARGO_BIN_EXE_project-memory-cli"))
        .args([
            "--project-root",
            project_root.to_str().expect("project root utf-8"),
            "watch",
            "--interval-ms",
            "100",
            "--timeout-ms",
            "5000",
            "--max-events",
            "1",
        ])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("spawn watch process");

    thread::sleep(Duration::from_millis(1200));
    write_file(
        &project_root,
        "src/checkout.rs",
        "pub fn checkout_total() -> i64 {\n    // REQ-001\n    // watched update\n    126\n}\n",
    );

    let output = child.wait_with_output().expect("watch process output");
    assert!(output.status.success(), "watch should complete successfully");
    let stdout = String::from_utf8(output.stdout).expect("watch stdout utf-8");
    let json: Value = serde_json::from_str(&stdout).expect("watch JSON output");

    assert_eq!(json["command"], "watch");
    assert_eq!(json["data"]["events_observed"], 1);
    assert_eq!(json["data"]["timed_out"], false);
    let iteration = &json["data"]["iterations"][0];
    assert!(
        iteration["changed_paths"]
            .as_array()
            .expect("changed paths array")
            .iter()
            .any(|path| path == "src/checkout.rs"),
        "watch should report src/checkout.rs as changed"
    );
}

#[test]
fn watch_times_out_cleanly_when_no_relevant_changes_arrive() {
    let project = build_fixture_project();

    let output = Command::new(env!("CARGO_BIN_EXE_project-memory-cli"))
        .args([
            "--project-root",
            project.path().to_str().expect("project root utf-8"),
            "watch",
            "--interval-ms",
            "100",
            "--timeout-ms",
            "300",
            "--max-events",
            "1",
        ])
        .output()
        .expect("run watch timeout case");

    assert!(output.status.success(), "watch timeout should still be successful");
    let stdout = String::from_utf8(output.stdout).expect("watch timeout stdout utf-8");
    let json: Value = serde_json::from_str(&stdout).expect("watch timeout JSON output");
    assert_eq!(json["command"], "watch");
    assert_eq!(json["data"]["timed_out"], true);
    assert_eq!(json["data"]["events_observed"], 0);
}

#[test]
fn query_can_use_rust_symbol_and_import_metadata() {
    let project = build_fixture_project();

    let (status, ingest_json) = run_cli(project.path(), &["ingest"]);
    assert!(status.success(), "ingest should succeed");
    assert_eq!(ingest_json["data"]["files_indexed"], 5);

    let (status, symbol_json) = run_cli(project.path(), &["query", "--symbol", "calculate_total"]);
    assert!(status.success(), "symbol query should succeed");
    assert!(
        symbol_json["data"]["results"]
            .as_array()
            .expect("symbol results")
            .iter()
            .any(|result| result["path"] == "src/pricing.rs"),
        "symbol query should find src/pricing.rs"
    );

    let (status, import_json) = run_cli(project.path(), &["query", "--import", "crate::pricing"]);
    assert!(status.success(), "import query should succeed");
    assert!(
        import_json["data"]["results"]
            .as_array()
            .expect("import results")
            .iter()
            .any(|result| result["path"] == "src/checkout.rs"),
        "import query should find src/checkout.rs"
    );
}

#[test]
fn query_text_returns_chunk_level_provenance_for_windowed_code() {
    let project = tempdir().expect("create temp dir");
    write_file(project.path(), ".gitignore", ".project-memory/\n");
    write_file(
        project.path(),
        "src/large.rs",
        &(1..=95)
            .map(|index| {
                if index == 67 {
                    "pub fn semantic_target() {\n    let marker = \"semantic retrieval anchor\";\n}".to_string()
                } else {
                    format!("pub fn handler_{index}() {{}}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
    );

    let (status, ingest_json) = run_cli(project.path(), &["ingest"]);
    assert!(status.success(), "ingest should succeed");
    assert_eq!(ingest_json["data"]["files_indexed"], 1);

    let (status, query_json) = run_cli(project.path(), &["query", "--text", "semantic retrieval anchor"]);
    assert!(status.success(), "query should succeed");
    assert_eq!(query_json["data"]["total_matches"], 1);
    assert_eq!(query_json["data"]["results"][0]["path"], "src/large.rs");
    assert_eq!(query_json["data"]["results"][0]["chunk_kind"], "window");
    assert!(query_json["data"]["results"][0]["chunk_id"]
        .as_str()
        .expect("chunk id")
        .contains("#chunk-"));
    assert_eq!(query_json["data"]["results"][0]["line_number"], 68);
    assert_eq!(query_json["data"]["results"][0]["start_line"], 41);
    assert_eq!(query_json["data"]["results"][0]["end_line"], 80);
}

#[test]
fn retrieve_returns_ranked_chunk_results_for_lexical_recall() {
    let project = tempdir().expect("create temp dir");
    write_file(project.path(), ".gitignore", ".project-memory/\n");
    write_file(
        project.path(),
        "docs/ops.md",
        "# Ops\n\n## Alerting\n\nThe on-call workflow uses semantic chunk recall for incident triage.\n\n## Follow-up\n\nPager escalation stays documented here.\n",
    );

    let (status, _ingest_json) = run_cli(project.path(), &["ingest"]);
    assert!(status.success(), "ingest should succeed");

    let (status, retrieve_json) = run_cli(
        project.path(),
        &["retrieve", "--text", "semantic chunk recall", "--limit", "3"],
    );
    assert!(status.success(), "retrieve should succeed");
    assert_eq!(retrieve_json["command"], "retrieve");
    assert_eq!(retrieve_json["data"]["retrieval_mode"], "hybrid_lexical_embedding");
    assert_eq!(retrieve_json["data"]["embedding_provider"], "local_hashed_v1");
    assert_eq!(retrieve_json["data"]["total_matches"], 1);
    assert_eq!(retrieve_json["data"]["results"][0]["path"], "docs/ops.md");
    assert_eq!(retrieve_json["data"]["results"][0]["chunk_kind"], "section");
    assert!(retrieve_json["data"]["results"][0]["chunk_id"].is_string());
    assert!(retrieve_json["data"]["results"][0]["semantic_score"].as_f64().is_some());
}

#[test]
fn retrieve_can_recall_chunk_without_exact_phrase_match() {
    let project = tempdir().expect("create temp dir");
    write_file(project.path(), ".gitignore", ".project-memory/\n");
    write_file(
        project.path(),
        "docs/incidents.md",
        "# Incidents\n\n## Triage\n\nThe workflow for incidents uses queue review and team escalation.\n",
    );

    let (status, _ingest_json) = run_cli(project.path(), &["ingest"]);
    assert!(status.success(), "ingest should succeed");

    let (status, retrieve_json) = run_cli(
        project.path(),
        &["retrieve", "--text", "incident workflow escalation", "--limit", "5"],
    );
    assert!(status.success(), "retrieve should succeed");
    assert_eq!(retrieve_json["data"]["results"][0]["path"], "docs/incidents.md");
    assert_eq!(retrieve_json["data"]["results"][0]["lexical_score"], 0.0);
    assert!(retrieve_json["data"]["results"][0]["semantic_score"]
        .as_f64()
        .expect("semantic score")
        > 0.18);
}

#[test]
fn validate_ignores_fenced_examples_and_rust_string_literals() {
    let project = tempdir().expect("create temp dir");
    write_file(project.path(), ".gitignore", ".project-memory/\n");
    write_file(
        project.path(),
        "docs/reference.md",
        "# Reference\n\nREQ-100 See `src/real.rs` and `docs/guide.md`.\n\n```bash\nproject-memory-cli --project-root <project-root> trace --path src/example.rs\n```\n",
    );
    write_file(project.path(), "src/real.rs", "// REQ-100\npub fn real() {}\n");
    write_file(project.path(), "docs/guide.md", "# Guide\n");
    write_file(
        project.path(),
        "tests/fixture.rs",
        "const SAMPLE: &str = \"REQ-999 uses src/ghost.rs\";\n",
    );

    let (status, ingest_json) = run_cli(project.path(), &["ingest"]);
    assert!(status.success(), "ingest should succeed");
    assert!(ingest_json["data"]["files_indexed"].as_u64().expect("files indexed") >= 3);

    let (status, validate_json) = run_cli(project.path(), &["validate"]);
    assert!(status.success(), "validate should succeed without false positives");
    assert_eq!(validate_json["data"]["summary"]["errors"], 0);

    let (status, req_trace) = run_cli(project.path(), &["trace", "--requirement", "REQ-999"]);
    assert!(status.success(), "trace should succeed");
    assert_eq!(req_trace["data"]["edge_count"], 0);
}

#[test]
fn trace_supports_pmem_ids_and_root_relative_path_fallback() {
    let project = tempdir().expect("create temp dir");
    write_file(project.path(), ".gitignore", ".project-memory/\n");
    write_file(
        project.path(),
        "docs/backlog.md",
        "# Backlog\n\nPMEM-001 lives in `docs/issues/PMEM-001.md` and is implemented by `src/feature.rs`.\n",
    );
    write_file(project.path(), "docs/issues/PMEM-001.md", "# PMEM-001\n");
    write_file(project.path(), "src/feature.rs", "// PMEM-001\npub fn shipped() {}\n");

    let (status, ingest_json) = run_cli(project.path(), &["ingest"]);
    assert!(status.success(), "ingest should succeed");
    assert!(ingest_json["data"]["files_indexed"].as_u64().expect("files indexed") >= 3);

    let (status, trace_json) = run_cli(project.path(), &["trace", "--requirement", "PMEM-001"]);
    assert!(status.success(), "trace should succeed");
    let edges = trace_json["data"]["edges"].as_array().expect("trace edges");
    assert!(
        edges.iter().any(|edge| edge["target"]["id"] == "docs/issues/PMEM-001.md"),
        "PMEM-001 should resolve the root-relative backlog path"
    );
    assert!(
        edges.iter().any(|edge| edge["target"]["id"] == "src/feature.rs"),
        "PMEM-001 should link to the Rust comment coverage artifact"
    );
}
