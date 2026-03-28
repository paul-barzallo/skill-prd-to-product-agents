use crate::model::{
    EdgeStatus, EdgeType, NodeKind, NodeRef, Severity, Snapshot, ValidationFinding,
    ValidationReport, ValidationSummary,
};
use std::collections::{BTreeMap, BTreeSet};

pub fn validate_snapshot(snapshot: &Snapshot, fail_on_warnings: bool) -> ValidationReport {
    let mut findings = Vec::new();
    let mut requirements = BTreeSet::new();
    let mut declaration_paths = BTreeMap::new();

    for edge in &snapshot.trace_edges {
        if edge.source.kind == NodeKind::Requirement {
            requirements.insert(edge.source.id.clone());
            if edge.edge_type == EdgeType::DeclaredIn && edge.target.kind == NodeKind::File {
                declaration_paths.insert(edge.source.id.clone(), edge.target.id.clone());
            }
        }
    }

    for requirement_id in requirements {
        let declaration_path = declaration_paths
            .get(&requirement_id)
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());

        let downstream: Vec<NodeRef> = snapshot
            .trace_edges
            .iter()
            .filter(|edge| {
                edge.source.kind == NodeKind::Requirement
                    && edge.source.id == requirement_id
                    && edge.status == EdgeStatus::Present
                    && matches!(edge.edge_type, EdgeType::Covers | EdgeType::ReferencesArtifact)
            })
            .map(|edge| edge.target.clone())
            .collect();

        if downstream.is_empty() {
            findings.push(ValidationFinding {
                rule: "requirement_coverage".to_string(),
                severity: Severity::Warning,
                message: format!("requirement {requirement_id} has no downstream artifact coverage"),
                source: NodeRef {
                    kind: NodeKind::Requirement,
                    id: requirement_id.clone(),
                },
                evidence_path: declaration_path,
                related: Vec::new(),
            });
        }
    }

    for edge in &snapshot.trace_edges {
        if edge.status != EdgeStatus::Missing {
            continue;
        }

        if !matches!(edge.edge_type, EdgeType::ReferencesFile | EdgeType::ReferencesArtifact) {
            continue;
        }

        findings.push(ValidationFinding {
            rule: "broken_reference".to_string(),
            severity: Severity::Error,
            message: format!(
                "{} references missing {} {}",
                edge.source.id,
                match edge.edge_type {
                    EdgeType::ReferencesFile => "file",
                    EdgeType::ReferencesArtifact => "artifact",
                    _ => "target",
                },
                edge.target.id
            ),
            source: edge.source.clone(),
            evidence_path: edge.evidence.source_path.clone(),
            related: vec![edge.target.clone()],
        });
    }

    findings.sort_by(|left, right| {
        left.severity
            .cmp(&right.severity)
            .then(left.rule.cmp(&right.rule))
            .then(left.message.cmp(&right.message))
    });

    let errors = findings
        .iter()
        .filter(|finding| finding.severity == Severity::Error)
        .count();
    let warnings = findings
        .iter()
        .filter(|finding| finding.severity == Severity::Warning)
        .count();

    ValidationReport {
        fail_on_warnings,
        summary: ValidationSummary { errors, warnings },
        findings,
    }
}
