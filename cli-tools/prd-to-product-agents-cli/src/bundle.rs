use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::util;

const CHECKSUM_MANIFEST: &str = "checksums.sha256";
const SBOM_FILE: &str = "sbom.spdx.json";
const PROVENANCE_POLICY_FILE: &str = "provenance-policy.json";

struct BundleSpec {
    label: &'static str,
    dir: PathBuf,
}

#[derive(Debug, Deserialize)]
struct ProvenancePolicy {
    schema_version: u32,
    required: bool,
    repo: String,
    signer_workflow: String,
    source_ref: String,
    predicate_type: String,
}

pub fn verify_consumed_bundle_integrity(skill_root: &Path) -> Result<Vec<String>> {
    let source_checkout = is_source_checkout(skill_root);
    let mut lines = Vec::new();
    for bundle in consumed_bundles(skill_root) {
        lines.push(verify_bundle(&bundle, source_checkout)?);
    }
    Ok(lines)
}

pub fn verify_packaged_bundle_integrity(skill_root: &Path) -> Result<()> {
    let _ = verify_consumed_bundle_integrity(skill_root)?;
    Ok(())
}

fn consumed_bundles(skill_root: &Path) -> Vec<BundleSpec> {
    vec![
        BundleSpec {
            label: "skill bootstrap bundle",
            dir: skill_root.join("bin"),
        },
        BundleSpec {
            label: "workspace runtime bundle",
            dir: skill_root
                .join("templates")
                .join("workspace")
                .join(".agents")
                .join("bin")
                .join("prd-to-product-agents"),
        },
    ]
}

fn verify_bundle(bundle: &BundleSpec, source_checkout: bool) -> Result<String> {
    let manifest = verify_checksum_manifest(&bundle.dir, bundle.label)?;
    verify_sbom_manifest(&bundle.dir, bundle.label, &manifest)?;
    let provenance = verify_provenance_policy(&bundle.dir, bundle.label, &manifest, source_checkout)?;
    Ok(format!(
        "{}: pass ({CHECKSUM_MANIFEST}, {SBOM_FILE}, {PROVENANCE_POLICY_FILE}; {provenance})",
        bundle.label
    ))
}

fn verify_checksum_manifest(bundle_dir: &Path, label: &str) -> Result<BTreeMap<String, String>> {
    let manifest_path = bundle_dir.join(CHECKSUM_MANIFEST);
    if !manifest_path.is_file() {
        bail!(
            "{label} checksum manifest not found: {}",
            manifest_path.display()
        );
    }

    let manifest = fs::read_to_string(&manifest_path)?;
    let mut expected = BTreeMap::new();
    for (index, line) in manifest.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let Some(hash) = parts.next() else {
            bail!("{label} checksum manifest line {} is malformed", index + 1);
        };
        let Some(file_name) = parts.next() else {
            bail!("{label} checksum manifest line {} is malformed", index + 1);
        };
        if parts.next().is_some() {
            bail!("{label} checksum manifest line {} is malformed", index + 1);
        }
        expected.insert(file_name.to_string(), hash.to_ascii_lowercase());
    }

    if expected.is_empty() {
        bail!("{label} checksum manifest is empty");
    }

    let actual_files: Vec<String> = fs::read_dir(bundle_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|kind| kind.is_file()).unwrap_or(false))
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .filter(|name| {
            name != CHECKSUM_MANIFEST
                && name != SBOM_FILE
                && name != PROVENANCE_POLICY_FILE
        })
        .collect();

    let mut errors = Vec::new();
    for (file_name, expected_hash) in &expected {
        let path = bundle_dir.join(file_name);
        if !path.is_file() {
            errors.push(format!("missing {file_name}"));
            continue;
        }
        let actual_hash = util::file_hash_bytes(&path)?;
        if actual_hash.to_ascii_lowercase() != *expected_hash {
            errors.push(format!("checksum mismatch for {file_name}"));
        }
    }

    for file_name in &actual_files {
        if !expected.contains_key(file_name) {
            errors.push(format!("untracked bundled file {file_name}"));
        }
    }

    if errors.is_empty() {
        Ok(expected)
    } else {
        bail!("{label} integrity failed:\n  {}", errors.join("\n  "))
    }
}

