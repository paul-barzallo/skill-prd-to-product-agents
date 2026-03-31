use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Generate an auto-ID with the given prefix (e.g. "ho-", "fi-", "ee-").
pub fn new_auto_id(prefix: &str) -> String {
    let short = &Uuid::new_v4().to_string().replace('-', "")[..8];
    format!("{prefix}{short}")
}

/// Generate next release ID by scanning existing IDs in the YAML content.
pub fn next_release_id(yaml_content: &str) -> String {
    let re = regex::Regex::new(r"id:\s*R(\d+)").unwrap();
    let max_n = re
        .captures_iter(yaml_content)
        .filter_map(|c| c.get(1)?.as_str().parse::<u32>().ok())
        .max()
        .unwrap_or(0);
    format!("R{}", max_n + 1)
}

/// SHA-256 of a string, returned as hex.
pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// SHA-256 of file content.
pub fn file_content_hash(path: &Path) -> Result<String> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("reading {}", path.display()))?;
    Ok(sha256_hex(&content))
}

/// Advisory file lock using a `.lock` sidecar.
pub struct YamlLock {
    lock_path: PathBuf,
}

impl YamlLock {
    /// Acquire lock with 5-second timeout (100ms retry interval).
    pub fn acquire(yaml_path: &Path) -> Result<Self> {
        let lock_path = yaml_path.with_extension("yaml.lock");
        let deadline = Instant::now() + Duration::from_secs(5);
        let stale_after = Duration::from_secs(30);

        loop {
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock_path)
            {
                Ok(_) => {
                    let _ = fs::write(
                        &lock_path,
                        format!(
                            "pid={}\ncreated_at={}\npath={}\n",
                            std::process::id(),
                            now_utc_iso(),
                            yaml_path.display()
                        ),
                    );
                    return Ok(Self { lock_path });
                }
                Err(_) if Instant::now() < deadline => {
                    if let Ok(metadata) = fs::metadata(&lock_path) {
                        if let Ok(modified) = metadata.modified() {
                            if modified
                                .elapsed()
                                .map(|age| age > stale_after)
                                .unwrap_or(false)
                            {
                                let _ = fs::remove_file(&lock_path);
                                continue;
                            }
                        }
                    }
                    thread::sleep(Duration::from_millis(100));
                }
                Err(e) => bail!(
                    "Failed to acquire lock on {}: {e}",
                    yaml_path.display()
                ),
            }
        }
    }

    /// Release lock explicitly.
    #[allow(dead_code)]
    pub fn release(self) {
        // Drop triggers cleanup
        drop(self);
    }
}

impl Drop for YamlLock {
    fn drop(&mut self) {
        for _ in 0..10 {
            if fs::remove_file(&self.lock_path).is_ok() {
                return;
            }
            thread::sleep(Duration::from_millis(50));
        }
    }
}

/// Atomic write: write to `.tmp.PID`, then rename.
pub fn atomic_write(path: &Path, content: &str) -> Result<()> {
    let tmp = path.with_extension(format!("tmp.{}", std::process::id()));
    fs::write(&tmp, content)
        .with_context(|| format!("writing temp file {}", tmp.display()))?;

    // On Windows, fs::rename fails if target exists — remove first
    if path.exists() {
        fs::remove_file(path)
            .with_context(|| format!("removing old file {}", path.display()))?;
    }
    fs::rename(&tmp, path)
        .with_context(|| format!("renaming {} -> {}", tmp.display(), path.display()))?;
    Ok(())
}

/// Normalize line endings to LF.
pub fn normalize_lf(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// Read a YAML file, ensuring parent dirs exist, creating default content if missing.
pub fn ensure_yaml_file(path: &Path, header: &str) -> Result<String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if !path.exists() {
        fs::write(path, header)?;
        return Ok(header.to_string());
    }
    let content = fs::read_to_string(path)
        .with_context(|| format!("reading {}", path.display()))?;
    Ok(normalize_lf(&content))
}

/// Get the current UTC date as YYYY-MM-DD.
pub fn today_utc() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

/// Get the current UTC timestamp as ISO 8601.
pub fn now_utc_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Escape double quotes in a YAML string value.
pub fn yaml_escape(s: &str) -> String {
    s.replace('"', "\\\"")
}

/// Read a single YAML entry by ID from a YAML list (regex-based, no full parse).
pub fn read_yaml_entry_field(content: &str, id: &str, field: &str) -> Option<String> {
    // Find the entry block starting with `- id: <id>`
    let pattern = format!(r"(?m)^  - id:\s*{}\s*$", regex::escape(id));
    let re = regex::Regex::new(&pattern).ok()?;
    let m = re.find(content)?;
    let rest = &content[m.start()..];

    // Find the field within the entry (before next `- id:` or end)
    let end_pattern = regex::Regex::new(r"(?m)^  - id:").ok()?;
    let block = if let Some(next) = end_pattern.find(&rest[1..]) {
        &rest[..next.start() + 1]
    } else {
        rest
    };

    let field_re = regex::Regex::new(&format!(r"(?m)^\s+{field}:\s*(.+)$")).ok()?;
    let caps = field_re.captures(block)?;
    Some(caps.get(1)?.as_str().trim().trim_matches('"').to_string())
}

/// Check if an entry with the given ID exists in the YAML content.
pub fn entry_exists(content: &str, id: &str) -> bool {
    let pattern = format!("id: {id}");
    content.contains(&pattern)
}
