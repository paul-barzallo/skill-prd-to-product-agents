use crate::cli::{QueryArgs, RetrieveArgs};
use crate::embeddings;
use crate::model::{ChunkEmbeddingRecord, ChunkRecord, FileRecord, FileType, QueryMatch, QueryReport, RetrieveReport, Snapshot};
use crate::store;
use crate::util;
use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use std::path::Path;
use std::str::FromStr;

const RETRIEVAL_MODE_HYBRID_LEXICAL_EMBEDDING: &str = "hybrid_lexical_embedding";

pub fn run(snapshot: &Snapshot, args: &QueryArgs) -> Result<(Vec<String>, QueryReport)> {
    let filters = QueryFilters::from_query_args(args)?;
    let mut matches = collect_matches(snapshot, &filters);

    matches.sort_by(|left, right| right.score.cmp(&left.score).then(left.path.cmp(&right.path)));
    let total_matches = matches.len();
    matches.truncate(args.limit);

    Ok((
        Vec::new(),
        QueryReport {
            query: args.text.clone(),
            symbol: args.symbol.clone(),
            import: args.import.clone(),
            file_type: filters.file_type.clone().map(|value| value.to_string()),
            path_contains: args.path_contains.clone(),
            total_matches,
            returned_matches: matches.len(),
            results: matches,
        },
    ))
}

pub fn run_retrieve(
    project_root: &Path,
    snapshot: &Snapshot,
    args: &RetrieveArgs,
    embedding_service: &embeddings::EmbeddingService,
) -> Result<(Vec<String>, RetrieveReport)> {
    let filters = QueryFilters::from_retrieve_args(args)?;
    let mut warnings = Vec::new();
    let query_embedding = embedding_service.embed_query_with_fallback(
        filters
            .needle
            .as_deref()
            .context("retrieve requires a text query")?,
    )?;
    if query_embedding.diagnostics.fallback_used {
        warnings.push(format!(
            "retrieve fell back from `{}` to `{}`: {}",
            query_embedding.diagnostics.configured_provider,
            query_embedding.diagnostics.effective_provider,
            query_embedding
                .diagnostics
                .fallback_reason
                .as_deref()
                .unwrap_or("primary provider failed")
        ));
    }
    let (chunk_embeddings, cache_status, chunk_remote_access) = resolve_chunk_embeddings(
        project_root,
        snapshot,
        embedding_service,
        &query_embedding.effective_config,
        &mut warnings,
    )?;
    let mut matches = collect_retrieve_matches(snapshot, &filters, &query_embedding.value, &chunk_embeddings);

    matches.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then(left.path.cmp(&right.path))
            .then(left.start_line.cmp(&right.start_line))
    });
    let total_matches = matches.len();
    matches.truncate(args.limit);

    if query_embedding.diagnostics.remote_access || chunk_remote_access {
        warnings.push("retrieve used an external embedding provider over the network".to_string());
    }

    Ok((
        warnings,
        RetrieveReport {
            query: args.text.clone(),
            retrieval_mode: RETRIEVAL_MODE_HYBRID_LEXICAL_EMBEDDING,
            configured_embedding_provider: query_embedding.diagnostics.configured_provider.clone(),
            configured_embedding_model: query_embedding.diagnostics.configured_model.clone(),
            embedding_provider: query_embedding.diagnostics.effective_provider.clone(),
            embedding_model: query_embedding.diagnostics.effective_model.clone(),
            remote_access: query_embedding.diagnostics.remote_access,
            cost_risk: query_embedding.diagnostics.cost_risk.to_string(),
            cache_status,
            fallback_used: query_embedding.diagnostics.fallback_used,
            fallback_reason: query_embedding.diagnostics.fallback_reason.clone(),
            file_type: filters.file_type.map(|value| value.to_string()),
            path_contains: args.path_contains.clone(),
            total_matches,
            returned_matches: matches.len(),
            results: matches,
        },
    ))
}

#[derive(Clone)]
struct QueryFilters {
    needle: Option<String>,
    symbol_filter: Option<String>,
    import_filter: Option<String>,
    file_type: Option<FileType>,
    path_filter: Option<String>,
}

