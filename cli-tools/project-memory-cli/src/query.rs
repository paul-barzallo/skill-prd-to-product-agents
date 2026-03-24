use crate::cli::QueryArgs;
use crate::model::{FileType, QueryMatch, QueryReport, Snapshot};
use crate::util;
use anyhow::{bail, Result};
use std::str::FromStr;

pub fn run(snapshot: &Snapshot, args: &QueryArgs) -> Result<(Vec<String>, QueryReport)> {
    let file_type = match &args.file_type {
        Some(value) => Some(FileType::from_str(value).map_err(anyhow::Error::msg)?),
        None => None,
    };

    if args.limit == 0 {
        bail!("--limit must be greater than zero");
    }

    let needle = args.text.as_ref().map(|value| value.to_ascii_lowercase());
    let symbol_filter = args.symbol.as_ref().map(|value| value.to_ascii_lowercase());
    let import_filter = args.import.as_ref().map(|value| value.to_ascii_lowercase());
    let path_filter = args.path_contains.as_ref().map(|value| value.to_ascii_lowercase());

    let mut matches = Vec::new();

    for file in &snapshot.files {
        if let Some(expected) = &file_type {
            if &file.file_type != expected {
                continue;
            }
        }

        if let Some(path_contains) = &path_filter {
            if !file.path.to_ascii_lowercase().contains(path_contains) {
                continue;
            }
        }

        let matching_symbols = match &symbol_filter {
            Some(filter) => file
                .symbols
                .iter()
                .filter(|symbol| symbol.name.to_ascii_lowercase().contains(filter))
                .map(|symbol| symbol.name.clone())
                .collect::<Vec<_>>(),
            None => Vec::new(),
        };

        if symbol_filter.is_some() && matching_symbols.is_empty() {
            continue;
        }

        let matching_imports = match &import_filter {
            Some(filter) => file
                .imports
                .iter()
                .filter(|import| import.to_ascii_lowercase().contains(filter))
                .cloned()
                .collect::<Vec<_>>(),
            None => Vec::new(),
        };

        if import_filter.is_some() && matching_imports.is_empty() {
            continue;
        }

        let (mut score, line_number, snippet) = match &needle {
            Some(value) => match find_match(&file.content, value) {
                Some(result) => result,
                None => {
                    if !matching_symbols.is_empty() {
                        (
                            3,
                            None,
                            format!("symbol match: {}", matching_symbols.join(", ")),
                        )
                    } else if !matching_imports.is_empty() {
                        (
                            2,
                            None,
                            format!("import match: {}", matching_imports.join(", ")),
                        )
                    } else {
                        continue;
                    }
                }
            },
            None => (
                1,
                None,
                if !matching_symbols.is_empty() {
                    format!("symbol match: {}", matching_symbols.join(", "))
                } else if !matching_imports.is_empty() {
                    format!("import match: {}", matching_imports.join(", "))
                } else {
                    file.title
                        .clone()
                        .unwrap_or_else(|| util::truncate(file.content.trim(), 160))
                },
            ),
        };

        if !matching_symbols.is_empty() {
            score += matching_symbols.len() * 3;
        }
        if !matching_imports.is_empty() {
            score += matching_imports.len() * 2;
        }

        matches.push(QueryMatch {
            path: file.path.clone(),
            file_type: file.file_type.clone(),
            score,
            line_number,
            snippet,
            requirement_ids: file.requirement_ids.clone(),
            symbols: matching_symbols,
            imports: matching_imports,
        });
    }

    matches.sort_by(|left, right| right.score.cmp(&left.score).then(left.path.cmp(&right.path)));
    let total_matches = matches.len();
    matches.truncate(args.limit);

    Ok((
        Vec::new(),
        QueryReport {
            query: args.text.clone(),
            symbol: args.symbol.clone(),
            import: args.import.clone(),
            file_type: file_type.map(|value| value.to_string()),
            path_contains: args.path_contains.clone(),
            total_matches,
            returned_matches: matches.len(),
            results: matches,
        },
    ))
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
