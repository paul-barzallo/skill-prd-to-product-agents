use anyhow::{bail, Result};
use clap::Args;
use colored::Colorize;
use git2::Repository;
use serde_json::json;
use serde::Serialize;
use std::path::Path;

use crate::common::enums::{Role, ValidationStatus};
use crate::common::yaml_ops;

#[derive(Args)]
pub struct FinalizeArgs {
    /// Agent role performing the finalization
    #[arg(long, value_enum)]
    agent_role: Role,
    /// Summary of what was done
    #[arg(long)]
    summary: String,
    /// GitHub issue reference (e.g. GH-42)
    #[arg(long)]
    issue_ref: Option<String>,
    /// Commit message (Conventional Commits format)
    #[arg(long)]
    commit_message: Option<String>,
    /// Files changed (comma-separated)
    #[arg(long)]
    files_changed: Option<String>,
    /// Canonical docs changed (comma-separated)
    #[arg(long)]
    canonical_docs_changed: Option<String>,
    /// Handoff IDs created/updated (comma-separated)
    #[arg(long)]
    handoffs: Option<String>,
    /// Finding IDs created/updated (comma-separated)
    #[arg(long)]
    findings: Option<String>,
    /// Validation status
    #[arg(long, value_enum, default_value = "not-run")]
    validation_status: ValidationStatus,
    /// Optional notes
    #[arg(long)]
    notes: Option<String>,
    /// Auto-stage all changed files
    #[arg(long)]
    auto_stage_all: bool,
}

#[derive(Serialize)]
struct WorkUnitReport {
    timestamp_utc: String,
    agent_role: String,
    issue_ref: String,
    branch: String,
    mode: String,
    summary: String,
    validation_status: String,
    notes: String,
    files_changed: Vec<String>,
    canonical_docs_changed: Vec<String>,
    handoffs_created_or_updated: Vec<String>,
    findings_created_or_updated: Vec<String>,
    result: String,
    commit_hash: String,
    local_history_record: String,
}

pub fn run(workspace: &Path, args: FinalizeArgs) -> Result<()> {
    println!("{}", "=== Finalize Work Unit ===".cyan().bold());

    let files_changed = parse_csv(&args.files_changed);
    let canonical_docs = parse_csv(&args.canonical_docs_changed);
    let handoffs = parse_csv(&args.handoffs);
    let findings = parse_csv(&args.findings);

    let git_mode = match crate::common::capability_contract::policy_enabled(workspace, "git")? {
        Some(false) => false,
        _ => Repository::open(workspace).is_ok() && workspace.join(".git").exists(),
    };

    if git_mode {
        run_git_mode(
            workspace,
            &args,
            &files_changed,
            &canonical_docs,
            &handoffs,
            &findings,
        )
    } else {
        run_local_mode(
            workspace,
            &args,
            &files_changed,
            &canonical_docs,
            &handoffs,
            &findings,
        )
    }
}

