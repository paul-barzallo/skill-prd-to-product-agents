use crate::cli::IngestArgs;
use crate::embeddings::EmbeddingService;
use crate::model::{ChunkKind, ChunkRecord, FileRecord, FileType, IngestReport, Snapshot, SnapshotStats, SymbolKind, SymbolRecord};
use crate::{store, trace, util, validate};
use anyhow::{Context, Result};
use chrono::Utc;
use ignore::WalkBuilder;
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const SNAPSHOT_SCHEMA_VERSION: &str = "pmem.snapshot.v3";
const MAX_SECTION_CHARS: usize = 1_200;
const MAX_WINDOW_LINES: usize = 40;

pub fn ingest(
    project_root: &Path,
    args: &IngestArgs,
    embedding_service: &EmbeddingService,
) -> Result<(Vec<String>, IngestReport)> {
    let previous_snapshot = if args.force {
        None
    } else {
        store::load_snapshot(project_root)
            .ok()
            .filter(|snapshot| snapshot.schema_version == SNAPSHOT_SCHEMA_VERSION)
    };

    let previous_by_path = previous_snapshot
        .as_ref()
        .map(|snapshot| {
            snapshot
                .files
                .iter()
                .map(|file| (file.path.clone(), file.clone()))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();

    let candidate_files = collect_candidate_files(project_root)?;
    let mut files = Vec::new();
    let mut changed_files = 0usize;
    let mut reused_files = 0usize;
    let mut skipped_files = 0usize;
    let mut seen_paths = BTreeSet::new();

    for path in candidate_files {
        let relative_path = util::to_relative_posix(&path, project_root);
        seen_paths.insert(relative_path.clone());

        let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        if !util::is_probably_text(&bytes) {
            skipped_files += 1;
            continue;
        }

        let hash = util::sha256_hex(&bytes);
        if let Some(previous) = previous_by_path.get(&relative_path) {
            if previous.hash == hash {
                files.push(previous.clone());
                reused_files += 1;
                continue;
            }
        }

        let content = util::normalize_lf(&String::from_utf8_lossy(&bytes));
        let file_type = detect_file_type(&relative_path);
        let title = extract_title(&content);
        let requirement_text = extract_requirement_text(&file_type, &content);
        let reference_text = extract_reference_text(&file_type, &content);
        let requirement_ids = extract_requirement_ids(&requirement_text);
        let requirement_references = extract_requirement_references(&reference_text, &path, project_root);
        let referenced_paths = extract_referenced_paths(&reference_text, &path, project_root);
        let (symbols, imports) = extract_structural_metadata(&file_type, &content);
        let chunks = extract_chunks(&relative_path, &file_type, &content);
        let lines = content.lines().count();

        files.push(FileRecord {
            path: relative_path,
            file_type,
            bytes: bytes.len(),
            lines,
            hash,
            title,
            content,
            chunks,
            requirement_ids,
            requirement_references,
            referenced_paths,
            symbols,
            imports,
        });
        changed_files += 1;
    }

    let deleted_files = previous_by_path
        .keys()
        .filter(|path| !seen_paths.contains(*path))
        .count();

    files.sort_by(|left, right| left.path.cmp(&right.path));

    let trace_edges = trace::build_trace_edges(&files, project_root);
    let requirement_count = files
        .iter()
        .flat_map(|file| file.requirement_ids.iter().cloned())
        .collect::<BTreeSet<_>>()
        .len();

    let snapshot = Snapshot {
        schema_version: SNAPSHOT_SCHEMA_VERSION.to_string(),
        project_root: project_root.display().to_string(),
        generated_at: Utc::now().to_rfc3339(),
        files,
        trace_edges,
        stats: SnapshotStats {
            files_indexed: changed_files + reused_files,
            requirements_detected: requirement_count,
            trace_edges: 0,
            skipped_files,
        },
    };

    let mut snapshot = snapshot;
    snapshot.stats.trace_edges = snapshot.trace_edges.len();

    let validation_report = validate::validate_snapshot(&snapshot, false);
    let chunk_embeddings = embedding_service.build_chunk_embeddings_with_fallback(&snapshot)?;
    let snapshot_path = store::save_snapshot_with_embeddings(
        project_root,
        &snapshot,
        &chunk_embeddings.value,
        &chunk_embeddings.diagnostics.effective_provider,
        chunk_embeddings.diagnostics.effective_model.as_deref(),
    )?;

    let mut warnings = Vec::new();
    if skipped_files > 0 {
        warnings.push(format!("skipped {skipped_files} non-text file(s) during ingest"));
    }
    if deleted_files > 0 {
        warnings.push(format!("removed {deleted_files} file(s) from the snapshot because they no longer exist"));
    }
    if chunk_embeddings.diagnostics.fallback_used {
        warnings.push(format!(
            "ingest fell back from `{}` to `{}`: {}",
            chunk_embeddings.diagnostics.configured_provider,
            chunk_embeddings.diagnostics.effective_provider,
            chunk_embeddings
                .diagnostics
                .fallback_reason
                .as_deref()
                .unwrap_or("primary provider failed")
        ));
    }
    if chunk_embeddings.diagnostics.remote_access {
        warnings.push("ingest used an external embedding provider over the network".to_string());
    }

    Ok((
        warnings,
        IngestReport {
            snapshot_path: snapshot_path.display().to_string(),
            embedding_provider: chunk_embeddings.diagnostics.effective_provider.clone(),
            embedding_model: chunk_embeddings.diagnostics.effective_model.clone(),
            files_indexed: snapshot.stats.files_indexed,
            changed_files,
            reused_files,
            deleted_files,
            skipped_files,
            requirements_detected: snapshot.stats.requirements_detected,
            trace_edges: snapshot.stats.trace_edges,
            validation_findings: validation_report.findings.len(),
        },
    ))
}

fn collect_candidate_files(project_root: &Path) -> Result<Vec<PathBuf>> {
    let mut builder = WalkBuilder::new(project_root);
    builder.hidden(true);
    builder.git_ignore(true);
    builder.git_exclude(true);
    builder.git_global(true);
    builder.standard_filters(true);

    let mut files = Vec::new();
    for entry in builder.build() {
        let entry = entry?;
        let path = entry.into_path();
        if !path.is_file() {
            continue;
        }
        if should_skip_path(&path, project_root) {
            continue;
        }
        files.push(path);
    }

    files.extend(collect_explicit_hidden_files(project_root, ".github")?);
    files.sort();
    files.dedup();

    Ok(files)
}

fn collect_explicit_hidden_files(project_root: &Path, relative_root: &str) -> Result<Vec<PathBuf>> {
    let root = project_root.join(relative_root);
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut builder = WalkBuilder::new(&root);
    builder.standard_filters(false);
    builder.hidden(false);
    builder.parents(true);
    builder.ignore(true);
    builder.git_ignore(true);
    builder.git_exclude(true);
    builder.git_global(true);

    let mut files = Vec::new();
    for entry in builder.build() {
        let entry = entry?;
        let path = entry.into_path();
        if !path.is_file() {
            continue;
        }
        if should_skip_path(&path, project_root) {
            continue;
        }
        files.push(path);
    }

    Ok(files)
}

fn should_skip_path(path: &Path, project_root: &Path) -> bool {
    let relative = util::to_relative_posix(path, project_root);
    relative.starts_with(".git/")
        || relative.starts_with(".project-memory/")
        || relative.contains("/target/")
        || relative.contains("/target-staging/")
}

fn detect_file_type(relative_path: &str) -> FileType {
    let lower = relative_path.to_ascii_lowercase();

    if lower.ends_with("cargo.lock") {
        FileType::Config
    } else if lower.ends_with("readme.md") {
        FileType::Readme
    } else if lower.contains("/decisions/adr-") && lower.ends_with(".md") {
        FileType::Adr
    } else if lower.contains("prd") && lower.ends_with(".md") {
        FileType::Prd
    } else if lower.contains("spec") && lower.ends_with(".md") {
        FileType::Spec
    } else if lower.ends_with(".prompt.md") {
        FileType::Prompt
    } else if lower.ends_with("skill.md") {
        FileType::Skill
    } else if lower.ends_with(".rs") {
        FileType::RustSource
    } else if lower.ends_with(".ts")
        || lower.ends_with(".tsx")
        || lower.ends_with(".js")
        || lower.ends_with(".jsx")
        || lower.ends_with(".py")
        || lower.ends_with(".go")
    {
        FileType::Source
    } else if lower.ends_with(".yaml") || lower.ends_with(".yml") {
        FileType::Yaml
    } else if lower.ends_with(".json") {
        FileType::Json
    } else if lower.ends_with(".toml") {
        FileType::Toml
    } else if lower.ends_with(".ini") || lower.ends_with(".cfg") || lower.ends_with(".config") {
        FileType::Config
    } else if lower.ends_with(".md") {
        FileType::Markdown
    } else if lower.ends_with(".txt") {
        FileType::Text
    } else {
        FileType::OtherText
    }
}

fn extract_title(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let title = trimmed.trim_start_matches('#').trim();
        return Some(util::truncate(title, 120));
    }

    None
}

fn extract_chunks(path: &str, file_type: &FileType, content: &str) -> Vec<ChunkRecord> {
    if content.trim().is_empty() {
        return Vec::new();
    }

    match file_type {
        FileType::Yaml => extract_yaml_chunks(path, content),
        FileType::Prd
        | FileType::Readme
        | FileType::Adr
        | FileType::Spec
        | FileType::Prompt
        | FileType::Skill
        | FileType::Markdown => extract_markdown_chunks(path, content),
        _ => extract_window_chunks(path, content),
    }
}

fn extract_yaml_chunks(path: &str, content: &str) -> Vec<ChunkRecord> {
    let lines: Vec<(usize, &str)> = content.lines().enumerate().map(|(index, line)| (index + 1, line)).collect();
    let mut chunks = Vec::new();
    let mut current_lines: Vec<(usize, &str)> = Vec::new();
    let mut current_title: Option<String> = None;
    let mut root_key: Option<String> = None;
    let mut current_job: Option<String> = None;

    for (line_number, line) in &lines {
        let trimmed = line.trim();
        let indent = yaml_indent_width(line);

        if let Some(anchor_title) = yaml_anchor_title(trimmed, indent, root_key.as_deref(), current_job.as_deref()) {
            if !current_lines.is_empty() {
                push_chunk(path, &mut chunks, ChunkKind::Section, current_title.take(), &current_lines);
                current_lines.clear();
            }
            current_title = Some(anchor_title);
        }

        update_yaml_context(trimmed, indent, &mut root_key, &mut current_job);

        if trimmed.is_empty() && current_lines.is_empty() {
            continue;
        }

        current_lines.push((*line_number, *line));
    }

    if !current_lines.is_empty() {
        push_chunk(path, &mut chunks, ChunkKind::Section, current_title.take(), &current_lines);
    }

    if chunks.is_empty() {
        extract_window_chunks(path, content)
    } else {
        chunks
    }
}

fn yaml_indent_width(line: &str) -> usize {
    line.chars().take_while(|character| *character == ' ').count()
}

fn yaml_anchor_title(
    trimmed: &str,
    indent: usize,
    root_key: Option<&str>,
    current_job: Option<&str>,
) -> Option<String> {
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    if let Some((key, value)) = parse_yaml_mapping(trimmed) {
        if indent == 0 {
            return Some(match value {
                Some(value) if key == "name" && !value.is_empty() => format!("name {value}"),
                _ => key,
            });
        }

        if root_key == Some("jobs") && indent == 2 && value.is_none() {
            return Some(format!("job {key}"));
        }

        if let Some(job) = current_job {
            if indent == 4 && value.is_none() {
                return Some(format!("job {job} > {key}"));
            }
            if indent >= 6 && matches!(key.as_str(), "name" | "uses" | "run" | "if" | "needs" | "path") {
                return Some(match value {
                    Some(value) if !value.is_empty() => format!("job {job} > {key} {value}"),
                    _ => format!("job {job} > {key}"),
                });
            }
        }
    }

    if let Some((key, value)) = parse_yaml_sequence_mapping(trimmed) {
        if let Some(job) = current_job {
            return Some(match key.as_str() {
                "name" => format!("job {job} > step {}", value.unwrap_or_else(|| "unnamed".to_string())),
                "uses" => format!("job {job} > step uses {}", value.unwrap_or_else(|| "action".to_string())),
                "run" => format!("job {job} > step run {}", value.unwrap_or_else(|| "command".to_string())),
                _ => format!("job {job} > step {key}"),
            });
        }
    }

    None
}

fn update_yaml_context(
    trimmed: &str,
    indent: usize,
    root_key: &mut Option<String>,
    current_job: &mut Option<String>,
) {
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return;
    }

    if let Some((key, _)) = parse_yaml_mapping(trimmed) {
        if indent == 0 {
            *root_key = Some(key.clone());
            if key != "jobs" {
                *current_job = None;
            }
            return;
        }

        if root_key.as_deref() == Some("jobs") && indent == 2 {
            *current_job = Some(key);
            return;
        }

        if indent <= 2 && root_key.as_deref() != Some("jobs") {
            *current_job = None;
        }
    }
}

