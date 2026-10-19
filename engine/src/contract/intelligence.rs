use super::{
    context::RepositoryContext,
    error::ContractError,
    result::{CanonicalResult, MAX_PATH_LENGTH, MAX_PATHS, MAX_RELATIONSHIPS},
};
use crate::engines::{
    chie::model::ChangeAnalysis,
    coie::model::CodeGraph,
    tie::model::{TestEntity, TestRelationship},
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CoieQueryKind {
    SearchSymbols,
    GetSymbol,
    QueryRelationships,
}

/// Bounded semantic query request for CoIE. The fixed enum prevents callers
/// from treating a query string as arbitrary Cypher or arbitrary engine code.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoieQueryRequest {
    pub context: RepositoryContext,
    pub kind: CoieQueryKind,
    pub query: String,
    pub limit: usize,
}

impl CoieQueryRequest {
    pub fn validate(&self) -> Result<(), ContractError> {
        self.context.validate()?;
        if self.query.trim().is_empty() {
            return Err(ContractError::InvalidInput(
                "CoIE query is required".to_owned(),
            ));
        }
        if self.query.len() > 4_096 {
            return Err(ContractError::LimitExceeded(
                "CoIE query exceeds 4096 bytes".to_owned(),
            ));
        }
        validate_result_limit(self.limit, "CoIE limit")
    }
}

/// Bounded TIE query request. An empty target list means a bounded repository-
/// context test discovery query; populated targets must be canonical IDs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TieQueryRequest {
    pub context: RepositoryContext,
    pub target_entity_ids: Vec<String>,
    pub limit: usize,
}

impl TieQueryRequest {
    pub fn validate(&self) -> Result<(), ContractError> {
        self.context.validate()?;
        validate_result_limit(self.limit, "TIE limit")?;
        if self.target_entity_ids.len() > MAX_RELATIONSHIPS {
            return Err(ContractError::LimitExceeded(
                "TIE target list exceeds the bounded request limit".to_owned(),
            ));
        }
        let mut seen = HashSet::with_capacity(self.target_entity_ids.len());
        for id in &self.target_entity_ids {
            if id.trim().is_empty() || !seen.insert(id.as_str()) {
                return Err(ContractError::InvalidInput(
                    "TIE targets must be non-empty unique canonical IDs".to_owned(),
                ));
            }
        }
        Ok(())
    }
}

/// Change-intelligence request: unlike a generic engine dispatch message, this
/// explicitly requires a selected symbol/file and bounded propagation limits.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangeAnalysisRequest {
    pub context: RepositoryContext,
    pub target_entity_id: String,
    pub max_depth: usize,
    pub max_results: usize,
}

impl ChangeAnalysisRequest {
    pub fn validate(&self) -> Result<(), ContractError> {
        self.context.validate()?;
        if self.target_entity_id.trim().is_empty() {
            return Err(ContractError::InvalidInput(
                "a selected symbol or file ID is required for change analysis".to_owned(),
            ));
        }
        if self.max_depth > MAX_PATH_LENGTH {
            return Err(ContractError::LimitExceeded(format!(
                "max_depth exceeds the supported limit of {MAX_PATH_LENGTH}"
            )));
        }
        validate_result_limit(self.max_results, "change-analysis result limit")
    }

    pub fn validate_for_context(&self, expected: &RepositoryContext) -> Result<(), ContractError> {
        self.validate()?;
        self.context.ensure_matches(expected)?;
        Ok(())
    }
}

/// Compatibility name for existing internal references. Its contract is the
/// explicit change-analysis request above, not arbitrary engine dispatch.
pub type IntelligenceRequest = ChangeAnalysisRequest;

