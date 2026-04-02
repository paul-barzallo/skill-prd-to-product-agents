use anyhow::{bail, Context, Result};
use clap::{Args, ValueEnum};
use colored::Colorize;
use prdtp_agents_shared::yaml_ops;
use serde_json::json;
use serde_yaml::{Mapping, Value};
use std::fs;
use std::path::Path;

const MAX_BRANCH_PROTECTION_APPROVAL_QUORUM: u64 = 6;

#[derive(Args)]
pub struct ConfigureArgs {
    #[arg(long)]
    pub owner: String,
    #[arg(long)]
    pub repo: String,
    #[arg(long)]
    pub release_gate_login: String,
    #[arg(long)]
    pub release_gate_extra_logins: Option<String>,
    #[arg(long)]
    pub release_gate_approval_quorum: Option<u64>,
    #[arg(long)]
    pub reviewer_product: String,
    #[arg(long)]
    pub reviewer_architecture: String,
    #[arg(long)]
    pub reviewer_tech_lead: String,
    #[arg(long)]
    pub reviewer_qa: String,
    #[arg(long)]
    pub reviewer_devops: String,
    #[arg(long)]
    pub reviewer_infra: String,
    #[arg(long)]
    pub reviewer_infra_login: String,
    #[arg(long)]
    pub immutable_governance_approval_quorum: Option<u64>,
    #[arg(long, value_enum)]
    pub operating_profile: Option<OperatingProfileArg>,
    #[arg(long, value_enum)]
    pub github_auth_mode: Option<GithubAuthModeArg>,
    #[arg(long, value_enum)]
    pub audit_mode: Option<AuditModeArg>,
    #[arg(long)]
    pub audit_remote_endpoint: Option<String>,
    #[arg(long)]
    pub audit_remote_auth_header_env: Option<String>,
    #[arg(long)]
    pub audit_remote_timeout_seconds: Option<u64>,
}

#[derive(Args)]
pub struct ProvisionEnterpriseArgs {
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Args, Default)]
pub struct PromoteEnterpriseReadinessArgs {}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum OperatingProfileArg {
    CoreLocal,
    Enterprise,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum GithubAuthModeArg {
    GhCli,
    TokenApi,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum AuditModeArg {
    LocalHashchain,
    Remote,
}

struct GovernanceReviewers {
    product: String,
    architecture: String,
    tech_lead: String,
    qa: String,
    devops: String,
    infra: String,
}

impl std::fmt::Display for OperatingProfileArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CoreLocal => write!(f, "core-local"),
            Self::Enterprise => write!(f, "enterprise"),
        }
    }
}

impl std::fmt::Display for GithubAuthModeArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GhCli => write!(f, "gh-cli"),
            Self::TokenApi => write!(f, "token-api"),
        }
    }
}

impl std::fmt::Display for AuditModeArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LocalHashchain => write!(f, "local-hashchain"),
            Self::Remote => write!(f, "remote"),
        }
    }
}