fn parse_yaml_mapping(trimmed: &str) -> Option<(String, Option<String>)> {
    if trimmed.starts_with('-') {
        return None;
    }

    let (raw_key, raw_value) = trimmed.split_once(':')?;
    let key = raw_key.trim();
    if key.is_empty() || key.contains(' ') {
        return None;
    }

    let value = raw_value.trim();
    Some((key.to_string(), if value.is_empty() { None } else { Some(value.to_string()) }))
}

fn parse_yaml_sequence_mapping(trimmed: &str) -> Option<(String, Option<String>)> {
    let stripped = trimmed.strip_prefix('-')?.trim_start();
    let (raw_key, raw_value) = stripped.split_once(':')?;
    let key = raw_key.trim();
    if key.is_empty() || key.contains(' ') {
        return None;
    }

    let value = raw_value.trim();
    Some((key.to_string(), if value.is_empty() { None } else { Some(value.to_string()) }))
}

fn extract_markdown_chunks(path: &str, content: &str) -> Vec<ChunkRecord> {
    let mut chunks = Vec::new();
    let mut current_lines: Vec<(usize, &str)> = Vec::new();
    let mut current_title: Option<String> = None;

    for (line_index, line) in content.lines().enumerate() {
        let line_number = line_index + 1;
        let trimmed = line.trim();
        let is_heading = trimmed.starts_with('#');

        if is_heading && !current_lines.is_empty() {
            flush_markdown_section(path, &mut chunks, &current_lines, current_title.take());
            current_lines.clear();
        }

        if is_heading {
            let title = trimmed.trim_start_matches('#').trim();
            current_title = Some(util::truncate(title, 160));
        }

        current_lines.push((line_number, line));
    }

    if !current_lines.is_empty() {
        flush_markdown_section(path, &mut chunks, &current_lines, current_title.take());
    }

    chunks
}

