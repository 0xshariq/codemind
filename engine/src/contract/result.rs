use crate::source::RepositoryContext;
use serde::{Deserialize, Serialize};

pub const MAX_RELATIONSHIPS: usize = 100;
pub const MAX_PATHS: usize = 50;
pub const MAX_PATH_LENGTH: usize = 8;
pub const MAX_EVIDENCE: usize = 100;
pub const MAX_SERIALIZED_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Success,
    Empty,
    Partial,
    Unsupported,
    Unavailable,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Completeness {
    Complete,
    Truncated,
    Incomplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CanonicalResult<T> {
    pub outcome: Outcome,
    pub completeness: Completeness,
    pub freshness: Freshness,
    pub context: RepositoryContext,
    pub data: T,
    pub evidence: Vec<EvidenceReference>,
    pub diagnostics: Vec<String>,
    pub limitations: Vec<String>,
}

impl<T> CanonicalResult<T> {
    pub fn validate_context(&self, expected: &RepositoryContext) -> Result<(), String> {
        if &self.context != expected {
            return Err("result context does not match the requested context".to_owned());
        }
        Ok(())
    }
}

pub fn bounded<T>(mut result: CanonicalResult<Vec<T>>, limit: usize) -> CanonicalResult<Vec<T>> {
    let limit = limit.min(MAX_RELATIONSHIPS);
    if result.data.len() > limit {
        result.data.truncate(limit);
        result.completeness = Completeness::Truncated;
        result.outcome = Outcome::Partial;
        result
            .limitations
            .push(format!("result limited to {limit} relationships"));
    }
    if result.data.is_empty() && result.outcome == Outcome::Success {
        result.outcome = Outcome::Empty;
    }
    result
}

pub fn serialized_size<T: Serialize>(value: &T) -> Result<usize, serde_json::Error> {
    Ok(serde_json::to_vec(value)?.len())
}
