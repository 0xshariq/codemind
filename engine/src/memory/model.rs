use crate::source::RepositoryContext;
use serde::{Deserialize, Serialize};

pub const MAX_PAYLOAD_BYTES: usize = 16 * 1024;
/// Conversation turns are stored as bounded individual graph nodes and may
/// exceed the small single-memory payload cap in aggregate.
pub const MAX_REVIEW_CONTEXT_BYTES: usize = 512 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceReference {
    pub path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub snapshot_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryRecord {
    pub memory_id: String,
    #[serde(default)]
    pub owner_id: String,
    pub kind: String,
    pub scope: String,
    pub lifecycle: String,
    pub freshness: String,
    pub payload_json: String,
    pub context: RepositoryContext,
    pub evidence: Vec<EvidenceReference>,
    pub idempotency_key: String,
    pub version: u64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HandoffRecord {
    pub handoff_id: String,
    pub schema_version: String,
    pub operation_id: String,
    pub review_episode_id: String,
    pub source_stage: String,
    pub target_stage: String,
    pub task: String,
    pub findings: Vec<String>,
    pub evidence_references: Vec<EvidenceReference>,
    pub context: RepositoryContext,
    pub open_questions: Vec<String>,
    pub status: String,
    pub idempotency_key: String,
    pub version: u64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConversationSession {
    pub session_id: String,
    pub scope: String,
    pub context: RepositoryContext,
    pub started_at: String,
    pub ended_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConversationTurn {
    pub turn_id: String,
    pub session_id: String,
    pub role: String,
    pub summary: String,
    pub context: RepositoryContext,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewEpisode {
    pub episode_id: String,
    pub session_id: String,
    pub title: String,
    pub context: RepositoryContext,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewFinding {
    pub finding_id: String,
    pub episode_id: String,
    pub summary: String,
    pub evidence: Vec<EvidenceReference>,
    pub context: RepositoryContext,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewContextRecord {
    pub context_id: String,
    pub scope: String,
    #[serde(default)]
    pub owner_id: String,
    pub session: ConversationSession,
    pub turns: Vec<ConversationTurn>,
    pub episode: ReviewEpisode,
    pub findings: Vec<ReviewFinding>,
    pub context: RepositoryContext,
    pub updated_at: String,
}