fn flush_markdown_section(
    path: &str,
    chunks: &mut Vec<ChunkRecord>,
    section_lines: &[(usize, &str)],
    title: Option<String>,
) {
    let mut bucket: Vec<(usize, &str)> = Vec::new();
    let mut bucket_chars = 0usize;

    for (index, line) in section_lines {
        let projected = bucket_chars + line.len() + 1;
        let split_at_blank = line.trim().is_empty() && !bucket.is_empty();
        if projected > MAX_SECTION_CHARS && !bucket.is_empty() {
            push_chunk(path, chunks, ChunkKind::Section, title.clone(), &bucket);
            bucket.clear();
            bucket_chars = 0;
        } else if split_at_blank && bucket_chars >= MAX_SECTION_CHARS / 2 {
            push_chunk(path, chunks, ChunkKind::Section, title.clone(), &bucket);
            bucket.clear();
            bucket_chars = 0;
        }

        bucket.push((*index, *line));
        bucket_chars += line.len() + 1;
    }

    if !bucket.is_empty() {
        push_chunk(path, chunks, ChunkKind::Section, title, &bucket);
    }
}

fn extract_window_chunks(path: &str, content: &str) -> Vec<ChunkRecord> {
    let lines: Vec<(usize, &str)> = content.lines().enumerate().map(|(index, line)| (index + 1, line)).collect();
    let mut chunks = Vec::new();
    let mut start = 0usize;

    while start < lines.len() {
        let end = (start + MAX_WINDOW_LINES).min(lines.len());
        push_chunk(path, &mut chunks, ChunkKind::Window, None, &lines[start..end]);
        start = end;
    }

    chunks
}

