use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::path::{Component, Path, PathBuf};

pub fn resolve_project_root(raw_root: &Path) -> Result<PathBuf> {
    raw_root
        .canonicalize()
        .with_context(|| format!("project root does not exist: {}", raw_root.display()))
}

pub fn normalize_lf(input: &str) -> String {
    input.replace("\r\n", "\n").replace('\r', "\n")
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub fn to_relative_posix(path: &Path, root: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    relative.to_string_lossy().replace('\\', "/")
}

pub fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
        }
    }

    normalized
}

pub fn is_probably_text(bytes: &[u8]) -> bool {
    if bytes.contains(&0) {
        return false;
    }

    let control_count = bytes
        .iter()
        .filter(|byte| {
            matches!(byte, 0x01..=0x08 | 0x0B | 0x0C | 0x0E..=0x1F)
        })
        .count();

    control_count.saturating_mul(10) < bytes.len().max(1)
}

pub fn truncate(input: &str, max_chars: usize) -> String {
    let mut result = String::new();
    let mut chars = input.chars();

    for _ in 0..max_chars {
        match chars.next() {
            Some(ch) => result.push(ch),
            None => return result,
        }
    }

    if chars.next().is_some() {
        result.push_str("...");
    }

    result
}