impl QueryFilters {
    fn from_query_args(args: &QueryArgs) -> Result<Self> {
        if args.limit == 0 {
            bail!("--limit must be greater than zero");
        }

        Ok(Self {
            needle: args.text.as_ref().map(|value| value.to_ascii_lowercase()),
            symbol_filter: args.symbol.as_ref().map(|value| value.to_ascii_lowercase()),
            import_filter: args.import.as_ref().map(|value| value.to_ascii_lowercase()),
            file_type: parse_file_type(&args.file_type)?,
            path_filter: args.path_contains.as_ref().map(|value| value.to_ascii_lowercase()),
        })
    }

    fn from_retrieve_args(args: &RetrieveArgs) -> Result<Self> {
        if args.limit == 0 {
            bail!("--limit must be greater than zero");
        }

        Ok(Self {
            needle: Some(args.text.to_ascii_lowercase()),
            symbol_filter: None,
            import_filter: None,
            file_type: parse_file_type(&args.file_type)?,
            path_filter: args.path_contains.as_ref().map(|value| value.to_ascii_lowercase()),
        })
    }
}

fn parse_file_type(raw: &Option<String>) -> Result<Option<FileType>> {
    match raw {
        Some(value) => Ok(Some(FileType::from_str(value).map_err(anyhow::Error::msg)?)),
        None => Ok(None),
    }
}

fn collect_matches(snapshot: &Snapshot, filters: &QueryFilters) -> Vec<QueryMatch> {
    let mut matches = Vec::new();

    for file in &snapshot.files {
        if let Some(expected) = &filters.file_type {
            if &file.file_type != expected {
                continue;
            }
        }

        if let Some(path_contains) = &filters.path_filter {
            if !file.path.to_ascii_lowercase().contains(path_contains) {
                continue;
            }
        }

        let matching_symbols = match &filters.symbol_filter {
            Some(filter) => file
                .symbols
                .iter()
                .filter(|symbol| symbol.name.to_ascii_lowercase().contains(filter))
                .map(|symbol| symbol.name.clone())
                .collect::<Vec<_>>(),
            None => Vec::new(),
        };

        if filters.symbol_filter.is_some() && matching_symbols.is_empty() {
            continue;
        }

        let matching_imports = match &filters.import_filter {
            Some(filter) => file
                .imports
                .iter()
                .filter(|import| import.to_ascii_lowercase().contains(filter))
                .cloned()
                .collect::<Vec<_>>(),
            None => Vec::new(),
        };

        if filters.import_filter.is_some() && matching_imports.is_empty() {
            continue;
        }

        let base_score = matching_symbols.len() * 3 + matching_imports.len() * 2;

        match &filters.needle {
            Some(value) => {
                let text_matches = build_text_matches(
                    file,
                    value,
                    &matching_symbols,
                    &matching_imports,
                    base_score,
                );

                if text_matches.is_empty() {
                    if matching_symbols.is_empty() && matching_imports.is_empty() {
                        continue;
                    }

                    matches.push(build_file_level_match(
                        file,
                        &matching_symbols,
                        &matching_imports,
                        base_score.max(1),
                    ));
                } else {
                    matches.extend(text_matches);
                }
            }
            None => matches.push(build_file_level_match(
                file,
                &matching_symbols,
                &matching_imports,
                base_score.max(1),
            )),
        }
    }

    matches
}

fn find_match(content: &str, needle: &str) -> Option<(usize, Option<usize>, String)> {
    let lowercase = content.to_ascii_lowercase();
    let first_offset = lowercase.find(needle)?;
    let score = lowercase.matches(needle).count();

    let mut accumulated = 0usize;
    for (index, line) in content.lines().enumerate() {
        let next = accumulated + line.len() + 1;
        if first_offset < next {
            return Some((
                score,
                Some(index + 1),
                util::truncate(line.trim(), 200),
            ));
        }
        accumulated = next;
    }

    Some((score, None, util::truncate(content.trim(), 200)))
}

