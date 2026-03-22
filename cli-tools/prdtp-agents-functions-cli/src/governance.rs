use anyhow::{bail, Context, Result};
use chrono::Utc;
use clap::Args;
use colored::Colorize;
use prdtp_agents_shared::yaml_ops;
use serde::{Deserialize, Serialize};
use serde_yaml::{Mapping, Value};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[derive(Args)]
pub struct ImmutableTokenArgs {
    /// Reason for requesting edit access
    #[arg(long)]
    pub reason: String,
    /// Files to unlock (relative to workspace root)
    #[arg(long, num_args = 1..)]
    pub files: Vec<String>,
    /// Who is requesting
    #[arg(long, default_value = "agent")]
    pub author: String,
}

#[derive(Args)]
pub struct ConfigureArgs {
    #[arg(long)]
    pub owner: String,
    #[arg(long)]
    pub repo: String,
    #[arg(long)]
    pub release_gate_login: String,
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
}

#[derive(Serialize, Deserialize)]
struct ImmutableToken {
    author: String,
    reason: String,
    files: Vec<String>,
    created_epoch: i64,
    expires_epoch: i64,
    integrity: String,
}

struct GovernanceReviewers {
    product: String,
    architecture: String,
    tech_lead: String,
    qa: String,
    devops: String,
    infra: String,
}

const TOKEN_TTL_SECS: i64 = 3600;

fn compute_token_integrity(
    author: &str,
    reason: &str,
    files: &[String],
    created: i64,
    expires: i64,
    workspace: &Path,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(author.as_bytes());
    hasher.update(b"|");
    hasher.update(reason.as_bytes());
    hasher.update(b"|");
    for f in files {
        hasher.update(f.as_bytes());
        hasher.update(b",");
    }
    hasher.update(b"|");
    hasher.update(created.to_le_bytes());
    hasher.update(expires.to_le_bytes());
    hasher.update(b"|");
    let ws_id = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf())
        .to_string_lossy()
        .to_string();
    hasher.update(ws_id.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn verify_token_integrity(token_json: &str, workspace: &Path) -> bool {
    let token: ImmutableToken = match serde_json::from_str(token_json) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let expected = compute_token_integrity(
        &token.author,
        &token.reason,
        &token.files,
        token.created_epoch,
        token.expires_epoch,
        workspace,
    );
    token.integrity == expected
}

pub fn run_immutable_token(workspace: &Path, args: ImmutableTokenArgs) -> Result<()> {
    tracing::info!(
        workspace = %workspace.display(),
        author = %args.author,
        file_count = args.files.len(),
        "requesting immutable-edit token"
    );
    if args.files.is_empty() {
        bail!("Must specify at least one file to unlock with --files");
    }
    if args.reason.trim().is_empty() {
        bail!("Must provide a non-empty --reason");
    }

    for f in &args.files {
        let full = workspace.join(f);
        if !full.exists() {
            bail!("Target file does not exist: {f}");
        }
    }

    let immutable_set = load_immutable_manifest(workspace)?;
    let unauthorized: Vec<&String> = args
        .files
        .iter()
        .filter(|file| !immutable_set.contains(file.as_str()))
        .collect();
    if !unauthorized.is_empty() {
        let files = unauthorized
            .iter()
            .map(|file| format!("  {file}"))
            .collect::<Vec<_>>()
            .join("\n");
        bail!(
            "Immutable-edit tokens may only cover files listed in .github/immutable-files.txt:\n{}",
            files
        );
    }

    let now = Utc::now().timestamp();
    let expires = now + TOKEN_TTL_SECS;
    let integrity = compute_token_integrity(
        &args.author,
        &args.reason,
        &args.files,
        now,
        expires,
        workspace,
    );
    let token = ImmutableToken {
        author: args.author,
        reason: args.reason,
        files: args.files,
        created_epoch: now,
        expires_epoch: expires,
        integrity,
    };

    let state_dir = workspace.join(".state");
    fs::create_dir_all(&state_dir)?;

    let token_path = state_dir.join(".immutable-edit-token");
    let json = serde_json::to_string_pretty(&token)?;
    fs::write(&token_path, &json)?;

    let expires_utc = chrono::DateTime::from_timestamp(token.expires_epoch, 0)
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
        .unwrap_or_else(|| "unknown".to_string());

    println!("{} Immutable-edit token created", "✓".green().bold());
    println!("  Token: .state/.immutable-edit-token");
    println!("  TTL: {TOKEN_TTL_SECS}s (expires: {expires_utc})");
    println!("  Files unlocked:");
    for f in &token.files {
        println!("    - {f}");
    }

    tracing::warn!(
        token_path = %token_path.display(),
        expires_utc = %expires_utc,
        files = ?token.files,
        "immutable-edit token created"
    );

    Ok(())
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
    let release_gate_login = normalize_login("release-gate-login", &args.release_gate_login)?;
    let reviewers = GovernanceReviewers {
        product: normalize_handle("reviewer-product", &args.reviewer_product)?,
        architecture: normalize_handle("reviewer-architecture", &args.reviewer_architecture)?,
        tech_lead: normalize_handle("reviewer-tech-lead", &args.reviewer_tech_lead)?,
        qa: normalize_handle("reviewer-qa", &args.reviewer_qa)?,
        devops: normalize_handle("reviewer-devops", &args.reviewer_devops)?,
        infra: normalize_handle("reviewer-infra", &args.reviewer_infra)?,
    };

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

    update_governance_value(&mut governance, &owner, &repo, &release_gate_login, &reviewers)?;
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

    println!("{} Governance configured", "✓".green().bold());
    println!("  Repository: {owner}/{repo}");
    println!("  Release gate login: {release_gate_login}");
    println!("  Readiness status: configured");
    println!("  Note: production-ready remains a separate manual transition.");

    Ok(())
}

fn load_immutable_manifest(workspace: &Path) -> Result<HashSet<String>> {
    let immutable_list_path = workspace.join(".github/immutable-files.txt");
    if !immutable_list_path.exists() {
        return Ok(HashSet::new());
    }
    let immutable_content = fs::read_to_string(&immutable_list_path)?;
    Ok(immutable_content
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| line.trim().to_string())
        .collect())
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
    release_gate_login: &str,
    reviewers: &GovernanceReviewers,
) -> Result<()> {
    let readiness = mapping_mut(governance, &["readiness"])?;
    set_string(
        readiness,
        "notes",
        "Governance configured locally via prdtp-agents-functions-cli governance configure. production-ready remains a separate manual transition.",
    );
    set_string(readiness, "status", "configured");

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

    let release_gate = mapping_mut(governance, &["github", "release_gate"])?;
    set_string(release_gate, "reviewer_handles", &reviewers.devops);
    set_string(release_gate, "reviewer_logins", release_gate_login);

    if let Ok(project) = mapping_mut(governance, &["github", "project"]) {
        set_string(project, "owner", owner);
    }

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
