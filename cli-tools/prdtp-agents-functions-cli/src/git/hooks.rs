use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::path::Path;

const HOOK_CONTENT: &str = r#"#!/bin/sh
set -e

# Pre-commit hook - delegates to the workspace-local runtime CLI for governance enforcement.
REPO_ROOT=$(git rev-parse --show-toplevel)
BASE_DIR="$REPO_ROOT/.agents/bin/prd-to-product-agents"
CLI=""

# Use OS detection to find the correct binary, with fallback
if command -v uname >/dev/null 2>&1; then
    case "$(uname -s)" in
        Darwin) CLI="$BASE_DIR/prdtp-agents-functions-cli-darwin-arm64" ;;
        Linux) CLI="$BASE_DIR/prdtp-agents-functions-cli-linux-x64" ;;
        MINGW*|MSYS*|CYGWIN*|Windows_NT) CLI="$BASE_DIR/prdtp-agents-functions-cli-windows-x64.exe" ;;
    esac
fi

if [ -z "$CLI" ] || [ ! -f "$CLI" ]; then
    if [ -f "$BASE_DIR/prdtp-agents-functions-cli-windows-x64.exe" ]; then
        CLI="$BASE_DIR/prdtp-agents-functions-cli-windows-x64.exe"
    elif [ -f "$BASE_DIR/prdtp-agents-functions-cli-darwin-arm64" ]; then
        CLI="$BASE_DIR/prdtp-agents-functions-cli-darwin-arm64"
    elif [ -f "$BASE_DIR/prdtp-agents-functions-cli-linux-x64" ]; then
        CLI="$BASE_DIR/prdtp-agents-functions-cli-linux-x64"
    fi
fi

if [ -z "$CLI" ] || [ ! -f "$CLI" ]; then
    echo "[CRITICAL] Pre-commit hook failed: Target CLI binary not found in $BASE_DIR!" >&2
    echo "Expected one of the OS-specific prdtp-agents-functions-cli binaries to enforce governance." >&2
    exit 1
fi

exec "$CLI" --workspace "$REPO_ROOT" git pre-commit-validate "$@"
"#;

pub fn run(workspace: &Path) -> Result<()> {
    tracing::info!(workspace = %workspace.display(), "installing pre-commit hook");
    let hooks_dir = workspace.join(".git/hooks");
    if !hooks_dir.exists() {
        fs::create_dir_all(&hooks_dir)?;
    }
    let hook_path = hooks_dir.join("pre-commit");
    fs::write(&hook_path, HOOK_CONTENT)?;

    // On Unix, make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&hook_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&hook_path, perms)?;
    }

    tracing::debug!(hook_path = %hook_path.display(), "pre-commit hook file written");
    println!(
        "{} pre-commit hook installed at {}",
        "✓".green().bold(),
        hook_path.display()
    );
    Ok(())
}
