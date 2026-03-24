use serde_json::Value;
use std::fs;
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
    let output = Command::new(env!("CARGO_BIN_EXE_project-memory-cli"))
        .args(["--project-root", project_root.to_str().expect("project root utf-8")])
        .args(args)
        .output()
        .expect("run project-memory-cli");

    let stdout = String::from_utf8(output.stdout).expect("stdout utf-8");
    let json = serde_json::from_str::<Value>(&stdout).expect("valid JSON output");
    (output.status, json)
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
