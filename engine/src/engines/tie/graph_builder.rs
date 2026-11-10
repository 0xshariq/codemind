use super::model::{
    TestEntity, TestNodeKind, TestRelationship, TestRelationshipKind, TestResolution,
    TestTargetKind,
};
use crate::engines::coie::model::{CodeGraph, EntityKind};

pub fn validate(tests: &[TestEntity], relationships: &[TestRelationship]) -> Result<(), String> {
    let mut ids = std::collections::HashMap::<&str, &TestEntity>::with_capacity(tests.len());
    for test in tests {
        if let Some(first) = ids.insert(test.id.as_str(), test) {
            return Err(format!(
                "duplicate TIE test ID `{}`: first `{}` at {}:{} ({:?}), repeated `{}` at {}:{} ({:?})",
                test.id,
                first.name,
                first.path,
                first.line,
                first.kind,
                test.name,
                test.path,
                test.line,
                test.kind,
            ));
        }
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
        let Some(source) = tests.iter().find(|test| test.id == relationship.test_id) else {
            return Err(format!(
                "TIE relationship references unknown source node: {}",
                relationship.test_id
            ));
        };
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
        let requires_resolved_target = !matches!(
            relationship.kind,
            TestRelationshipKind::Targets
                | TestRelationshipKind::DeclaredBy
                | TestRelationshipKind::SetupBy
                | TestRelationshipKind::TeardownBy
        ) || relationship.resolution == TestResolution::Resolved;
        if requires_resolved_target && relationship.entity_id.is_none() {
            return Err(format!(
                "resolved TIE relationship has no target ID: {}",
                relationship.id
            ));
        }
        match relationship.kind {
            TestRelationshipKind::Contains => {
                if source.kind != TestNodeKind::TestSuite
                    || relationship.target_kind != TestTargetKind::Test
                {
                    return Err(format!(
                        "CONTAINS has invalid endpoint kinds: {}",
                        relationship.id
                    ));
                }
            }
            TestRelationshipKind::UsesFixture => {
                if !matches!(source.kind, TestNodeKind::Test | TestNodeKind::TestSuite)
                    || relationship.target_kind != TestTargetKind::Test
                {
                    return Err(format!(
                        "USES_FIXTURE has invalid endpoint kinds: {}",
                        relationship.id
                    ));
                }
            }
            TestRelationshipKind::DependsOn => {
                if source.kind != TestNodeKind::Fixture
                    || relationship.target_kind != TestTargetKind::Test
                {
                    return Err(format!(
                        "DEPENDS_ON has invalid endpoint kinds: {}",
                        relationship.id
                    ));
                }
            }
            TestRelationshipKind::HasCase => {
                if source.kind != TestNodeKind::Test
                    || relationship.target_kind != TestTargetKind::Test
                {
                    return Err(format!(
                        "HAS_CASE has invalid endpoint kinds: {}",
                        relationship.id
                    ));
                }
            }
            TestRelationshipKind::DeclaredBy | TestRelationshipKind::Targets => {
                if relationship.target_kind != TestTargetKind::Code {
                    return Err(format!(
                        "code relationship has non-code target: {}",
                        relationship.id
                    ));
                }
                if relationship.kind == TestRelationshipKind::Targets
                    && source.kind != TestNodeKind::Test
                {
                    return Err(format!(
                        "TARGETS source is not a Test node: {}",
                        relationship.id
                    ));
                }
                if relationship.kind == TestRelationshipKind::DeclaredBy
                    && source.kind != TestNodeKind::Test
                {
                    return Err(format!(
                        "DECLARED_BY source is not a Test node: {}",
                        relationship.id
                    ));
                }
            }
            TestRelationshipKind::SetupBy | TestRelationshipKind::TeardownBy => {
                if !matches!(source.kind, TestNodeKind::Test | TestNodeKind::TestSuite)
                    || relationship.target_kind != TestTargetKind::Code
                {
                    return Err(format!(
                        "hook relationship has invalid endpoint kinds: {}",
                        relationship.id
                    ));
                }
            }
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
            if relationship.target_kind == TestTargetKind::Code {
                continue;
            }
            return Err(format!(
                "TIE relationship is missing a Test Graph target: {}",
                relationship.id
            ));
        };
        if relationship.target_kind != TestTargetKind::Code {
            let Some(target) = tests.iter().find(|test| test.id == entity_id) else {
                return Err(format!(
                    "TIE relationship references absent Test Graph entity: {entity_id}"
                ));
            };
            if target.context != relationship.context {
                return Err(format!(
                    "TIE relationship crosses Test Graph context: {}",
                    relationship.id
                ));
            }
            match relationship.kind {
                TestRelationshipKind::Contains => {
                    if !matches!(
                        target.kind,
                        TestNodeKind::TestSuite | TestNodeKind::Test | TestNodeKind::TestCase
                    ) {
                        return Err(format!("CONTAINS target has invalid kind: {entity_id}"));
                    }
                }
                TestRelationshipKind::UsesFixture | TestRelationshipKind::DependsOn => {
                    if target.kind != TestNodeKind::Fixture {
                        return Err(format!(
                            "fixture relationship target is not a Fixture: {entity_id}"
                        ));
                    }
                }
                TestRelationshipKind::HasCase if target.kind != TestNodeKind::TestCase => {
                    return Err(format!("HAS_CASE target is not a TestCase: {entity_id}"));
                }
                _ => {}
            }
            continue;
        }
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
            TestRelationshipKind::DeclaredBy => {
                if entity.kind != EntityKind::File && entity.kind != EntityKind::Symbol {
                    return Err(format!(
                        "DECLARED_BY target is not a file or symbol: {entity_id}"
                    ));
                }
            }
            TestRelationshipKind::Targets
            | TestRelationshipKind::SetupBy
            | TestRelationshipKind::TeardownBy
                if entity.kind != EntityKind::Symbol =>
            {
                return Err(format!(
                    "code test relationship target is not a symbol: {entity_id}"
                ));
            }
            _ => {}
        }
    }
    Ok(())
}