pub fn configure(workspace: &Path, args: ConfigureArgs) -> Result<()> {
    tracing::info!(
        workspace = %workspace.display(),
        owner = %args.owner,
        repo = %args.repo,
        "configuring local GitHub governance"
    );

    let owner = normalize_slug_like("owner", &args.owner)?;
    let repo = normalize_slug_like("repo", &args.repo)?;
    let release_gate_primary_login = normalize_login("release-gate-login", &args.release_gate_login)?;
    let release_gate_extra_logins = normalize_login_list(
        "release-gate-extra-logins",
        args.release_gate_extra_logins.as_deref(),
    )?;
    let release_gate_logins = merge_unique_logins(
        &release_gate_primary_login,
        &release_gate_extra_logins,
    );
    let reviewer_infra_login = normalize_login("reviewer-infra-login", &args.reviewer_infra_login)?;
    if release_gate_logins
        .iter()
        .any(|login| login == &reviewer_infra_login)
    {
        bail!(
            "reviewer-infra-login must differ from every release-gate login so immutable governance keeps a separate reviewer identity"
        );
    }
    let release_gate_approval_quorum = validate_approval_quorum(
        "release-gate-approval-quorum",
        args.release_gate_approval_quorum.unwrap_or(1),
        release_gate_logins.len() as u64,
        Some(MAX_BRANCH_PROTECTION_APPROVAL_QUORUM),
    )?;
    let immutable_governance_approval_quorum = validate_approval_quorum(
        "immutable-governance-approval-quorum",
        args.immutable_governance_approval_quorum.unwrap_or(1),
        2,
        None,
    )?;
    let reviewers = GovernanceReviewers {
        product: normalize_handle("reviewer-product", &args.reviewer_product)?,
        architecture: normalize_handle("reviewer-architecture", &args.reviewer_architecture)?,
        tech_lead: normalize_handle("reviewer-tech-lead", &args.reviewer_tech_lead)?,
        qa: normalize_handle("reviewer-qa", &args.reviewer_qa)?,
        devops: normalize_handle("reviewer-devops", &args.reviewer_devops)?,
        infra: normalize_handle("reviewer-infra", &args.reviewer_infra)?,
    };
    let operating_profile = args.operating_profile.unwrap_or(OperatingProfileArg::CoreLocal);
    let github_auth_mode = args.github_auth_mode.unwrap_or_else(|| match operating_profile {
        OperatingProfileArg::CoreLocal => GithubAuthModeArg::GhCli,
        OperatingProfileArg::Enterprise => GithubAuthModeArg::TokenApi,
    });
    let audit_mode = args.audit_mode.unwrap_or_else(|| match operating_profile {
        OperatingProfileArg::CoreLocal => AuditModeArg::LocalHashchain,
        OperatingProfileArg::Enterprise => AuditModeArg::Remote,
    });
    validate_profile_contract(
        operating_profile,
        github_auth_mode,
        audit_mode,
        args.audit_remote_endpoint.as_deref(),
        args.audit_remote_auth_header_env.as_deref(),
    )?;

    let governance_path = workspace.join(".github/github-governance.yaml");
    let codeowners_path = workspace.join(".github/CODEOWNERS");
    if !governance_path.is_file() {
        bail!("Missing {}", governance_path.display());
    }
    if !codeowners_path.is_file() {
        bail!("Missing {}", codeowners_path.display());
    }

    let governance_raw = fs::read_to_string(&governance_path)
        .with_context(|| format!("reading {}", governance_path.display()))?;
    let mut governance: Value = serde_yaml::from_str(&governance_raw)
        .with_context(|| format!("parsing {}", governance_path.display()))?;

    update_governance_value(
        &mut governance,
        &owner,
        &repo,
        &release_gate_logins,
        &reviewer_infra_login,
        release_gate_approval_quorum,
        immutable_governance_approval_quorum,
        &reviewers,
        operating_profile,
        github_auth_mode,
        audit_mode,
        args.audit_remote_endpoint.as_deref(),
        args.audit_remote_auth_header_env.as_deref(),
        args.audit_remote_timeout_seconds,
    )?;
    let rendered_governance = serde_yaml::to_string(&governance)?;
    let rendered_codeowners = render_codeowners(&reviewers);

    if contains_placeholder_marker(&rendered_governance) {
        bail!("generated github-governance.yaml still contains placeholders");
    }
    if contains_placeholder_marker(&rendered_codeowners) {
        bail!("generated CODEOWNERS still contains placeholders");
    }

    yaml_ops::atomic_write(&governance_path, &rendered_governance)?;
    yaml_ops::atomic_write(&codeowners_path, &rendered_codeowners)?;

    println!("{} Governance configured", "OK:".green().bold());
    println!("  Repository: {owner}/{repo}");
    println!("  Operating profile: {operating_profile}");
    println!("  GitHub auth mode: {github_auth_mode}");
    println!("  Audit mode: {audit_mode}");
    println!("  Release gate logins: {}", release_gate_logins.join(", "));
    println!("  Release gate approval quorum: {release_gate_approval_quorum}");
    println!(
        "  Immutable governance logins: {release_gate_primary_login}, {reviewer_infra_login}"
    );
    println!(
        "  Immutable governance approval quorum: {immutable_governance_approval_quorum}"
    );
    println!("  Immutable governance: remote PR approval required");
    println!("  Readiness status: configured");
    crate::audit::events::record_sensitive_action(
        workspace,
        "governance.configure",
        "runtime-cli",
        "success",
        json!({
            "repository": format!("{owner}/{repo}"),
            "operating_profile": operating_profile.to_string(),
            "github_auth_mode": github_auth_mode.to_string(),
            "audit_mode": audit_mode.to_string(),
            "release_gate_login": release_gate_primary_login.clone(),
            "release_gate_logins": release_gate_logins.clone(),
            "release_gate_approval_quorum": release_gate_approval_quorum,
            "immutable_governance_logins": [
                release_gate_primary_login.clone(),
                reviewer_infra_login.clone()
            ],
            "immutable_governance_approval_quorum": immutable_governance_approval_quorum,
            "immutable_governance_mode": "remote-pr-approval",
            "readiness_status": "configured"
        }),
    )?;

    Ok(())
}