fn push_chunk(
    path: &str,
    chunks: &mut Vec<ChunkRecord>,
    kind: ChunkKind,
    title: Option<String>,
    lines: &[(usize, &str)],
) {
    let start_line = lines.first().map(|(line, _)| *line).unwrap_or(1);
    let end_line = lines.last().map(|(line, _)| *line).unwrap_or(start_line);
    let content = lines.iter().map(|(_, line)| *line).collect::<Vec<_>>().join("\n");

    if content.trim().is_empty() {
        return;
    }

    let ordinal = chunks.len();
    let content_hash = util::sha256_hex(content.as_bytes());
    let chunk_id = format!("{path}#chunk-{ordinal:04}");

    chunks.push(ChunkRecord {
        chunk_id,
        kind,
        ordinal,
        title,
        start_line,
        end_line,
        content,
        content_hash,
    });
}

fn extract_requirement_text(file_type: &FileType, content: &str) -> String {
    match file_type {
        FileType::RustSource => scrub_example_requirement_mentions(&extract_rust_comment_text(content)),
        FileType::Source => scrub_example_requirement_mentions(&extract_generic_source_comment_text(content)),
        FileType::Readme
        | FileType::Adr
        | FileType::Spec
        | FileType::Prompt
        | FileType::Skill
        | FileType::Markdown
        | FileType::Prd => strip_markdown_inline_code(&strip_markdown_fenced_code_blocks(content)),
        FileType::Text | FileType::OtherText => content.to_string(),
        FileType::Yaml | FileType::Json | FileType::Toml | FileType::Config => String::new(),
    }
}

