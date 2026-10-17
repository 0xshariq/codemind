use super::{context::RepositoryContext, error::ContractError};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const MAX_RELATIONSHIPS: usize = 100;
/// General result-list cap. Kept separate from the relationship-specific cap so
/// the policies can evolve independently without changing unrelated queries.
pub const MAX_RESULT_ITEMS: usize = 100;
pub const MAX_PATHS: usize = 50;
pub const MAX_PATH_LENGTH: usize = 8;
pub const MAX_EVIDENCE: usize = 100;
pub const MAX_SERIALIZED_BYTES: usize = 256 * 1024;
pub const MAX_DIAGNOSTICS: usize = 100;
pub const MAX_LIMITATIONS: usize = 100;
pub const MAX_METADATA_ITEM_BYTES: usize = 4_096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Success,
    Empty,
    Partial,
    Unsupported,
    Unavailable,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Completeness {
    Complete,
    Truncated,
    Incomplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    Current,
    Stale,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceReference {
    pub path: String,
    pub line: u32,
    pub snapshot_id: String,
}

impl EvidenceReference {
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.path.trim().is_empty()
            || self.path.contains('\\')
            || !crate::source::limits::is_safe_relative(Path::new(&self.path))
            || self.line == 0
            || self.snapshot_id.trim().is_empty()
        {
            return Err(ContractError::InvalidInput(
                "evidence reference requires a safe relative path, positive line, and snapshot ID"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Correlation {
    pub request_id: String,
    pub trace_id: String,
    pub operation_id: String,
}

impl Correlation {
    pub fn validate(&self) -> Result<(), ContractError> {
        for (field, value) in [
            ("request_id", self.request_id.as_str()),
            ("trace_id", self.trace_id.as_str()),
            ("operation_id", self.operation_id.as_str()),
        ] {
            if value.trim().is_empty() || value.len() > MAX_METADATA_ITEM_BYTES {
                return Err(ContractError::InvalidInput(format!(
                    "correlation {field} is empty or too long"
                )));
            }
            if value.chars().any(char::is_control) {
                return Err(ContractError::InvalidInput(format!(
                    "correlation {field} contains a control character"
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CanonicalResult<T> {
    pub outcome: Outcome,
    pub completeness: Completeness,
    pub freshness: Freshness,
    pub context: RepositoryContext,
    pub correlation: Correlation,
    pub data: T,
    pub evidence: Vec<EvidenceReference>,
    pub diagnostics: Vec<String>,
    pub limitations: Vec<String>,
}

impl<T> CanonicalResult<T> {
    pub fn new(
        context: RepositoryContext,
        correlation: Correlation,
        outcome: Outcome,
        completeness: Completeness,
        freshness: Freshness,
        data: T,
    ) -> Self {
        Self {
            outcome,
            completeness,
            freshness,
            context,
            correlation,
            data,
            evidence: Vec::new(),
            diagnostics: Vec::new(),
            limitations: Vec::new(),
        }
    }

    /// Compatibility helper retained for existing query callers.
    pub fn validate_context(&self, expected: &RepositoryContext) -> Result<(), String> {
        self.context
            .ensure_matches(expected)
            .map_err(|error| error.to_string())
    }

    pub fn validate_context_contract(
        &self,
        expected: &RepositoryContext,
    ) -> Result<(), ContractError> {
        self.context.ensure_matches(expected)?;
        Ok(())
    }
}

impl<T: Serialize> CanonicalResult<T> {
    /// Validate fields that can be checked without knowing the domain schema.
    /// Domain-specific `data` must be validated by its CoIE/TIE/ChIE contract.
    pub fn validate(&self) -> Result<(), ContractError> {
        self.context.validate()?;
        self.correlation.validate()?;

        if self.evidence.len() > MAX_EVIDENCE
            || self.diagnostics.len() > MAX_DIAGNOSTICS
            || self.limitations.len() > MAX_LIMITATIONS
        {
            return Err(ContractError::LimitExceeded(
                "result metadata exceeds the bounded contract".to_owned(),
            ));
        }
        for reference in &self.evidence {
            reference.validate()?;
        }
        for value in self.diagnostics.iter().chain(self.limitations.iter()) {
            if value.trim().is_empty() || value.len() > MAX_METADATA_ITEM_BYTES {
                return Err(ContractError::InvalidInput(
                    "result diagnostic/limitation is empty or too long".to_owned(),
                ));
            }
        }
        if self.completeness == Completeness::Truncated && self.outcome != Outcome::Partial {
            return Err(ContractError::InvalidInput(
                "a truncated canonical result must use the partial outcome".to_owned(),
            ));
        }
        if self.completeness != Completeness::Complete && self.outcome == Outcome::Success {
            return Err(ContractError::InvalidInput(
                "an incomplete canonical result cannot use the success outcome".to_owned(),
            ));
        }
        let size = serialized_size(self)?;
        if size > MAX_SERIALIZED_BYTES {
            return Err(ContractError::LimitExceeded(format!(
                "serialized result is {size} bytes; maximum is {MAX_SERIALIZED_BYTES}"
            )));
        }
        Ok(())
    }

    /// Historical method name kept for compatibility; it now performs all
    /// canonical metadata checks rather than checking list counts alone.
    pub fn validate_metadata(&self) -> Result<(), String> {
        self.validate().map_err(|error| error.to_string())
    }
}

/// Bound a general result list. For graph relationships prefer
/// `bounded_relationships` so the limit's meaning stays explicit.
pub fn bounded_items<T>(
    mut result: CanonicalResult<Vec<T>>,
    limit: usize,
    item_label: &str,
) -> CanonicalResult<Vec<T>> {
    let limit = limit.min(MAX_RESULT_ITEMS);
    if result.data.len() > limit {
        result.data.truncate(limit);
        result.completeness = Completeness::Truncated;
        result.outcome = Outcome::Partial;
        if result.limitations.len() < MAX_LIMITATIONS {
            let label = if item_label.trim().is_empty() {
                "items"
            } else {
                item_label
            };
            result
                .limitations
                .push(format!("{label} result was limited to {limit} items"));
        }
    }
    if result.data.is_empty() && result.outcome == Outcome::Success {
        result.outcome = Outcome::Empty;
    }
    result
}

pub fn bounded_relationships<T>(
    result: CanonicalResult<Vec<T>>,
    limit: usize,
) -> CanonicalResult<Vec<T>> {
    bounded_items(result, limit, "relationship")
}

/// Backwards-compatible alias for the former relationship-list helper.
pub fn bounded<T>(result: CanonicalResult<Vec<T>>, limit: usize) -> CanonicalResult<Vec<T>> {
    bounded_relationships(result, limit)
}

pub fn serialized_size<T: Serialize>(value: &T) -> Result<usize, serde_json::Error> {
    Ok(serde_json::to_vec(value)?.len())
}

#[cfg(test)]
mod tests {
    use super::{
        CanonicalResult, Completeness, Correlation, EvidenceReference, Freshness, Outcome,
        bounded_relationships, serialized_size,
    };
    use crate::contract::context::RepositoryContext;

    fn context() -> RepositoryContext {
        RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        }
    }

    fn result(data: Vec<String>) -> CanonicalResult<Vec<String>> {
        CanonicalResult::new(
            context(),
            Correlation {
                request_id: "request".into(),
                trace_id: "trace".into(),
                operation_id: "operation".into(),
            },
            Outcome::Success,
            Completeness::Complete,
            Freshness::Current,
            data,
        )
    }

    #[test]
    fn validates_context_correlation_evidence_and_serialized_bound() {
        let mut value = result(vec!["symbol".into()]);
        value.evidence.push(EvidenceReference {
            path: "src/lib.rs".into(),
            line: 1,
            snapshot_id: "snapshot".into(),
        });
        assert!(value.validate().is_ok());
        assert!(serialized_size(&value).unwrap() > 0);

        value.evidence[0].line = 0;
        assert!(value.validate().is_err());
    }

    #[test]
    fn truncation_is_reported_as_partial_not_complete_success() {
        let value = bounded_relationships(result(vec!["a".into(), "b".into()]), 1);
        assert_eq!(value.outcome, Outcome::Partial);
        assert_eq!(value.completeness, Completeness::Truncated);
        assert_eq!(value.data.len(), 1);
        assert!(value.validate().is_ok());
    }

    #[test]
    fn empty_success_becomes_empty_outcome() {
        let value = bounded_relationships(result(Vec::<String>::new()), 5);
        assert_eq!(value.outcome, Outcome::Empty);
        assert_eq!(value.completeness, Completeness::Complete);
    }
}
