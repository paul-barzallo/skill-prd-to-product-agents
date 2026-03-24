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
fn build_workflow_release_gate_runs_before_merge() {
    let workflow = read_workflow_yaml(".github/workflows/build-skill-binaries.yml");
    let release_gate = workflow_job(&workflow, "release-gate");
    let publish = workflow_job(&workflow, "publish");

    let release_gate_if = mapping_get(release_gate, "if")
        .as_str()
        .expect("expected release-gate.if to be a string");
    let publish_if = mapping_get(publish, "if")
        .as_str()
        .expect("expected publish.if to be a string");

    assert!(
        !release_gate_if.contains("github.ref == 'refs/heads/main' && github.event_name != 'pull_request'"),
        "Release gate must not be restricted to post-merge execution only"
    );
    assert!(
        release_gate_if.contains("github.actor != 'github-actions[bot]'"),
        "Release gate should skip bot-authored publish pushes to avoid redundant loops"
    );
    assert!(
        publish_if == "github.ref == 'refs/heads/main' && github.event_name != 'pull_request' && github.actor != 'github-actions[bot]'",
        "Publish job must remain restricted to push on main"
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
        "sha256sum prd-to-product-agents-cli-* > checksums.sha256",
        "sha256sum prdtp-agents-functions-cli-* > checksums.sha256",
    ];

    let mut missing = Vec::new();
    for entry in expected_entries {
        if !content.contains(entry) {
            missing.push(entry);
        }
    }

    assert!(
        missing.is_empty(),
        "Build workflow publish step must refresh checksum manifests when it updates binaries:\n  {}",
        missing.join("\n  ")
    );
}

#[test]
fn build_workflow_skips_redundant_bot_publish_runs() {
    let workflow = read_workflow_yaml(".github/workflows/build-skill-binaries.yml");
    let build = workflow_job(&workflow, "build");
    let test = workflow_job(&workflow, "test");
    let release_gate = workflow_job(&workflow, "release-gate");
    let publish = workflow_job(&workflow, "publish");

    let expected_guard = "github.actor != 'github-actions[bot]'";

    for (job_name, job) in [("build", build), ("test", test), ("release-gate", release_gate), ("publish", publish)] {
        let condition = mapping_get(job, "if")
            .as_str()
            .unwrap_or_else(|| panic!("expected {job_name}.if to be a string"));
        assert!(
            condition.contains(expected_guard),
            "{job_name} must skip bot-authored publish pushes to avoid redundant workflow runs"
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
