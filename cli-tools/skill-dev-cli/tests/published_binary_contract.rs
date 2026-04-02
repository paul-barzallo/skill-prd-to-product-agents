use std::path::{Path, PathBuf};
use std::process::Command;

use serde_yaml::{Mapping, Value};

const ALL_PUBLISHED_BINARIES: &[&str] = &[
    ".agents/skills/prd-to-product-agents/bin/prd-to-product-agents-cli-linux-x64",
    ".agents/skills/prd-to-product-agents/bin/prd-to-product-agents-cli-darwin-arm64",
    ".agents/skills/prd-to-product-agents/bin/prd-to-product-agents-cli-windows-x64.exe",
    ".agents/skills/prd-to-product-agents/templates/workspace/.agents/bin/prd-to-product-agents/prdtp-agents-functions-cli-linux-x64",
    ".agents/skills/prd-to-product-agents/templates/workspace/.agents/bin/prd-to-product-agents/prdtp-agents-functions-cli-darwin-arm64",
    ".agents/skills/prd-to-product-agents/templates/workspace/.agents/bin/prd-to-product-agents/prdtp-agents-functions-cli-windows-x64.exe",
    "bin/skill-dev-cli-linux-x64",
    "bin/skill-dev-cli-darwin-arm64",
    "bin/skill-dev-cli-windows-x64.exe",
];

const UNIX_PUBLISHED_BINARIES: &[&str] = &[
    ".agents/skills/prd-to-product-agents/bin/prd-to-product-agents-cli-linux-x64",
    ".agents/skills/prd-to-product-agents/bin/prd-to-product-agents-cli-darwin-arm64",
    ".agents/skills/prd-to-product-agents/templates/workspace/.agents/bin/prd-to-product-agents/prdtp-agents-functions-cli-linux-x64",
    ".agents/skills/prd-to-product-agents/templates/workspace/.agents/bin/prd-to-product-agents/prdtp-agents-functions-cli-darwin-arm64",
];

fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(Path::parent)
        .expect("could not resolve repo root from CARGO_MANIFEST_DIR")
        .to_path_buf()
}

fn read_repo_file(relative_path: &str) -> String {
    std::fs::read_to_string(repo_root().join(relative_path))
        .unwrap_or_else(|error| panic!("failed to read {relative_path}: {error}"))
}

fn read_workflow_yaml(relative_path: &str) -> Value {
    serde_yaml::from_str(&read_repo_file(relative_path))
        .unwrap_or_else(|error| panic!("failed to parse {relative_path} as YAML: {error}"))
}

fn read_checksum_entries(relative_path: &str) -> Vec<String> {
    read_repo_file(relative_path)
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return None;
            }
            trimmed.split_whitespace().nth(1).map(str::to_owned)
        })
        .collect()
}

fn mapping_get<'a>(mapping: &'a Mapping, key: &str) -> &'a Value {
    mapping
        .get(Value::String(key.to_owned()))
        .unwrap_or_else(|| panic!("missing key '{key}'"))
}

fn as_mapping<'a>(value: &'a Value, context: &str) -> &'a Mapping {
    value
        .as_mapping()
        .unwrap_or_else(|| panic!("expected {context} to be a YAML mapping"))
}

fn as_sequence<'a>(value: &'a Value, context: &str) -> &'a Vec<Value> {
    value
        .as_sequence()
        .unwrap_or_else(|| panic!("expected {context} to be a YAML sequence"))
}

fn string_entries(sequence: &[Value], context: &str) -> Vec<String> {
    sequence
        .iter()
        .map(|value| {
            value
                .as_str()
                .unwrap_or_else(|| panic!("expected entries in {context} to be strings"))
                .to_owned()
        })
        .collect()
}

fn workflow_job<'a>(workflow: &'a Value, job_name: &str) -> &'a Mapping {
    let workflow_mapping = as_mapping(workflow, "workflow");
    let jobs = as_mapping(mapping_get(workflow_mapping, "jobs"), "jobs");
    as_mapping(mapping_get(jobs, job_name), job_name)
}

fn matrix_os_entries(job: &Mapping) -> Vec<String> {
    let strategy = as_mapping(mapping_get(job, "strategy"), "strategy");
    let matrix = as_mapping(mapping_get(strategy, "matrix"), "matrix");
    let include = as_sequence(mapping_get(matrix, "include"), "matrix.include");

    include
        .iter()
        .map(|entry| {
            let entry_mapping = as_mapping(entry, "matrix.include entry");
            mapping_get(entry_mapping, "os")
                .as_str()
                .unwrap_or_else(|| panic!("expected matrix.include.os to be a string"))
                .to_owned()
        })
        .collect()
}