fn build_text_matches(
    file: &FileRecord,
    needle: &str,
    matching_symbols: &[String],
    matching_imports: &[String],
    base_score: usize,
) -> Vec<QueryMatch> {
    let chunks = if file.chunks.is_empty() {
        vec![ChunkRecord {
            chunk_id: format!("{}#chunk-0000", file.path),
            kind: crate::model::ChunkKind::Window,
            ordinal: 0,
            title: file.title.clone(),
            start_line: 1,
            end_line: file.lines.max(1),
            content: file.content.clone(),
            content_hash: file.hash.clone(),
        }]
    } else {
        file.chunks.clone()
    };

    let mut matches = Vec::new();

    for chunk in chunks {
        let Some((occurrences, relative_line, snippet)) = find_match(&chunk.content, needle) else {
            continue;
        };

        let title_boost = chunk
            .title
            .as_ref()
            .map(|title| title.to_ascii_lowercase().matches(needle).count())
            .unwrap_or(0);
        let score = occurrences * 10 + title_boost * 3 + base_score;
        let absolute_line = relative_line.map(|line| chunk.start_line + line.saturating_sub(1));

        matches.push(QueryMatch {
            path: file.path.clone(),
            file_type: file.file_type.clone(),
            score,
            lexical_score: Some(score as f32),
            semantic_score: None,
            chunk_id: Some(chunk.chunk_id.clone()),
            chunk_kind: Some(chunk.kind.clone()),
            chunk_title: chunk.title.clone(),
            start_line: Some(chunk.start_line),
            end_line: Some(chunk.end_line),
            line_number: absolute_line,
            snippet,
            requirement_ids: file.requirement_ids.clone(),
            symbols: matching_symbols.to_vec(),
            imports: matching_imports.to_vec(),
        });
    }

    matches
}

fn build_file_level_match(
    file: &FileRecord,
    matching_symbols: &[String],
    matching_imports: &[String],
    score: usize,
) -> QueryMatch {
    QueryMatch {
        path: file.path.clone(),
        file_type: file.file_type.clone(),
        score,
        lexical_score: None,
        semantic_score: None,
        chunk_id: None,
        chunk_kind: None,
        chunk_title: None,
        start_line: None,
        end_line: None,
        line_number: None,
        snippet: if !matching_symbols.is_empty() {
            format!("symbol match: {}", matching_symbols.join(", "))
        } else if !matching_imports.is_empty() {
            format!("import match: {}", matching_imports.join(", "))
        } else {
            file.title
                .clone()
                .unwrap_or_else(|| util::truncate(file.content.trim(), 160))
        },
        requirement_ids: file.requirement_ids.clone(),
        symbols: matching_symbols.to_vec(),
        imports: matching_imports.to_vec(),
    }
}

fn collect_retrieve_matches(
    snapshot: &Snapshot,
    filters: &QueryFilters,
    query_embedding: &[f32],
    chunk_embeddings: &BTreeMap<String, ChunkEmbeddingRecord>,
) -> Vec<QueryMatch> {
    let mut matches = Vec::new();
    let Some(needle) = &filters.needle else {
        return matches;
    };

    for file in &snapshot.files {
        if let Some(expected) = &filters.file_type {
            if &file.file_type != expected {
                continue;
            }
        }

        if let Some(path_contains) = &filters.path_filter {
            if !file.path.to_ascii_lowercase().contains(path_contains) {
                continue;
            }
        }

        let chunks = if file.chunks.is_empty() {
            vec![ChunkRecord {
                chunk_id: format!("{}#chunk-0000", file.path),
                kind: crate::model::ChunkKind::Window,
                ordinal: 0,
                title: file.title.clone(),
                start_line: 1,
                end_line: file.lines.max(1),
                content: file.content.clone(),
                content_hash: file.hash.clone(),
            }]
        } else {
            file.chunks.clone()
        };

        for chunk in chunks {
            let lexical = lexical_chunk_score(&chunk, needle);
            let semantic = chunk_embeddings
                .get(&chunk.chunk_id)
                .map(|record| embeddings::cosine_similarity(query_embedding, &record.vector))
                .unwrap_or(0.0);

            if lexical == 0.0 && semantic < 0.18 {
                continue;
            }

            let overall = lexical + semantic * 100.0;
            let (line_number, snippet) = if lexical > 0.0 {
                match find_match(&chunk.content, needle) {
                    Some((_, relative_line, snippet)) => (
                        relative_line.map(|line| chunk.start_line + line.saturating_sub(1)),
                        snippet,
                    ),
                    None => (Some(chunk.start_line), best_chunk_snippet(&chunk)),
                }
            } else {
                (Some(chunk.start_line), best_chunk_snippet(&chunk))
            };

            matches.push(QueryMatch {
                path: file.path.clone(),
                file_type: file.file_type.clone(),
                score: overall.round() as usize,
                lexical_score: Some(lexical),
                semantic_score: Some(semantic),
                chunk_id: Some(chunk.chunk_id.clone()),
                chunk_kind: Some(chunk.kind.clone()),
                chunk_title: chunk.title.clone(),
                start_line: Some(chunk.start_line),
                end_line: Some(chunk.end_line),
                line_number,
                snippet,
                requirement_ids: file.requirement_ids.clone(),
                symbols: Vec::new(),
                imports: Vec::new(),
            });
        }
    }

    matches
}