fn extract_reference_text(file_type: &FileType, content: &str) -> String {
    match file_type {
        FileType::RustSource => extract_rust_comment_text(content),
        FileType::Source => extract_generic_source_comment_text(content),
        FileType::Readme
        | FileType::Adr
        | FileType::Spec
        | FileType::Prompt
        | FileType::Skill
        | FileType::Markdown
        | FileType::Prd => strip_markdown_fenced_code_blocks(content),
        FileType::Text | FileType::OtherText => content.to_string(),
        FileType::Yaml | FileType::Json | FileType::Toml | FileType::Config => String::new(),
    }
}

fn extract_requirement_ids(content: &str) -> Vec<String> {
    let mut items = BTreeSet::new();
    for capture in requirement_regex().captures_iter(content) {
        items.insert(capture[0].to_ascii_uppercase());
    }
    items.into_iter().collect()
}

fn extract_referenced_paths(content: &str, source_path: &Path, project_root: &Path) -> Vec<String> {
    let mut items = BTreeSet::new();

    for resolved in extract_paths_from_text(content, source_path, project_root) {
        items.insert(resolved);
    }

    items.into_iter().collect()
}

fn extract_requirement_references(
    content: &str,
    source_path: &Path,
    project_root: &Path,
) -> BTreeMap<String, Vec<String>> {
    let mut items: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut active_requirements = BTreeSet::new();
    let document_scope = issue_document_scope(source_path);

    for line in content.lines() {
        if let Some(scope) = &document_scope {
            active_requirements = scope.iter().cloned().collect();
        } else if line.trim().is_empty() {
            active_requirements.clear();
            continue;
        } else {
            let line_requirements = extract_requirement_ids(line)
                .into_iter()
                .collect::<BTreeSet<_>>();

            if !line_requirements.is_empty() {
                active_requirements = line_requirements;
            }
        }

        if document_scope.is_none() && line.trim().is_empty() {
            active_requirements.clear();
            continue;
        }

        if active_requirements.is_empty() {
            continue;
        }

        let resolved_paths = extract_paths_from_text(line, source_path, project_root);
        if resolved_paths.is_empty() {
            continue;
        }

        for requirement_id in &active_requirements {
            let entry = items.entry(requirement_id.clone()).or_default();
            for resolved_path in &resolved_paths {
                entry.insert(resolved_path.clone());
            }
        }
    }

    items
        .into_iter()
        .map(|(requirement_id, paths)| (requirement_id, paths.into_iter().collect()))
        .collect()
}