#[test]
fn unix_published_binaries_are_tracked_as_executable() {
    let output = Command::new("git")
        .current_dir(repo_root())
        .args(["ls-files", "--stage", "--"])
        .args(UNIX_PUBLISHED_BINARIES)
        .output()
        .expect("failed to inspect git index for published binaries");

    assert!(
        output.status.success(),
        "git ls-files --stage failed:\nSTDOUT:\n{}\nSTDERR:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut missing = Vec::new();

    for path in UNIX_PUBLISHED_BINARIES {
        let line = stdout.lines().find(|line| line.ends_with(path));
        match line {
            Some(line) if line.starts_with("100755 ") => {}
            Some(line) => missing.push(format!("{path}: expected mode 100755, found '{line}'")),
            None => missing.push(format!("{path}: not tracked in git index")),
        }
    }

    assert!(
        missing.is_empty(),
        "Published Unix binaries must be executable in git index:\n  {}",
        missing.join("\n  ")
    );
}

#[test]
fn published_binaries_are_tracked_as_binary() {
    let output = Command::new("git")
        .current_dir(repo_root())
        .args(["ls-files", "--eol", "--"])
        .args(ALL_PUBLISHED_BINARIES)
        .output()
        .expect("failed to inspect git attributes for published binaries");

    assert!(
        output.status.success(),
        "git ls-files --eol failed:\nSTDOUT:\n{}\nSTDERR:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut wrong = Vec::new();

    for path in ALL_PUBLISHED_BINARIES {
        let line = stdout.lines().find(|line| line.ends_with(path));
        match line {
            Some(line) if line.contains("attr/-text") => {}
            Some(line) => wrong.push(format!("{path}: expected attr/-text, found '{line}'")),
            None => wrong.push(format!("{path}: not tracked in git index")),
        }
    }

    assert!(
        wrong.is_empty(),
        "Published binaries must be tracked as binary with -text attributes:\n  {}",
        wrong.join("\n  ")
    );
}

#[test]
fn unix_release_gate_workflow_sets_execute_bits_for_collected_binaries() {
    let workflow = repo_root()
        .join(".github")
        .join("workflows")
        .join("build-skill-binaries.yml");
    let content = std::fs::read_to_string(&workflow)
        .expect("failed to read build-skill-binaries workflow");

    let expected_entries = [
        "collected/skill-dev-cli-${{ matrix.suffix }}",
        "collected/prd-to-product-agents-cli-${{ matrix.suffix }}",
        "collected/prdtp-agents-functions-cli-${{ matrix.suffix }}",
    ];

    let mut missing = Vec::new();
    for entry in expected_entries {
        if !content.contains(entry) {
            missing.push(entry);
        }
    }

    assert!(
        missing.is_empty(),
        "Unix release-gate workflow must chmod all collected binaries before execution:\n  {}",
        missing.join("\n  ")
    );
}

#[test]
fn build_workflow_tracks_multi_os_relevant_paths() {
    let workflow = read_workflow_yaml(".github/workflows/build-skill-binaries.yml");
    let workflow_mapping = as_mapping(&workflow, "workflow");
    let triggers = as_mapping(mapping_get(workflow_mapping, "on"), "on");
    let push = as_mapping(mapping_get(triggers, "push"), "push");
    let pull_request = as_mapping(mapping_get(triggers, "pull_request"), "pull_request");

    let push_paths = string_entries(as_sequence(mapping_get(push, "paths"), "push.paths"), "push.paths");
    let pull_request_paths = string_entries(
        as_sequence(mapping_get(pull_request, "paths"), "pull_request.paths"),
        "pull_request.paths",
    );

    let expected_entries = [
        ".agents/skills/prd-to-product-agents/**",
        "cli-tools/**",
        "bin/**",
        ".github/workflows/**",
    ];

    let mut missing = Vec::new();
    for entry in expected_entries {
        if !push_paths.iter().any(|path| path == entry) {
            missing.push(format!("push.paths missing '{entry}'"));
        }
        if !pull_request_paths.iter().any(|path| path == entry) {
            missing.push(format!("pull_request.paths missing '{entry}'"));
        }
    }

    assert!(
        missing.is_empty(),
        "Build workflow must track all multi-OS-relevant paths:\n  {}",
        missing.join("\n  ")
    );
}

#[test]
fn repo_validation_workflow_includes_project_memory_cli_and_skill_source_paths() {
    let workflow = read_workflow_yaml(".github/workflows/repo-validation.yml");
    let workflow_mapping = as_mapping(&workflow, "workflow");
    let triggers = as_mapping(mapping_get(workflow_mapping, "on"), "on");
    let push = as_mapping(mapping_get(triggers, "push"), "push");
    let pull_request = as_mapping(mapping_get(triggers, "pull_request"), "pull_request");

    let push_paths = string_entries(as_sequence(mapping_get(push, "paths"), "push.paths"), "push.paths");
    let pull_request_paths = string_entries(
        as_sequence(mapping_get(pull_request, "paths"), "pull_request.paths"),
        "pull_request.paths",
    );

    for expected in [".agents/skills/prd-to-product-agents/**", "cli-tools/**", "docs/**"] {
        assert!(
            push_paths.iter().any(|path| path == expected),
            "push.paths missing '{expected}'"
        );
        assert!(
            pull_request_paths.iter().any(|path| path == expected),
            "pull_request.paths missing '{expected}'"
        );
    }

    let validate = workflow_job(&workflow, "validate");
    let steps = as_sequence(mapping_get(validate, "steps"), "validate.steps");
    let mut found = false;

    for step in steps {
        let step = as_mapping(step, "step");
        if let Some(run) = step.get(Value::String("run".to_string())) {
            if let Some(command) = run.as_str() {
                if command.contains("cli-tools/project-memory-cli/Cargo.toml") {
                    found = true;
                    break;
                }
            }
        }
    }

    assert!(
        found,
        "repo-validation workflow must run project-memory-cli tests when repository-side tooling changes"
    );
}

#[test]
fn build_workflow_release_gate_runs_before_merge() {
    let workflow_content = read_repo_file(".github/workflows/build-skill-binaries.yml");
    let workflow = read_workflow_yaml(".github/workflows/build-skill-binaries.yml");
    let release_gate = workflow_job(&workflow, "release-gate");
    let publish = workflow_job(&workflow, "publish");

    let release_gate_if = release_gate
        .get(Value::String("if".to_string()))
        .and_then(Value::as_str)
        .unwrap_or("");
    let publish_if = mapping_get(publish, "if")
        .as_str()
        .expect("expected publish.if to be a string");

    assert!(
        !release_gate_if.contains("github.ref == 'refs/heads/main' && github.event_name != 'pull_request'"),
        "Release gate must not be restricted to post-merge execution only"
    );
    assert!(
        workflow_content.contains("peter-evans/create-pull-request@v7"),
        "Publish flow must create a reviewable PR instead of mutating main directly"
    );
    assert!(
        publish_if == "github.ref == 'refs/heads/main' && github.event_name != 'pull_request' && github.actor != 'github-actions[bot]'",
        "Publish job must remain restricted to push on main"
    );
    assert!(
        !workflow_content.contains("git push"),
        "Build workflow must not push tracked binaries directly to main"
    );
}

#[test]
fn build_workflow_uses_least_privilege_until_binary_refresh_step() {
    let workflow = read_workflow_yaml(".github/workflows/build-skill-binaries.yml");
    let workflow_mapping = as_mapping(&workflow, "workflow");
    let workflow_permissions = as_mapping(mapping_get(workflow_mapping, "permissions"), "permissions");
    let publish = workflow_job(&workflow, "publish");
    let publish_permissions = as_mapping(mapping_get(publish, "permissions"), "publish.permissions");

    assert_eq!(
        mapping_get(workflow_permissions, "contents").as_str(),
        Some("read"),
        "workflow-wide permissions should default to contents: read"
    );
    assert_eq!(
        mapping_get(publish_permissions, "contents").as_str(),
        Some("write"),
        "publish job needs scoped contents: write to open the binary refresh PR"
    );
    assert_eq!(
        mapping_get(publish_permissions, "pull-requests").as_str(),
        Some("write"),
        "publish job needs pull-requests: write to open the binary refresh PR"
    );
}

#[test]
fn build_workflow_keeps_windows_linux_and_macos_matrix_entries() {
    let workflow = read_workflow_yaml(".github/workflows/build-skill-binaries.yml");
    let build = workflow_job(&workflow, "build");
    let test = workflow_job(&workflow, "test");
    let release_gate = workflow_job(&workflow, "release-gate");

    let build_entries = matrix_os_entries(build);
    let test_entries = matrix_os_entries(test);
    let release_gate_entries = matrix_os_entries(release_gate);

    let expected_entries = ["ubuntu-latest", "macos-latest", "windows-latest"];

    let mut missing = Vec::new();
    for entry in expected_entries {
        if !build_entries.iter().any(|value| value == entry) {
            missing.push(format!("build matrix missing '{entry}'"));
        }
        if !test_entries.iter().any(|value| value == entry) {
            missing.push(format!("test matrix missing '{entry}'"));
        }
        if !release_gate_entries.iter().any(|value| value == entry) {
            missing.push(format!("release-gate matrix missing '{entry}'"));
        }
    }

    assert!(
        missing.is_empty(),
        "Build workflow must keep Linux, macOS, and Windows matrix entries:\n  {}",
        missing.join("\n  ")
    );
}

#[test]
fn build_workflow_publish_refreshes_checksum_manifests() {
    let workflow = repo_root()
        .join(".github")
        .join("workflows")
        .join("build-skill-binaries.yml");
    let content = std::fs::read_to_string(&workflow)
        .expect("failed to read build-skill-binaries workflow");

    let expected_entries = [
        "python .github/scripts/generate_bundle_metadata.py --bundle-dir \"${ROOT_BIN_DIR}\"",
        "python .github/scripts/generate_bundle_metadata.py --bundle-dir \"${SKILL_BIN_DIR}\"",
        "python .github/scripts/generate_bundle_metadata.py --bundle-dir \"${WORKSPACE_RUNTIME_BIN_DIR}\"",
    ];

    let mut missing = Vec::new();
    for entry in expected_entries {
        if !content.contains(entry) {
            missing.push(entry);
        }
    }

    assert!(
        missing.is_empty(),
        "Build workflow publish step must refresh bundle metadata through the canonical generator:\n  {}",
        missing.join("\n  ")
    );

    assert!(
        repo_root()
            .join(".github")
            .join("scripts")
            .join("generate_bundle_metadata.py")
            .is_file(),
        "canonical bundle metadata generator must exist at .github/scripts/generate_bundle_metadata.py"
    );
}

#[test]
fn build_workflow_publishes_runtime_with_published_skill_contract_feature() {
    let content = read_repo_file(".github/workflows/build-skill-binaries.yml");
    assert!(
        content.contains(
            "cargo build --release --target ${{ matrix.target }} --manifest-path cli-tools/prdtp-agents-functions-cli/Cargo.toml --features published-skill-contract"
        ),
        "build workflow must compile the published runtime CLI with the published-skill-contract feature"
    );
}

#[test]
fn build_workflow_package_acceptance_checks_hidden_runtime_commands() {
    let content = read_repo_file(".github/workflows/build-skill-binaries.yml");
    for expected in [
        "governance --help | grep -qE '^[[:space:]]+promote-enterprise-readiness([[:space:]]|$)'",
        "audit --help | grep -qE '^[[:space:]]+export([[:space:]]|$)'",
        "--help | grep -qE '^[[:space:]]+github([[:space:]]|$)'",
    ] {
        assert!(
            content.contains(expected),
            "package contract acceptance must check hidden runtime command drift: missing '{expected}'"
        );
    }
}

#[test]
fn enterprise_sandbox_workflow_uses_isolated_packaged_skill_candidate() {
    let content = read_repo_file(".github/workflows/enterprise-readiness-sandbox.yml");

    for expected in [
        "cp -R .agents/skills/prd-to-product-agents \"$skill_root\"",
        "package-validate.txt",
        "bootstrap-report.md",
        "bootstrap-manifest.txt",
        "packaged-skill-paths.txt",
        "workflow-context.txt",
        "${{ steps.skill.outputs.bootstrap_cli }}",
        "${{ steps.skill.outputs.runtime_cli }}",
        "governance promote-enterprise-readiness",
    ] {
        assert!(
            content.contains(expected),
            "enterprise sandbox workflow must prove the packaged skill release candidate: missing '{expected}'"
        );
    }

    assert!(
        !content.contains("cargo run --quiet --manifest-path cli-tools/prd-to-product-agents-cli/Cargo.toml"),
        "enterprise sandbox workflow must not bootstrap from the source checkout"
    );

    let source_runtime_invocations = content
        .matches("cargo run --quiet --manifest-path cli-tools/prdtp-agents-functions-cli/Cargo.toml")
        .count();
    assert_eq!(
        source_runtime_invocations,
        1,
        "enterprise sandbox workflow should use the source runtime only for the maintainer-only readiness promotion step"
    );
}

#[test]
fn build_workflow_validates_published_help_contract_on_every_runner() {
    let content = read_repo_file(".github/workflows/build-skill-binaries.yml");
    for expected in [
        "Validate published binary contract (Unix)",
        "Validate published binary contract (Windows)",
        "validate package --help",
        "validate all --help",
        "governance --help",
        "audit --help",
    ] {
        assert!(
            content.contains(expected),
            "build workflow must validate the published help contract before uploading artifacts: missing '{expected}'"
        );
    }
}

#[test]
fn published_checksum_manifests_cover_policy_and_sbom_files() {
    let bundles = [
        "bin/checksums.sha256",
        ".agents/skills/prd-to-product-agents/bin/checksums.sha256",
        ".agents/skills/prd-to-product-agents/templates/workspace/.agents/bin/prd-to-product-agents/checksums.sha256",
    ];

    for manifest in bundles {
        let entries = read_checksum_entries(manifest);
        assert!(
            entries.iter().any(|entry| entry == "provenance-policy.json"),
            "{manifest} must track provenance-policy.json"
        );
        assert!(
            entries.iter().any(|entry| entry == "sbom.spdx.json"),
            "{manifest} must track sbom.spdx.json"
        );
    }
}

#[test]
fn published_bundles_ship_sbom_and_provenance_policy() {
    let required_metadata = [
        "bin/checksums.sha256",
        "bin/sbom.spdx.json",
        "bin/provenance-policy.json",
        ".agents/skills/prd-to-product-agents/bin/checksums.sha256",
        ".agents/skills/prd-to-product-agents/bin/sbom.spdx.json",
        ".agents/skills/prd-to-product-agents/bin/provenance-policy.json",
        ".agents/skills/prd-to-product-agents/templates/workspace/.agents/bin/prd-to-product-agents/checksums.sha256",
        ".agents/skills/prd-to-product-agents/templates/workspace/.agents/bin/prd-to-product-agents/sbom.spdx.json",
        ".agents/skills/prd-to-product-agents/templates/workspace/.agents/bin/prd-to-product-agents/provenance-policy.json",
    ];

    let mut missing = Vec::new();
    for path in required_metadata {
        if !repo_root().join(path).is_file() {
            missing.push(path);
        }
    }

    assert!(
        missing.is_empty(),
        "Published bundles must ship checksum, SBOM, and provenance-policy metadata:\n  {}",
        missing.join("\n  ")
    );
}

#[test]
fn build_workflow_skips_redundant_bot_publish_runs() {
    let workflow = read_workflow_yaml(".github/workflows/build-skill-binaries.yml");
    let publish = workflow_job(&workflow, "publish");

    let expected_guard = "github.actor != 'github-actions[bot]'";
    let condition = mapping_get(publish, "if")
        .as_str()
        .expect("expected publish.if to be a string");
    assert!(
        condition.contains(expected_guard),
        "publish must skip bot-authored reruns to avoid redundant binary-refresh PR churn"
    );
}

#[test]
fn release_checklist_declares_ci_pr_only_binary_refresh() {
    let content = read_repo_file("docs/repo-release-checklist.md");
    for expected in [
        "The only supported refresh path for tracked binaries is `.github/workflows/build-skill-binaries.yml`",
        "Do not hand-refresh tracked binaries",
        "Local binary rebuilds are diagnostic only",
        "Treat any attempt to bypass the workflow PR path for tracked binaries as a release-process failure",
    ] {
        assert!(
            content.contains(expected),
            "repo release checklist must declare the CI PR-only binary refresh path: missing '{expected}'"
        );
    }
}

#[test]
fn release_checklist_requires_enterprise_evidence_publication_and_artifact_review() {
    let content = read_repo_file("docs/repo-release-checklist.md");
    for expected in [
        "confirm `.github/workflows/enterprise-readiness-sandbox.yml` is published on the remote branch or tag candidate under review",
        "run the enterprise sandbox only after the branch under review contains the tracked binary-refresh result",
        "Review the uploaded `enterprise-readiness-evidence` artifact for at least",
        "`package-validate.txt`",
        "`bootstrap-report.md`",
        "`bootstrap-manifest.txt`",
        "`governance-promote-enterprise-readiness.txt`",
        "Do not approve release if the workflow is not remotely dispatchable",
    ] {
        assert!(
            content.contains(expected),
            "repo release checklist must make enterprise evidence a concrete release gate: missing '{expected}'"
        );
    }
}

#[test]
fn release_docs_require_local_drift_review_and_unix_mode_verification() {
    let checklist = read_repo_file("docs/repo-release-checklist.md");
    for expected in [
        "Treat `test repo-validation` plus `test workflow-release-gate` as the local drift-review pair",
        "Confirm published Unix binaries in all tracked bundle scopes still preserve `100755` executable mode in the git index.",
        "Do not approve release if `test repo-validation` or `test release-gate` reports Unix executable-bit drift in published binaries.",
    ] {
        assert!(
            checklist.contains(expected),
            "repo release checklist must keep the release-drift and executable-bit review path explicit: missing '{expected}'"
        );
    }

    let runbook = read_repo_file("docs/maintainer-runbook.md");
    for expected in [
        "review `.github/workflows/build-skill-binaries.yml`,",
        "`.github/workflows/dependency-review.yml`, and `docs/repo-release-checklist.md`",
        "`test repo-validation` is the local regression proof for release-doc/workflow drift and for published Unix executable-bit integrity.",
        "must stay `100755` in the git index.",
    ] {
        assert!(
            runbook.contains(expected),
            "maintainer runbook must keep the release-drift and executable-bit guardrails explicit: missing '{expected}'"
        );
    }
}

#[test]
fn workflows_use_node24_ready_action_pins() {
    let workflow_checks = [
        (
            ".github/workflows/repo-validation.yml",
            ["actions/checkout@v6", "actions/setup-node@v6"].as_slice(),
            ["actions/checkout@v4", "actions/setup-node@v4"].as_slice(),
        ),
        (
            ".github/workflows/build-skill-binaries.yml",
            [
                "actions/checkout@v6",
                "actions/upload-artifact@v7",
                "actions/download-artifact@v8",
            ]
            .as_slice(),
            [
                "actions/checkout@v4",
                "actions/upload-artifact@v4",
                "actions/download-artifact@v4",
            ]
            .as_slice(),
        ),
        (
            ".github/workflows/release-binaries.yml",
            ["actions/checkout@v6", "actions/upload-artifact@v7"].as_slice(),
            ["actions/checkout@v4", "actions/upload-artifact@v4"].as_slice(),
        ),
        (
            ".github/workflows/enterprise-readiness-sandbox.yml",
            ["actions/checkout@v6", "actions/upload-artifact@v7"].as_slice(),
            ["actions/checkout@v4", "actions/upload-artifact@v4"].as_slice(),
        ),
        (
            ".github/workflows/dependency-review.yml",
            ["actions/checkout@v6", "actions/dependency-review-action@v4"].as_slice(),
            ["actions/checkout@v4", "actions/dependency-review-action@v3"].as_slice(),
        ),
        (
            ".agents/skills/prd-to-product-agents/templates/workspace/.github/workflows/smoke-tests.yml",
            ["actions/checkout@v6"].as_slice(),
            ["actions/checkout@v4"].as_slice(),
        ),
        (
            ".agents/skills/prd-to-product-agents/templates/workspace/.github/workflows/pr-governance.yml",
            ["actions/checkout@v6"].as_slice(),
            ["actions/checkout@v4"].as_slice(),
        ),
    ];

    let mut missing = Vec::new();
    let mut forbidden = Vec::new();

    for (path, expected, disallowed) in workflow_checks {
        let content = read_repo_file(path);

        for entry in expected {
            if !content.contains(entry) {
                missing.push(format!("{path}: missing '{entry}'"));
            }
        }

        for entry in disallowed {
            if content.contains(entry) {
                forbidden.push(format!("{path}: still contains deprecated '{entry}'"));
            }
        }
    }

    assert!(
        missing.is_empty() && forbidden.is_empty(),
        "Workflow action pins must stay on Node24-ready majors:\n  {}",
        missing
            .into_iter()
            .chain(forbidden)
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}
