use crate::cli::{ImpactArgs, TraceArgs};
use crate::model::{
    EdgeStatus, EdgeType, FileRecord, ImpactReport, NodeKind, NodeRef, Snapshot, TraceEdge,
    TraceEvidence, TraceFilters, TraceReport,
};
use crate::util;
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub fn build_trace_edges(files: &[FileRecord], project_root: &Path) -> Vec<TraceEdge> {
    let mut edges = Vec::new();
    let mut seen = BTreeSet::new();
    let file_set: BTreeSet<String> = files.iter().map(|file| file.path.clone()).collect();
    let mut requirement_mentions: BTreeMap<String, Vec<&FileRecord>> = BTreeMap::new();

    for file in files {
        for requirement_id in &file.requirement_ids {
            requirement_mentions
                .entry(requirement_id.clone())
                .or_default()
                .push(file);
        }
    }

    for (requirement_id, mentioned_files) in requirement_mentions {
        let declaration_path = select_declaration_path(&requirement_id, &mentioned_files);

        for file in mentioned_files {
            let edge_type = if file.path == declaration_path {
                EdgeType::DeclaredIn
            } else if file.file_type.is_code_like() {
                EdgeType::Covers
            } else {
                EdgeType::MentionedIn
            };

            push_edge(
                &mut edges,
                &mut seen,
                TraceEdge {
                    source: NodeRef {
                        kind: NodeKind::Requirement,
                        id: requirement_id.clone(),
                    },
                    target: NodeRef {
                        kind: NodeKind::File,
                        id: file.path.clone(),
                    },
                    edge_type,
                    status: EdgeStatus::Present,
                    evidence: TraceEvidence {
                        source_path: file.path.clone(),
                        detail: "requirement identifier found in file content".to_string(),
                    },
                },
            );
        }
    }

    for file in files {
        for referenced_path in &file.referenced_paths {
            let status = if file_set.contains(referenced_path)
                || project_root.join(referenced_path).is_file()
            {
                EdgeStatus::Present
            } else {
                EdgeStatus::Missing
            };

            push_edge(
                &mut edges,
                &mut seen,
                TraceEdge {
                    source: NodeRef {
                        kind: NodeKind::File,
                        id: file.path.clone(),
                    },
                    target: NodeRef {
                        kind: NodeKind::File,
                        id: referenced_path.clone(),
                    },
                    edge_type: EdgeType::ReferencesFile,
                    status: status.clone(),
                    evidence: TraceEvidence {
                        source_path: file.path.clone(),
                        detail: "file path reference extracted from indexed content".to_string(),
                    },
                },
            );

            for (requirement_id, referenced_paths) in &file.requirement_references {
                if !referenced_paths.iter().any(|path| path == referenced_path) {
                    continue;
                }

                push_edge(
                    &mut edges,
                    &mut seen,
                    TraceEdge {
                        source: NodeRef {
                            kind: NodeKind::Requirement,
                            id: requirement_id.clone(),
                        },
                        target: NodeRef {
                            kind: NodeKind::File,
                            id: referenced_path.clone(),
                        },
                        edge_type: EdgeType::ReferencesArtifact,
                        status: status.clone(),
                        evidence: TraceEvidence {
                            source_path: file.path.clone(),
                            detail: "requirement and artifact reference found in the same file".to_string(),
                        },
                    },
                );
            }
        }
    }

    edges.sort_by(|left, right| {
        left.source
            .id
            .cmp(&right.source.id)
            .then(left.target.id.cmp(&right.target.id))
            .then(left.edge_type.cmp(&right.edge_type))
            .then(left.status.cmp(&right.status))
    });
    edges
}

pub fn trace_report(
    snapshot: &Snapshot,
    project_root: &Path,
    args: &TraceArgs,
) -> Result<(Vec<String>, TraceReport)> {
    let requirement = args
        .requirement
        .as_ref()
        .map(|value| value.trim().to_ascii_uppercase());
    let path = args
        .path
        .as_ref()
        .map(|value| normalize_filter_path(value, project_root));

    let edges: Vec<TraceEdge> = snapshot
        .trace_edges
        .iter()
        .filter(|edge| {
            let matches_requirement = requirement.as_ref().map_or(true, |value| {
                edge.source.kind == NodeKind::Requirement
                    && edge.source.id.eq_ignore_ascii_case(value)
            });
            let matches_path = path.as_ref().map_or(true, |value| {
                matches_file_node(&edge.source, value) || matches_file_node(&edge.target, value)
            });
            matches_requirement && matches_path
        })
        .cloned()
        .collect();

    let unresolved_edges = edges
        .iter()
        .filter(|edge| edge.status == EdgeStatus::Missing)
        .count();

    Ok((
        Vec::new(),
        TraceReport {
            filters: TraceFilters {
                requirement,
                path,
            },
            edge_count: edges.len(),
            unresolved_edges,
            edges,
        },
    ))
}