fn run_git_mode(
    workspace: &Path,
    args: &FinalizeArgs,
    files_changed: &[String],
    canonical_docs: &[String],
    handoffs: &[String],
    findings: &[String],
) -> Result<()> {
    println!("  Mode: {}", "Git".green());

    let repo = Repository::open(workspace)?;
    let branch = repo
        .head()
        .ok()
        .and_then(|head| head.shorthand().map(String::from))
        .unwrap_or_default();

    if branch == "main" || branch == "develop" {
        write_git_report(
            workspace,
            args,
            &branch,
            files_changed,
            canonical_docs,
            handoffs,
            findings,
            ValidationStatus::Failed,
            "blocked-branch",
            "",
            Some(
                "Cannot finalize on a protected base branch; switch to a task branch first."
                    .to_string(),
            ),
        )?;
        bail!("Cannot finalize on '{branch}' - must be on a task branch");
    }
    if !branch.is_empty() {
        println!("  Branch: {branch}");
    }

    let issue_ref = args.issue_ref.as_deref().unwrap_or("");
    let commit_msg = args.commit_message.as_deref().unwrap_or("");

    if issue_ref.is_empty() {
        bail!("--issue-ref is required in Git mode");
    }
    if commit_msg.is_empty() {
        bail!("--commit-message is required in Git mode");
    }

    let commit_re = regex::Regex::new(
        r"^(feat|fix|chore|docs|test|refactor|ci|perf|style)(\([a-z0-9-]+\))?:\s*(GH-\d+|#\d+)\s+.+",
    )?;
    if !commit_re.is_match(commit_msg) {
        bail!(
            "Invalid commit message format: '{commit_msg}'\nExpected: <type>(<scope>): GH-<id> <description>"
        );
    }

    let mut index = repo.index()?;
    if args.auto_stage_all {
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
    } else {
        for file in files_changed {
            let file_path = Path::new(file);
            if workspace.join(file_path).exists() {
                index.add_path(file_path)?;
            }
        }
    }
    index.write()?;

    println!("  Running pre-commit validation...");
    if let Err(error) = crate::validate::workspace::run(workspace) {
        write_git_report(
            workspace,
            args,
            &branch,
            files_changed,
            canonical_docs,
            handoffs,
            findings,
            ValidationStatus::Failed,
            "validation-failed",
            "",
            Some(note_with_error(
                args.notes.as_deref(),
                &format!("Workspace validation failed before commit creation: {error}"),
            )),
        )?;
        bail!("Workspace validation failed before commit creation: {error}");
    }

    println!("  Running governance enforcement...");
    let all_changed: Vec<String> = {
        let mut changed = files_changed.to_vec();
        changed.extend(canonical_docs.iter().cloned());
        changed
    };
    if let Err(error) = crate::git::pre_commit::run_governance_checks(workspace, &all_changed) {
        write_git_report(
            workspace,
            args,
            &branch,
            files_changed,
            canonical_docs,
            handoffs,
            findings,
            ValidationStatus::Passed,
            "governance-blocked",
            "",
            Some(note_with_error(
                args.notes.as_deref(),
                &format!("Governance checks blocked commit creation: {error}"),
            )),
        )?;
        return Err(error);
    }

    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let head = repo.head()?;
    let parent = head.peel_to_commit()?;
    let sig = repo.signature()?;

    let commit_oid = repo.commit(Some("HEAD"), &sig, &sig, commit_msg, &tree, &[&parent])?;
    let short_hash = &commit_oid.to_string()[..8];
    println!("  Commit: {short_hash}");

    write_git_report(
        workspace,
        args,
        &branch,
        files_changed,
        canonical_docs,
        handoffs,
        findings,
        ValidationStatus::Passed,
        "committed",
        short_hash,
        None,
    )?;
    let _ = crate::audit::events::record_sensitive_action(
        workspace,
        "git.finalize",
        &args.agent_role.to_string(),
        "success",
        json!({
            "mode": "git",
            "branch": branch,
            "issue_ref": issue_ref,
            "commit_hash": short_hash,
            "validation_status": args.validation_status.to_string()
        }),
    );

    println!(
        "{} Work unit finalized - commit {short_hash}",
        "OK:".green().bold()
    );
    Ok(())
}

fn run_local_mode(
    workspace: &Path,
    args: &FinalizeArgs,
    files_changed: &[String],
    canonical_docs: &[String],
    handoffs: &[String],
    findings: &[String],
) -> Result<()> {
    println!("  Mode: {}", "Local-only".yellow());

    let ts = yaml_ops::now_utc_iso();
    let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%S").to_string();

    let history_dir = workspace.join(".state/local-history");
    std::fs::create_dir_all(&history_dir)?;

    let filename = format!("{stamp}-{}.md", args.agent_role);
    let md_content = format!(
        "# Local Change Record\n\n- **Timestamp**: {ts}\n- **Agent**: {}\n- **Summary**: {}\n- **Files**: {}\n- **Canonical docs**: {}\n- **Notes**: {}\n",
        args.agent_role,
        args.summary,
        files_changed.join(", "),
        canonical_docs.join(", "),
        args.notes.as_deref().unwrap_or("")
    );
    std::fs::write(history_dir.join(&filename), &md_content)?;

    let report = WorkUnitReport {
        timestamp_utc: ts,
        agent_role: args.agent_role.to_string(),
        issue_ref: args.issue_ref.clone().unwrap_or_default(),
        branch: String::new(),
        mode: "local-only".to_string(),
        summary: args.summary.clone(),
        validation_status: args.validation_status.to_string(),
        notes: args.notes.clone().unwrap_or_default(),
        files_changed: files_changed.to_vec(),
        canonical_docs_changed: canonical_docs.to_vec(),
        handoffs_created_or_updated: handoffs.to_vec(),
        findings_created_or_updated: findings.to_vec(),
        result: "recorded-local-history".to_string(),
        commit_hash: String::new(),
        local_history_record: format!(".state/local-history/{filename}"),
    };

    write_work_unit_report(workspace, &report)?;
    let _ = crate::audit::events::record_sensitive_action(
        workspace,
        "git.finalize",
        &args.agent_role.to_string(),
        "success",
        json!({
            "mode": "local-only",
            "issue_ref": args.issue_ref.clone().unwrap_or_default(),
            "local_history_record": format!(".state/local-history/{filename}"),
            "validation_status": args.validation_status.to_string()
        }),
    );

    println!(
        "{} Work unit recorded locally - {filename}",
        "OK:".yellow().bold()
    );
    Ok(())
}

