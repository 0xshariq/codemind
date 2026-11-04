use super::{context::RepositoryContext, error::ContractError};
use crate::memory::{
    model::{
        HandoffRecord, MAX_PAYLOAD_BYTES, MAX_REVIEW_CONTEXT_BYTES, MemoryRecord,
        ReviewContextRecord,
    },
    validation,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const MAX_MEMORY_READ_LIMIT: usize = 100;
pub const MAX_MEMORY_EVIDENCE_REFERENCES: usize = 100;
pub const MAX_HANDOFF_OPEN_QUESTIONS: usize = 32;

/// Approved hackathon memory scopes. A scope label is classification, not
/// authorization; the caller must still enforce principal/workspace access.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryScope {
    User,
    Team,
    Repository,
}

impl MemoryScope {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Team => "team",
            Self::Repository => "repository",
        }
    }

    pub fn parse(value: &str) -> Result<Self, ContractError> {
        match value {
            "user" => Ok(Self::User),
            "team" => Ok(Self::Team),
            "repository" => Ok(Self::Repository),
            _ => Err(ContractError::InvalidInput(
                "scope must be user, team, or repository".to_owned(),
            )),
        }
    }

    fn validate_owner(self, owner_id: &str) -> Result<(), ContractError> {
        if matches!(self, Self::User | Self::Team) && owner_id.trim().is_empty() {
            return Err(ContractError::InvalidInput(format!(
                "owner_id is required for {} scope",
                self.as_str()
            )));
        }
        if owner_id.len() > 512 || owner_id.chars().any(char::is_control) {
            return Err(ContractError::InvalidInput(
                "owner_id is malformed or too long".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryWriteRequest {
    pub context: RepositoryContext,
    pub scope: String,
    pub owner_id: String,
    pub record: MemoryRecord,
}

impl MemoryWriteRequest {
    pub fn validate(&self) -> Result<(), ContractError> {
        self.context.validate()?;
        let scope = MemoryScope::parse(&self.scope)?;
        scope.validate_owner(&self.owner_id)?;
        self.context.ensure_matches(&self.record.context)?;
        if self.record.scope != self.scope || self.record.owner_id != self.owner_id {
            return Err(ContractError::InvalidInput(
                "memory write envelope does not match its record".to_owned(),
            ));
        }
        validation::validate_memory(&self.record).map_err(ContractError::InvalidInput)?;
        if self.record.evidence.len() > MAX_MEMORY_EVIDENCE_REFERENCES {
            return Err(ContractError::LimitExceeded(
                "memory evidence exceeds the bounded contract".to_owned(),
            ));
        }
        if serde_json::from_str::<serde_json::Value>(&self.record.payload_json).is_err() {
            return Err(ContractError::InvalidInput(
                "memory payload_json must contain a valid JSON value".to_owned(),
            ));
        }
        for evidence in &self.record.evidence {
            validate_evidence_reference(evidence)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryReadRequest {
    pub context: RepositoryContext,
    pub scope: String,
    pub owner_id: String,
    pub kind: String,
    pub limit: usize,
}

impl MemoryReadRequest {
    pub fn validate(&self) -> Result<(), ContractError> {
        self.context.validate()?;
        let scope = MemoryScope::parse(&self.scope)?;
        scope.validate_owner(&self.owner_id)?;
        if !matches!(self.kind.as_str(), "semantic" | "procedure") {
            return Err(ContractError::InvalidInput(
                "memory kind must be semantic or procedure".to_owned(),
            ));
        }
        if self.limit == 0 || self.limit > MAX_MEMORY_READ_LIMIT {
            return Err(ContractError::LimitExceeded(format!(
                "memory limit must be between 1 and {MAX_MEMORY_READ_LIMIT}"
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HandoffWriteRequest {
    pub context: RepositoryContext,
    pub record: HandoffRecord,
}

impl HandoffWriteRequest {
    pub fn validate(&self) -> Result<(), ContractError> {
        self.context.ensure_matches(&self.record.context)?;
        validation::validate_handoff(&self.record).map_err(ContractError::InvalidInput)?;
        if self.record.evidence_references.len() > MAX_MEMORY_EVIDENCE_REFERENCES
            || self.record.open_questions.len() > MAX_HANDOFF_OPEN_QUESTIONS
            || self
                .record
                .findings
                .iter()
                .any(|finding| finding.trim().is_empty())
            || self
                .record
                .open_questions
                .iter()
                .any(|question| question.trim().is_empty())
        {
            return Err(ContractError::LimitExceeded(
                "handoff evidence/findings/questions exceed or violate the bounded contract"
                    .to_owned(),
            ));
        }
        for evidence in &self.record.evidence_references {
            validate_evidence_reference(evidence)?;
        }
        if serde_json::to_vec(&self.record)?.len() > MAX_PAYLOAD_BYTES {
            return Err(ContractError::LimitExceeded(
                "handoff record exceeds the bounded payload size".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewWriteRequest {
    pub context: RepositoryContext,
    pub record: ReviewContextRecord,
}

impl ReviewWriteRequest {
    pub fn validate(&self) -> Result<(), ContractError> {
        self.context.ensure_matches(&self.record.context)?;
        let scope = MemoryScope::parse(&self.record.scope)?;
        if self.record.context_id.trim().is_empty()
            || self.record.session.session_id.trim().is_empty()
            || self.record.episode.episode_id.trim().is_empty()
            || self.record.turns.len() > 100
            || self.record.findings.len() > 64
        {
            return Err(ContractError::InvalidInput(
                "review record is missing required identity or exceeds item limits".to_owned(),
            ));
        }
        if self.record.session.scope != scope.as_str() {
            return Err(ContractError::InvalidInput(
                "review session scope does not match the review envelope".to_owned(),
            ));
        }
        self.context.ensure_matches(&self.record.session.context)?;
        self.context.ensure_matches(&self.record.episode.context)?;
        for turn in &self.record.turns {
            self.context.ensure_matches(&turn.context)?;
            if turn.turn_id.trim().is_empty()
                || turn.session_id != self.record.session.session_id
                || turn.role.trim().is_empty()
                || turn.summary.trim().is_empty()
            {
                return Err(ContractError::InvalidInput(
                    "review turn identity/session/summary is invalid".to_owned(),
                ));
            }
        }
        for finding in &self.record.findings {
            self.context.ensure_matches(&finding.context)?;
            if finding.finding_id.trim().is_empty()
                || finding.episode_id != self.record.episode.episode_id
                || finding.summary.trim().is_empty()
                || finding.evidence.len() > MAX_MEMORY_EVIDENCE_REFERENCES
            {
                return Err(ContractError::InvalidInput(
                    "review finding identity/episode/evidence is invalid".to_owned(),
                ));
            }
            for evidence in &finding.evidence {
                validate_evidence_reference(evidence)?;
            }
        }
        let payload_size = serde_json::to_vec(&self.record)?.len();
        if payload_size > MAX_REVIEW_CONTEXT_BYTES {
            return Err(ContractError::LimitExceeded(
                "review record exceeds the bounded payload size".to_owned(),
            ));
        }
        Ok(())
    }
}

fn validate_evidence_reference(
    reference: &crate::memory::model::EvidenceReference,
) -> Result<(), ContractError> {
    let path = Path::new(&reference.path);
    if reference.path.trim().is_empty()
        || reference.path.contains('\\')
        || !crate::source::limits::is_safe_relative(path)
        || reference.start_line == 0
        || reference.end_line < reference.start_line
        || reference.snapshot_id.trim().is_empty()
    {
        return Err(ContractError::InvalidInput(
            "memory evidence requires a safe relative path, valid line range, and snapshot ID"
                .to_owned(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod level1_tests {
    use super::MemoryScope;

    #[test]
    fn memory_scope_parser_accepts_only_the_three_declared_scopes() {
        assert_eq!(MemoryScope::parse("user").unwrap(), MemoryScope::User);
        assert_eq!(MemoryScope::parse("team").unwrap(), MemoryScope::Team);
        assert_eq!(
            MemoryScope::parse("repository").unwrap(),
            MemoryScope::Repository
        );
        for unsupported in ["", "global", "workspace", "User", " repository "] {
            assert!(MemoryScope::parse(unsupported).is_err());
        }
    }

    #[test]
    fn memory_scope_owner_validation_is_scope_sensitive_and_bounded() {
        assert!(MemoryScope::User.validate_owner("user-1").is_ok());
        assert!(MemoryScope::Team.validate_owner("team-1").is_ok());
        assert!(MemoryScope::Repository.validate_owner("").is_ok());

        assert!(MemoryScope::User.validate_owner("  ").is_err());
        assert!(MemoryScope::Team.validate_owner("").is_err());
        assert!(
            MemoryScope::Repository
                .validate_owner(&"x".repeat(513))
                .is_err()
        );
        assert!(MemoryScope::Repository.validate_owner("owner\nid").is_err());
    }
}