fn verify_sbom_manifest(
    bundle_dir: &Path,
    label: &str,
    expected: &BTreeMap<String, String>,
) -> Result<()> {
    let sbom_path = bundle_dir.join(SBOM_FILE);
    if !sbom_path.is_file() {
        bail!("{label} SBOM not found: {}", sbom_path.display());
    }
    let raw = fs::read_to_string(&sbom_path)?;
    let sbom: Value = serde_json::from_str(&raw)
        .with_context(|| format!("parsing {}", sbom_path.display()))?;
    let files = sbom["files"]
        .as_array()
        .context("SBOM must contain a top-level files array")?;
    let mut listed = BTreeMap::new();
    for file in files {
        let Some(name) = file["fileName"].as_str() else {
            continue;
        };
        let normalized = name.trim_start_matches("./").to_string();
        let hash = file["checksums"]
            .as_array()
            .into_iter()
            .flatten()
            .find_map(|checksum| {
                let algorithm = checksum["algorithm"].as_str()?;
                if algorithm.eq_ignore_ascii_case("SHA256") {
                    checksum["checksumValue"]
                        .as_str()
                        .map(|value| value.to_ascii_lowercase())
                } else {
                    None
                }
            });
        if let Some(hash) = hash {
            listed.insert(normalized, hash);
        }
    }

    let mut errors = Vec::new();
    for (file_name, expected_hash) in expected {
        match listed.get(file_name) {
            Some(actual_hash) if actual_hash == expected_hash => {}
            Some(_) => errors.push(format!("SBOM checksum mismatch for {file_name}")),
            None => errors.push(format!("SBOM missing {file_name}")),
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        bail!("{label} SBOM verification failed:\n  {}", errors.join("\n  "))
    }
}

fn verify_provenance_policy(
    bundle_dir: &Path,
    label: &str,
    expected: &BTreeMap<String, String>,
    source_checkout: bool,
) -> Result<&'static str> {
    let policy_path = bundle_dir.join(PROVENANCE_POLICY_FILE);
    if !policy_path.is_file() {
        bail!(
            "{label} provenance policy not found: {}",
            policy_path.display()
        );
    }
    let raw = fs::read_to_string(&policy_path)?;
    let policy: ProvenancePolicy = serde_json::from_str(&raw)
        .with_context(|| format!("parsing {}", policy_path.display()))?;

    if policy.schema_version != 1 {
        bail!("{label} provenance policy schema_version must be 1");
    }
    if !policy.required {
        bail!("{label} provenance policy must set required=true");
    }
    if policy.repo.trim().is_empty()
        || policy.signer_workflow.trim().is_empty()
        || policy.source_ref.trim().is_empty()
        || policy.predicate_type.trim().is_empty()
    {
        bail!("{label} provenance policy contains empty required fields");
    }

    if source_checkout {
        return Ok("attestation verification skipped for source checkout");
    }

    verify_gh_attestation_access()?;
    for file_name in expected.keys() {
        let file_path = bundle_dir.join(file_name);
        let output = Command::new("gh")
            .current_dir(bundle_dir)
            .args([
                "attestation",
                "verify",
                &file_path.to_string_lossy(),
                "--repo",
                &policy.repo,
                "--signer-workflow",
                &policy.signer_workflow,
                "--source-ref",
                &policy.source_ref,
                "--predicate-type",
                &policy.predicate_type,
            ])
            .output()
            .with_context(|| format!("running gh attestation verify for {}", file_path.display()))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if stderr.is_empty() {
                bail!(
                    "{label} attestation verification failed for {file_name} with exit code {:?}",
                    output.status.code()
                );
            }
            bail!("{label} attestation verification failed for {file_name}: {stderr}");
        }
    }

    Ok("attestation verified")
}

fn verify_gh_attestation_access() -> Result<()> {
    let version = Command::new("gh")
        .args(["attestation", "verify", "--help"])
        .output()
        .context("running `gh attestation verify --help`")?;
    if !version.status.success() {
        bail!("consumer-side provenance verification requires gh attestation support");
    }

    let auth = Command::new("gh")
        .args(["auth", "status"])
        .output()
        .context("running `gh auth status` for provenance verification")?;
    if !auth.status.success() {
        let stderr = String::from_utf8_lossy(&auth.stderr).trim().to_string();
        if stderr.is_empty() {
            bail!("consumer-side provenance verification requires authenticated gh access");
        }
        bail!("consumer-side provenance verification requires authenticated gh access: {stderr}");
    }

    Ok(())
}

fn is_source_checkout(skill_root: &Path) -> bool {
    if std::env::var("PRDTP_TRUST_SOURCE_CHECKOUT")
        .map(|value| value == "1")
        .unwrap_or(false)
    {
        return true;
    }
    skill_root.ancestors().any(|ancestor| {
        ancestor.join(".git").exists()
            && ancestor
                .join("cli-tools")
                .join("prd-to-product-agents-cli")
                .join("Cargo.toml")
                .is_file()
    })
}