pub fn provision_enterprise(workspace: &Path, args: ProvisionEnterpriseArgs) -> Result<()> {
    println!("{}", "=== Provision Enterprise Governance ===".cyan().bold());
    crate::common::capability_contract::require_policy_enabled(
        workspace,
        "gh",
        "governance provision-enterprise",
    )?;

    let governance_path = workspace.join(".github/github-governance.yaml");
    let governance_raw = fs::read_to_string(&governance_path)
        .with_context(|| format!("reading {}", governance_path.display()))?;
    let governance: Value = serde_yaml::from_str(&governance_raw)
        .with_context(|| format!("parsing {}", governance_path.display()))?;

    let profile = crate::github_api::operating_profile(&governance)?;
    if profile != crate::github_api::OperatingProfile::Enterprise {
        bail!("governance provision enterprise requires operating_profile=enterprise");
    }
    crate::github_api::require_enterprise_api_mode(&governance)?;

    let repo = crate::github_api::repository_full_name(&governance)?;
    let protected_branches = crate::github_api::parse_csv(
        &crate::github_api::yaml_string(
            &governance,
            &["github", "branch_protection", "protected_branches"],
        )
        .unwrap_or_default(),
    );
    if protected_branches.is_empty() {
        bail!("github.branch_protection.protected_branches must contain at least one branch");
    }

    if args.dry_run {
        println!("  Repository: {repo}");
        println!("  Branch protection: {}", protected_branches.join(", "));
        println!("  Labels: role, kind, priority, status, criticality");
        println!("{} Dry-run complete", "OK:".green().bold());
        return Ok(());
    }

    for branch in &protected_branches {
        provision_branch_protection(&governance, branch)?;
        println!("  {} branch protection for {}", "✓".green(), branch);
    }
    provision_labels(&governance)?;
    println!("  {} governance labels ensured", "✓".green());

    crate::validate::readiness::validate_remote_governance(workspace, &governance)?;
    crate::audit::events::record_sensitive_action(
        workspace,
        "governance.provision-enterprise",
        "runtime-cli",
        "success",
        json!({
            "repository": repo,
            "protected_branches": protected_branches
        }),
    )?;
    println!("{} Enterprise governance provisioned", "OK:".green().bold());
    Ok(())
}

