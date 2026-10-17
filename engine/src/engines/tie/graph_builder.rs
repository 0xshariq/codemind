use super::model::{TestEntity, TestRelationship, TestRelationshipKind, TestResolution};
use crate::engines::coie::model::{CodeGraph, EntityKind};

pub fn validate(tests: &[TestEntity], relationships: &[TestRelationship]) -> Result<(), String> {
    let ids: std::collections::HashSet<&str> = tests.iter().map(|test| test.id.as_str()).collect();
    if ids.len() != tests.len() {
        return Err("duplicate TIE test ID".into());
    }
    let mut relationship_ids = std::collections::HashSet::new();
    for test in tests {
        test.context.validate().map_err(|error| error.to_string())?;
        if test.id.trim().is_empty()
            || test.name.trim().is_empty()
            || test.path.trim().is_empty()
            || test.line == 0
        {
            return Err("TIE test has invalid identity or source location".into());
        }
        if !crate::source::limits::is_safe_relative(std::path::Path::new(&test.path)) {
            return Err(format!("TIE test has unsafe path: {}", test.path));
        }
    }
    for relationship in relationships {
        relationship
            .context
            .validate()
            .map_err(|error| error.to_string())?;
        if !ids.contains(relationship.test_id.as_str()) {
            return Err(format!(
                "TIE relationship references unknown test: {}",
                relationship.test_id
            ));
        }
        if relationship.id.trim().is_empty() || !relationship_ids.insert(relationship.id.as_str()) {
            return Err("TIE relationship ID is empty or duplicated".into());
        }
        if relationship.evidence_path.trim().is_empty()
            || !crate::source::limits::is_safe_relative(std::path::Path::new(
                &relationship.evidence_path,
            ))
        {
            return Err("TIE relationship has unsafe evidence path".into());
        }
        if let Some(evidence) = &relationship.evidence
            && (evidence.snapshot_id != relationship.context.source_snapshot_id
                || evidence.path != relationship.evidence_path
                || evidence.start_line == 0
                || evidence.end_line < evidence.start_line)
        {
            return Err(format!(
                "TIE evidence does not match relationship context: {}",
                relationship.id
            ));
        }
        if relationship.kind == TestRelationshipKind::Targets
            && relationship.resolution == TestResolution::Resolved
            && relationship.entity_id.is_none()
        {
            return Err(format!(
                "resolved TARGETS relationship has no canonical target ID: {}",
                relationship.id
            ));
        }
        if relationship.kind == TestRelationshipKind::DeclaredBy
            && relationship.resolution == TestResolution::Resolved
            && relationship.entity_id.is_none()
        {
            return Err(format!(
                "resolved DECLARED_BY relationship has no canonical declaring entity ID: {}",
                relationship.id
            ));
        }
    }
    Ok(())
}

pub fn validate_against_code_graph(
    tests: &[TestEntity],
    relationships: &[TestRelationship],
    code_graph: &CodeGraph,
) -> Result<(), String> {
    validate(tests, relationships)?;
    for relationship in relationships {
        let Some(entity_id) = relationship.entity_id.as_deref() else {
            continue;
        };
        let Some(entity) = code_graph
            .entities
            .iter()
            .find(|entity| entity.id == entity_id)
        else {
            return Err(format!(
                "TIE relationship references absent CoIE entity: {entity_id}"
            ));
        };
        if entity.context != relationship.context {
            return Err(format!(
                "TIE relationship crosses CoIE context: {}",
                relationship.id
            ));
        }
        match relationship.kind {
            TestRelationshipKind::DeclaredBy
                if entity.kind != EntityKind::File && entity.kind != EntityKind::Symbol =>
            {
                return Err(format!(
                    "DECLARED_BY target is not a file or symbol: {entity_id}"
                ));
            }
            TestRelationshipKind::Targets if entity.kind != EntityKind::Symbol => {
                return Err(format!("TARGETS target is not a symbol: {entity_id}"));
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_against_code_graph;
    use crate::engines::coie::model::{CodeGraph, EntityKind, GraphEntity, ResolutionStatus};
    use crate::engines::tie::model::{TestEntity, TestStatus};
    use crate::source::{Language, RepositoryContext};

    #[test]
    fn graph_builder_accepts_a_test_with_a_canonical_file_target() {
        let context = RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        };
        let file = GraphEntity {
            id: "file".into(),
            kind: EntityKind::File,
            name: "tests/lib.rs".into(),
            path: "tests/lib.rs".into(),
            line: 1,
            context: context.clone(),
            language: Some(Language::Rust),
            native_kind: Some("file".into()),
            range: None,
            resolution: ResolutionStatus::Resolved,
            evidence: None,
        };
        let test = TestEntity {
            id: "test".into(),
            name: "works".into(),
            path: "tests/lib.rs".into(),
            line: 1,
            framework: "rust".into(),
            status: TestStatus::Supported,
            context: context.clone(),
        };
        assert!(
            validate_against_code_graph(
                &[test],
                &[],
                &CodeGraph {
                    context,
                    entities: vec![file],
                    relationships: Vec::new(),
                    diagnostics: Vec::new()
                }
            )
            .is_ok()
        );
    }
}
