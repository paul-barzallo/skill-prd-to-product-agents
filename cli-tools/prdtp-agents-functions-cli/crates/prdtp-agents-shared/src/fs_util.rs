use std::fs;
use std::path::Path;

/// Write a file with UTF-8 no-BOM encoding, creating parent dirs.
#[allow(dead_code)]
pub fn write_utf8(path: &Path, content: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)
}

/// Check if a path exists (file or directory).
#[allow(dead_code)]
pub fn exists(path: &Path) -> bool {
    path.exists()
}