pub fn promote_enterprise_readiness(
    workspace: &Path,
    _args: PromoteEnterpriseReadinessArgs,
) -> Result<()> {
    println!("{}", "=== Promote Enterprise Readiness ===".cyan().bold());

    let governance_path = workspace.join(".github/github-governance.yaml");
    let governance_raw = fs::read_to_string(&governance_path)
        .with_context(|| format!("reading {}", governance_path.display()))?;
    let mut governance: Value = serde_yaml::from_str(&governance_raw)
        .with_context(|| format!("parsing {}", governance_path.display()))?;

    if crate::github_api::operating_profile(&governance)?
        != crate::github_api::OperatingProfile::Enterprise
    {
        bail!("governance promote-enterprise-readiness requires operating_profile=enterprise");
    }
    crate::github_api::require_enterprise_api_mode(&governance)?;
    if crate::github_api::audit_mode(&governance)? != crate::github_api::AuditMode::Remote {
        bail!("governance promote-enterprise-readiness requires audit.mode=remote");
    }
    let _ = crate::github_api::audit_remote_config(&governance)?
        .context("governance promote-enterprise-readiness requires audit.remote.* to be configured")?;

    let readiness = mapping_mut(&mut governance, &["readiness"])?;
    set_string(readiness, "status", "production-ready");

    let branch_protection = mapping_mut(&mut governance, &["github", "branch_protection"])?;
    set_bool(branch_protection, "enabled", true);

    if let Ok(project) = mapping_mut(&mut governance, &["github", "project"]) {
        set_bool(project, "enabled", false);
    }

    let rendered_governance = serde_yaml::to_string(&governance)?;
    yaml_ops::atomic_write(&governance_path, &rendered_governance)?;

    crate::audit::events::record_sensitive_action(
        workspace,
        "governance.promote-enterprise-readiness",
        "runtime-cli",
        "success",
        json!({
            "readiness_status": "production-ready",
            "branch_protection_enabled": true,
            "github_project_enabled": false
        }),
    )?;

    println!("  Readiness status: production-ready");
    println!("  Branch protection flag: true");
    println!("  GitHub Project flag: false");
    println!("{} Enterprise readiness promoted", "OK:".green().bold());
    Ok(())
}

fn normalize_handle(label: &str, value: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        bail!("{label} must be non-empty");
    }
    if trimmed.contains(char::is_whitespace) {
        bail!("{label} must be a single GitHub handle or team");
    }
    Ok(if trimmed.starts_with('@') {
        trimmed.to_string()
    } else {
        format!("@{trimmed}")
    })
}

fn normalize_login(label: &str, value: &str) -> Result<String> {
    let trimmed = value.trim().trim_start_matches('@').to_string();
    if trimmed.is_empty() {
        bail!("{label} must be non-empty");
    }
    if trimmed.contains(char::is_whitespace) || trimmed.contains('/') {
        bail!("{label} must be a single GitHub login");
    }
    Ok(trimmed)
}

fn normalize_login_list(label: &str, value: Option<&str>) -> Result<Vec<String>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };

    let mut logins = Vec::new();
    for candidate in value.split(',').map(str::trim).filter(|item| !item.is_empty()) {
        let normalized = normalize_login(label, candidate)?;
        if !logins.contains(&normalized) {
            logins.push(normalized);
        }
    }

    Ok(logins)
}

fn normalize_slug_like(label: &str, value: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        bail!("{label} must be non-empty");
    }
    if trimmed.contains(char::is_whitespace) {
        bail!("{label} must not contain whitespace");
    }
    Ok(trimmed.to_string())
}