fn extract_paths_from_text(content: &str, source_path: &Path, project_root: &Path) -> Vec<String> {
    let mut items = BTreeSet::new();

    for capture in path_reference_regex().captures_iter(content) {
        if let Some(path_match) = capture.name("path") {
            if let Some(resolved) = resolve_reference(path_match.as_str(), source_path, project_root) {
                items.insert(resolved);
            }
        }
    }

    items.into_iter().collect()
}

fn resolve_reference(raw: &str, source_path: &Path, project_root: &Path) -> Option<String> {
    let cleaned = raw
        .split('#')
        .next()
        .unwrap_or(raw)
        .trim_start_matches(|ch| matches!(ch, '`' | '(' | ')' | '[' | ']' | '{' | '}' | '"' | '\''))
        .trim_end_matches(|ch| matches!(ch, '`' | '(' | ')' | '[' | ']' | '{' | '}' | '"' | '\'' | '.' | ',' | ';' | ':'));

    if cleaned.is_empty() || cleaned.contains("://") {
        return None;
    }

    let base = source_path.parent().unwrap_or(project_root);
    let candidate = if Path::new(cleaned).is_absolute() {
        util::normalize_path(Path::new(cleaned))
    } else if cleaned.starts_with("./") || cleaned.starts_with("../") {
        util::normalize_path(&base.join(cleaned))
    } else {
        let source_relative = util::normalize_path(&base.join(cleaned));
        if source_relative.starts_with(project_root) && source_relative.exists() {
            source_relative
        } else {
            let project_relative = util::normalize_path(&project_root.join(cleaned));
            if project_relative.starts_with(project_root) {
                project_relative
            } else {
                source_relative
            }
        }
    };

    if candidate.starts_with(project_root) {
        let relative = util::to_relative_posix(&candidate, project_root);
        if should_skip_relative(&relative) {
            None
        } else if candidate.is_file() {
            Some(relative)
        } else if let Some(resolved) = resolve_reference_from_package_roots(cleaned, project_root) {
            Some(resolved)
        } else {
            Some(relative)
        }
    } else if let Some(resolved) = resolve_reference_from_package_roots(cleaned, project_root) {
        Some(resolved)
    } else {
        None
    }
}

fn resolve_reference_from_package_roots(raw: &str, project_root: &Path) -> Option<String> {
    let skill_root = project_root
        .join(".agents")
        .join("skills")
        .join("prd-to-product-agents");
    let workspace_template_root = skill_root.join("templates").join("workspace");

    let candidate_roots = [skill_root.as_path(), workspace_template_root.as_path()];
    for root in candidate_roots {
        let candidate = util::normalize_path(&root.join(raw));
        if candidate.starts_with(project_root) && candidate.is_file() {
            let relative = util::to_relative_posix(&candidate, project_root);
            if !should_skip_relative(&relative) {
                return Some(relative);
            }
        }
    }

    None
}

fn requirement_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\b(?:REQ|FR|NFR|US|STORY|PMEM)-[A-Z0-9][A-Z0-9_-]*\b")
            .expect("valid requirement regex")
    })
}

fn path_reference_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?P<path>(?:\./|\.\./)?(?:[A-Za-z0-9_.-]+/)+[A-Za-z0-9_.-]+\.[A-Za-z0-9_.-]+)")
            .expect("valid path reference regex")
    })
}

