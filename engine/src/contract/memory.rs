use super::{context::RepositoryContext, error::ContractError};
use crate::memory::{
    model::{HandoffRecord, MAX_PAYLOAD_BYTES, MemoryRecord, ReviewContextRecord},
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
        if payload_size > MAX_PAYLOAD_BYTES {
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
mod tests {
    use super::{HandoffWriteRequest, MemoryReadRequest, MemoryScope, ReviewWriteRequest};
    use crate::{
        contract::context::RepositoryContext,
        memory::model::{
            ConversationSession, ConversationTurn, HandoffRecord, ReviewContextRecord,
            ReviewEpisode, ReviewFinding,
        },
    };

    fn context() -> RepositoryContext {
        RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        }
    }

    #[test]
    fn scope_parser_accepts_only_approved_scope_names() {
        assert_eq!(
            MemoryScope::parse("repository").unwrap(),
            MemoryScope::Repository
        );
        assert!(MemoryScope::parse("global").is_err());
    }

    #[test]
    fn handoff_and_review_envelopes_validate_nested_context_and_references() {
        let context = context();
        let handoff = HandoffRecord {
            handoff_id: "handoff-1".into(),
            schema_version: "1".into(),
            operation_id: "operation-1".into(),
            review_episode_id: "episode-1".into(),
            source_stage: "chie".into(),
            target_stage: "agent".into(),
            task: "Continue investigating the selected symbol".into(),
            findings: vec!["A dependent symbol was found".into()],
            evidence_references: Vec::new(),
            context: context.clone(),
            open_questions: Vec::new(),
            status: "pending".into(),
            idempotency_key: "handoff-idempotency-1".into(),
            version: 1,
            created_at: "2026-10-16T00:00:00Z".into(),
            updated_at: "2026-10-16T00:00:00Z".into(),
        };
        assert!(
            HandoffWriteRequest {
                context: context.clone(),
                record: handoff.clone()
            }
            .validate()
            .is_ok()
        );
        let mut mismatched_handoff = handoff;
        mismatched_handoff.context.graph_version = "graph-other".into();
        assert!(
            HandoffWriteRequest {
                context: context.clone(),
                record: mismatched_handoff
            }
            .validate()
            .is_err()
        );

        let session = ConversationSession {
            session_id: "session-1".into(),
            scope: "repository".into(),
            context: context.clone(),
            started_at: "2026-10-16T00:00:00Z".into(),
            ended_at: None,
        };
        let episode = ReviewEpisode {
            episode_id: "episode-1".into(),
            session_id: session.session_id.clone(),
            title: "Selected-symbol investigation".into(),
            context: context.clone(),
            created_at: "2026-10-16T00:00:00Z".into(),
        };
        let review = ReviewContextRecord {
            context_id: "review-1".into(),
            scope: "repository".into(),
            owner_id: "repository-owner".into(),
            session: session.clone(),
            turns: vec![ConversationTurn {
                turn_id: "turn-1".into(),
                session_id: session.session_id,
                role: "user".into(),
                summary: "Investigate the selected symbol".into(),
                context: context.clone(),
                created_at: "2026-10-16T00:00:01Z".into(),
            }],
            episode,
            findings: vec![ReviewFinding {
                finding_id: "finding-1".into(),
                episode_id: "episode-1".into(),
                summary: "One supported dependency path exists".into(),
                evidence: Vec::new(),
                context: context.clone(),
                created_at: "2026-10-16T00:00:02Z".into(),
            }],
            context: context.clone(),
            updated_at: "2026-10-16T00:00:03Z".into(),
        };
        assert!(
            ReviewWriteRequest {
                context: context.clone(),
                record: review.clone()
            }
            .validate()
            .is_ok()
        );
        let mut broken_review = review;
        broken_review.findings[0].episode_id = "episode-missing".into();
        assert!(
            ReviewWriteRequest {
                context,
                record: broken_review
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn read_request_enforces_kind_owner_and_limit_contract() {
        let request = MemoryReadRequest {
            context: context(),
            scope: "user".into(),
            owner_id: "user-1".into(),
            kind: "semantic".into(),
            limit: 10,
        };
        assert!(request.validate().is_ok());
        let invalid = MemoryReadRequest {
            owner_id: String::new(),
            ..request
        };
        assert!(invalid.validate().is_err());
    }
}