fn update_governance_value(
    governance: &mut Value,
    owner: &str,
    repo: &str,
    release_gate_logins: &[String],
    reviewer_infra_login: &str,
    release_gate_approval_quorum: u64,
    immutable_governance_approval_quorum: u64,
    reviewers: &GovernanceReviewers,
    operating_profile: OperatingProfileArg,
    github_auth_mode: GithubAuthModeArg,
    audit_mode: AuditModeArg,
    audit_remote_endpoint: Option<&str>,
    audit_remote_auth_header_env: Option<&str>,
    audit_remote_timeout_seconds: Option<u64>,
) -> Result<()> {
    let readiness = mapping_mut(governance, &["readiness"])?;
    set_string(
        readiness,
        "notes",
        &format!(
            "Governance configured locally via prdtp-agents-functions-cli governance configure. operating_profile={} uses github.auth.mode={} and audit.mode={}. production-ready and immutable governance changes must be promoted through reviewed GitHub PR controls.",
            operating_profile, github_auth_mode, audit_mode
        ),
    );
    set_string(readiness, "status", "configured");
    if let Some(root) = governance.as_mapping_mut() {
        root.insert(
            Value::String("schema_version".to_string()),
            Value::Number(serde_yaml::Number::from(2)),
        );
        root.insert(
            Value::String("operating_profile".to_string()),
            Value::String(operating_profile.to_string()),
        );
    }

    let auth = mapping_mut(governance, &["github", "auth"])?;
    set_string(auth, "mode", &github_auth_mode.to_string());
    let repository = mapping_mut(governance, &["github", "repository"])?;
    set_string(repository, "owner", owner);
    set_string(repository, "name", repo);

    let reviewers_map = mapping_mut(governance, &["github", "reviewers"])?;
    set_string(reviewers_map, "product", &reviewers.product);
    set_string(reviewers_map, "architecture", &reviewers.architecture);
    set_string(reviewers_map, "tech_lead", &reviewers.tech_lead);
    set_string(reviewers_map, "qa", &reviewers.qa);
    set_string(reviewers_map, "devops", &reviewers.devops);
    set_string(reviewers_map, "infra", &reviewers.infra);

    let release_gate_primary_login = release_gate_logins
        .first()
        .context("release gate login list must contain at least one login")?;
    let release_gate = mapping_mut(governance, &["github", "release_gate"])?;
    set_string(release_gate, "reviewer_handles", &reviewers.devops);
    set_string(release_gate, "reviewer_logins", &release_gate_logins.join(","));
    set_u64(
        release_gate,
        "approval_quorum",
        release_gate_approval_quorum,
    );

    let immutable_governance = mapping_mut(governance, &["github", "immutable_governance"])?;
    set_string(
        immutable_governance,
        "reviewer_handles",
        &format!("{},{}", reviewers.devops, reviewers.infra),
    );
    set_string(
        immutable_governance,
        "reviewer_logins",
        &format!("{release_gate_primary_login},{reviewer_infra_login}"),
    );
    set_u64(
        immutable_governance,
        "approval_quorum",
        immutable_governance_approval_quorum,
    );

    if let Ok(project) = mapping_mut(governance, &["github", "project"]) {
        set_string(project, "owner", owner);
    }

    let audit = mapping_mut(governance, &["audit"])?;
    set_string(audit, "mode", &audit_mode.to_string());
    let audit_remote = mapping_mut(governance, &["audit", "remote"])?;
    set_string(
        audit_remote,
        "endpoint",
        audit_remote_endpoint.unwrap_or_default(),
    );
    set_string(
        audit_remote,
        "auth_header_env",
        audit_remote_auth_header_env.unwrap_or_default(),
    );
    audit_remote.insert(
        Value::String("timeout_seconds".to_string()),
        Value::Number(serde_yaml::Number::from(
            audit_remote_timeout_seconds.unwrap_or(10),
        )),
    );

    Ok(())
}

fn mapping_mut<'a>(root: &'a mut Value, path: &[&str]) -> Result<&'a mut Mapping> {
    let mut current = root;
    for key in path {
        current = current
            .get_mut(*key)
            .with_context(|| format!("missing governance path {}", path.join(".")))?;
    }
    current
        .as_mapping_mut()
        .with_context(|| format!("governance path {} is not a mapping", path.join(".")))
}

fn set_string(mapping: &mut Mapping, key: &str, value: &str) {
    mapping.insert(
        Value::String(key.to_string()),
        Value::String(value.to_string()),
    );
}

fn set_bool(mapping: &mut Mapping, key: &str, value: bool) {
    mapping.insert(Value::String(key.to_string()), Value::Bool(value));
}

fn set_u64(mapping: &mut Mapping, key: &str, value: u64) {
    mapping.insert(
        Value::String(key.to_string()),
        Value::Number(serde_yaml::Number::from(value)),
    );
}

fn merge_unique_logins(primary: &str, extras: &[String]) -> Vec<String> {
    let mut logins = vec![primary.to_string()];
    for login in extras {
        if !logins.contains(login) {
            logins.push(login.clone());
        }
    }
    logins
}

fn validate_approval_quorum(
    label: &str,
    value: u64,
    reviewer_count: u64,
    max: Option<u64>,
) -> Result<u64> {
    if reviewer_count == 0 {
        bail!("{label} requires at least one configured reviewer login");
    }
    if value == 0 {
        bail!("{label} must be at least 1");
    }
    if value > reviewer_count {
        bail!(
            "{label}={value} exceeds the {reviewer_count} configured reviewer login(s)"
        );
    }
    if let Some(max) = max {
        if value > max {
            bail!(
                "{label}={value} exceeds the GitHub branch-protection maximum of {max} approving reviews"
            );
        }
    }
    Ok(value)
}