pub fn impact_report(
    snapshot: &Snapshot,
    project_root: &Path,
    args: &ImpactArgs,
) -> Result<(Vec<String>, ImpactReport)> {
    let normalized_node = normalize_input_node(&args.node, project_root, snapshot);
    let node_kind = if snapshot
        .files
        .iter()
        .any(|file| file.path.eq_ignore_ascii_case(&normalized_node))
    {
        "file".to_string()
    } else {
        "requirement".to_string()
    };

    let mut warnings = Vec::new();
    let edges: Vec<TraceEdge> = snapshot
        .trace_edges
        .iter()
        .filter(|edge| edge.source.id.eq_ignore_ascii_case(&normalized_node) || edge.target.id.eq_ignore_ascii_case(&normalized_node))
        .cloned()
        .collect();

    if edges.is_empty() {
        warnings.push(format!("node '{normalized_node}' has no trace edges in the current snapshot"));
    }

    let mut impacted = BTreeSet::new();
    for edge in &edges {
        if edge.source.id.eq_ignore_ascii_case(&normalized_node) {
            impacted.insert(edge.target.clone());
        }
        if edge.target.id.eq_ignore_ascii_case(&normalized_node) {
            impacted.insert(edge.source.clone());
        }
    }

    Ok((
        warnings,
        ImpactReport {
            node: normalized_node,
            node_kind,
            edge_count: edges.len(),
            impacted_nodes: impacted.into_iter().collect(),
            edges,
        },
    ))
}

fn select_declaration_path(requirement_id: &str, files: &[&FileRecord]) -> String {
    files
        .iter()
        .copied()
        .min_by(|left, right| {
            let left_issue = declaration_rank(requirement_id, left.path.as_str());
            let right_issue = declaration_rank(requirement_id, right.path.as_str());
            let left_rank = if left.file_type.is_requirement_source() { 0 } else { 1 };
            let right_rank = if right.file_type.is_requirement_source() { 0 } else { 1 };

            left_issue
                .cmp(&right_issue)
                .then(left_rank.cmp(&right_rank))
                .then(left.path.cmp(&right.path))
        })
        .map(|file| file.path.clone())
        .unwrap_or_default()
}

fn declaration_rank(requirement_id: &str, path: &str) -> usize {
    let normalized_id = requirement_id.to_ascii_lowercase();
    let normalized_path = path.to_ascii_lowercase();
    let exact_issue_suffix = format!("/issues/{}.md", normalized_id);
    let exact_file_suffix = format!("/{}.md", normalized_id);

    if normalized_path.ends_with(&exact_issue_suffix) {
        0
    } else if normalized_path.ends_with(&exact_file_suffix) {
        1
    } else {
        2
    }
}

fn push_edge(edges: &mut Vec<TraceEdge>, seen: &mut BTreeSet<String>, edge: TraceEdge) {
    let key = format!(
        "{}|{}|{:?}|{:?}|{}",
        edge.source.id, edge.target.id, edge.edge_type, edge.status, edge.evidence.source_path
    );
    if seen.insert(key) {
        edges.push(edge);
    }
}

fn matches_file_node(node: &NodeRef, path: &str) -> bool {
    node.kind == NodeKind::File && node.id.eq_ignore_ascii_case(path)
}

fn normalize_filter_path(path: &Path, project_root: &Path) -> String {
    let candidate = if path.is_absolute() {
        util::normalize_path(path)
    } else {
        util::normalize_path(&project_root.join(path))
    };
    util::to_relative_posix(&candidate, project_root)
}

fn normalize_input_node(input: &str, project_root: &Path, snapshot: &Snapshot) -> String {
    let trimmed = input.trim();
    if snapshot
        .files
        .iter()
        .any(|file| file.path.eq_ignore_ascii_case(trimmed))
    {
        return trimmed.replace('\\', "/");
    }

    if trimmed.contains('/') || trimmed.contains('\\') || trimmed.ends_with(".md") || trimmed.ends_with(".rs") {
        let candidate = util::normalize_path(&project_root.join(trimmed));
        return util::to_relative_posix(&candidate, project_root);
    }

    trimmed.to_ascii_uppercase()
}
