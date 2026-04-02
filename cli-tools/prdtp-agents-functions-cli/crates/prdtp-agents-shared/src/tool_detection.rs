use std::env;
use std::path::{Path, PathBuf};

/// Check whether a command is discoverable without invoking it.
///
/// This intentionally avoids `command --version` probes because npm shims and
/// Windows launchers can be present on PATH while returning non-zero for
/// version checks.
pub fn command_exists(name: &str) -> bool {
    if name.trim().is_empty() {
        return false;
    }

    let candidate = Path::new(name);
    if candidate.is_absolute() || candidate.components().count() > 1 {
        return is_executable_candidate(candidate);
    }

    let Some(paths) = env::var_os("PATH") else {
        return false;
    };

    env::split_paths(&paths).any(|dir| {
        candidate_names(name)
            .into_iter()
            .map(|entry| dir.join(entry))
            .any(|path| is_executable_candidate(&path))
    })
}

#[cfg(windows)]
fn candidate_names(name: &str) -> Vec<PathBuf> {
    let path = Path::new(name);
    if path.extension().is_some() {
        return vec![PathBuf::from(name)];
    }

    let pathext = env::var_os("PATHEXT")
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".to_string());

    let mut candidates = Vec::new();
    for ext in pathext.split(';').map(str::trim).filter(|ext| !ext.is_empty()) {
        candidates.push(PathBuf::from(format!("{name}{ext}")));
    }

    if candidates.is_empty() {
        candidates.push(PathBuf::from(format!("{name}.exe")));
    }

    candidates
}

#[cfg(not(windows))]
fn candidate_names(name: &str) -> Vec<PathBuf> {
    vec![PathBuf::from(name)]
}

#[cfg(windows)]
fn is_executable_candidate(path: &Path) -> bool {
    path.is_file()
}

#[cfg(not(windows))]
fn is_executable_candidate(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    std::fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::command_exists;

    #[test]
    fn detects_current_executable_by_path() {
        let current_exe = std::env::current_exe().expect("failed to resolve current executable");
        assert!(command_exists(current_exe.to_string_lossy().as_ref()));
    }

    #[test]
    fn detects_cargo_from_path() {
        assert!(command_exists("cargo"));
    }

    #[test]
    fn rejects_missing_command() {
        let missing = format!("missing-command-for-detection-test-{}", std::process::id());
        assert!(!command_exists(&missing));
    }
}