fn validate_profile_contract(
    operating_profile: OperatingProfileArg,
    github_auth_mode: GithubAuthModeArg,
    audit_mode: AuditModeArg,
    audit_remote_endpoint: Option<&str>,
    audit_remote_auth_header_env: Option<&str>,
) -> Result<()> {
    if operating_profile == OperatingProfileArg::Enterprise
        && github_auth_mode != GithubAuthModeArg::TokenApi
    {
        bail!("operating_profile=enterprise requires github_auth_mode=token-api");
    }
    if operating_profile == OperatingProfileArg::Enterprise
        && audit_mode != AuditModeArg::Remote
    {
        bail!("operating_profile=enterprise requires audit_mode=remote");
    }
    if audit_mode == AuditModeArg::Remote {
        if audit_remote_endpoint.unwrap_or_default().trim().is_empty() {
            bail!("audit_mode=remote requires --audit-remote-endpoint");
        }
        if audit_remote_auth_header_env
            .unwrap_or_default()
            .trim()
            .is_empty()
        {
            bail!("audit_mode=remote requires --audit-remote-auth-header-env");
        }
    }
    Ok(())
}

fn provision_branch_protection(governance: &Value, branch: &str) -> Result<()> {
    let repo = crate::github_api::repository_full_name(governance)?;
    let endpoint = format!(
        "repos/{repo}/branches/{}/protection",
        urlencoding::encode(branch)
    );
    let require_code_owner_reviews = crate::github_api::yaml_bool(
        governance,
        &["github", "branch_protection", "require_code_owner_review"],
    )
    .unwrap_or(false);
    let require_conversation_resolution = crate::github_api::yaml_bool(
        governance,
        &["github", "branch_protection", "require_resolved_conversations"],
    )
    .unwrap_or(false);
    let require_release_gate_approval = crate::github_api::yaml_bool(
        governance,
        &["github", "branch_protection", "require_release_gate_approval"],
    )
    .unwrap_or(false);

    let review_count = if require_release_gate_approval {
        let reviewer_count = crate::github_api::parse_csv(
            &crate::github_api::yaml_string(
                governance,
                &["github", "release_gate", "reviewer_logins"],
            )
            .unwrap_or_default(),
        )
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>()
        .len() as u64;
        let configured = crate::github_api::yaml_u64(
            governance,
            &["github", "release_gate", "approval_quorum"],
        )
        .unwrap_or(1);
        validate_approval_quorum(
            "github.release_gate.approval_quorum",
            configured,
            reviewer_count,
            Some(MAX_BRANCH_PROTECTION_APPROVAL_QUORUM),
        )?
    } else {
        0
    };
    let body = json!({
        "required_status_checks": null,
        "enforce_admins": false,
        "required_pull_request_reviews": {
            "dismiss_stale_reviews": false,
            "require_code_owner_reviews": require_code_owner_reviews,
            "required_approving_review_count": review_count
        },
        "restrictions": null,
        "required_conversation_resolution": require_conversation_resolution,
        "allow_force_pushes": false,
        "allow_deletions": false,
        "block_creations": false,
        "required_linear_history": false,
        "lock_branch": false,
        "allow_fork_syncing": false
    });
    let _ = crate::github_api::api_put_json(governance, &endpoint, &body)?;
    Ok(())
}

fn provision_labels(governance: &Value) -> Result<()> {
    let repo = crate::github_api::repository_full_name(governance)?;
    let labels = [
        ("role", "1D76DB"),
        ("kind", "8250DF"),
        ("priority", "B60205"),
        ("status", "0E8A16"),
        ("criticality", "D73A4A"),
    ];

    for (key, color) in labels {
        let values = crate::github_api::parse_csv(
            &crate::github_api::yaml_string(governance, &["github", "labels", key])
                .unwrap_or_default(),
        );
        for label in values {
            let endpoint = format!("repos/{repo}/labels");
            let body = json!({
                "name": label,
                "color": color,
                "description": format!("Provisioned by prdtp-agents-functions-cli ({key})")
            });
            let result = crate::github_api::api_post_json(governance, &endpoint, &body);
            if let Err(error) = result {
                let text = format!("{error:#}");
                if !text.contains("already_exists") && !text.contains("Validation Failed") {
                    return Err(error);
                }
                let update_endpoint = format!(
                    "repos/{repo}/labels/{}",
                    urlencoding::encode(body["name"].as_str().unwrap_or_default())
                );
                let _ = crate::github_api::api_patch_json(governance, &update_endpoint, &body)?;
            }
        }
    }
    Ok(())
}