fn validate_result_limit(limit: usize, label: &str) -> Result<(), ContractError> {
    if limit == 0 || limit > MAX_RELATIONSHIPS {
        return Err(ContractError::LimitExceeded(format!(
            "{label} must be between 1 and {MAX_RELATIONSHIPS}"
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoieResult {
    pub graph: CodeGraph,
}

impl CoieResult {
    pub fn validate(&self, expected: &RepositoryContext) -> Result<(), ContractError> {
        self.graph.context.ensure_matches(expected)?;
        let mut entity_ids = HashSet::with_capacity(self.graph.entities.len());
        for entity in &self.graph.entities {
            entity.context.ensure_matches(expected)?;
            if entity.id.trim().is_empty()
                || entity.name.trim().is_empty()
                || entity.path.trim().is_empty()
                || entity.path.contains('\\')
                || !crate::source::limits::is_safe_relative(std::path::Path::new(&entity.path))
                || (entity.kind == crate::engines::coie::model::EntityKind::Symbol
                    && entity.line == 0)
            {
                return Err(ContractError::InvalidInput(
                    "CoIE graph contains an entity with an invalid ID, name, path, or source line"
                        .to_owned(),
                ));
            }
            if !entity_ids.insert(entity.id.as_str()) {
                return Err(ContractError::Conflict(format!(
                    "duplicate CoIE entity ID: {}",
                    entity.id
                )));
            }
        }

        let mut relationship_ids = HashSet::with_capacity(self.graph.relationships.len());
        for relationship in &self.graph.relationships {
            relationship.context.ensure_matches(expected)?;
            if relationship.id.trim().is_empty()
                || relationship.from.trim().is_empty()
                || relationship.to.trim().is_empty()
            {
                return Err(ContractError::InvalidInput(
                    "CoIE graph contains an incomplete relationship".to_owned(),
                ));
            }
            if !relationship_ids.insert(relationship.id.as_str()) {
                return Err(ContractError::Conflict(format!(
                    "duplicate CoIE relationship ID: {}",
                    relationship.id
                )));
            }
            if !entity_ids.contains(relationship.from.as_str())
                || !entity_ids.contains(relationship.to.as_str())
            {
                return Err(ContractError::InvalidInput(format!(
                    "CoIE relationship `{}` references an entity absent from the graph",
                    relationship.id
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TieResult {
    pub tests: Vec<TestEntity>,
    pub relationships: Vec<TestRelationship>,
}

impl TieResult {
    pub fn validate(&self, expected: &RepositoryContext) -> Result<(), ContractError> {
        // TieResult is the Type-A graph-construction output used by the
        // investigation pipeline. It is not a bounded Type-B query result.
        // Applying the semantic MAX_RELATIONSHIPS-derived result cap here
        // makes otherwise valid repositories fail merely because they contain
        // more tests than the query-result budget. Bounded semantic reads are
        // enforced by TieQueryRequest and the storage/query boundaries instead.
        let mut test_ids = HashSet::with_capacity(self.tests.len());
        for test in &self.tests {
            test.context.ensure_matches(expected)?;
            if test.id.trim().is_empty()
                || test.name.trim().is_empty()
                || test.path.trim().is_empty()
                || test.path.contains('\\')
                || !crate::source::limits::is_safe_relative(std::path::Path::new(&test.path))
                || test.line == 0
                || test.framework.trim().is_empty()
            {
                return Err(ContractError::InvalidInput(
                    "TIE result contains a test with invalid identity, source path/line, or framework".to_owned(),
                ));
            }
            if !test_ids.insert(test.id.as_str()) {
                return Err(ContractError::Conflict(format!(
                    "duplicate TIE test ID: {}",
                    test.id
                )));
            }
        }
        for relationship in &self.relationships {
            relationship.context.ensure_matches(expected)?;
            if relationship.test_id.trim().is_empty()
                || !test_ids.contains(relationship.test_id.as_str())
            {
                return Err(ContractError::InvalidInput(
                    "TIE relationship references an unknown test".to_owned(),
                ));
            }
            if relationship.evidence_path.trim().is_empty()
                || relationship.evidence_path.contains('\\')
                || !crate::source::limits::is_safe_relative(std::path::Path::new(
                    &relationship.evidence_path,
                ))
            {
                return Err(ContractError::InvalidInput(
                    "TIE relationship requires a safe source path as declaration/mapping evidence"
                        .to_owned(),
                ));
            }
            if relationship
                .target_symbol_name
                .as_deref()
                .is_some_and(|target| target.trim().is_empty())
            {
                return Err(ContractError::InvalidInput(
                    "TIE target name cannot be empty when supplied".to_owned(),
                ));
            }
            if let Some(evidence) = &relationship.evidence
                && (evidence.snapshot_id != relationship.context.source_snapshot_id
                    || evidence.path != relationship.evidence_path
                    || evidence.start_line == 0
                    || evidence.end_line < evidence.start_line)
            {
                return Err(ContractError::InvalidInput(
                    "TIE relationship evidence does not match its context/path/range".to_owned(),
                ));
            }
            if relationship.kind == crate::engines::tie::model::TestRelationshipKind::Targets
                && relationship.resolution == crate::engines::tie::model::TestResolution::Resolved
                && relationship.entity_id.is_none()
            {
                return Err(ContractError::InvalidInput(
                    "resolved TIE TARGETS relationship requires a canonical target entity ID"
                        .to_owned(),
                ));
            }
            if relationship.kind == crate::engines::tie::model::TestRelationshipKind::DeclaredBy
                && relationship.resolution == crate::engines::tie::model::TestResolution::Resolved
                && relationship.entity_id.is_none()
            {
                return Err(ContractError::InvalidInput(
                    "resolved TIE DECLARED_BY relationship requires a canonical declaring entity ID".to_owned(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChieResult {
    pub analysis: ChangeAnalysis,
}

impl ChieResult {
    pub fn validate(&self, expected: &RepositoryContext) -> Result<(), ContractError> {
        self.analysis.context.ensure_matches(expected)?;
        if self.analysis.selected_entity_id.trim().is_empty() {
            return Err(ContractError::InvalidInput(
                "ChIE result has no selected entity".to_owned(),
            ));
        }
        if self.analysis.impact_paths.len() > MAX_PATHS {
            return Err(ContractError::LimitExceeded(format!(
                "ChIE result exceeds the {MAX_PATHS}-path contract limit"
            )));
        }
        for path in &self.analysis.impact_paths {
            if path.entity_id.trim().is_empty()
                || path.depth == 0
                || path.depth > MAX_PATH_LENGTH
                || path.relationships.len() > MAX_PATH_LENGTH
            {
                return Err(ContractError::InvalidInput(
                    "ChIE result contains an invalid or unbounded impact path".to_owned(),
                ));
            }
            for relationship in &path.relationships {
                relationship.context.ensure_matches(expected)?;
            }
        }
        if self.analysis.test_candidates.len() > MAX_RELATIONSHIPS {
            return Err(ContractError::LimitExceeded(
                "ChIE test candidates exceed the bounded result limit".to_owned(),
            ));
        }
        for candidate in &self.analysis.test_candidates {
            if candidate.test_id.trim().is_empty()
                || candidate.name.trim().is_empty()
                || candidate.path.trim().is_empty()
                || candidate.path.contains('\\')
                || !crate::source::limits::is_safe_relative(std::path::Path::new(&candidate.path))
                || candidate.evidence_path.trim().is_empty()
                || candidate.evidence_path.contains('\\')
                || !crate::source::limits::is_safe_relative(std::path::Path::new(
                    &candidate.evidence_path,
                ))
            {
                return Err(ContractError::InvalidInput(
                    "ChIE test candidate lacks identity or evidence".to_owned(),
                ));
            }
        }
        Ok(())
    }
}

pub type CanonicalCodeResult = CanonicalResult<CoieResult>;
pub type CanonicalTestResult = CanonicalResult<TieResult>;
pub type CanonicalChangeResult = CanonicalResult<ChieResult>;

#[cfg(test)]
mod tests {
    use super::IntelligenceRequest;
    use crate::contract::context::RepositoryContext;

    fn context() -> RepositoryContext {
        RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        }
    }

    #[test]
    fn coie_and_tie_requests_are_bounded_and_typed() {
        use super::{CoieQueryKind, CoieQueryRequest, TieQueryRequest};
        let context = context();
        let coie = CoieQueryRequest {
            context: context.clone(),
            kind: CoieQueryKind::QueryRelationships,
            query: "symbol-1".into(),
            limit: 20,
        };
        assert!(coie.validate().is_ok());
        let tie = TieQueryRequest {
            context,
            target_entity_ids: vec!["symbol-1".into(), "symbol-2".into()],
            limit: 10,
        };
        assert!(tie.validate().is_ok());
        let invalid_tie = TieQueryRequest {
            target_entity_ids: vec!["symbol-1".into(), "symbol-1".into()],
            ..tie
        };
        assert!(invalid_tie.validate().is_err());
    }

    #[test]
    fn request_rejects_unbounded_depth_and_results() {
        let mut request = IntelligenceRequest {
            context: context(),
            target_entity_id: "symbol-1".into(),
            max_depth: 1,
            max_results: 10,
        };
        assert!(request.validate().is_ok());
        request.max_depth = super::MAX_PATH_LENGTH + 1;
        assert!(request.validate().is_err());
        request.max_depth = 1;
        request.max_results = 0;
        assert!(request.validate().is_err());
    }
}