fn lexical_chunk_score(chunk: &ChunkRecord, needle: &str) -> f32 {
    let body_score = find_match(&chunk.content, needle)
        .map(|(occurrences, _, _)| (occurrences * 10) as f32)
        .unwrap_or(0.0);
    let title_score = chunk
        .title
        .as_ref()
        .map(|title| title.to_ascii_lowercase().matches(needle).count() as f32 * 3.0)
        .unwrap_or(0.0);

    body_score + title_score
}

fn best_chunk_snippet(chunk: &ChunkRecord) -> String {
    chunk
        .content
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(|line| util::truncate(line, 200))
        .unwrap_or_else(|| util::truncate(chunk.content.trim(), 200))
}

fn resolve_chunk_embeddings(
    project_root: &Path,
    snapshot: &Snapshot,
    embedding_service: &embeddings::EmbeddingService,
    effective_config: &crate::config::EmbeddingRuntimeConfig,
    warnings: &mut Vec<String>,
) -> Result<(BTreeMap<String, ChunkEmbeddingRecord>, String, bool)> {
    let persisted = store::load_chunk_embeddings(project_root).unwrap_or_default();
    let expected_chunks = snapshot.files.iter().map(|file| file.chunks.len()).sum::<usize>();
    if persisted.len() == expected_chunks
        && persisted
            .values()
            .all(|record| {
                record.provider == effective_config.provider.as_str()
                    && record.model.as_deref()
                        == effective_config
                            .model
                            .as_deref()
                            .or(effective_config.deployment.as_deref())
            })
    {
        return Ok((persisted, "hit".to_string(), false));
    }

    if persisted.is_empty() {
        warnings.push(
            "retrieve did not find persisted embeddings for the active provider; computing them on demand"
                .to_string(),
        );
        let embeddings = embedding_service.build_chunk_embeddings_for_config(effective_config, snapshot)?;
        persist_recomputed_embeddings(project_root, &embeddings, effective_config, warnings);
        return Ok((
            embeddings
                .into_iter()
                .map(|record| (record.chunk_id.clone(), record))
                .collect(),
            "miss_recomputed".to_string(),
            matches!(
                effective_config.provider,
                embeddings::EmbeddingProviderKind::OpenAiCompatible
            ),
        ));
    } else {
        warnings.push(
            format!(
                "retrieve ignored persisted embeddings because they do not match provider `{}` or the current chunk set",
                effective_config.provider.as_str()
            ),
        );
        let embeddings = embedding_service.build_chunk_embeddings_for_config(effective_config, snapshot)?;
        persist_recomputed_embeddings(project_root, &embeddings, effective_config, warnings);
        return Ok((
            embeddings
                .into_iter()
                .map(|record| (record.chunk_id.clone(), record))
                .collect(),
            "mismatch_recomputed".to_string(),
            matches!(
                effective_config.provider,
                embeddings::EmbeddingProviderKind::OpenAiCompatible
            ),
        ));
    }
}

fn persist_recomputed_embeddings(
    project_root: &Path,
    embeddings: &[ChunkEmbeddingRecord],
    effective_config: &crate::config::EmbeddingRuntimeConfig,
    warnings: &mut Vec<String>,
) {
    if !store::has_database(project_root) {
        return;
    }

    if let Err(error) = store::save_chunk_embeddings(
        project_root,
        embeddings,
        effective_config.provider.as_str(),
        effective_config
            .model
            .as_deref()
            .or(effective_config.deployment.as_deref()),
    ) {
        warnings.push(format!(
            "retrieve recomputed embeddings but could not persist the refreshed cache: {error}"
        ));
    }
}