fn contains_placeholder_marker(content: &str) -> bool {
    content.contains("REPLACE_ME")
        || content.contains("@team-")
        || content.contains("github-governance-provisioned")
        || content.contains("TODO")
        || content.contains("<owner>")
        || content.contains("<repo>")
        || content.contains("<release-gate-login>")
        || content.contains("<@")
}

fn render_codeowners(reviewers: &GovernanceReviewers) -> String {
    format!(
        "# CODEOWNERS - Agent-role governance for workspace files.\n\
         # Generated by `prdtp-agents-functions-cli governance configure`.\n\
         # Reviewer mapping:\n\
         #   {product} = product-owner\n\
         #   {architecture} = software-architect\n\
         #   {tech_lead} = tech-lead\n\
         #   {qa} = qa-lead\n\
         #   {devops} = devops-release-engineer\n\
         #   {infra} = pm-orchestrator (infrastructure)\n\
         \n\
         # === Immutable after bootstrap - require explicit approval ===\n\
         .github/CODEOWNERS                        {infra} {devops}\n\
         .github/agents/identity/                 {infra}\n\
         .github/agents/CONTEXT_ZONE_DIVIDER.txt  {infra}\n\
         .github/copilot-instructions.md          {infra}\n\
         .github/PULL_REQUEST_TEMPLATE.md         {infra} {devops}\n\
         .github/ISSUE_TEMPLATE/                  {infra} {product} {tech_lead}\n\
         .github/prompts/clarify-prd.prompt.md    {infra} {product}\n\
         .github/instructions/                    {infra}\n\
         .github/project-governance.md            {infra} {devops} {tech_lead} {product}\n\
         .github/github-governance.yaml           {infra} {devops}\n\
         .github/workflows/pr-governance.yml      {infra} {devops}\n\
         .instructions.md                         {infra}\n\
         AGENTS.md                                {infra} {product}\n\
         \n\
         # === Schema ===\n\
         .state/memory-schema.sql                 {infra}\n\
         \n\
         # === Agent context (mutable via enrich prompts) ===\n\
         .github/agents/context/                  {architecture} {tech_lead}\n\
         \n\
         # === Canonical documentation ===\n\
         docs/project/vision.md                   {product}\n\
         docs/project/scope.md                    {product}\n\
         docs/project/backlog.yaml                {product}\n\
         docs/project/risks.md                    {product}\n\
         docs/project/stakeholders.md             {product}\n\
         docs/project/glossary.md                 {product}\n\
         docs/project/refined-stories.yaml        {tech_lead}\n\
         docs/project/architecture/               {architecture}\n\
         docs/project/decisions/                  {architecture}\n\
         docs/project/quality-gates.yaml          {qa}\n\
         docs/project/qa/                         {qa}\n\
         docs/project/releases.md                 {devops}\n\
         docs/project/releases.yaml               {devops}\n\
         docs/project/board.md                    {infra} {tech_lead}\n\
         docs/project/management-dashboard.md     {infra} {tech_lead} {devops}\n\
         \n\
         # === Operational YAML (multi-owner) ===\n\
         docs/project/handoffs.yaml               {infra}\n\
         docs/project/findings.yaml               {qa}\n\
         \n\
         # === Infrastructure ===\n\
         reporting-ui/                            {devops} {infra} {tech_lead}\n",
        product = reviewers.product,
        architecture = reviewers.architecture,
        tech_lead = reviewers.tech_lead,
        qa = reviewers.qa,
        devops = reviewers.devops,
        infra = reviewers.infra,
    )
}