fn extract_structural_metadata(file_type: &FileType, content: &str) -> (Vec<SymbolRecord>, Vec<String>) {
    match file_type {
        FileType::RustSource => (extract_rust_symbols(content), extract_rust_imports(content)),
        _ => (Vec::new(), Vec::new()),
    }
}

fn extract_rust_symbols(content: &str) -> Vec<SymbolRecord> {
    let mut symbols = Vec::new();

    for (line_index, line) in content.lines().enumerate() {
        if let Some(captures) = rust_symbol_regex().captures(line) {
            let kind = match captures.name("kind").map(|value| value.as_str()) {
                Some("fn") => SymbolKind::Function,
                Some("struct") => SymbolKind::Struct,
                Some("enum") => SymbolKind::Enum,
                Some("trait") => SymbolKind::Trait,
                Some("mod") => SymbolKind::Module,
                _ => continue,
            };

            if let Some(name) = captures.name("name") {
                symbols.push(SymbolRecord {
                    name: name.as_str().to_string(),
                    kind,
                    line: line_index + 1,
                });
            }
        }
    }

    symbols
}

fn extract_rust_imports(content: &str) -> Vec<String> {
    let mut imports = BTreeSet::new();

    for captures in rust_import_regex().captures_iter(content) {
        if let Some(path) = captures.name("path") {
            imports.insert(path.as_str().trim().to_string());
        }
    }

    imports.into_iter().collect()
}

fn rust_symbol_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?P<kind>fn|struct|enum|trait|mod)\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
        )
        .expect("valid rust symbol regex")
    })
}

fn rust_import_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?m)^\s*use\s+(?P<path>[^;]+);$").expect("valid rust import regex")
    })
}

fn extract_rust_comment_text(content: &str) -> String {
    let mut lines = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim_start();
        if let Some(comment) = trimmed.strip_prefix("///") {
            lines.push(comment.trim_start().to_string());
        } else if let Some(comment) = trimmed.strip_prefix("//") {
            lines.push(comment.trim_start().to_string());
        }
    }

    lines.join("\n")
}

fn extract_generic_source_comment_text(content: &str) -> String {
    let mut lines = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim_start();
        if let Some(comment) = trimmed.strip_prefix("//") {
            lines.push(comment.trim_start().to_string());
        } else if let Some(comment) = trimmed.strip_prefix('#') {
            lines.push(comment.trim_start().to_string());
        }
    }

    lines.join("\n")
}

fn strip_markdown_fenced_code_blocks(content: &str) -> String {
    let mut result = Vec::new();
    let mut in_fence = false;

    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }

        if !in_fence {
            result.push(line);
        }
    }

    result.join("\n")
}

fn strip_markdown_inline_code(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let mut in_code = false;

    for ch in content.chars() {
        if ch == '`' {
            in_code = !in_code;
            continue;
        }

        if !in_code {
            result.push(ch);
        }
    }

    result
}

fn should_skip_relative(relative: &str) -> bool {
    relative.starts_with(".git/")
        || relative.starts_with(".project-memory/")
        || relative.contains("/target/")
        || relative.contains("/target-staging/")
}

fn issue_document_scope(source_path: &Path) -> Option<Vec<String>> {
    let parent_name = source_path.parent()?.file_name()?.to_str()?;
    if !parent_name.eq_ignore_ascii_case("issues") {
        return None;
    }

    let stem = source_path.file_stem()?.to_str()?;
    let matches = extract_requirement_ids(stem);
    if matches.len() == 1 {
        Some(matches)
    } else {
        None
    }
}

fn scrub_example_requirement_mentions(content: &str) -> String {
    example_requirement_regex().replace_all(content, "$prefix").into_owned()
}

fn example_requirement_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)(?P<prefix>\b(?:e\.g\.|eg|for example)\s+)(?:REQ|FR|NFR|US|STORY|PMEM)-[A-Z0-9][A-Z0-9_-]*\b")
            .expect("valid example requirement regex")
    })
}
