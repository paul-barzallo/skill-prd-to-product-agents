use anyhow::Result;
use prdtp_agents_shared::capabilities::{render_capabilities_yaml, CapabilitySnapshotInput};
use std::path::Path;

use crate::util;

pub fn write_workspace_capabilities(target: &Path) -> Result<()> {
    let caps_path = target.join(".github").join("workspace-capabilities.yaml");

    let existing_updated = util::yaml_scalar(&caps_path, "last_updated").ok().flatten();
    let preserve = existing_updated
        .as_deref()
        .map_or(false, |value| !value.is_empty() && value != "1970-01-01T00:00:00Z");

    let git_installed = util::command_exists("git");
    let git_identity = if git_installed {
        has_git_identity(target)
    } else {
        false
    };
    let gh_installed = util::command_exists("gh");
    let gh_authenticated = if gh_installed {
        util::command_ok("gh", &["auth", "status"])
    } else {
        false
    };
    let sqlite_installed = util::sqlite_runtime_available();
    let db_initialized = target.join(".state").join("project_memory.db").exists();
    let node_installed = util::command_exists("node");
    let npm_installed = util::command_exists("npm");
    let markdownlint_installed = util::command_exists("markdownlint");
    let ui_available = target.join("reporting-ui/index.html").exists();
    let xlsx_ready = target.join("reporting-ui/vendor/xlsx.mini.min.js").exists();

    let git_authorized = if preserve {
        util::yaml_bool(
            &caps_path,
            "capabilities.git.authorized.enabled",
            false,
        )
    } else {
        false
    };
    let gh_authorized = if preserve {
        util::yaml_bool(
            &caps_path,
            "capabilities.gh.authorized.enabled",
            false,
        ) && git_authorized
    } else {
        false
    };
    let sqlite_authorized = if preserve {
        util::yaml_bool(
            &caps_path,
            "capabilities.sqlite.authorized.enabled",
            sqlite_installed,
        )
    } else {
        sqlite_installed
    };
    let markdownlint_authorized = if preserve {
        util::yaml_bool(
            &caps_path,
            "capabilities.markdownlint.authorized.enabled",
            markdownlint_installed,
        )
    } else {
        markdownlint_installed
    };
    let local_history_authorized = if preserve {
        util::yaml_bool(
            &caps_path,
            "capabilities.local_history.authorized.enabled",
            true,
        )
    } else {
        true
    };
    let reporting_authorized = if preserve {
        util::yaml_bool(&caps_path, "capabilities.reporting.authorized.enabled", true)
    } else {
        true
    };

    let yaml = render_capabilities_yaml(CapabilitySnapshotInput {
        host: util::detect_host().to_string(),
        os: util::detect_os().to_string(),
        git_installed,
        git_identity_configured: git_identity,
        git_authorized,
        git_authorization_source: if git_authorized {
            "manual-maintainer".to_string()
        } else {
            "manual-default-deny".to_string()
        },
        git_mode: if git_authorized {
            "full".to_string()
        } else {
            "local-only".to_string()
        },
        gh_installed,
        gh_authenticated,
        gh_authorized,
        gh_authorization_source: if gh_authorized {
            "manual-maintainer".to_string()
        } else {
            "manual-default-deny".to_string()
        },
        sqlite_installed,
        db_initialized,
        sqlite_authorized,
        sqlite_authorization_source: if sqlite_authorized {
            "detected-default".to_string()
        } else {
            "missing-runtime".to_string()
        },
        sqlite_mode: if sqlite_authorized && db_initialized {
            "ledger".to_string()
        } else {
            "spool-only".to_string()
        },
        node_installed,
        npm_installed,
        node_native_linux: node_installed,
        markdownlint_installed,
        markdownlint_native_linux: markdownlint_installed,
        markdownlint_authorized,
        markdownlint_authorization_source: if markdownlint_authorized {
            "detected-default".to_string()
        } else {
            "missing-runtime".to_string()
        },
        local_history_authorized,
        local_history_authorization_source: "detected-default".to_string(),
        local_history_format: "markdown+json".to_string(),
        local_history_path: ".state/local-history".to_string(),
        reporting_ui_available: ui_available,
        reporting_xlsx_export_ready: xlsx_ready,
        reporting_pdf_export_ready: false,
        reporting_authorized,
        reporting_authorization_source: if reporting_authorized {
            "detected-default".to_string()
        } else {
            "manual-maintainer".to_string()
        },
        reporting_visibility_mode: if gh_authorized {
            "auto".to_string()
        } else {
            "local-only".to_string()
        },
        last_updated: util::now_utc(),
    })?;

    util::write_utf8_lf(&caps_path, &yaml)
}

pub fn has_git_identity(target: &Path) -> bool {
    let name = std::process::Command::new("git")
        .args(["config", "--get", "user.name"])
        .current_dir(target)
        .output()
        .map(|output| !output.stdout.is_empty())
        .unwrap_or(false);
    let email = std::process::Command::new("git")
        .args(["config", "--get", "user.email"])
        .current_dir(target)
        .output()
        .map(|output| !output.stdout.is_empty())
        .unwrap_or(false);
    name && email
}