fn write_work_unit_report(workspace: &Path, report: &WorkUnitReport) -> Result<()> {
    let dir = workspace.join(".state/work-units");
    std::fs::create_dir_all(&dir)?;

    let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%S").to_string();
    let base = format!("work-unit-{stamp}-{}", report.agent_role);

    let json_path = dir.join(format!("{base}.json"));
    let json = serde_json::to_string_pretty(report)?;
    std::fs::write(&json_path, &json)?;

    let md_path = dir.join(format!("{base}.md"));
    let md = format!(
        "# Work Unit Report\n\n\
         - **Timestamp**: {}\n\
         - **Agent**: {}\n\
         - **Issue**: {}\n\
         - **Branch**: {}\n\
         - **Mode**: {}\n\
         - **Summary**: {}\n\
         - **Validation**: {}\n\
         - **Result**: {}\n\
         - **Commit**: {}\n\
         - **Files changed**: {}\n\
         - **Canonical docs**: {}\n\
         - **Handoffs**: {}\n\
         - **Findings**: {}\n\
         - **Notes**: {}\n",
        report.timestamp_utc,
        report.agent_role,
        report.issue_ref,
        report.branch,
        report.mode,
        report.summary,
        report.validation_status,
        report.result,
        report.commit_hash,
        report.files_changed.join(", "),
        report.canonical_docs_changed.join(", "),
        report.handoffs_created_or_updated.join(", "),
        report.findings_created_or_updated.join(", "),
        report.notes,
    );
    std::fs::write(&md_path, &md)?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_git_report(
    workspace: &Path,
    args: &FinalizeArgs,
    branch: &str,
    files_changed: &[String],
    canonical_docs: &[String],
    handoffs: &[String],
    findings: &[String],
    validation_status: ValidationStatus,
    result: &str,
    commit_hash: &str,
    notes_override: Option<String>,
) -> Result<()> {
    let report = WorkUnitReport {
        timestamp_utc: yaml_ops::now_utc_iso(),
        agent_role: args.agent_role.to_string(),
        issue_ref: args.issue_ref.clone().unwrap_or_default(),
        branch: branch.to_string(),
        mode: "git".to_string(),
        summary: args.summary.clone(),
        validation_status: validation_status.to_string(),
        notes: notes_override.unwrap_or_else(|| args.notes.clone().unwrap_or_default()),
        files_changed: files_changed.to_vec(),
        canonical_docs_changed: canonical_docs.to_vec(),
        handoffs_created_or_updated: handoffs.to_vec(),
        findings_created_or_updated: findings.to_vec(),
        result: result.to_string(),
        commit_hash: commit_hash.to_string(),
        local_history_record: String::new(),
    };
    write_work_unit_report(workspace, &report)
}

fn note_with_error(existing: Option<&str>, error: &str) -> String {
    match existing {
        Some(notes) if !notes.trim().is_empty() => format!("{}\n{}", notes.trim(), error),
        _ => error.to_string(),
    }
}

fn parse_csv(input: &Option<String>) -> Vec<String> {
    match input {
        Some(value) => value
            .split(',')
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .collect(),
        None => Vec::new(),
    }
}
