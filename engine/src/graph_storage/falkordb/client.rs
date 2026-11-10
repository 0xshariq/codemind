use crate::engines::coie::model::{
    CodeGraph, EntityKind, GraphEntity, GraphRelationship, RelationshipKind,
};
use crate::engines::history::model::{
    HistoryEntity, HistoryGraph, HistoryGraphContext, HistoryRelationship,
};
use crate::engines::tie::model::{
    TestEntity, TestRelationship, TestRelationshipKind, TestResolution,
};
use crate::graph_storage::port::{
    ChangeGraphVerificationExpectation, ChangeGraphVerificationReport, GraphRetentionReport,
    GraphStorage, GraphVerificationExpectation, GraphVerificationReport, StorageError,
    digest_identity_set,
};
use crate::memory::model::{HandoffRecord, MemoryRecord, ReviewContextRecord};
use crate::memory::repository::MemoryRepository;
use crate::memory::validation;
use crate::source::RepositoryContext;
use crate::source::identity::digest_text;
use async_trait::async_trait;
use chrono::Utc;
use falkordb::{
    AsyncGraph, EntityType, FalkorAsyncClient, FalkorClientBuilder, FalkorConnectionInfo, IndexType,
};
use futures::StreamExt;
use std::{collections::{HashMap, HashSet}, num::NonZeroUsize, sync::Arc, time::Duration};
use tokio::sync::RwLock;

/// FalkorDB adapter for the database-independent [`GraphStorage`] port.
///
/// Keep this concrete type at the application composition root and inside the
/// graph-storage adapter; domain engines should accept `GraphStorage` instead.
pub struct FalkorDbStorage {
    client: FalkorAsyncClient,
    graph_namespace: String,
    query_timeout_ms: i64,
    schema_ready: Arc<RwLock<HashSet<String>>>,
}

/// Aggregate queries must return exactly one row, even when the aggregate is zero.
/// A missing row is an abnormal storage response, not an empty graph.
pub(super) fn required_count_row<T, E>(
    row: Option<Result<T, E>>,
    operation: &str,
) -> Result<T, StorageError>
where
    E: std::fmt::Display,
{
    let row =
        row.ok_or_else(|| StorageError::Query(format!("{operation} returned no aggregate row")))?;
    row.map_err(|error| StorageError::Query(error.to_string()))
}

pub(super) fn nonnegative_count(value: i64, operation: &str) -> Result<usize, StorageError> {
    usize::try_from(value).map_err(|_| {
        StorageError::Query(format!(
            "{operation} returned an invalid negative count: {value}"
        ))
    })
}

/// Append a length-prefixed string so identity fields cannot collide by delimiter.
fn append_identity_component(material: &mut Vec<u8>, value: &str) {
    material.extend_from_slice(&(value.len() as u64).to_be_bytes());
    material.extend_from_slice(value.as_bytes());
}

/// Compute the stable identity key for one entity within a source snapshot.
fn code_entity_identity_key(entity: &GraphEntity) -> String {
    let mut material = b"codemind-code-entity-identity-v1\0".to_vec();
    append_identity_component(&mut material, &format!("{:?}", entity.kind));
    append_identity_component(&mut material, &entity.path);
    match &entity.kind {
        EntityKind::Repository => append_identity_component(&mut material, &entity.name),
        EntityKind::RepositorySnapshot => append_identity_component(&mut material, &entity.name),
        EntityKind::Directory | EntityKind::File => {}
        EntityKind::Module | EntityKind::Package => {
            append_identity_component(&mut material, &entity.name);
            append_identity_component(
                &mut material,
                entity.native_kind.as_deref().unwrap_or_default(),
            );
        }
        EntityKind::Symbol => {
            append_identity_component(&mut material, &entity.name);
            append_identity_component(&mut material, &entity.line.to_string());
            append_identity_component(
                &mut material,
                entity.native_kind.as_deref().unwrap_or_default(),
            );
            append_identity_component(
                &mut material,
                entity.language.map(|language| language.stable_name()).unwrap_or_default(),
            );
            if let Some(range) = &entity.range {
                append_identity_component(&mut material, &range.start_byte.to_string());
                append_identity_component(&mut material, &range.end_byte.to_string());
            }
        }
    }
    format!(
        "coie-identity:{}",
        crate::source::identity::digest_bytes(&material)
    )
}

/// Build a conservative key for migrating pre-identity-map CodeEntity records.
fn legacy_code_entity_identity_key(kind: &str, path: &str, name: &str, line: i64) -> String {
    format!("legacy-entity:{kind}:{path}:{name}:{line}")
}

/// Compute a stable relationship key, including an ordinal for duplicate call sites.
fn code_relationship_identity_key(
    relationship: &GraphRelationship,
    occurrence: usize,
) -> String {
    let mut material = b"codemind-code-relationship-identity-v1\0".to_vec();
    for value in [
        format!("{:?}", relationship.kind),
        relationship.from.clone(),
        relationship.to.clone(),
        relationship
            .evidence
            .as_ref()
            .map(|evidence| evidence.path.as_str())
            .unwrap_or_default()
            .to_owned(),
        relationship
            .evidence
            .as_ref()
            .map(|evidence| evidence.start_line.to_string())
            .unwrap_or_default(),
        relationship
            .evidence
            .as_ref()
            .map(|evidence| evidence.end_line.to_string())
            .unwrap_or_default(),
        relationship
            .evidence
            .as_ref()
            .map(|evidence| evidence.provenance.as_str())
            .unwrap_or_default()
            .to_owned(),
        format!("{:?}", relationship.resolution),
        occurrence.to_string(),
    ] {
        append_identity_component(&mut material, &value);
    }
    format!(
        "coie-relationship-identity:{}",
        crate::source::identity::digest_bytes(&material)
    )
}

/// Build a conservative key for migrating pre-identity-map CoIE relationships.
fn legacy_code_relationship_identity_key(
    kind: &str,
    from: &str,
    to: &str,
    evidence_path: &str,
    start_line: i64,
    end_line: i64,
) -> String {
    format!(
        "legacy-relationship:{kind}:{from}:{to}:{evidence_path}:{start_line}:{end_line}"
    )
}

impl FalkorDbStorage {
    /// Connect to FalkorDB using the default query and connection limits.
    pub async fn connect(
        url: &str,
        graph_namespace: impl Into<String>,
    ) -> Result<Self, StorageError> {
        Self::connect_with_limits(url, graph_namespace, 30_000, 64).await
    }

    /// Connect to FalkorDB with explicit connection and query limits.
    pub async fn connect_with_limits(
        url: &str,
        graph_namespace: impl Into<String>,
        query_timeout_ms: i64,
        max_inflight: usize,
    ) -> Result<Self, StorageError> {
        if query_timeout_ms <= 0 {
            return Err(StorageError::Unavailable(
                "FalkorDB query timeout must be greater than zero".to_owned(),
            ));
        }
        let max_inflight = NonZeroUsize::new(max_inflight).ok_or_else(|| {
            StorageError::Unavailable(
                "FalkorDB max in-flight command limit must be greater than zero".to_owned(),
            )
        })?;
        let graph_namespace = graph_namespace.into();
        if graph_namespace.trim().is_empty()
            || !graph_namespace.chars().all(|character| {
                character.is_ascii_alphanumeric() || character == '_' || character == '-'
            })
        {
            return Err(StorageError::Unavailable(
                "FALKORDB_GRAPH namespace may contain only ASCII letters, numbers, '_' or '-'"
                    .to_owned(),
            ));
        }
        let connection_info = FalkorConnectionInfo::try_from(url)
            .map_err(|error| StorageError::Unavailable(error.to_string()))?;
        let client = FalkorClientBuilder::new_async()
            .with_connection_info(connection_info)
            .with_max_inflight(max_inflight)
            .with_response_timeout(Some(Duration::from_millis(
                (query_timeout_ms as u64).saturating_add(5_000),
            )))
            .build()
            .await
            .map_err(|error| StorageError::Unavailable(error.to_string()))?;
        Ok(Self {
            client,
            graph_namespace,
            query_timeout_ms,
            schema_ready: Arc::new(RwLock::new(HashSet::new())),
        })
    }

    async fn ready_graph(&self, graph_name: &str) -> Result<AsyncGraph, StorageError> {
        if self.schema_ready.read().await.contains(graph_name) {
            return Ok(self.client.select_graph(graph_name));
        }

        let mut graph = self.client.select_graph(graph_name);
        self.ensure_schema(&mut graph).await?;

        self.schema_ready
            .write()
            .await
            .insert(graph_name.to_owned());

        Ok(graph)
    }

    async fn ensure_schema(&self, graph: &mut AsyncGraph) -> Result<(), StorageError> {
        let mut rows = graph
            .query(
                "MERGE (s:CodeMindStorageSchema {id: $id}) \
                 ON CREATE SET s.schema_version = $schema_version, s.created_at = $created_at \
                 RETURN s.schema_version AS schema_version",
            )
            .with_param(
                "id",
                crate::graph_storage::falkordb::schema::STORAGE_SCHEMA_MARKER_ID,
            )
            .with_param(
                "schema_version",
                crate::graph_storage::falkordb::schema::GRAPH_STORAGE_SCHEMA_VERSION as i64,
            )
            .with_param("created_at", chrono::Utc::now().to_rfc3339())
            .with_timeout(self.query_timeout_ms)
            .execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;

        let row = rows
            .data
            .next()
            .await
            .ok_or_else(|| {
                StorageError::Query("storage schema marker query returned no row".to_owned())
            })?
            .map_err(|error| StorageError::Query(error.to_string()))?;

        let schema_version: i64 = row
            .try_get("schema_version")
            .map_err(|error| StorageError::Query(error.to_string()))?;

        let expected_version =
            crate::graph_storage::falkordb::schema::GRAPH_STORAGE_SCHEMA_VERSION as i64;
        if schema_version != expected_version {
            // Schemas 2-4 can be upgraded in place because the intervening
            // changes add indexes and generation/evidence metadata. Old graph
            // facts are not trusted as verified under the current contract:
            // the active-generation verifier requires a published, verified
            // HistoryGeneration with a content-addressed storage generation.
            if matches!(schema_version, 2..=4) && expected_version == 5 {
                graph
                    .query(
                        "MATCH (s:CodeMindStorageSchema {id: $id}) SET s.schema_version = $schema_version",
                    )
                    .with_param(
                        "id",
                        crate::graph_storage::falkordb::schema::STORAGE_SCHEMA_MARKER_ID,
                    )
                    .with_param("schema_version", expected_version)
                    .with_timeout(self.query_timeout_ms)
                    .execute()
                    .await
                    .map_err(|error| StorageError::Query(error.to_string()))?;
            } else {
                return Err(StorageError::SchemaIncompatible {
                    expected: crate::graph_storage::falkordb::schema::GRAPH_STORAGE_SCHEMA_VERSION,
                    actual: u32::try_from(schema_version).unwrap_or(u32::MAX),
                });
            }
        }

        self.ensure_required_indexes(graph).await
    }

    async fn ensure_required_indexes(&self, graph: &mut AsyncGraph) -> Result<(), StorageError> {
        use crate::graph_storage::falkordb::schema::{
            REQUIRED_EDGE_INDEXES, REQUIRED_NODE_INDEXES,
        };

        for &(label, property) in REQUIRED_NODE_INDEXES {
            self.ensure_range_index(graph, EntityType::Node, label, property)
                .await?;
        }

        for &(label, property) in REQUIRED_EDGE_INDEXES {
            self.ensure_range_index(graph, EntityType::Edge, label, property)
                .await?;
        }

        Ok(())
    }

    async fn ensure_range_index(
        &self,
        graph: &mut AsyncGraph,
        entity_type: EntityType,
        label: &str,
        property: &str,
    ) -> Result<(), StorageError> {
        match graph
            .create_index_op(IndexType::Range, entity_type, label, &[property], None)
            .wait()
            .await
        {
            Ok(()) => Ok(()),
            Err(error) if is_existing_index_error(&error) => Ok(()),
            Err(error) => Err(StorageError::Query(format!(
                "failed to initialize required index {entity_type:?} {label}.{property}: {error}"
            ))),
        }
    }

    fn validate_context(context: &RepositoryContext) -> Result<(), StorageError> {
        context.validate().map_err(|_| StorageError::InvalidContext)
    }

    fn graph_name_for_workspace(&self, workspace_id: &str) -> Result<String, StorageError> {
        resolve_workspace_graph_name(&self.graph_namespace, workspace_id)
    }
}

/// FalkorDB versions may report an idempotent duplicate-index attempt as
/// either "already exists" or "<property> is already indexed". A newly
/// connected adapter must accept both responses when it reinitializes the
/// schema for a graph whose indexes were created by an earlier connection.
fn is_existing_index_error(error: &impl std::fmt::Display) -> bool {
    let message = error.to_string().to_ascii_lowercase();
    message.contains("already exists") || message.contains("already indexed")
}

/// Resolve a physical FalkorDB graph from the validated workspace security scope.
///
/// `FALKORDB_GRAPH` is a trusted namespace prefix, not a graph name to select
/// directly. The opaque workspace ID is domain-separated and SHA-256 hashed so
/// raw caller-controlled identifiers never become part of a database graph name.
pub(super) fn resolve_workspace_graph_name(
    graph_namespace: &str,
    workspace_id: &str,
) -> Result<String, StorageError> {
    if graph_namespace.trim().is_empty()
        || !graph_namespace.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '_' || character == '-'
        })
    {
        return Err(StorageError::Unavailable(
            "FALKORDB_GRAPH namespace may contain only ASCII letters, numbers, '_' or '-'"
                .to_owned(),
        ));
    }

    if workspace_id.trim().is_empty()
        || workspace_id.len() > crate::contract::context::MAX_CONTEXT_FIELD_BYTES
        || workspace_id.chars().any(char::is_control)
    {
        return Err(StorageError::InvalidContext);
    }

    let identity = format!("codemind-workspace-graph-v1\0{workspace_id}");
    Ok(format!("{graph_namespace}_{}", digest_text(&identity)))
}

impl FalkorDbStorage {
    async fn count_nodes_with_labels(
        &self,
        context: &RepositoryContext,
        labels: &[&str],
    ) -> Result<usize, StorageError> {
        let graph_name = self.graph_name_for_workspace(&context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;
        let mut rows = graph
            .ro_query(
                "MATCH (n) WHERE n.workspace_id = $workspace AND n.repository_id = $repository AND n.snapshot_id = $snapshot AND n.graph_version = $graph_version AND ANY(label IN labels(n) WHERE label IN $labels) RETURN count(n) AS count",
            )
            .with_param("workspace", context.workspace_id.clone())
            .with_param("repository", context.repository_id.clone())
            .with_param("snapshot", context.source_snapshot_id.clone())
            .with_param("graph_version", context.graph_version.clone())
            .with_param("labels", labels.iter().map(|v| (*v).to_owned()).collect::<Vec<_>>())
            .with_timeout(self.query_timeout_ms).execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let row = required_count_row(rows.data.next().await, "node count query")?;
        let count: i64 = row
            .try_get("count")
            .map_err(|error| StorageError::Query(error.to_string()))?;
        nonnegative_count(count, "node count query")
    }

    async fn count_relationships_with_types(
        &self,
        context: &RepositoryContext,
        types: &[&str],
    ) -> Result<usize, StorageError> {
        let graph_name = self.graph_name_for_workspace(&context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;
        let mut rows = graph
            .ro_query(
                "MATCH ()-[r]->() WHERE r.workspace_id = $workspace AND r.repository_id = $repository AND r.snapshot_id = $snapshot AND r.graph_version = $graph_version AND type(r) IN $types RETURN count(r) AS count",
            )
            .with_param("workspace", context.workspace_id.clone())
            .with_param("repository", context.repository_id.clone())
            .with_param("snapshot", context.source_snapshot_id.clone())
            .with_param("graph_version", context.graph_version.clone())
            .with_param("types", types.iter().map(|v| (*v).to_owned()).collect::<Vec<_>>())
            .with_timeout(self.query_timeout_ms).execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let row = required_count_row(rows.data.next().await, "relationship count query")?;
        let count: i64 = row
            .try_get("count")
            .map_err(|error| StorageError::Query(error.to_string()))?;
        nonnegative_count(count, "relationship count query")
    }

    async fn count_relationships_from_label(
        &self,
        context: &RepositoryContext,
        source_label: &str,
        types: &[&str],
    ) -> Result<usize, StorageError> {
        let graph_name = self.graph_name_for_workspace(&context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;
        let query = format!(
            "MATCH (source:{source_label})-[r]->() WHERE source.workspace_id = $workspace AND source.repository_id = $repository AND source.snapshot_id = $snapshot AND source.graph_version = $graph_version AND r.workspace_id = $workspace AND r.repository_id = $repository AND r.snapshot_id = $snapshot AND r.graph_version = $graph_version AND type(r) IN $types RETURN count(r) AS count"
        );
        let mut rows = graph
            .ro_query(&query)
            .with_param("workspace", context.workspace_id.clone())
            .with_param("repository", context.repository_id.clone())
            .with_param("snapshot", context.source_snapshot_id.clone())
            .with_param("graph_version", context.graph_version.clone())
            .with_param(
                "types",
                types
                    .iter()
                    .map(|value| (*value).to_owned())
                    .collect::<Vec<_>>(),
            )
            .with_timeout(self.query_timeout_ms)
            .execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let row = required_count_row(rows.data.next().await, "domain relationship count query")?;
        let count: i64 = row
            .try_get("count")
            .map_err(|error| StorageError::Query(error.to_string()))?;
        nonnegative_count(count, "domain relationship count query")
    }

    async fn count_history_entities(
        &self,
        context: &HistoryGraphContext,
        generation_id: &str,
    ) -> Result<usize, StorageError> {
        let graph_name = self.graph_name_for_workspace(&context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;
        let mut rows = graph.ro_query(
            "MATCH (g:HistoryGeneration {repository_generation_id: $generation, workspace_id: $workspace, repository_id: $repository}) WITH g ORDER BY g.created_at DESC, g.id DESC LIMIT 1 MATCH (n) WHERE n.workspace_id = $workspace AND n.repository_id = $repository AND n.generation_id = g.id RETURN count(n) AS count",
        ).with_param("workspace", context.workspace_id.clone()).with_param("repository", context.repository_id.clone()).with_param("generation", generation_id.to_owned()).with_timeout(self.query_timeout_ms).execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
        let row = required_count_row(rows.data.next().await, "history entity count query")?;
        let count: i64 = row
            .try_get("count")
            .map_err(|error| StorageError::Query(error.to_string()))?;
        nonnegative_count(count, "history entity count query")
    }

    async fn count_history_relationships(
        &self,
        context: &HistoryGraphContext,
        generation_id: &str,
    ) -> Result<usize, StorageError> {
        let graph_name = self.graph_name_for_workspace(&context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;
        let mut rows = graph.ro_query(
            "MATCH (g:HistoryGeneration {repository_generation_id: $generation, workspace_id: $workspace, repository_id: $repository}) WITH g ORDER BY g.created_at DESC, g.id DESC LIMIT 1 MATCH ()-[r]->() WHERE r.workspace_id = $workspace AND r.repository_id = $repository AND r.generation_id = g.id RETURN count(r) AS count",
        ).with_param("workspace", context.workspace_id.clone()).with_param("repository", context.repository_id.clone()).with_param("generation", generation_id.to_owned()).with_timeout(self.query_timeout_ms).execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
        let row = required_count_row(rows.data.next().await, "history relationship count query")?;
        let count: i64 = row
            .try_get("count")
            .map_err(|error| StorageError::Query(error.to_string()))?;
        nonnegative_count(count, "history relationship count query")
    }

    async fn identity_digest_for_nodes(
        &self,
        context: &RepositoryContext,
        labels: &[&str],
        observed_count: usize,
    ) -> Result<String, StorageError> {
        let graph_name = self.graph_name_for_workspace(&context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;
        let page_size = crate::graph_storage::falkordb::read::bounded_limit(1_000);
        let mut ids: Vec<String> = Vec::with_capacity(observed_count);
        let mut offset = 0usize;
        while offset < observed_count {
            let limit = page_size.min(observed_count.saturating_sub(offset));
            let mut rows = graph
                .ro_query(
                    "MATCH (n) WHERE n.workspace_id = $workspace AND n.repository_id = $repository AND n.snapshot_id = $snapshot AND n.graph_version = $graph_version AND ANY(label IN labels(n) WHERE label IN $labels) RETURN n.id AS id ORDER BY n.id ASC SKIP $offset LIMIT $limit",
                )
                .with_param("workspace", context.workspace_id.clone())
                .with_param("repository", context.repository_id.clone())
                .with_param("snapshot", context.source_snapshot_id.clone())
                .with_param("graph_version", context.graph_version.clone())
                .with_param("labels", labels.iter().map(|v| (*v).to_owned()).collect::<Vec<_>>())
                .with_param("offset", offset as i64)
                .with_param("limit", limit as i64)
                .with_timeout(self.query_timeout_ms)
                .execute()
                .await
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let mut page = 0usize;
            while let Some(row) = rows.data.next().await {
                let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
                let id: String = row
                    .try_get("id")
                    .map_err(|error| StorageError::Query(error.to_string()))?;
                ids.push(id);
                page += 1;
            }
            if page == 0 {
                return Err(StorageError::Query(
                    "semantic node verification returned fewer identities than observed".to_owned(),
                ));
            }
            offset += page;
        }
        if ids.len() != observed_count {
            return Err(StorageError::Query(
                "semantic node verification count changed during read-back".to_owned(),
            ));
        }
        Ok(digest_identity_set(&ids))
    }

    async fn identity_digest_for_relationships(
        &self,
        context: &RepositoryContext,
        source_label: &str,
        types: &[&str],
        observed_count: usize,
    ) -> Result<String, StorageError> {
        let graph_name = self.graph_name_for_workspace(&context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;
        let page_size = crate::graph_storage::falkordb::read::bounded_limit(1_000);
        let mut ids: Vec<String> = Vec::with_capacity(observed_count);
        let mut offset = 0usize;
        while offset < observed_count {
            let limit = page_size.min(observed_count.saturating_sub(offset));
            let query = format!(
                "MATCH (source:{source_label})-[r]->() WHERE source.workspace_id = $workspace AND source.repository_id = $repository AND source.snapshot_id = $snapshot AND source.graph_version = $graph_version AND r.workspace_id = $workspace AND r.repository_id = $repository AND r.snapshot_id = $snapshot AND r.graph_version = $graph_version AND type(r) IN $types RETURN r.id AS id ORDER BY r.id ASC SKIP $offset LIMIT $limit"
            );
            let mut rows = graph
                .ro_query(&query)
                .with_param("workspace", context.workspace_id.clone())
                .with_param("repository", context.repository_id.clone())
                .with_param("snapshot", context.source_snapshot_id.clone())
                .with_param("graph_version", context.graph_version.clone())
                .with_param(
                    "types",
                    types.iter().map(|v| (*v).to_owned()).collect::<Vec<_>>(),
                )
                .with_param("offset", offset as i64)
                .with_param("limit", limit as i64)
                .with_timeout(self.query_timeout_ms)
                .execute()
                .await
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let mut page = 0usize;
            while let Some(row) = rows.data.next().await {
                let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
                let id: String = row
                    .try_get("id")
                    .map_err(|error| StorageError::Query(error.to_string()))?;
                ids.push(id);
                page += 1;
            }
            if page == 0 {
                return Err(StorageError::Query(
                    "semantic relationship verification returned fewer identities than observed"
                        .to_owned(),
                ));
            }
            offset += page;
        }
        if ids.len() != observed_count {
            return Err(StorageError::Query(
                "semantic relationship verification count changed during read-back".to_owned(),
            ));
        }
        Ok(digest_identity_set(&ids))
    }

    async fn verify_history_head_and_completeness(
        &self,
        context: &HistoryGraphContext,
        expected_head_commit: &str,
        repository_generation_id: &str,
    ) -> Result<(String, bool, u64, u64), StorageError> {
        let graph_name = self.graph_name_for_workspace(&context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;
        let mut summary_rows = graph.ro_query(
            "MATCH (g:HistoryGeneration {repository_generation_id: $generation, workspace_id: $workspace, repository_id: $repository}) RETURN g.id AS storage_generation_id, g.head_commit AS head_commit, g.history_complete AS history_complete, g.omitted_commits_at_least AS omitted_commits, g.omitted_path_changes_at_least AS omitted_paths, g.expected_entity_count AS expected_entity_count, g.expected_relationship_count AS expected_relationship_count, g.expected_entity_digest AS expected_entity_digest, g.expected_relationship_digest AS expected_relationship_digest, g.expected_entity_content_digest AS expected_entity_content_digest, g.expected_relationship_content_digest AS expected_relationship_content_digest, g.schema_version AS schema_version ORDER BY g.created_at DESC, g.id DESC LIMIT 1",
        ).with_param("generation", repository_generation_id.to_owned()).with_param("workspace", context.workspace_id.clone()).with_param("repository", context.repository_id.clone()).with_timeout(self.query_timeout_ms).execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
        let row = required_count_row(
            summary_rows.data.next().await,
            "history generation summary query",
        )?;
        let storage_generation_id: String = row
            .try_get("storage_generation_id")
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let head_commit: String = row
            .try_get("head_commit")
            .map_err(|error| StorageError::Query(error.to_string()))?;
        if !head_commit.eq_ignore_ascii_case(expected_head_commit) {
            return Err(StorageError::Query(
                "verified History Graph head commit does not match expectation".to_owned(),
            ));
        }
        let history_complete: bool = row
            .try_get("history_complete")
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let omitted_commits_at_least = nonnegative_count(
            row.try_get::<i64>("omitted_commits")
                .map_err(|error| StorageError::Query(error.to_string()))?,
            "history omitted commit count",
        )? as u64;
        let omitted_path_changes_at_least = nonnegative_count(
            row.try_get::<i64>("omitted_paths")
                .map_err(|error| StorageError::Query(error.to_string()))?,
            "history omitted path count",
        )? as u64;
        let expected_entity_count = nonnegative_count(
            row.try_get::<i64>("expected_entity_count")
                .map_err(|error| StorageError::Query(error.to_string()))?,
            "history expected entity count",
        )?;
        let expected_relationship_count = nonnegative_count(
            row.try_get::<i64>("expected_relationship_count")
                .map_err(|error| StorageError::Query(error.to_string()))?,
            "history expected relationship count",
        )?;
        let expected_entity_digest: String = row
            .try_get("expected_entity_digest")
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let expected_relationship_digest: String = row
            .try_get("expected_relationship_digest")
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let expected_entity_content_digest: String = row
            .try_get("expected_entity_content_digest")
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let expected_relationship_content_digest: String = row
            .try_get("expected_relationship_content_digest")
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let schema_version: i64 = row
            .try_get("schema_version")
            .map_err(|error| StorageError::Query(error.to_string()))?;
        if schema_version != crate::engines::history::model::HISTORY_GRAPH_SCHEMA_VERSION as i64 {
            return Err(StorageError::Query(
                "History Graph schema version mismatch".to_owned(),
            ));
        }
        if history_complete && (omitted_commits_at_least != 0 || omitted_path_changes_at_least != 0)
        {
            return Err(StorageError::Query(
                "History Graph completeness summary is internally inconsistent".to_owned(),
            ));
        }

        let mut entity_rows = graph.ro_query(
            "MATCH (n) WHERE n.workspace_id = $workspace AND n.repository_id = $repository AND n.generation_id = $generation RETURN n.id AS storage_id, n.logical_id AS logical_id, n.payload_json AS payload_json, labels(n) AS entity_labels ORDER BY n.logical_id ASC",
        ).with_param("workspace", context.workspace_id.clone()).with_param("repository", context.repository_id.clone()).with_param("generation", storage_generation_id.clone()).with_timeout(self.query_timeout_ms).execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
        let mut entity_ids = Vec::new();
        let mut actual_entities = Vec::new();
        let mut observed_head = false;
        while let Some(row) = entity_rows.data.next().await {
            let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
            let storage_id: String = row
                .try_get("storage_id")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let logical_id: String = row
                .try_get("logical_id")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let payload: String = row
                .try_get("payload_json")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let entity_labels: Vec<String> = row
                .try_get("entity_labels")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let entity: HistoryEntity = serde_json::from_str(&payload)
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let expected_label = match entity.data.entity_kind() {
                crate::engines::history::model::HistoryEntityKind::Repository => {
                    "HistoryRepository"
                }
                crate::engines::history::model::HistoryEntityKind::Commit => "HistoryCommit",
                crate::engines::history::model::HistoryEntityKind::Directory => "HistoryDirectory",
                crate::engines::history::model::HistoryEntityKind::File => "HistoryFile",
                crate::engines::history::model::HistoryEntityKind::Change => "HistoryChange",
            };
            if !entity_labels.iter().any(|label| label == expected_label)
                || storage_id != history_generation_storage_id(&storage_generation_id, &logical_id)
                || entity.id != logical_id
                || entity.context != *context
            {
                return Err(StorageError::Query("History Graph entity payload does not match its generation-scoped storage identity".to_owned()));
            }
            if let crate::engines::history::model::HistoryEntityData::Commit { commit_id, .. } =
                &entity.data
                && commit_id.eq_ignore_ascii_case(&head_commit)
            {
                observed_head = true;
            }
            entity_ids.push(logical_id);
            actual_entities.push(entity);
        }
        if entity_ids.len() != expected_entity_count
            || digest_identity_set(&entity_ids) != expected_entity_digest
            || digest_serialized_records(&actual_entities)? != expected_entity_content_digest
            || !observed_head
        {
            return Err(StorageError::Query("History Graph entity generation is incomplete or does not match its expected identity/content set".to_owned()));
        }

        let mut relationship_rows = graph.ro_query(
            "MATCH (a)-[r]->(b) WHERE r.workspace_id = $workspace AND r.repository_id = $repository AND r.generation_id = $generation RETURN r.id AS storage_id, r.logical_id AS logical_id, r.from_logical_id AS from_id, r.to_logical_id AS to_id, a.logical_id AS actual_from, b.logical_id AS actual_to, a.generation_id AS actual_from_generation, b.generation_id AS actual_to_generation, a.workspace_id AS actual_from_workspace, b.workspace_id AS actual_to_workspace, a.repository_id AS actual_from_repository, b.repository_id AS actual_to_repository, r.path_role AS path_role, r.domain_kind AS domain_kind, r.payload_json AS payload_json, type(r) AS relationship_type ORDER BY r.logical_id ASC",
        ).with_param("workspace", context.workspace_id.clone()).with_param("repository", context.repository_id.clone()).with_param("generation", storage_generation_id.clone()).with_timeout(self.query_timeout_ms).execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
        let mut relationship_ids = Vec::new();
        let mut actual_relationships = Vec::new();
        while let Some(row) = relationship_rows.data.next().await {
            let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
            let storage_id: String = row
                .try_get("storage_id")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let logical_id: String = row
                .try_get("logical_id")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let from_id: String = row
                .try_get("from_id")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let to_id: String = row
                .try_get("to_id")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let actual_from: String = row
                .try_get("actual_from")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let actual_to: String = row
                .try_get("actual_to")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let actual_from_generation: String = row
                .try_get("actual_from_generation")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let actual_to_generation: String = row
                .try_get("actual_to_generation")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let actual_from_workspace: String = row
                .try_get("actual_from_workspace")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let actual_to_workspace: String = row
                .try_get("actual_to_workspace")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let actual_from_repository: String = row
                .try_get("actual_from_repository")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let actual_to_repository: String = row
                .try_get("actual_to_repository")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let domain_kind: String = row
                .try_get("domain_kind")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let relationship_type: String = row
                .try_get("relationship_type")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let path_role: Option<String> = row
                .try_get("path_role")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let payload: String = row
                .try_get("payload_json")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let relationship: HistoryRelationship = serde_json::from_str(&payload)
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let expected_type = match domain_kind.as_str() {
                "ContainsCommit" => "CONTAINS_COMMIT",
                "ParentCommit" => "PARENT_COMMIT",
                "RecordsChange" => "RECORDS_CHANGE",
                "ChangesPath" => "CHANGES_PATH",
                "ContainsPath" => "CONTAINS_PATH",
                _ => {
                    return Err(StorageError::Query(
                        "History Graph contains an unknown relationship kind".to_owned(),
                    ));
                }
            };
            let expected_path_role = relationship
                .path_role
                .as_ref()
                .map(|role| format!("{:?}", role));
            if storage_id != history_generation_storage_id(&storage_generation_id, &logical_id)
                || from_id != actual_from
                || to_id != actual_to
                || relationship_type != expected_type
                || actual_from_generation != storage_generation_id
                || actual_to_generation != storage_generation_id
                || actual_from_workspace != context.workspace_id
                || actual_to_workspace != context.workspace_id
                || actual_from_repository != context.repository_id
                || actual_to_repository != context.repository_id
                || relationship.id != logical_id
                || relationship.from != from_id
                || relationship.to != to_id
                || relationship.context != *context
                || format!("{:?}", relationship.kind) != domain_kind
                || expected_path_role.as_deref() != path_role.as_deref()
                || (domain_kind == "ChangesPath"
                    && !matches!(path_role.as_deref(), Some("Old") | Some("New")))
                || (domain_kind != "ChangesPath" && path_role.is_some())
            {
                return Err(StorageError::Query("History Graph relationship type, payload, path role, or endpoint does not match its generation-scoped semantics".to_owned()));
            }
            relationship_ids.push(logical_id);
            actual_relationships.push(relationship);
        }
        if relationship_ids.len() != expected_relationship_count
            || digest_identity_set(&relationship_ids) != expected_relationship_digest
            || digest_serialized_records(&actual_relationships)?
                != expected_relationship_content_digest
        {
            return Err(StorageError::Query("History Graph relationship generation is incomplete or does not match its expected identity/content set".to_owned()));
        }
        Ok((
            head_commit,
            history_complete,
            omitted_commits_at_least,
            omitted_path_changes_at_least,
        ))
    }

    async fn verify_history_state(
        &self,
        context: &HistoryGraphContext,
        expected_head_commit: &str,
        generation_id: &str,
    ) -> Result<(String, bool, u64, u64), StorageError> {
        self.verify_history_head_and_completeness(context, expected_head_commit, generation_id)
            .await
    }

    async fn verify_change_slice(
        &self,
        context: &RepositoryContext,
    ) -> Result<(usize, usize), StorageError> {
        Ok((
            self.count_nodes_with_labels(
                context,
                crate::graph_storage::falkordb::schema::CHANGE_ENTITY_LABELS,
            )
            .await?,
            self.count_relationships_with_types(
                context,
                crate::graph_storage::falkordb::schema::CHANGE_RELATIONSHIP_LABELS,
            )
            .await?,
        ))
    }
}

impl FalkorDbStorage {
    async fn query_relationships_impl(
        &self,
        context: &RepositoryContext,
        entity_id: &str,
        incoming: bool,
        limit: usize,
    ) -> Result<Vec<GraphRelationship>, StorageError> {
        Self::validate_context(context)?;
        let limit = crate::graph_storage::falkordb::read::bounded_limit(limit);
        let graph_name = self.graph_name_for_workspace(&context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;
        let pattern = if incoming {
            "MATCH (a:CodeEntity {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})-[r]->(b:CodeEntity {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})"
        } else {
            "MATCH (a:CodeEntity {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})-[r]->(b:CodeEntity {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})"
        };
        let query = format!(
            "{pattern} WHERE r.workspace_id = $workspace AND r.repository_id = $repository AND r.snapshot_id = $snapshot AND r.graph_version = $graph_version AND type(r) IN ['HAS_SNAPSHOT', 'CONTAINS', 'DEFINES', 'IMPORTS', 'CALLS'] RETURN r.id AS relationship_id, type(r) AS relationship_type, a.id AS from_id, b.id AS to_id,                 r.resolution AS resolution, r.evidence_path AS evidence_path, r.evidence_start_line AS evidence_start_line,                 r.evidence_end_line AS evidence_end_line, r.evidence_snapshot_id AS evidence_snapshot_id, r.evidence_provenance AS evidence_provenance LIMIT $limit"
        );
        let mut result = graph
            .ro_query(&query)
            .with_param("id", entity_id.to_owned())
            .with_param("workspace", context.workspace_id.clone())
            .with_param("repository", context.repository_id.clone())
            .with_param("snapshot", context.source_snapshot_id.clone())
            .with_param("graph_version", context.graph_version.clone())
            .with_param("limit", limit as i64)
            .with_timeout(self.query_timeout_ms)
            .execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;

        let mut relationships = Vec::new();
        while let Some(row) = result.data.next().await {
            let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
            let resolution = parse_resolution(
                row.try_get("resolution")
                    .map_err(|error| StorageError::Query(error.to_string()))?,
            )?;
            let evidence_path: Option<String> = row
                .try_get("evidence_path")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let evidence = match evidence_path.filter(|path| !path.is_empty()) {
                Some(path) => Some(crate::engines::coie::model::SourceEvidence {
                    path,
                    start_line: row
                        .try_get::<Option<i64>>("evidence_start_line")
                        .map_err(|error| StorageError::Query(error.to_string()))?
                        .unwrap_or(0) as u32,
                    end_line: row
                        .try_get::<Option<i64>>("evidence_end_line")
                        .map_err(|error| StorageError::Query(error.to_string()))?
                        .unwrap_or(0) as u32,
                    snapshot_id: row
                        .try_get::<Option<String>>("evidence_snapshot_id")
                        .map_err(|error| StorageError::Query(error.to_string()))?
                        .unwrap_or_default(),
                    provenance: row
                        .try_get::<Option<String>>("evidence_provenance")
                        .map_err(|error| StorageError::Query(error.to_string()))?
                        .unwrap_or_default(),
                }),
                None => None,
            };
            relationships.push(GraphRelationship {
                id: row
                    .try_get("relationship_id")
                    .map_err(|error| StorageError::Query(error.to_string()))?,
                kind: parse_relationship(
                    row.try_get("relationship_type")
                        .map_err(|error| StorageError::Query(error.to_string()))?,
                )?,
                from: row
                    .try_get("from_id")
                    .map_err(|error| StorageError::Query(error.to_string()))?,
                to: row
                    .try_get("to_id")
                    .map_err(|error| StorageError::Query(error.to_string()))?,
                context: context.clone(),
                resolution,
                evidence,
            });
        }
        Ok(relationships)
    }
}

/// Derive a storage identity that is unique to a staged history generation.
/// Domain IDs remain stable inside serialized payloads; the adapter-level IDs
/// prevent an unverified write from overwriting facts in the active generation.
fn history_generation_storage_id(generation_id: &str, logical_id: &str) -> String {
    format!(
        "history-generation-{}",
        digest_text(&format!(
            "codemind-history-generation-v1\0{generation_id}\0{logical_id}"
        ))
    )
}

fn digest_serialized_records<T: serde::Serialize>(records: &[T]) -> Result<String, StorageError> {
    let mut serialized = records
        .iter()
        .map(|record| serde_json::to_string(record))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| StorageError::Query(error.to_string()))?;
    serialized.sort_unstable();
    Ok(digest_text(&serialized.join("\n")))
}

impl FalkorDbStorage {
    async fn verify_code_graph_semantics(
        &self,
        context: &RepositoryContext,
    ) -> Result<(), StorageError> {
        let graph_name = self.graph_name_for_workspace(&context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;
        let mut nodes = graph.ro_query(
            "MATCH (n:CodeEntity {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) RETURN n.id AS id, n.kind AS kind, n.name AS name, n.path AS path, n.line AS line, n.resolution AS resolution, n.evidence_path AS evidence_path, n.evidence_start_line AS evidence_start_line, n.evidence_end_line AS evidence_end_line, n.evidence_snapshot_id AS evidence_snapshot_id, n.evidence_provenance AS evidence_provenance",
        ).with_param("workspace", context.workspace_id.clone()).with_param("repository", context.repository_id.clone()).with_param("snapshot", context.source_snapshot_id.clone()).with_param("graph_version", context.graph_version.clone()).with_timeout(self.query_timeout_ms).execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
        while let Some(row) = nodes.data.next().await {
            let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
            let id: String = row
                .try_get("id")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let kind: String = row
                .try_get("kind")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let name: String = row
                .try_get("name")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let path: String = row
                .try_get("path")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let line: i64 = row
                .try_get("line")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let resolution: String = row
                .try_get("resolution")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let evidence_path: String = row
                .try_get("evidence_path")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let evidence_start: i64 = row
                .try_get("evidence_start_line")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let evidence_end: i64 = row
                .try_get("evidence_end_line")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let evidence_snapshot: String = row
                .try_get("evidence_snapshot_id")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let evidence_provenance: String = row
                .try_get("evidence_provenance")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            if id.trim().is_empty()
                || name.trim().is_empty()
                || path.trim().is_empty()
                || !crate::source::limits::is_safe_relative(std::path::Path::new(&path))
                || !matches!(
                    kind.as_str(),
                    "Repository"
                        | "RepositorySnapshot"
                        | "Directory"
                        | "File"
                        | "Module"
                        | "Package"
                        | "Symbol"
                )
                || line <= 0
                || !matches!(
                    resolution.as_str(),
                    "Resolved" | "Unresolved" | "Ambiguous" | "Unsupported"
                )
                || (!evidence_path.is_empty()
                    && (!crate::source::limits::is_safe_relative(std::path::Path::new(
                        &evidence_path,
                    )) || evidence_start <= 0
                        || evidence_end < evidence_start
                        || evidence_snapshot != context.source_snapshot_id
                        || evidence_provenance.trim().is_empty()))
                || (evidence_path.is_empty()
                    && (evidence_start != 0
                        || evidence_end != 0
                        || !evidence_snapshot.is_empty()
                        || !evidence_provenance.is_empty()))
            {
                return Err(StorageError::Query("stored Code Graph entity violates its typed identity, resolution, or evidence contract".to_owned()));
            }
        }
        let mut relationships = graph.ro_query(
            "MATCH (a:CodeEntity {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})-[r]->(b:CodeEntity {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) WHERE r.workspace_id = $workspace AND r.repository_id = $repository AND r.snapshot_id = $snapshot AND r.graph_version = $graph_version RETURN r.id AS id, type(r) AS relationship_type, r.kind AS kind, r.from_id AS from_id, r.to_id AS to_id, a.id AS actual_from, b.id AS actual_to, a.kind AS actual_from_kind, b.kind AS actual_to_kind, r.resolution AS resolution, r.evidence_path AS evidence_path, r.evidence_start_line AS evidence_start_line, r.evidence_end_line AS evidence_end_line, r.evidence_snapshot_id AS evidence_snapshot_id, r.evidence_provenance AS evidence_provenance",
        ).with_param("workspace", context.workspace_id.clone()).with_param("repository", context.repository_id.clone()).with_param("snapshot", context.source_snapshot_id.clone()).with_param("graph_version", context.graph_version.clone()).with_timeout(self.query_timeout_ms).execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
        let mut observed_relationships = 0usize;
        while let Some(row) = relationships.data.next().await {
            let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
            observed_relationships += 1;
            let id: String = row
                .try_get("id")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let relationship_type: String = row
                .try_get("relationship_type")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let kind: String = row
                .try_get("kind")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let from_id: String = row
                .try_get("from_id")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let to_id: String = row
                .try_get("to_id")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let actual_from: String = row
                .try_get("actual_from")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let actual_to: String = row
                .try_get("actual_to")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let actual_from_kind: String = row
                .try_get("actual_from_kind")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let actual_to_kind: String = row
                .try_get("actual_to_kind")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let resolution: String = row
                .try_get("resolution")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let evidence_path: String = row
                .try_get("evidence_path")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let evidence_start: i64 = row
                .try_get("evidence_start_line")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let evidence_end: i64 = row
                .try_get("evidence_end_line")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let evidence_snapshot: String = row
                .try_get("evidence_snapshot_id")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let evidence_provenance: String = row
                .try_get("evidence_provenance")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let expected_type = match kind.as_str() {
                "HasSnapshot" => "HAS_SNAPSHOT",
                "Contains" => "CONTAINS",
                "Defines" => "DEFINES",
                "Imports" => "IMPORTS",
                "Calls" => "CALLS",
                _ => {
                    return Err(StorageError::Query(
                        "stored Code Graph relationship has an unknown kind".to_owned(),
                    ));
                }
            };
            let endpoint_kinds_valid = match kind.as_str() {
                "HasSnapshot" => {
                    actual_from_kind == "Repository" && actual_to_kind == "RepositorySnapshot"
                }
                "Defines" => {
                    matches!(actual_from_kind.as_str(), "File" | "Module" | "Package")
                        && actual_to_kind == "Symbol"
                }
                "Calls" => actual_from_kind == "Symbol" && actual_to_kind == "Symbol",
                "Imports" => matches!(
                    (actual_from_kind.as_str(), actual_to_kind.as_str()),
                    ("File", "File") | ("File", "Module") | ("Module", "Module")
                ),
                "Contains" => matches!(
                    (actual_from_kind.as_str(), actual_to_kind.as_str()),
                    ("RepositorySnapshot", "Directory")
                        | ("RepositorySnapshot", "File")
                        | ("Directory", "Directory")
                        | ("Directory", "File")
                        | ("Package", "Module")
                        | ("Module", "Symbol")
                ),
                _ => false,
            };
            if id.trim().is_empty()
                || relationship_type != expected_type
                || from_id != actual_from
                || to_id != actual_to
                || !endpoint_kinds_valid
                || !matches!(
                    resolution.as_str(),
                    "Resolved" | "Unresolved" | "Ambiguous" | "Unsupported"
                )
                || (!evidence_path.is_empty()
                    && (!crate::source::limits::is_safe_relative(std::path::Path::new(
                        &evidence_path,
                    )) || evidence_start <= 0
                        || evidence_end < evidence_start
                        || evidence_snapshot != context.source_snapshot_id
                        || evidence_provenance.trim().is_empty()))
                || (evidence_path.is_empty()
                    && resolution == "Resolved"
                    && matches!(kind.as_str(), "Calls" | "Imports"))
            {
                return Err(StorageError::Query("stored Code Graph relationship violates its typed direction, endpoint, resolution, or evidence contract".to_owned()));
            }
        }
        let persisted_relationships = self
            .count_relationships_from_label(
                context,
                crate::graph_storage::falkordb::schema::CODE_ENTITY_LABEL,
                crate::graph_storage::falkordb::schema::CODE_RELATIONSHIP_LABELS,
            )
            .await?;
        if observed_relationships != persisted_relationships {
            return Err(StorageError::Query(
                "Code Graph contains a relationship with a missing or incorrectly typed endpoint"
                    .to_owned(),
            ));
        }
        Ok(())
    }

    async fn verify_test_graph_semantics(
        &self,
        context: &RepositoryContext,
    ) -> Result<(), StorageError> {
        let graph_name = self.graph_name_for_workspace(&context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;
        let mut nodes = graph.ro_query(
            "MATCH (n:TestEntity {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) RETURN n.id AS id, n.name AS name, n.path AS path, n.line AS line, n.framework AS framework, n.status AS status, n.kind AS kind",
        ).with_param("workspace", context.workspace_id.clone()).with_param("repository", context.repository_id.clone()).with_param("snapshot", context.source_snapshot_id.clone()).with_param("graph_version", context.graph_version.clone()).with_timeout(self.query_timeout_ms).execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
        while let Some(row) = nodes.data.next().await {
            let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
            let id: String = row
                .try_get("id")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let name: String = row
                .try_get("name")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let path: String = row
                .try_get("path")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let line: i64 = row
                .try_get("line")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let _framework: String = row
                .try_get("framework")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let status: String = row
                .try_get("status")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let kind: String = row
                .try_get("kind")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            if id.trim().is_empty()
                || name.trim().is_empty()
                || path.trim().is_empty()
                || line <= 0
                || !crate::source::limits::is_safe_relative(std::path::Path::new(&path))
                || !matches!(status.as_str(), "Supported" | "Partial" | "Unsupported")
                || !matches!(kind.as_str(), "Test" | "TestSuite" | "Fixture" | "TestCase")
            {
                return Err(StorageError::Query("stored Test Graph entity violates its node-kind, status, or source-location contract".to_owned()));
            }
        }
        let mut relationships = graph.ro_query(
            "MATCH (test:TestEntity {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})-[r]->(target) WHERE r.workspace_id = $workspace AND r.repository_id = $repository AND r.snapshot_id = $snapshot AND r.graph_version = $graph_version AND target.workspace_id = $workspace AND target.repository_id = $repository AND target.snapshot_id = $snapshot AND target.graph_version = $graph_version RETURN r.id AS id, type(r) AS relationship_type, r.kind AS kind, r.test_id AS source_id, test.id AS actual_source, r.entity_id AS target_id, target.id AS actual_target, r.target_kind AS target_kind, labels(target) AS target_labels, r.resolution AS resolution, r.evidence_path AS evidence_path",
        ).with_param("workspace", context.workspace_id.clone()).with_param("repository", context.repository_id.clone()).with_param("snapshot", context.source_snapshot_id.clone()).with_param("graph_version", context.graph_version.clone()).with_timeout(self.query_timeout_ms).execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
        let mut observed_relationships = 0usize;
        while let Some(row) = relationships.data.next().await {
            let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
            observed_relationships += 1;
            let id: String = row
                .try_get("id")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let relationship_type: String = row
                .try_get("relationship_type")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let kind: String = row
                .try_get("kind")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let source_id: String = row
                .try_get("source_id")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let actual_source: String = row
                .try_get("actual_source")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let target_id: String = row
                .try_get("target_id")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let actual_target: String = row
                .try_get("actual_target")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let target_kind: String = row
                .try_get("target_kind")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let target_labels: Vec<String> = row
                .try_get("target_labels")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let resolution: String = row
                .try_get("resolution")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let evidence_path: String = row
                .try_get("evidence_path")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let expected_type = match kind.as_str() {
                "Contains" => "CONTAINS",
                "UsesFixture" => "USES_FIXTURE",
                "DependsOn" => "DEPENDS_ON",
                "DeclaredBy" => "DECLARED_BY",
                "Targets" => "TARGETS",
                "HasCase" => "HAS_CASE",
                "SetupBy" => "SETUP_BY",
                "TeardownBy" => "TEARDOWN_BY",
                _ => {
                    return Err(StorageError::Query(
                        "stored Test Graph relationship has an unknown kind".to_owned(),
                    ));
                }
            };
            let expected_target_label = match target_kind.as_str() {
                "Code" => "CodeEntity",
                "Test" => "TestEntity",
                _ => {
                    return Err(StorageError::Query(
                        "stored Test Graph relationship has an unknown target kind".to_owned(),
                    ));
                }
            };
            if id.trim().is_empty()
                || relationship_type != expected_type
                || source_id != actual_source
                || target_id != actual_target
                || !target_labels
                    .iter()
                    .any(|label| label == expected_target_label)
                || resolution != "Resolved"
                || evidence_path.trim().is_empty()
                || !crate::source::limits::is_safe_relative(std::path::Path::new(&evidence_path))
            {
                return Err(StorageError::Query("stored Test Graph relationship violates its typed direction, target, or resolution contract".to_owned()));
            }
        }
        let persisted_relationships = self
            .count_relationships_from_label(
                context,
                crate::graph_storage::falkordb::schema::TEST_ENTITY_LABEL,
                crate::graph_storage::falkordb::schema::TEST_RELATIONSHIP_LABELS,
            )
            .await?;
        if observed_relationships != persisted_relationships {
            return Err(StorageError::Query(
                "Test Graph contains a relationship with a missing or incorrectly typed target"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

#[async_trait]
impl GraphStorage for FalkorDbStorage {
    async fn persist_operation(
        &self,
        operation: &crate::contract::operation::OperationRecord,
    ) -> Result<crate::contract::operation::OperationRecord, StorageError> {
        operation
            .validate()
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let payload = serde_json::to_string(operation)
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let graph_name = self.graph_name_for_workspace(&operation.context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;

        match graph
            .ro_query(
                "MATCH (o:OperationRecord {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})              RETURN o.payload_json AS payload LIMIT 1",
            )
            .with_param("id", operation.operation_id.clone())
            .with_param("workspace", operation.context.workspace_id.clone())
            .with_param("repository", operation.context.repository_id.clone())
            .with_param("snapshot", operation.context.source_snapshot_id.clone())
            .with_param("graph_version", operation.context.graph_version.clone())
            .with_timeout(self.query_timeout_ms).execute()
            .await
        {
            Ok(mut existing) => {
                if let Some(row) = existing.data.next().await {
                    let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
                    let old_payload: String = row
                        .try_get("payload")
                        .map_err(|error| StorageError::Query(error.to_string()))?;
                    let old = serde_json::from_str::<crate::contract::operation::OperationRecord>(
                        &old_payload,
                    )
                    .map_err(|error| StorageError::Query(error.to_string()))?;
                    if !crate::contract::operation::OperationRecord::can_transition(
                        old.state,
                        operation.state,
                    ) {
                        return Err(StorageError::Query(
                            "operation state transition conflict".to_owned(),
                        ));
                    }
                    if operation.latest_event_sequence < old.latest_event_sequence {
                        return Err(StorageError::Query(
                            "operation event sequence moved backwards".to_owned(),
                        ));
                    }
                }
            }
            Err(error) if error.to_string().contains("Invalid graph operation on empty key") => {
                // FalkorDB does not materialize a graph key until its first
                // write. The MERGE below is that first write, so a missing
                // graph must be treated as an empty operation store rather
                // than as a storage failure.
            }
            Err(error) => return Err(StorageError::Query(error.to_string())),
        }
        graph.query(
            "MERGE (o:OperationRecord {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) \
             SET o.payload_json = $payload, o.state = $state, o.latest_event_sequence = $sequence, \
                 o.updated_at = $updated_at, o.created_at = coalesce(o.created_at, $created_at)"
        )
        .with_param("id", operation.operation_id.clone())
        .with_param("workspace", operation.context.workspace_id.clone())
        .with_param("repository", operation.context.repository_id.clone())
        .with_param("snapshot", operation.context.source_snapshot_id.clone())
        .with_param("graph_version", operation.context.graph_version.clone())
        .with_param("payload", payload)
        .with_param("state", format!("{:?}", operation.state))
        .with_param("sequence", operation.latest_event_sequence as i64)
        .with_param("updated_at", operation.updated_at.to_rfc3339())
        .with_param("created_at", operation.created_at.to_rfc3339())
        .with_timeout(self.query_timeout_ms).execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
        Ok(operation.clone())
    }

    async fn append_operation_event(
        &self,
        event: &crate::contract::events::EngineEvent,
    ) -> Result<(), StorageError> {
        event
            .validate()
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let metadata = event.metadata();
        let payload =
            serde_json::to_string(event).map_err(|error| StorageError::Query(error.to_string()))?;
        let graph_name = self.graph_name_for_workspace(&metadata.context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;
        graph.query(
            "MERGE (e:OperationEvent {id: $id, operation_id: $operation, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) \
             SET e.payload_json = $payload, e.sequence = $sequence"
        )
        .with_param("id", metadata.event_id.clone())
        .with_param("operation", metadata.operation_id.clone())
        .with_param("workspace", metadata.context.workspace_id.clone())
        .with_param("repository", metadata.context.repository_id.clone())
        .with_param("snapshot", metadata.context.source_snapshot_id.clone())
        .with_param("graph_version", metadata.context.graph_version.clone())
        .with_param("payload", payload)
        .with_param("sequence", metadata.sequence as i64)
        .with_timeout(self.query_timeout_ms).execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
        Ok(())
    }

    async fn retrieve_operation(
        &self,
        context: &RepositoryContext,
        operation_id: &str,
    ) -> Result<crate::contract::operation::OperationRecord, StorageError> {
        Self::validate_context(context)?;
        let graph_name = self.graph_name_for_workspace(&context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;
        let mut rows = graph.ro_query(
            "MATCH (o:OperationRecord {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) \
             RETURN o.payload_json AS payload LIMIT 1"
        )
        .with_param("id", operation_id.to_owned())
        .with_param("workspace", context.workspace_id.clone())
        .with_param("repository", context.repository_id.clone())
        .with_param("snapshot", context.source_snapshot_id.clone())
        .with_param("graph_version", context.graph_version.clone())
        .with_timeout(self.query_timeout_ms).execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
        let Some(row) = rows.data.next().await else {
            return Err(StorageError::Query("operation not found".to_owned()));
        };
        let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
        let payload: String = row
            .try_get("payload")
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let operation =
            serde_json::from_str::<crate::contract::operation::OperationRecord>(&payload)
                .map_err(|error| StorageError::Query(error.to_string()))?;
        operation
            .validate()
            .map_err(|error| StorageError::Query(error.to_string()))?;
        Ok(operation)
    }

    async fn events_since(
        &self,
        context: &RepositoryContext,
        operation_id: &str,
        after_sequence: u64,
        limit: usize,
    ) -> Result<Vec<crate::contract::events::EngineEvent>, StorageError> {
        Self::validate_context(context)?;
        let graph_name = self.graph_name_for_workspace(&context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;
        let mut rows = graph.ro_query(
            "MATCH (e:OperationEvent {operation_id: $operation, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) \
             WHERE e.sequence > $after RETURN e.payload_json AS payload ORDER BY e.sequence ASC LIMIT $limit"
        )
        .with_param("operation", operation_id.to_owned())
        .with_param("workspace", context.workspace_id.clone())
        .with_param("repository", context.repository_id.clone())
        .with_param("snapshot", context.source_snapshot_id.clone())
        .with_param("graph_version", context.graph_version.clone())
        .with_param("after", after_sequence as i64)
        .with_param("limit", limit.clamp(1, 100) as i64)
        .with_timeout(self.query_timeout_ms).execute().await.map_err(|error| StorageError::Query(error.to_string()))?;

        let mut events = Vec::new();
        while let Some(row) = rows.data.next().await {
            let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
            let payload: String = row
                .try_get("payload")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let event = serde_json::from_str::<crate::contract::events::EngineEvent>(&payload)
                .map_err(|error| StorageError::Query(error.to_string()))?;
            event
                .validate()
                .map_err(|error| StorageError::Query(error.to_string()))?;
            events.push(event);
        }
        Ok(events)
    }

    async fn write_code_graph(&self, graph_data: &CodeGraph) -> Result<CodeGraph, StorageError> {
        Self::validate_context(&graph_data.context)?;
        let graph_name = self.graph_name_for_workspace(&graph_data.context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;

        let incoming_entity_ids: HashSet<&str> = graph_data
            .entities
            .iter()
            .map(|entity| entity.id.as_str())
            .collect();
        for entity in &graph_data.entities {
            if entity.context != graph_data.context {
                return Err(StorageError::InvalidContext);
            }
        }
        for relationship in &graph_data.relationships {
            if relationship.context != graph_data.context {
                return Err(StorageError::InvalidContext);
            }
            if !incoming_entity_ids.contains(relationship.from.as_str())
                || !incoming_entity_ids.contains(relationship.to.as_str())
            {
                return Err(StorageError::Query(
                    "graph relationship references an entity absent from the code graph".to_owned(),
                ));
            }
        }

        // Resolve fresh extraction IDs through the persisted per-snapshot
        // identity map before writing. This preserves the CodeEntity IDs that
        // TIE and saved ChIE investigations already reference.
        let mut existing_entity_rows = graph
            .ro_query(
                "MATCH (n:CodeEntity {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) RETURN n.id AS id, coalesce(n.identity_key, '') AS identity_key, n.kind AS kind, n.name AS name, n.path AS path, n.line AS line",
            )
            .with_param("workspace", graph_data.context.workspace_id.clone())
            .with_param("repository", graph_data.context.repository_id.clone())
            .with_param("snapshot", graph_data.context.source_snapshot_id.clone())
            .with_param("graph_version", graph_data.context.graph_version.clone())
            .with_timeout(self.query_timeout_ms)
            .execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let mut exact_entity_ids = HashMap::<String, String>::new();
        let mut legacy_entity_ids = HashMap::<String, Vec<String>>::new();
        while let Some(row) = existing_entity_rows.data.next().await {
            let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
            let id: String = row
                .try_get("id")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let identity_key: String = row
                .try_get("identity_key")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let kind: String = row
                .try_get("kind")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let name: String = row
                .try_get("name")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let path: String = row
                .try_get("path")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let line: i64 = row
                .try_get("line")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            if !identity_key.is_empty() {
                if exact_entity_ids.insert(identity_key, id).is_some() {
                    return Err(StorageError::Query(
                        "persisted CoIE identity map contains duplicate entity keys".to_owned(),
                    ));
                }
            } else {
                legacy_entity_ids
                    .entry(legacy_code_entity_identity_key(&kind, &path, &name, line))
                    .or_default()
                    .push(id);
            }
        }

        let mut canonical_graph = graph_data.clone();
        let mut incoming_legacy_counts = HashMap::<String, usize>::new();
        for entity in &canonical_graph.entities {
            *incoming_legacy_counts
                .entry(legacy_code_entity_identity_key(
                    &format!("{:?}", entity.kind),
                    &entity.path,
                    &entity.name,
                    entity.line as i64,
                ))
                .or_default() += 1;
        }
        let mut entity_id_map = HashMap::<String, String>::new();
        let mut used_entity_ids = HashSet::<String>::new();
        for entity in &mut canonical_graph.entities {
            let original_id = entity.id.clone();
            let exact_key = code_entity_identity_key(entity);
            let legacy_key = legacy_code_entity_identity_key(
                &format!("{:?}", entity.kind),
                &entity.path,
                &entity.name,
                entity.line as i64,
            );
            let persisted_id = exact_entity_ids.get(&exact_key).cloned().or_else(|| {
                if incoming_legacy_counts.get(&legacy_key) == Some(&1) {
                    legacy_entity_ids
                        .get(&legacy_key)
                        .filter(|ids| ids.len() == 1)
                        .and_then(|ids| ids.first())
                        .cloned()
                } else {
                    None
                }
            });
            let canonical_id = persisted_id.unwrap_or_else(|| original_id.clone());
            if !used_entity_ids.insert(canonical_id.clone()) {
                return Err(StorageError::Query(
                    "persisted CoIE identity mapping resolved multiple entities to one ID"
                        .to_owned(),
                ));
            }
            entity_id_map.insert(original_id, canonical_id.clone());
            entity.id = canonical_id;
        }

        for relationship in &mut canonical_graph.relationships {
            relationship.from = entity_id_map
                .get(&relationship.from)
                .cloned()
                .ok_or_else(|| {
                    StorageError::Query(
                        "CoIE relationship source is missing from the canonical identity map"
                            .to_owned(),
                    )
                })?;
            relationship.to = entity_id_map
                .get(&relationship.to)
                .cloned()
                .ok_or_else(|| {
                    StorageError::Query(
                        "CoIE relationship target is missing from the canonical identity map"
                            .to_owned(),
                    )
                })?;
        }

        // Reuse relationship IDs as well. This keeps evidence and other
        // persisted references stable when the same snapshot is ingested again.
        let mut existing_relationship_rows = graph
            .ro_query(
                "MATCH (:CodeEntity {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})-[r]->(:CodeEntity {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) WHERE type(r) IN ['HAS_SNAPSHOT', 'CONTAINS', 'DEFINES', 'IMPORTS', 'CALLS'] RETURN r.id AS id, coalesce(r.identity_key, '') AS identity_key, r.kind AS kind, r.from_id AS from_id, r.to_id AS to_id, coalesce(r.evidence_path, '') AS evidence_path, coalesce(r.evidence_start_line, 0) AS start_line, coalesce(r.evidence_end_line, 0) AS end_line",
            )
            .with_param("workspace", graph_data.context.workspace_id.clone())
            .with_param("repository", graph_data.context.repository_id.clone())
            .with_param("snapshot", graph_data.context.source_snapshot_id.clone())
            .with_param("graph_version", graph_data.context.graph_version.clone())
            .with_timeout(self.query_timeout_ms)
            .execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let mut exact_relationship_ids = HashMap::<String, String>::new();
        let mut legacy_relationship_ids = HashMap::<String, Vec<String>>::new();
        while let Some(row) = existing_relationship_rows.data.next().await {
            let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
            let id: String = row
                .try_get("id")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let identity_key: String = row
                .try_get("identity_key")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let kind: String = row
                .try_get("kind")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let from: String = row
                .try_get("from_id")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let to: String = row
                .try_get("to_id")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let evidence_path: String = row
                .try_get("evidence_path")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let start_line: i64 = row
                .try_get("start_line")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let end_line: i64 = row
                .try_get("end_line")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            if !identity_key.is_empty() {
                if exact_relationship_ids.insert(identity_key, id).is_some() {
                    return Err(StorageError::Query(
                        "persisted CoIE identity map contains duplicate relationship keys"
                            .to_owned(),
                    ));
                }
            } else {
                legacy_relationship_ids
                    .entry(legacy_code_relationship_identity_key(
                        &kind,
                        &from,
                        &to,
                        &evidence_path,
                        start_line,
                        end_line,
                    ))
                    .or_default()
                    .push(id);
            }
        }

        let mut incoming_legacy_relationship_counts = HashMap::<String, usize>::new();
        for relationship in &canonical_graph.relationships {
            let legacy_key = legacy_code_relationship_identity_key(
                &format!("{:?}", relationship.kind),
                &relationship.from,
                &relationship.to,
                relationship
                    .evidence
                    .as_ref()
                    .map(|evidence| evidence.path.as_str())
                    .unwrap_or_default(),
                relationship
                    .evidence
                    .as_ref()
                    .map(|evidence| evidence.start_line as i64)
                    .unwrap_or_default(),
                relationship
                    .evidence
                    .as_ref()
                    .map(|evidence| evidence.end_line as i64)
                    .unwrap_or_default(),
            );
            *incoming_legacy_relationship_counts
                .entry(legacy_key)
                .or_default() += 1;
        }
        let mut used_relationship_ids = HashSet::<String>::new();
        let mut relationship_occurrences = HashMap::<String, usize>::new();
        for relationship in &mut canonical_graph.relationships {
            let original_id = relationship.id.clone();
            let base_key = code_relationship_identity_key(relationship, 0);
            let occurrence = relationship_occurrences.entry(base_key.clone()).or_default();
            let exact_key = code_relationship_identity_key(relationship, *occurrence);
            *occurrence += 1;
            let legacy_key = legacy_code_relationship_identity_key(
                &format!("{:?}", relationship.kind),
                &relationship.from,
                &relationship.to,
                relationship
                    .evidence
                    .as_ref()
                    .map(|evidence| evidence.path.as_str())
                    .unwrap_or_default(),
                relationship
                    .evidence
                    .as_ref()
                    .map(|evidence| evidence.start_line as i64)
                    .unwrap_or_default(),
                relationship
                    .evidence
                    .as_ref()
                    .map(|evidence| evidence.end_line as i64)
                    .unwrap_or_default(),
            );
            let persisted_id = exact_relationship_ids
                .get(&exact_key)
                .cloned()
                .or_else(|| {
                    if incoming_legacy_relationship_counts.get(&legacy_key) == Some(&1) {
                        legacy_relationship_ids
                            .get(&legacy_key)
                            .filter(|ids| ids.len() == 1)
                            .and_then(|ids| ids.first())
                            .cloned()
                    } else {
                        None
                    }
                });
            let canonical_id = persisted_id.unwrap_or_else(|| original_id.clone());
            if !used_relationship_ids.insert(canonical_id.clone()) {
                return Err(StorageError::Query(
                    "persisted CoIE relationship mapping resolved multiple relationships to one ID"
                        .to_owned(),
                ));
            }
            relationship.id = canonical_id;
        }

        let entity_ids: HashSet<&str> = canonical_graph
            .entities
            .iter()
            .map(|entity| entity.id.as_str())
            .collect();
        let relationship_ids: HashSet<&str> = canonical_graph
            .relationships
            .iter()
            .map(|relationship| relationship.id.as_str())
            .collect();

        for entity in &canonical_graph.entities {
            let merge_key = crate::graph_storage::falkordb::write::stable_merge_key(
                &entity.id,
                &entity.context.workspace_id,
                &entity.context.source_snapshot_id,
            )
            .map_err(StorageError::Query)?;
            let identity_key = code_entity_identity_key(entity);
            graph
                .query(
                    "MERGE (n:CodeEntity {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) SET n.kind = $kind, n.name = $name, n.path = $path, n.line = $line, n.workspace_id = $workspace, n.repository_id = $repository, n.snapshot_id = $snapshot, n.graph_version = $graph_version, n.resolution = $resolution, n.evidence_path = $evidence_path, n.evidence_start_line = $evidence_start_line, n.evidence_end_line = $evidence_end_line, n.evidence_snapshot_id = $evidence_snapshot_id, n.evidence_provenance = $evidence_provenance, n.merge_key = $merge_key, n.identity_key = $identity_key",
                )
                .with_param("id", entity.id.clone())
                .with_param("kind", format!("{:?}", entity.kind))
                .with_param("name", entity.name.clone())
                .with_param("path", entity.path.clone())
                .with_param("line", entity.line as i64)
                .with_param("workspace", entity.context.workspace_id.clone())
                .with_param("repository", entity.context.repository_id.clone())
                .with_param("snapshot", entity.context.source_snapshot_id.clone())
                .with_param("graph_version", entity.context.graph_version.clone())
                .with_param("merge_key", merge_key)
                .with_param("identity_key", identity_key)
                .with_param("resolution", format!("{:?}", entity.resolution))
                .with_param("evidence_path", entity.evidence.as_ref().map(|e| e.path.clone()).unwrap_or_default())
                .with_param("evidence_start_line", entity.evidence.as_ref().map(|e| e.start_line as i64).unwrap_or(0))
                .with_param("evidence_end_line", entity.evidence.as_ref().map(|e| e.end_line as i64).unwrap_or(0))
                .with_param("evidence_snapshot_id", entity.evidence.as_ref().map(|e| e.snapshot_id.clone()).unwrap_or_default())
                .with_param("evidence_provenance", entity.evidence.as_ref().map(|e| e.provenance.clone()).unwrap_or_default())
                .with_timeout(self.query_timeout_ms)
                .execute()
                .await
                .map_err(|error| StorageError::Query(error.to_string()))?;
        }

        let mut write_relationship_occurrences = HashMap::<String, usize>::new();
        for relationship in &canonical_graph.relationships {
            let relationship_type = relationship_type(&relationship.kind);
            let base_key = code_relationship_identity_key(relationship, 0);
            let occurrence = write_relationship_occurrences
                .entry(base_key)
                .or_default();
            let identity_key = code_relationship_identity_key(relationship, *occurrence);
            *occurrence += 1;
            let query = format!(
                "MATCH (a:CodeEntity {{id: $from, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}}), (b:CodeEntity {{id: $to, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}}) MERGE (a)-[r:{relationship_type} {{id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}}]->(b) SET r.kind = $kind, r.from_id = $from_id, r.to_id = $to_id, r.resolution = $resolution, r.evidence_path = $evidence_path, r.evidence_start_line = $evidence_start_line, r.evidence_end_line = $evidence_end_line, r.evidence_snapshot_id = $evidence_snapshot_id, r.evidence_provenance = $evidence_provenance, r.identity_key = $identity_key"
            );
            graph
                .query(query)
                .with_param("from", relationship.from.clone())
                .with_param("to", relationship.to.clone())
                .with_param("id", relationship.id.clone())
                .with_param("workspace", relationship.context.workspace_id.clone())
                .with_param("repository", relationship.context.repository_id.clone())
                .with_param("snapshot", relationship.context.source_snapshot_id.clone())
                .with_param("graph_version", relationship.context.graph_version.clone())
                .with_param("kind", format!("{:?}", relationship.kind))
                .with_param("from_id", relationship.from.clone())
                .with_param("to_id", relationship.to.clone())
                .with_param("resolution", format!("{:?}", relationship.resolution))
                .with_param(
                    "evidence_path",
                    relationship
                        .evidence
                        .as_ref()
                        .map(|evidence| evidence.path.clone())
                        .unwrap_or_default(),
                )
                .with_param(
                    "evidence_start_line",
                    relationship
                        .evidence
                        .as_ref()
                        .map(|evidence| evidence.start_line as i64)
                        .unwrap_or(0),
                )
                .with_param(
                    "evidence_end_line",
                    relationship
                        .evidence
                        .as_ref()
                        .map(|evidence| evidence.end_line as i64)
                        .unwrap_or(0),
                )
                .with_param(
                    "evidence_snapshot_id",
                    relationship
                        .evidence
                        .as_ref()
                        .map(|evidence| evidence.snapshot_id.clone())
                        .unwrap_or_default(),
                )
                .with_param(
                    "evidence_provenance",
                    relationship
                        .evidence
                        .as_ref()
                        .map(|evidence| evidence.provenance.clone())
                        .unwrap_or_default(),
                )
                .with_param("identity_key", identity_key)
                .with_timeout(self.query_timeout_ms)
                .execute()
                .await
                .map_err(|error| StorageError::Query(error.to_string()))?;
        }

        // Reconcile only after every incoming entity and relationship has been
        // persisted successfully. A failed MERGE must never delete facts from
        // the last complete projection for this immutable snapshot context.
        // Retain stale CodeEntity nodes that are still referenced by TIE or
        // saved ChIE records: this writer does not rebuild those cross-domain
        // relationships, and deleting the node would destroy saved evidence.
        graph
            .query(
                "MATCH (n:CodeEntity {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) WHERE NOT n.id IN $entity_ids AND NOT (n)-[:TARGETS]-() AND NOT (n)-[:DECLARED_BY]-() AND NOT (n)-[:AFFECTS]-() AND NOT (n)-[:RECOMMENDS]-() DETACH DELETE n",
            )
            .with_param("workspace", canonical_graph.context.workspace_id.clone())
            .with_param("repository", canonical_graph.context.repository_id.clone())
            .with_param("snapshot", canonical_graph.context.source_snapshot_id.clone())
            .with_param("graph_version", canonical_graph.context.graph_version.clone())
            .with_param("entity_ids", entity_ids.iter().map(|id| (*id).to_owned()).collect::<Vec<_>>())
            .with_timeout(self.query_timeout_ms)
            .execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;
        graph
            .query(
                "MATCH (:CodeEntity {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})-[r]->(:CodeEntity {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) WHERE type(r) IN ['HAS_SNAPSHOT', 'CONTAINS', 'DEFINES', 'IMPORTS', 'CALLS'] AND NOT r.id IN $relationship_ids DELETE r",
            )
            .with_param("workspace", canonical_graph.context.workspace_id.clone())
            .with_param("repository", canonical_graph.context.repository_id.clone())
            .with_param("snapshot", canonical_graph.context.source_snapshot_id.clone())
            .with_param("graph_version", canonical_graph.context.graph_version.clone())
            .with_param("relationship_ids", relationship_ids.iter().map(|id| (*id).to_owned()).collect::<Vec<_>>())
            .with_timeout(self.query_timeout_ms)
            .execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;
        Ok(canonical_graph)
    }

    async fn mutate_code_graph(
        &self,
        previous_context: &RepositoryContext,
        graph_data: &CodeGraph,
        affected_paths: &[String],
    ) -> Result<(), StorageError> {
        let _ = (previous_context, graph_data, affected_paths);
        Err(StorageError::Unavailable(
            "incremental Code Graph mutation is disabled until generation-staged publication is implemented and accepted".to_owned(),
        ))
    }

    async fn mutate_test_graph(
        &self,
        previous_context: &RepositoryContext,
        context: &RepositoryContext,
        tests: &[TestEntity],
        relationships: &[TestRelationship],
        affected_paths: &[String],
        affected_code_entity_ids: &[String],
    ) -> Result<(), StorageError> {
        let _ = (
            previous_context,
            context,
            tests,
            relationships,
            affected_paths,
            affected_code_entity_ids,
        );
        Err(StorageError::Unavailable(
            "incremental Test Graph mutation is disabled until generation-staged publication is implemented and accepted".to_owned(),
        ))
    }

    async fn persist_change_analysis(
        &self,
        change_set: &crate::engines::chie::ChangeSet,
        change_record: &crate::engines::chie::ChangeRecord,
        analysis_run: &crate::engines::chie::AnalysisRun,
        analysis: &crate::engines::chie::ChangeAnalysis,
    ) -> Result<(), StorageError> {
        Self::validate_context(&analysis.context)?;
        change_set.validate().map_err(StorageError::Query)?;
        for context in [
            &change_set.context,
            &change_record.context,
            &analysis_run.context,
        ] {
            Self::validate_context(context)?;
            if *context != analysis.context {
                return Err(StorageError::InvalidContext);
            }
        }
        if change_set.selected_entity_id != analysis.selected_entity_id
            || change_record.selected_entity_id != analysis.selected_entity_id
            || analysis_run.change_set_id != change_set.id
        {
            return Err(StorageError::Query(
                "Change Graph publication envelope does not match the analysis".to_owned(),
            ));
        }
        analysis.impact.as_ref().ok_or_else(|| {
            StorageError::Query("Change analysis has no impact result".to_owned())
        })?;
        analysis_run.validate().map_err(StorageError::Query)?;

        let change_set_payload = serde_json::to_string(change_set)
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let change_record_payload = serde_json::to_string(change_record)
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let run_payload = serde_json::to_string(analysis_run)
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let impact = analysis.impact.as_ref().expect("validated above");
        for entity_id in &impact.impacted_entity_ids {
            if self
                .entity_for(&analysis.context, entity_id)
                .await?
                .is_none()
            {
                return Err(StorageError::Query(format!(
                    "Change Graph impact references an entity absent from the canonical graph: {entity_id}"
                )));
            }
        }
        if self
            .entity_for(&analysis.context, &analysis.selected_entity_id)
            .await?
            .is_none()
        {
            return Err(StorageError::Query(
                "Change Graph selected entity is absent from the canonical graph".to_owned(),
            ));
        }
        let impact_id = crate::source::identity::derive_sha256_id(
            "chie-impact",
            "analysis-run",
            &analysis_run.id,
        );
        let risk_id = crate::source::identity::derive_sha256_id(
            "chie-risk",
            "analysis-run",
            &analysis_run.id,
        );
        let graph_name = self.graph_name_for_workspace(&analysis.context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;

        graph
            .query(
                "MERGE (n:ChangeSet {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) SET n.payload_json = $payload",
            )
            .with_param("id", change_set.id.clone())
            .with_param("workspace", analysis.context.workspace_id.clone())
            .with_param("repository", analysis.context.repository_id.clone())
            .with_param("snapshot", analysis.context.source_snapshot_id.clone())
            .with_param("graph_version", analysis.context.graph_version.clone())
            .with_param("payload", change_set_payload)
            .with_timeout(self.query_timeout_ms).execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;

        graph
            .query(
                "MERGE (n:ChangeRecord {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) SET n.payload_json = $payload",
            )
            .with_param("id", crate::source::identity::derive_sha256_id(
                "chie-change-record",
                "change-set",
                &change_record.change_set_id,
            ))
            .with_param("workspace", analysis.context.workspace_id.clone())
            .with_param("repository", analysis.context.repository_id.clone())
            .with_param("snapshot", analysis.context.source_snapshot_id.clone())
            .with_param("graph_version", analysis.context.graph_version.clone())
            .with_param("payload", change_record_payload)
            .with_timeout(self.query_timeout_ms).execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;

        graph
            .query(
                "MERGE (n:AnalysisRun {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) SET n.payload_json = $payload, n.status = $status",
            )
            .with_param("id", analysis_run.id.clone())
            .with_param("workspace", analysis.context.workspace_id.clone())
            .with_param("repository", analysis.context.repository_id.clone())
            .with_param("snapshot", analysis.context.source_snapshot_id.clone())
            .with_param("graph_version", analysis.context.graph_version.clone())
            .with_param("payload", run_payload)
            .with_param("status", analysis_run.status.clone())
            .with_timeout(self.query_timeout_ms).execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;

        graph
            .query(
                "MERGE (n:ImpactAnalysis {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) SET n.payload_json = $payload, n.availability = $availability, n.truncated = $truncated",
            )
            .with_param("id", impact_id.clone())
            .with_param("workspace", analysis.context.workspace_id.clone())
            .with_param("repository", analysis.context.repository_id.clone())
            .with_param("snapshot", analysis.context.source_snapshot_id.clone())
            .with_param("graph_version", analysis.context.graph_version.clone())
            .with_param("payload", serde_json::to_string(impact).map_err(|error| StorageError::Query(error.to_string()))?)
            .with_param("availability", format!("{:?}", impact.availability))
            .with_param("truncated", impact.truncated)
            .with_timeout(self.query_timeout_ms).execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;

        if let Some(risk) = analysis.risk_assessment.as_ref() {
            graph
                .query(
                    "MERGE (n:RiskAssessment {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) SET n.payload_json = $payload, n.level = $level",
                )
                .with_param("id", risk_id.clone())
                .with_param("workspace", analysis.context.workspace_id.clone())
                .with_param("repository", analysis.context.repository_id.clone())
                .with_param("snapshot", analysis.context.source_snapshot_id.clone())
                .with_param("graph_version", analysis.context.graph_version.clone())
                .with_param("payload", serde_json::to_string(risk).map_err(|error| StorageError::Query(error.to_string()))?)
                .with_param("level", format!("{:?}", risk.level))
                .with_timeout(self.query_timeout_ms).execute()
                .await
                .map_err(|error| StorageError::Query(error.to_string()))?;
        }

        for evidence in &analysis.evidence_records {
            graph
                .query(
                    "MERGE (n:Evidence {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) SET n.payload_json = $payload, n.derivation = $derivation",
                )
                .with_param("id", evidence.id.clone())
                .with_param("workspace", analysis.context.workspace_id.clone())
                .with_param("repository", analysis.context.repository_id.clone())
                .with_param("snapshot", analysis.context.source_snapshot_id.clone())
                .with_param("graph_version", analysis.context.graph_version.clone())
                .with_param("payload", serde_json::to_string(evidence).map_err(|error| StorageError::Query(error.to_string()))?)
                .with_param("derivation", evidence.derivation.clone())
                .with_timeout(self.query_timeout_ms).execute()
                .await
                .map_err(|error| StorageError::Query(error.to_string()))?;
        }

        graph
            .query(
                "MATCH (run:AnalysisRun {id: $run_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}), (set:ChangeSet {id: $set_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) MERGE (run)-[:ANALYZES]->(set)",
            )
            .with_param("run_id", analysis_run.id.clone())
            .with_param("set_id", change_set.id.clone())
            .with_param("workspace", analysis.context.workspace_id.clone())
            .with_param("repository", analysis.context.repository_id.clone())
            .with_param("snapshot", analysis.context.source_snapshot_id.clone())
            .with_param("graph_version", analysis.context.graph_version.clone())
            .with_timeout(self.query_timeout_ms).execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;

        graph
            .query(
                "MATCH (set:ChangeSet {id: $set_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}), (record:ChangeRecord {id: $record_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) MERGE (set)-[:CONTAINS_CHANGE]->(record)",
            )
            .with_param("set_id", change_set.id.clone())
            .with_param("record_id", crate::source::identity::derive_sha256_id(
                "chie-change-record",
                "change-set",
                &change_record.change_set_id,
            ))
            .with_param("workspace", analysis.context.workspace_id.clone())
            .with_param("repository", analysis.context.repository_id.clone())
            .with_param("snapshot", analysis.context.source_snapshot_id.clone())
            .with_param("graph_version", analysis.context.graph_version.clone())
            .with_timeout(self.query_timeout_ms).execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;

        graph
            .query(
                "MATCH (run:AnalysisRun {id: $run_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}), (impact:ImpactAnalysis {id: $impact_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) MERGE (run)-[:PRODUCES]->(impact)",
            )
            .with_param("run_id", analysis_run.id.clone())
            .with_param("impact_id", impact_id.clone())
            .with_param("workspace", analysis.context.workspace_id.clone())
            .with_param("repository", analysis.context.repository_id.clone())
            .with_param("snapshot", analysis.context.source_snapshot_id.clone())
            .with_param("graph_version", analysis.context.graph_version.clone())
            .with_timeout(self.query_timeout_ms).execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;

        for entity_id in &impact.impacted_entity_ids {
            graph
                .query(
                    "MATCH (impact:ImpactAnalysis {id: $impact_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}), (entity:CodeEntity {id: $entity_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) MERGE (impact)-[:AFFECTS]->(entity)",
                )
                .with_param("impact_id", impact_id.clone())
                .with_param("entity_id", entity_id.clone())
                .with_param("workspace", analysis.context.workspace_id.clone())
                .with_param("repository", analysis.context.repository_id.clone())
                .with_param("snapshot", analysis.context.source_snapshot_id.clone())
                .with_param("graph_version", analysis.context.graph_version.clone())
                .with_timeout(self.query_timeout_ms).execute()
                .await
                .map_err(|error| StorageError::Query(error.to_string()))?;
        }

        if analysis.risk_assessment.is_some() {
            graph
                .query(
                    "MATCH (run:AnalysisRun {id: $run_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}), (risk:RiskAssessment {id: $risk_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) MERGE (run)-[:HAS_RISK]->(risk)",
                )
                .with_param("run_id", analysis_run.id.clone())
                .with_param("risk_id", risk_id.clone())
                .with_param("workspace", analysis.context.workspace_id.clone())
                .with_param("repository", analysis.context.repository_id.clone())
                .with_param("snapshot", analysis.context.source_snapshot_id.clone())
                .with_param("graph_version", analysis.context.graph_version.clone())
                .with_timeout(self.query_timeout_ms).execute()
                .await
                .map_err(|error| StorageError::Query(error.to_string()))?;

            for evidence in &analysis.evidence_records {
                graph
                    .query(
                        "MATCH (risk:RiskAssessment {id: $risk_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}), (e:Evidence {id: $evidence_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) MERGE (risk)-[:SUPPORTED_BY]->(e)",
                    )
                    .with_param("risk_id", risk_id.clone())
                    .with_param("evidence_id", evidence.id.clone())
                    .with_param("workspace", analysis.context.workspace_id.clone())
                    .with_param("repository", analysis.context.repository_id.clone())
                    .with_param("snapshot", analysis.context.source_snapshot_id.clone())
                    .with_param("graph_version", analysis.context.graph_version.clone())
                    .with_timeout(self.query_timeout_ms).execute()
                    .await
                    .map_err(|error| StorageError::Query(error.to_string()))?;
            }
        }

        for candidate in &analysis.test_candidates {
            graph
                .query(
                    "MATCH (run:AnalysisRun {id: $run_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}), (test:TestEntity {id: $test_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) MERGE (run)-[:RECOMMENDS]->(test)",
                )
                .with_param("run_id", analysis_run.id.clone())
                .with_param("test_id", candidate.test_id.clone())
                .with_param("workspace", analysis.context.workspace_id.clone())
                .with_param("repository", analysis.context.repository_id.clone())
                .with_param("snapshot", analysis.context.source_snapshot_id.clone())
                .with_param("graph_version", analysis.context.graph_version.clone())
                .with_timeout(self.query_timeout_ms).execute()
                .await
                .map_err(|error| StorageError::Query(error.to_string()))?;

            for evidence in &analysis.evidence_records {
                if evidence.test_id.as_deref() != Some(candidate.test_id.as_str()) {
                    continue;
                }
                graph
                    .query(
                        "MATCH (test:TestEntity {id: $test_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}), (e:Evidence {id: $evidence_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) MERGE (test)-[:SUPPORTED_BY]->(e)",
                    )
                    .with_param("test_id", candidate.test_id.clone())
                    .with_param("evidence_id", evidence.id.clone())
                    .with_param("workspace", analysis.context.workspace_id.clone())
                    .with_param("repository", analysis.context.repository_id.clone())
                    .with_param("snapshot", analysis.context.source_snapshot_id.clone())
                    .with_param("graph_version", analysis.context.graph_version.clone())
                    .with_timeout(self.query_timeout_ms).execute()
                    .await
                    .map_err(|error| StorageError::Query(error.to_string()))?;
            }
        }

        for evidence in &analysis.evidence_records {
            graph
                .query(
                    "MATCH (impact:ImpactAnalysis {id: $impact_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}), (e:Evidence {id: $evidence_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) MERGE (impact)-[:SUPPORTED_BY]->(e)",
                )
                .with_param("impact_id", impact_id.clone())
                .with_param("evidence_id", evidence.id.clone())
                .with_param("workspace", analysis.context.workspace_id.clone())
                .with_param("repository", analysis.context.repository_id.clone())
                .with_param("snapshot", analysis.context.source_snapshot_id.clone())
                .with_param("graph_version", analysis.context.graph_version.clone())
                .with_timeout(self.query_timeout_ms).execute()
                .await
                .map_err(|error| StorageError::Query(error.to_string()))?;
        }
        Ok(())
    }

    async fn relationships_for(
        &self,
        context: &RepositoryContext,
        entity_id: &str,
        limit: usize,
    ) -> Result<Vec<GraphRelationship>, StorageError> {
        self.query_relationships_impl(context, entity_id, false, limit)
            .await
    }

    async fn incoming_relationships_for(
        &self,
        context: &RepositoryContext,
        entity_id: &str,
        limit: usize,
    ) -> Result<Vec<GraphRelationship>, StorageError> {
        self.query_relationships_impl(context, entity_id, true, limit)
            .await
    }

    async fn derived_relationships_for_run(
        &self,
        context: &RepositoryContext,
        analysis_run_id: &str,
        limit: usize,
    ) -> Result<Vec<crate::graph_storage::port::DerivedRelationship>, StorageError> {
        Self::validate_context(context)?;
        let graph_name = self.graph_name_for_workspace(&context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;
        let mut rows = graph
            .ro_query(
                "MATCH (run:AnalysisRun {id: $run_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})-[r1:PRODUCES|ANALYZES|HAS_RISK|RECOMMENDS]->(target) RETURN type(r1) AS kind, run.id AS from_id, target.id AS to_id UNION MATCH (impact:ImpactAnalysis {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})-[r2:AFFECTS|SUPPORTED_BY]->(target) MATCH (run:AnalysisRun {id: $run_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})-[:PRODUCES]->(impact) RETURN type(r2) AS kind, impact.id AS from_id, target.id AS to_id UNION MATCH (set:ChangeSet {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})-[r3:CONTAINS_CHANGE]->(target) MATCH (run:AnalysisRun {id: $run_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})-[:ANALYZES]->(set) RETURN type(r3) AS kind, set.id AS from_id, target.id AS to_id UNION MATCH (risk:RiskAssessment {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})-[r4:SUPPORTED_BY]->(target) MATCH (run:AnalysisRun {id: $run_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})-[:HAS_RISK]->(risk) RETURN type(r4) AS kind, risk.id AS from_id, target.id AS to_id UNION MATCH (test:TestEntity {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})-[r5:SUPPORTED_BY]->(e:Evidence {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) MATCH (run:AnalysisRun {id: $run_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})-[:RECOMMENDS]->(test) MATCH (run)-[:PRODUCES]->(impact:ImpactAnalysis)-[:SUPPORTED_BY]->(e) RETURN type(r5) AS kind, test.id AS from_id, e.id AS to_id LIMIT $limit",
            )
            .with_param("run_id", analysis_run_id.to_owned())
            .with_param("workspace", context.workspace_id.clone())
            .with_param("repository", context.repository_id.clone())
            .with_param("snapshot", context.source_snapshot_id.clone())
            .with_param("graph_version", context.graph_version.clone())
            .with_param("limit", crate::graph_storage::falkordb::read::bounded_limit(limit) as i64)
            .with_timeout(self.query_timeout_ms).execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let mut relationships = Vec::new();
        while let Some(row) = rows.data.next().await {
            let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
            relationships.push(crate::graph_storage::port::DerivedRelationship {
                kind: row
                    .try_get("kind")
                    .map_err(|error| StorageError::Query(error.to_string()))?,
                from: row
                    .try_get("from_id")
                    .map_err(|error| StorageError::Query(error.to_string()))?,
                to: row
                    .try_get("to_id")
                    .map_err(|error| StorageError::Query(error.to_string()))?,
            });
        }
        Ok(relationships)
    }

    async fn entity_for(
        &self,
        context: &RepositoryContext,
        entity_id: &str,
    ) -> Result<Option<GraphEntity>, StorageError> {
        Self::validate_context(context)?;
        let graph_name = self.graph_name_for_workspace(&context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;
        let mut rows = graph
            .ro_query(
                "MATCH (n:CodeEntity {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) \
                 RETURN n.id AS id, n.kind AS kind, n.name AS name, n.path AS path, n.line AS line LIMIT 1",
            )
            .with_param("id", entity_id.to_owned())
            .with_param("workspace", context.workspace_id.clone())
            .with_param("repository", context.repository_id.clone())
            .with_param("snapshot", context.source_snapshot_id.clone())
            .with_param("graph_version", context.graph_version.clone())
            .with_timeout(self.query_timeout_ms).execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;

        let Some(row) = rows.data.next().await else {
            return Ok(None);
        };
        let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
        let kind = parse_entity_kind(
            row.try_get("kind")
                .map_err(|error| StorageError::Query(error.to_string()))?,
        )?;

        Ok(Some(GraphEntity {
            id: row
                .try_get("id")
                .map_err(|error| StorageError::Query(error.to_string()))?,
            kind,
            name: row
                .try_get("name")
                .map_err(|error| StorageError::Query(error.to_string()))?,
            path: row
                .try_get("path")
                .map_err(|error| StorageError::Query(error.to_string()))?,
            line: row
                .try_get::<i64>("line")
                .map_err(|error| StorageError::Query(error.to_string()))? as u32,
            context: context.clone(),
            language: None,
            native_kind: None,
            range: None,
            resolution: crate::engines::coie::model::ResolutionStatus::Resolved,
            evidence: None,
        }))
    }

    async fn exact_code_entities_by_name(
        &self,
        context: &RepositoryContext,
        name: &str,
    ) -> Result<Vec<GraphEntity>, StorageError> {
        Self::validate_context(context)?;
        let name = name.trim();
        if name.is_empty() || name.len() > 500 {
            return Err(StorageError::Query(
                "entity name must contain 1 to 500 characters".to_owned(),
            ));
        }
        let graph_name = self.graph_name_for_workspace(&context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;
        let mut rows = graph
            .ro_query(
                "MATCH (n:CodeEntity {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) WHERE n.name = $name AND n.kind IN ['Symbol', 'File'] RETURN n.id AS id, n.kind AS kind, n.name AS name, n.path AS path, n.line AS line ORDER BY n.path, n.line",
            )
            .with_param("name", name.to_owned())
            .with_param("workspace", context.workspace_id.clone())
            .with_param("repository", context.repository_id.clone())
            .with_param("snapshot", context.source_snapshot_id.clone())
            .with_param("graph_version", context.graph_version.clone())
            .with_timeout(self.query_timeout_ms).execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;

        let mut entities = Vec::new();
        while let Some(row) = rows.data.next().await {
            let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
            entities.push(GraphEntity {
                id: row
                    .try_get("id")
                    .map_err(|error| StorageError::Query(error.to_string()))?,
                kind: parse_entity_kind(
                    row.try_get("kind")
                        .map_err(|error| StorageError::Query(error.to_string()))?,
                )?,
                name: row
                    .try_get("name")
                    .map_err(|error| StorageError::Query(error.to_string()))?,
                path: row
                    .try_get("path")
                    .map_err(|error| StorageError::Query(error.to_string()))?,
                line: row
                    .try_get::<i64>("line")
                    .map_err(|error| StorageError::Query(error.to_string()))?
                    as u32,
                context: context.clone(),
                language: None,
                native_kind: None,
                range: None,
                resolution: crate::engines::coie::model::ResolutionStatus::Resolved,
                evidence: None,
            });
        }
        Ok(entities)
    }

    async fn search_code_entities(
        &self,
        context: &RepositoryContext,
        query: &str,
        limit: usize,
    ) -> Result<Vec<GraphEntity>, StorageError> {
        Self::validate_context(context)?;
        let query = query.trim();
        if query.is_empty() || query.len() > 500 {
            return Err(StorageError::Query(
                "search query must contain 1 to 500 characters".to_owned(),
            ));
        }
        let limit = crate::graph_storage::falkordb::read::bounded_search_limit(limit);
        let graph_name = self.graph_name_for_workspace(&context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;
        let mut rows = graph
            .ro_query(
                "MATCH (n:CodeEntity {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) \
                 WHERE toLower(n.name) CONTAINS toLower($query) OR toLower(n.path) CONTAINS toLower($query) \
                 RETURN n.id AS id, n.kind AS kind, n.name AS name, n.path AS path, n.line AS line \
                 ORDER BY n.path, n.line LIMIT $limit",
            )
            .with_param("query", query.to_owned())
            .with_param("workspace", context.workspace_id.clone())
            .with_param("repository", context.repository_id.clone())
            .with_param("snapshot", context.source_snapshot_id.clone())
            .with_param("graph_version", context.graph_version.clone())
            .with_param("limit", limit as i64)
            .with_timeout(self.query_timeout_ms).execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;

        let mut entities = Vec::new();
        while let Some(row) = rows.data.next().await {
            let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
            entities.push(GraphEntity {
                id: row
                    .try_get("id")
                    .map_err(|error| StorageError::Query(error.to_string()))?,
                kind: parse_entity_kind(
                    row.try_get("kind")
                        .map_err(|error| StorageError::Query(error.to_string()))?,
                )?,
                name: row
                    .try_get("name")
                    .map_err(|error| StorageError::Query(error.to_string()))?,
                path: row
                    .try_get("path")
                    .map_err(|error| StorageError::Query(error.to_string()))?,
                line: row
                    .try_get::<i64>("line")
                    .map_err(|error| StorageError::Query(error.to_string()))?
                    as u32,
                context: context.clone(),
                language: None,
                native_kind: None,
                range: None,
                resolution: crate::engines::coie::model::ResolutionStatus::Resolved,
                evidence: None,
            });
        }
        Ok(entities)
    }

    async fn dependency_paths_for(
        &self,
        context: &RepositoryContext,
        entity_id: &str,
        max_depth: usize,
        max_paths: usize,
    ) -> Result<crate::graph_storage::port::DependencyPathResult, StorageError> {
        Self::validate_context(context)?;
        if entity_id.trim().is_empty() || max_depth == 0 || max_paths == 0 {
            return Ok(crate::graph_storage::port::DependencyPathResult {
                paths: Vec::new(),
                truncated: false,
            });
        }

        let mut frontier = vec![(entity_id.to_owned(), Vec::<GraphRelationship>::new())];
        let mut completed = Vec::new();
        let mut truncated = false;

        for _ in 0..max_depth {
            if frontier.is_empty() || completed.len() >= max_paths {
                break;
            }

            let mut next = Vec::new();
            for (current, path) in frontier {
                let remaining = max_paths.saturating_sub(completed.len()).max(1);
                let incoming = self
                    .dependency_incoming_relationships_for(context, &current, remaining + 1)
                    .await?;

                if incoming.len() > remaining {
                    truncated = true;
                }

                for relationship in incoming.into_iter().take(remaining) {
                    let mut next_path = path.clone();
                    if next_path
                        .iter()
                        .any(|existing| existing.id == relationship.id)
                    {
                        continue;
                    }
                    next_path.push(relationship.clone());
                    completed.push(crate::graph_storage::port::GraphPath {
                        terminal_entity_id: relationship.from.clone(),
                        relationships: next_path.clone(),
                    });
                    if completed.len() >= max_paths {
                        break;
                    }
                    next.push((relationship.from, next_path));
                }
                if completed.len() >= max_paths {
                    truncated = true;
                    break;
                }
            }
            frontier = next;
        }

        Ok(crate::graph_storage::port::DependencyPathResult {
            paths: completed,
            truncated,
        })
    }

    async fn dependency_incoming_relationships_for(
        &self,
        context: &RepositoryContext,
        entity_id: &str,
        limit: usize,
    ) -> Result<Vec<GraphRelationship>, StorageError> {
        Self::validate_context(context)?;
        let limit = crate::graph_storage::falkordb::read::bounded_limit(limit);
        let graph_name = self.graph_name_for_workspace(&context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;
        let mut rows = graph
            .ro_query(
                "MATCH (a:CodeEntity {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})-[r:IMPORTS|CALLS]->(b:CodeEntity {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) WHERE r.workspace_id = $workspace AND r.repository_id = $repository AND r.snapshot_id = $snapshot AND r.graph_version = $graph_version RETURN r.id AS relationship_id, type(r) AS relationship_type, a.id AS from_id, b.id AS to_id, r.resolution AS resolution, r.evidence_path AS evidence_path, r.evidence_start_line AS evidence_start_line, r.evidence_end_line AS evidence_end_line, r.evidence_snapshot_id AS evidence_snapshot_id, r.evidence_provenance AS evidence_provenance LIMIT $limit",
            )
            .with_param("id", entity_id.to_owned())
            .with_param("workspace", context.workspace_id.clone())
            .with_param("repository", context.repository_id.clone())
            .with_param("snapshot", context.source_snapshot_id.clone())
            .with_param("graph_version", context.graph_version.clone())
            .with_param("limit", limit as i64)
            .with_timeout(self.query_timeout_ms).execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;

        let mut relationships = Vec::new();
        while let Some(row) = rows.data.next().await {
            let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
            let resolution = parse_resolution(
                row.try_get("resolution")
                    .map_err(|error| StorageError::Query(error.to_string()))?,
            )?;
            let evidence_path: Option<String> = row
                .try_get("evidence_path")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let evidence = match evidence_path.filter(|path| !path.is_empty()) {
                Some(path) => Some(crate::engines::coie::model::SourceEvidence {
                    path,
                    start_line: row
                        .try_get::<Option<i64>>("evidence_start_line")
                        .map_err(|error| StorageError::Query(error.to_string()))?
                        .unwrap_or(0) as u32,
                    end_line: row
                        .try_get::<Option<i64>>("evidence_end_line")
                        .map_err(|error| StorageError::Query(error.to_string()))?
                        .unwrap_or(0) as u32,
                    snapshot_id: row
                        .try_get::<Option<String>>("evidence_snapshot_id")
                        .map_err(|error| StorageError::Query(error.to_string()))?
                        .unwrap_or_default(),
                    provenance: row
                        .try_get::<Option<String>>("evidence_provenance")
                        .map_err(|error| StorageError::Query(error.to_string()))?
                        .unwrap_or_default(),
                }),
                None => None,
            };
            relationships.push(GraphRelationship {
                id: row
                    .try_get("relationship_id")
                    .map_err(|error| StorageError::Query(error.to_string()))?,
                kind: parse_relationship(
                    row.try_get("relationship_type")
                        .map_err(|error| StorageError::Query(error.to_string()))?,
                )?,
                from: row
                    .try_get("from_id")
                    .map_err(|error| StorageError::Query(error.to_string()))?,
                to: row
                    .try_get("to_id")
                    .map_err(|error| StorageError::Query(error.to_string()))?,
                context: context.clone(),
                resolution,
                evidence,
            });
        }
        Ok(relationships)
    }

    async fn tests_for_context(
        &self,
        context: &RepositoryContext,
        target_entity_ids: &[String],
        limit: usize,
    ) -> Result<crate::graph_storage::port::StoredTestGraph, StorageError> {
        Self::validate_context(context)?;
        // Keep this read on the same hard bound as every other graph read.
        // The old local cap of 100 could silently omit a requested TARGETS
        // edge on high-fan-in symbols even when the caller requested 200/500.
        let limit = crate::graph_storage::falkordb::read::bounded_limit(limit);
        let query_limit = limit.saturating_add(1);
        let graph_name = self.graph_name_for_workspace(&context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;

        let mut relationships = Vec::new();
        if !target_entity_ids.is_empty() {
            let mut rows = graph
                .ro_query(
                    // TARGETS are the primary answer for a code-entity lookup.
                    // Sort by the actual relationship label rather than a
                    // mutable auxiliary property so DECLARED_BY rows cannot
                    // consume the bounded page before TARGETS rows.
                    "MATCH (t:TestEntity {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})-[r:TARGETS|DECLARED_BY]->(e:CodeEntity {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) WHERE e.id IN $targets RETURN r.id AS relationship_id, t.id AS test_id, e.id AS entity_id, e.name AS target_symbol_name, type(r) AS relationship_type, r.evidence_path AS evidence_path, r.evidence_start_line AS start_line, r.evidence_end_line AS end_line, r.evidence_snapshot_id AS evidence_snapshot_id, r.evidence_provenance AS evidence_provenance, r.resolution AS resolution ORDER BY relationship_type DESC, relationship_id ASC LIMIT $limit",
                )
                .with_param("workspace", context.workspace_id.clone())
                .with_param("repository", context.repository_id.clone())
                .with_param("snapshot", context.source_snapshot_id.clone())
                .with_param("graph_version", context.graph_version.clone())
                .with_param("targets", target_entity_ids.to_vec())
                .with_param("limit", query_limit as i64)
                .with_timeout(self.query_timeout_ms).execute()
                .await
                .map_err(|error| StorageError::Query(error.to_string()))?;

            while let Some(row) = rows.data.next().await {
                let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
                let relationship_kind = parse_test_relationship(
                    row.try_get("relationship_type")
                        .map_err(|error| StorageError::Query(error.to_string()))?,
                )?;
                let evidence_path: Option<String> = row
                    .try_get("evidence_path")
                    .map_err(|error| StorageError::Query(error.to_string()))?;
                let evidence_path = evidence_path.unwrap_or_default();
                let evidence = if evidence_path.is_empty() {
                    None
                } else {
                    Some(crate::engines::coie::model::SourceEvidence {
                        path: evidence_path.clone(),
                        start_line: row
                            .try_get::<Option<i64>>("start_line")
                            .map_err(|error| StorageError::Query(error.to_string()))?
                            .unwrap_or(0) as u32,
                        end_line: row
                            .try_get::<Option<i64>>("end_line")
                            .map_err(|error| StorageError::Query(error.to_string()))?
                            .unwrap_or(0) as u32,
                        snapshot_id: row
                            .try_get::<Option<String>>("evidence_snapshot_id")
                            .map_err(|error| StorageError::Query(error.to_string()))?
                            .unwrap_or_default(),
                        provenance: row
                            .try_get::<Option<String>>("evidence_provenance")
                            .map_err(|error| StorageError::Query(error.to_string()))?
                            .unwrap_or_default(),
                    })
                };
                let resolution = parse_test_resolution(
                    row.try_get("resolution")
                        .map_err(|error| StorageError::Query(error.to_string()))?,
                )?;
                relationships.push(TestRelationship {
                    id: row
                        .try_get("relationship_id")
                        .map_err(|error| StorageError::Query(error.to_string()))?,
                    kind: relationship_kind,
                    test_id: row
                        .try_get("test_id")
                        .map_err(|error| StorageError::Query(error.to_string()))?,
                    // The relationship stores the target entity ID. Resolve its
                    // current semantic name from the matched CodeEntity so a
                    // storage read-back preserves the in-memory TIE contract.
                    target_symbol_name: Some(
                        row.try_get("target_symbol_name")
                            .map_err(|error| StorageError::Query(error.to_string()))?,
                    ),
                    entity_id: Some(
                        row.try_get("entity_id")
                            .map_err(|error| StorageError::Query(error.to_string()))?,
                    ),
                    evidence_path,
                    evidence,
                    resolution,
                    target_kind: crate::engines::tie::model::TestTargetKind::Code,
                    context: context.clone(),
                });
            }
        }

        let mut truncated = relationships.len() > limit;
        relationships.truncate(limit);
        let test_ids: Vec<String> = relationships
            .iter()
            .map(|relationship| relationship.test_id.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let mut tests = Vec::new();
        if !test_ids.is_empty() {
            let mut rows = graph
                .ro_query(
                    "MATCH (t:TestEntity {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) WHERE t.kind = 'Test' AND t.id IN $test_ids RETURN t.id AS id, t.name AS name, t.path AS path, t.line AS line, t.framework AS framework, t.status AS status, t.kind AS kind LIMIT $limit",
                )
                .with_param("workspace", context.workspace_id.clone())
                .with_param("repository", context.repository_id.clone())
                .with_param("snapshot", context.source_snapshot_id.clone())
                .with_param("graph_version", context.graph_version.clone())
                .with_param("test_ids", test_ids)
                .with_param("limit", query_limit as i64)
                .with_timeout(self.query_timeout_ms).execute()
                .await
                .map_err(|error| StorageError::Query(error.to_string()))?;

            while let Some(row) = rows.data.next().await {
                let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
                tests.push(TestEntity {
                    id: row
                        .try_get("id")
                        .map_err(|error| StorageError::Query(error.to_string()))?,
                    name: row
                        .try_get("name")
                        .map_err(|error| StorageError::Query(error.to_string()))?,
                    path: row
                        .try_get("path")
                        .map_err(|error| StorageError::Query(error.to_string()))?,
                    line: row
                        .try_get::<i64>("line")
                        .map_err(|error| StorageError::Query(error.to_string()))?
                        as u32,
                    framework: row
                        .try_get("framework")
                        .map_err(|error| StorageError::Query(error.to_string()))?,
                    status: parse_test_status(
                        row.try_get("status")
                            .map_err(|error| StorageError::Query(error.to_string()))?,
                    )?,
                    kind: parse_test_node_kind(
                        row.try_get("kind")
                            .map_err(|error| StorageError::Query(error.to_string()))?,
                    )?,
                    context: context.clone(),
                });
            }
        }

        truncated |= tests.len() > limit;
        tests.truncate(limit);
        Ok(crate::graph_storage::port::StoredTestGraph {
            tests,
            relationships,
            truncated,
        })
    }

    async fn write_history_graph(
        &self,
        history: &HistoryGraph,
        repository_generation_id: &str,
    ) -> Result<(), StorageError> {
        history
            .validate()
            .map_err(|error| StorageError::Query(error.to_string()))?;
        if repository_generation_id.trim().is_empty() || repository_generation_id.len() > 256 {
            return Err(StorageError::Query(
                "History Graph generation ID is invalid".to_owned(),
            ));
        }
        let context = &history.context;
        let graph_name = self.graph_name_for_workspace(&context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;
        let entity_ids: Vec<String> = history
            .entities
            .iter()
            .map(|entity| entity.id.clone())
            .collect();
        let relationship_ids: Vec<String> = history
            .relationships
            .iter()
            .map(|relationship| relationship.id.clone())
            .collect();
        let entity_digest = digest_identity_set(&entity_ids);
        let relationship_digest = digest_identity_set(&relationship_ids);
        let entity_content_digest = digest_serialized_records(&history.entities)?;
        let relationship_content_digest = digest_serialized_records(&history.relationships)?;
        let storage_generation_id = digest_text(&format!(
            "codemind-history-storage-generation-v3\0{repository_generation_id}\0{}\0{}\0{}\0{}\0{}\0{}",
            history.head_commit,
            entity_content_digest,
            relationship_content_digest,
            history.complete,
            history.omitted_commits_at_least,
            history.omitted_path_changes_at_least,
        ));
        // The summary and every fact are content-addressed. A failed new write
        // cannot overwrite the previously active History Graph generation.
        graph.query(
            "MERGE (g:HistoryGeneration {id: $storage_generation, workspace_id: $workspace, repository_id: $repository}) SET g.repository_generation_id = $repository_generation, g.head_commit = $head_commit, g.history_complete = $history_complete, g.omitted_commits_at_least = $omitted_commits, g.omitted_path_changes_at_least = $omitted_paths, g.schema_version = $schema_version, g.expected_entity_count = $entity_count, g.expected_relationship_count = $relationship_count, g.expected_entity_digest = $entity_digest, g.expected_relationship_digest = $relationship_digest, g.expected_entity_content_digest = $entity_content_digest, g.expected_relationship_content_digest = $relationship_content_digest, g.created_at = $created_at",
        ).with_param("storage_generation", storage_generation_id.clone()).with_param("repository_generation", repository_generation_id.to_owned()).with_param("workspace", context.workspace_id.clone()).with_param("repository", context.repository_id.clone()).with_param("head_commit", history.head_commit.clone()).with_param("history_complete", history.complete).with_param("omitted_commits", history.omitted_commits_at_least as i64).with_param("omitted_paths", history.omitted_path_changes_at_least as i64).with_param("schema_version", history.schema_version as i64).with_param("entity_count", entity_ids.len() as i64).with_param("relationship_count", relationship_ids.len() as i64).with_param("entity_digest", entity_digest).with_param("relationship_digest", relationship_digest).with_param("entity_content_digest", entity_content_digest).with_param("relationship_content_digest", relationship_content_digest).with_param("created_at", Utc::now().to_rfc3339()).with_timeout(self.query_timeout_ms).execute().await.map_err(|error| StorageError::Query(error.to_string()))?;

        for entity in &history.entities {
            let label = match entity.data.entity_kind() {
                crate::engines::history::model::HistoryEntityKind::Repository => {
                    "HistoryRepository"
                }
                crate::engines::history::model::HistoryEntityKind::Commit => "HistoryCommit",
                crate::engines::history::model::HistoryEntityKind::Directory => "HistoryDirectory",
                crate::engines::history::model::HistoryEntityKind::File => "HistoryFile",
                crate::engines::history::model::HistoryEntityKind::Change => "HistoryChange",
            };
            let payload = serde_json::to_string(entity)
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let storage_id = history_generation_storage_id(&storage_generation_id, &entity.id);
            let merge_key = crate::graph_storage::falkordb::write::history_stable_merge_key(
                &context.workspace_id,
                &context.repository_id,
                &storage_id,
            )
            .map_err(StorageError::Query)?;
            let commit_id = match &entity.data {
                crate::engines::history::model::HistoryEntityData::Commit { commit_id, .. } => {
                    Some(commit_id.clone())
                }
                _ => None,
            };
            let query = format!(
                "MERGE (n:{label} {{id: $id, workspace_id: $workspace, repository_id: $repository, generation_id: $generation}}) SET n.logical_id = $logical_id, n.payload_json = $payload, n.schema_version = $schema_version, n.merge_key = $merge_key, n.commit_id = $commit_id"
            );
            graph
                .query(query)
                .with_param("id", storage_id)
                .with_param("logical_id", entity.id.clone())
                .with_param("generation", storage_generation_id.clone())
                .with_param("workspace", context.workspace_id.clone())
                .with_param("repository", context.repository_id.clone())
                .with_param("payload", payload)
                .with_param("schema_version", history.schema_version as i64)
                .with_param("merge_key", merge_key)
                .with_param("commit_id", commit_id)
                .with_timeout(self.query_timeout_ms)
                .execute()
                .await
                .map_err(|error| StorageError::Query(error.to_string()))?;
        }
        for relationship in &history.relationships {
            let relationship_type =
                crate::graph_storage::falkordb::mapping::history_relationship_label(
                    &relationship.kind,
                );
            let from_storage_id =
                history_generation_storage_id(&storage_generation_id, &relationship.from);
            let to_storage_id =
                history_generation_storage_id(&storage_generation_id, &relationship.to);
            let storage_relationship_id =
                history_generation_storage_id(&storage_generation_id, &relationship.id);
            let payload = serde_json::to_string(relationship)
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let query = format!(
                "MATCH (a {{id: $from, workspace_id: $workspace, repository_id: $repository, generation_id: $generation}}), (b {{id: $to, workspace_id: $workspace, repository_id: $repository, generation_id: $generation}}) MERGE (a)-[r:{relationship_type} {{id: $id, workspace_id: $workspace, repository_id: $repository, generation_id: $generation}}]->(b) SET r.logical_id = $logical_id, r.from_logical_id = $from_logical_id, r.to_logical_id = $to_logical_id, r.domain_kind = $domain_kind, r.path_role = $path_role, r.payload_json = $payload"
            );
            graph
                .query(query)
                .with_param("from", from_storage_id)
                .with_param("to", to_storage_id)
                .with_param("id", storage_relationship_id)
                .with_param("logical_id", relationship.id.clone())
                .with_param("from_logical_id", relationship.from.clone())
                .with_param("to_logical_id", relationship.to.clone())
                .with_param("domain_kind", format!("{:?}", relationship.kind))
                .with_param("payload", payload)
                .with_param("generation", storage_generation_id.clone())
                .with_param("workspace", context.workspace_id.clone())
                .with_param("repository", context.repository_id.clone())
                .with_param(
                    "path_role",
                    relationship
                        .path_role
                        .as_ref()
                        .map(|value| format!("{:?}", value)),
                )
                .with_timeout(self.query_timeout_ms)
                .execute()
                .await
                .map_err(|error| StorageError::Query(error.to_string()))?;
        }
        Ok(())
    }

    async fn history_entities_for(
        &self,
        context: &HistoryGraphContext,
        limit: usize,
    ) -> Result<Vec<HistoryEntity>, StorageError> {
        context
            .validate()
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let graph_name = self.graph_name_for_workspace(&context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;
        let mut rows = graph.ro_query(
            "MATCH (active:CodeMindActiveGeneration {id: $active_id, workspace_id: $workspace, repository_id: $repository}), (g:HistoryGeneration {workspace_id: $workspace, repository_id: $repository}), (n) WHERE g.id = active.history_generation_id AND g.verified = true AND g.published = true AND g.repository_generation_id = active.generation_id AND n.workspace_id = $workspace AND n.repository_id = $repository AND n.generation_id = g.id AND ANY(label IN labels(n) WHERE label IN $labels) RETURN n.payload_json AS payload ORDER BY n.logical_id ASC LIMIT $limit",
        ).with_param("active_id", crate::graph_storage::falkordb::schema::ACTIVE_GENERATION_ID).with_param("workspace", context.workspace_id.clone()).with_param("repository", context.repository_id.clone()).with_param("labels", crate::graph_storage::falkordb::schema::HISTORY_ENTITY_LABELS.iter().map(|v| (*v).to_owned()).collect::<Vec<_>>()).with_param("limit", crate::graph_storage::falkordb::read::bounded_limit(limit) as i64).with_timeout(self.query_timeout_ms).execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
        let mut entities = Vec::new();
        while let Some(row) = rows.data.next().await {
            let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
            let payload: String = row
                .try_get("payload")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let entity: HistoryEntity = serde_json::from_str(&payload)
                .map_err(|error| StorageError::Query(error.to_string()))?;
            if entity.context != *context {
                return Err(StorageError::InvalidContext);
            }
            entities.push(entity);
        }
        Ok(entities)
    }

    async fn verify_graph(
        &self,
        context: &RepositoryContext,
        expected: &GraphVerificationExpectation,
    ) -> Result<GraphVerificationReport, StorageError> {
        Self::validate_context(context)?;
        expected.validate()?;
        if expected.context != *context {
            return Err(StorageError::InvalidContext);
        }
        let history_context = HistoryGraphContext::from_source_context(context)
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let (change_entities, change_relationships) = self.verify_change_slice(context).await?;
        let (
            history_head_commit,
            history_complete,
            history_omitted_commits_at_least,
            history_omitted_path_changes_at_least,
        ) = self
            .verify_history_state(
                &history_context,
                &expected.history_head_commit,
                &expected.generation_id,
            )
            .await?;
        let code_entities = self
            .count_nodes_with_labels(
                context,
                &[crate::graph_storage::falkordb::schema::CODE_ENTITY_LABEL],
            )
            .await?;
        let code_relationships = self
            .count_relationships_from_label(
                context,
                crate::graph_storage::falkordb::schema::CODE_ENTITY_LABEL,
                crate::graph_storage::falkordb::schema::CODE_RELATIONSHIP_LABELS,
            )
            .await?;
        let test_entities = self
            .count_nodes_with_labels(
                context,
                &[crate::graph_storage::falkordb::schema::TEST_ENTITY_LABEL],
            )
            .await?;
        let test_relationships = self
            .count_relationships_from_label(
                context,
                crate::graph_storage::falkordb::schema::TEST_ENTITY_LABEL,
                crate::graph_storage::falkordb::schema::TEST_RELATIONSHIP_LABELS,
            )
            .await?;
        // Persist the identities actually read back from storage. These values
        // become the durable verification receipt; they must not be copied from
        // the expectation because doing so would turn an assertion into proof.
        let code_entity_ids_digest = self
            .identity_digest_for_nodes(
                context,
                &[crate::graph_storage::falkordb::schema::CODE_ENTITY_LABEL],
                code_entities,
            )
            .await?;
        let code_relationship_ids_digest = self
            .identity_digest_for_relationships(
                context,
                crate::graph_storage::falkordb::schema::CODE_ENTITY_LABEL,
                crate::graph_storage::falkordb::schema::CODE_RELATIONSHIP_LABELS,
                code_relationships,
            )
            .await?;
        let test_entity_ids_digest = self
            .identity_digest_for_nodes(
                context,
                &[crate::graph_storage::falkordb::schema::TEST_ENTITY_LABEL],
                test_entities,
            )
            .await?;
        let test_relationship_ids_digest = self
            .identity_digest_for_relationships(
                context,
                crate::graph_storage::falkordb::schema::TEST_ENTITY_LABEL,
                crate::graph_storage::falkordb::schema::TEST_RELATIONSHIP_LABELS,
                test_relationships,
            )
            .await?;
        let semantic_match = code_entities == expected.code_entities
            && code_relationships == expected.code_relationships
            && test_entities == expected.test_entities
            && test_relationships == expected.test_relationships
            && code_entity_ids_digest == expected.code_entity_ids_digest
            && code_relationship_ids_digest == expected.code_relationship_ids_digest
            && test_entity_ids_digest == expected.test_entity_ids_digest
            && test_relationship_ids_digest == expected.test_relationship_ids_digest;
        if semantic_match {
            self.verify_code_graph_semantics(context).await?;
            self.verify_test_graph_semantics(context).await?;
        }
        let report = GraphVerificationReport {
            context: context.clone(),
            generation_id: expected.generation_id.clone(),
            code_entities,
            code_relationships,
            code_entity_ids_digest,
            code_relationship_ids_digest,
            test_entities,
            test_relationships,
            test_entity_ids_digest,
            test_relationship_ids_digest,
            history_entities: self
                .count_history_entities(&history_context, &expected.generation_id)
                .await?,
            history_relationships: self
                .count_history_relationships(&history_context, &expected.generation_id)
                .await?,
            history_head_commit,
            history_complete,
            history_omitted_commits_at_least,
            history_omitted_path_changes_at_least,
            change_entities,
            change_relationships,
            semantic_match,
        };
        if !report.satisfies(expected) {
            return Err(StorageError::Query(format!(
                "graph verification failed: expected {:?}, observed {:?}",
                expected, report
            )));
        }
        let graph_name = self.graph_name_for_workspace(&context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;
        graph.query(
            "MATCH (g:HistoryGeneration {repository_generation_id: $generation, workspace_id: $workspace, repository_id: $repository}) WITH g ORDER BY g.created_at DESC, g.id DESC LIMIT 1 SET g.verified = true",
        ).with_param("generation", expected.generation_id.clone()).with_param("workspace", context.workspace_id.clone()).with_param("repository", context.repository_id.clone()).with_timeout(self.query_timeout_ms).execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
        Ok(report)
    }

    async fn verified_snapshot_exists(
        &self,
        context: &RepositoryContext,
    ) -> Result<bool, StorageError> {
        Self::validate_context(context)?;
        let graph_name = self.graph_name_for_workspace(&context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;
        let mut rows = graph.ro_query("MATCH (n) WHERE n.workspace_id = $workspace AND n.repository_id = $repository AND n.snapshot_id = $snapshot AND n.graph_version = $graph_version AND (n:CodeEntity OR n:TestEntity OR n:ChangeSet OR n:ChangeRecord OR n:AnalysisRun) RETURN count(n) AS count")
            .with_param("workspace", context.workspace_id.clone()).with_param("repository", context.repository_id.clone()).with_param("snapshot", context.source_snapshot_id.clone()).with_param("graph_version", context.graph_version.clone()).with_timeout(self.query_timeout_ms).execute().await
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let row = required_count_row(rows.data.next().await, "verified snapshot existence query")?;
        let count: i64 = row
            .try_get("count")
            .map_err(|error| StorageError::Query(error.to_string()))?;
        if nonnegative_count(count, "verified snapshot existence query")? == 0 {
            return Ok(false);
        }
        let generation = crate::graph_storage::generation_id_for_context(context);
        self.verify_published_generation(context, &generation).await
    }

    async fn publish_verified_generation(
        &self,
        context: &RepositoryContext,
        generation_id: &str,
    ) -> Result<(), StorageError> {
        Self::validate_context(context)?;
        if generation_id.trim().is_empty() || generation_id.len() > 256 {
            return Err(StorageError::Query(
                "publication generation ID is invalid".to_owned(),
            ));
        }
        let graph_name = self.graph_name_for_workspace(&context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;
        graph.query(
            "MATCH (g:HistoryGeneration {repository_generation_id: $generation, workspace_id: $workspace, repository_id: $repository}) WITH g ORDER BY g.created_at DESC, g.id DESC LIMIT 1 WHERE g.verified = true MERGE (p:CodeMindActiveGeneration {id: $id, workspace_id: $workspace, repository_id: $repository}) SET g.published = true, p.generation_id = $generation, p.history_generation_id = g.id, p.source_snapshot_id = $snapshot, p.graph_version = $graph_version, p.published_at = $published_at",
        ).with_param("id", crate::graph_storage::falkordb::schema::ACTIVE_GENERATION_ID).with_param("workspace", context.workspace_id.clone()).with_param("repository", context.repository_id.clone()).with_param("generation", generation_id.to_owned()).with_param("snapshot", context.source_snapshot_id.clone()).with_param("graph_version", context.graph_version.clone()).with_param("published_at", Utc::now().to_rfc3339()).with_timeout(self.query_timeout_ms).execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
        let mut rows = graph.ro_query(
            "MATCH (p:CodeMindActiveGeneration {id: $id, workspace_id: $workspace, repository_id: $repository}) RETURN p.generation_id AS generation, p.history_generation_id AS history_generation LIMIT 1",
        ).with_param("id", crate::graph_storage::falkordb::schema::ACTIVE_GENERATION_ID).with_param("workspace", context.workspace_id.clone()).with_param("repository", context.repository_id.clone()).with_timeout(self.query_timeout_ms).execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
        let row = required_count_row(
            rows.data.next().await,
            "active generation publication query",
        )?;
        let published: String = row
            .try_get("generation")
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let published_history_generation: String = row
            .try_get("history_generation")
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let mut summary_rows = graph.ro_query(
            "MATCH (g:HistoryGeneration {repository_generation_id: $generation, workspace_id: $workspace, repository_id: $repository}) RETURN g.id AS storage_generation ORDER BY g.created_at DESC, g.id DESC LIMIT 1",
        ).with_param("generation", generation_id.to_owned()).with_param("workspace", context.workspace_id.clone()).with_param("repository", context.repository_id.clone()).with_timeout(self.query_timeout_ms).execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
        let summary = required_count_row(
            summary_rows.data.next().await,
            "published history generation summary",
        )?;
        let expected_history_generation: String = summary
            .try_get("storage_generation")
            .map_err(|error| StorageError::Query(error.to_string()))?;
        if published != generation_id || published_history_generation != expected_history_generation
        {
            return Err(StorageError::Query("cannot publish a History Graph generation that has not passed read-after-write verification".to_owned()));
        }
        Ok(())
    }

    async fn prune_obsolete_graph_projections(
        &self,
        context: &RepositoryContext,
        generation_id: &str,
        dry_run: bool,
    ) -> Result<GraphRetentionReport, StorageError> {
        Self::validate_context(context)?;
        if !self
            .verify_published_generation(context, generation_id)
            .await?
        {
            return Err(StorageError::Query(
                "graph retention requires the requested generation to be verified and published"
                    .to_owned(),
            ));
        }
        let graph_name = self.graph_name_for_workspace(&context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;
        let retained_labels = vec![
            "ChangeSet",
            "ChangeRecord",
            "AnalysisRun",
            "ImpactAnalysis",
            "RiskAssessment",
            "Evidence",
            "TestEntity",
            "MemoryRecord",
            "HandoffRecord",
            "ReviewContextRecord",
            "ConversationSession",
            "ReviewEpisode",
            "ConversationTurn",
            "ReviewFinding",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();

        // First produce a non-destructive candidate report. The payload scan is
        // intentionally conservative: a possible reference keeps the entity.
        // This avoids deleting historical analysis evidence whose selected or
        // impacted entity ID is serialized in a Change Graph payload.
        let candidate_query = |label: &str| {
            format!(
                "MATCH (n:{label} {{workspace_id: $workspace, repository_id: $repository}}) \
             WHERE n.snapshot_id <> $snapshot OR n.graph_version <> $graph_version \
             OPTIONAL MATCH (retained)-[r]-(n) \
             WHERE retained <> n AND retained.workspace_id = $workspace AND retained.repository_id = $repository \
               AND ANY(label IN labels(retained) WHERE label IN $retained_labels) \
             WITH n, count(DISTINCT retained) AS edge_references \
             OPTIONAL MATCH (payload_owner) \
             WHERE payload_owner <> n AND payload_owner.workspace_id = $workspace AND payload_owner.repository_id = $repository \
               AND ANY(label IN labels(payload_owner) WHERE label IN $retained_labels) \
               AND payload_owner.payload_json IS NOT NULL AND payload_owner.payload_json CONTAINS n.id \
             WITH n, edge_references + count(DISTINCT payload_owner) AS retained_reference_count \
             WHERE retained_reference_count = 0 RETURN count(n) AS count"
            )
        };
        let code_candidate_query = candidate_query("CodeEntity");
        let test_candidate_query = candidate_query("TestEntity");
        let mut code_rows = graph
            .ro_query(&code_candidate_query)
            .with_param("workspace", context.workspace_id.clone())
            .with_param("repository", context.repository_id.clone())
            .with_param("snapshot", context.source_snapshot_id.clone())
            .with_param("graph_version", context.graph_version.clone())
            .with_param("retained_labels", retained_labels.clone())
            .with_timeout(self.query_timeout_ms)
            .execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let code_row =
            required_count_row(code_rows.data.next().await, "Code Graph retention preview")?;
        let eligible_code_entities = nonnegative_count(
            code_row
                .try_get("count")
                .map_err(|error| StorageError::Query(error.to_string()))?,
            "Code Graph retention preview",
        )?;
        let mut test_rows = graph
            .ro_query(&test_candidate_query)
            .with_param("workspace", context.workspace_id.clone())
            .with_param("repository", context.repository_id.clone())
            .with_param("snapshot", context.source_snapshot_id.clone())
            .with_param("graph_version", context.graph_version.clone())
            .with_param("retained_labels", retained_labels.clone())
            .with_timeout(self.query_timeout_ms)
            .execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let test_row =
            required_count_row(test_rows.data.next().await, "Test Graph retention preview")?;
        let eligible_test_entities = nonnegative_count(
            test_row
                .try_get("count")
                .map_err(|error| StorageError::Query(error.to_string()))?,
            "Test Graph retention preview",
        )?;
        let mut report = GraphRetentionReport {
            workspace_id: context.workspace_id.clone(),
            repository_id: context.repository_id.clone(),
            active_snapshot_id: context.source_snapshot_id.clone(),
            active_graph_version: context.graph_version.clone(),
            eligible_code_entities,
            eligible_test_entities,
            deleted_code_entities: 0,
            deleted_test_entities: 0,
            dry_run,
        };
        if dry_run || (eligible_code_entities == 0 && eligible_test_entities == 0) {
            return Ok(report);
        }

        // Revalidate the active pointer immediately before deletion. Each delete
        // query independently rechecks Change Graph references at execution time.
        if !self
            .verify_published_generation(context, generation_id)
            .await?
        {
            return Err(StorageError::Query(
                "active graph generation changed before retention could execute".to_owned(),
            ));
        }
        let delete_query = |label: &str| {
            format!(
                "MATCH (n:{label} {{workspace_id: $workspace, repository_id: $repository}}) \
             WHERE n.snapshot_id <> $snapshot OR n.graph_version <> $graph_version \
             OPTIONAL MATCH (retained)-[r]-(n) \
             WHERE retained <> n AND retained.workspace_id = $workspace AND retained.repository_id = $repository \
               AND ANY(label IN labels(retained) WHERE label IN $retained_labels) \
             WITH n, count(DISTINCT retained) AS edge_references \
             OPTIONAL MATCH (payload_owner) \
             WHERE payload_owner <> n AND payload_owner.workspace_id = $workspace AND payload_owner.repository_id = $repository \
               AND ANY(label IN labels(payload_owner) WHERE label IN $retained_labels) \
               AND payload_owner.payload_json IS NOT NULL AND payload_owner.payload_json CONTAINS n.id \
             WITH n, edge_references + count(DISTINCT payload_owner) AS retained_reference_count \
             WHERE retained_reference_count = 0 DETACH DELETE n RETURN count(n) AS count"
            )
        };
        let delete_test_query = delete_query("TestEntity");
        let mut deleted_test_rows = graph
            .query(&delete_test_query)
            .with_param("workspace", context.workspace_id.clone())
            .with_param("repository", context.repository_id.clone())
            .with_param("snapshot", context.source_snapshot_id.clone())
            .with_param("graph_version", context.graph_version.clone())
            .with_param("retained_labels", retained_labels.clone())
            .with_timeout(self.query_timeout_ms)
            .execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let deleted_test_row = required_count_row(
            deleted_test_rows.data.next().await,
            "Test Graph retention deletion",
        )?;
        report.deleted_test_entities = nonnegative_count(
            deleted_test_row
                .try_get("count")
                .map_err(|error| StorageError::Query(error.to_string()))?,
            "Test Graph retention deletion",
        )?;

        let delete_code_query = delete_query("CodeEntity");
        let mut deleted_code_rows = graph
            .query(&delete_code_query)
            .with_param("workspace", context.workspace_id.clone())
            .with_param("repository", context.repository_id.clone())
            .with_param("snapshot", context.source_snapshot_id.clone())
            .with_param("graph_version", context.graph_version.clone())
            .with_param("retained_labels", retained_labels)
            .with_timeout(self.query_timeout_ms)
            .execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let deleted_code_row = required_count_row(
            deleted_code_rows.data.next().await,
            "Code Graph retention deletion",
        )?;
        report.deleted_code_entities = nonnegative_count(
            deleted_code_row
                .try_get("count")
                .map_err(|error| StorageError::Query(error.to_string()))?,
            "Code Graph retention deletion",
        )?;
        report.dry_run = false;
        Ok(report)
    }

    async fn verify_published_generation(
        &self,
        context: &RepositoryContext,
        generation_id: &str,
    ) -> Result<bool, StorageError> {
        Self::validate_context(context)?;
        let graph_name = self.graph_name_for_workspace(&context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;
        let mut rows = graph.ro_query(
            "MATCH (p:CodeMindActiveGeneration {id: $id, workspace_id: $workspace, repository_id: $repository}), (g:HistoryGeneration {workspace_id: $workspace, repository_id: $repository}) WHERE g.id = p.history_generation_id AND p.generation_id = $generation AND p.source_snapshot_id = $snapshot AND p.graph_version = $graph_version AND g.repository_generation_id = p.generation_id AND g.verified = true AND g.published = true RETURN p.generation_id AS generation LIMIT 1",
        ).with_param("id", crate::graph_storage::falkordb::schema::ACTIVE_GENERATION_ID).with_param("workspace", context.workspace_id.clone()).with_param("repository", context.repository_id.clone()).with_param("generation", generation_id.to_owned()).with_param("snapshot", context.source_snapshot_id.clone()).with_param("graph_version", context.graph_version.clone()).with_timeout(self.query_timeout_ms).execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
        Ok(rows.data.next().await.is_some())
    }

    async fn verify_change_analysis(
        &self,
        expected: &ChangeGraphVerificationExpectation,
    ) -> Result<ChangeGraphVerificationReport, StorageError> {
        Self::validate_context(&expected.context)?;
        if expected.analysis_run_id.trim().is_empty() {
            return Err(StorageError::Query(
                "analysis run ID is required for Change Graph verification".to_owned(),
            ));
        }
        let graph_name = self.graph_name_for_workspace(&expected.context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;
        let expected_count = expected.expected_entity_ids.len();
        let mut rows = graph
            .ro_query(
                "MATCH (n) WHERE n.workspace_id = $workspace AND n.repository_id = $repository AND n.snapshot_id = $snapshot AND n.graph_version = $graph_version AND n.id IN $ids AND ANY(label IN labels(n) WHERE label IN $labels) RETURN n.id AS id ORDER BY n.id ASC LIMIT $limit",
            )
            .with_param("workspace", expected.context.workspace_id.clone())
            .with_param("repository", expected.context.repository_id.clone())
            .with_param("snapshot", expected.context.source_snapshot_id.clone())
            .with_param("graph_version", expected.context.graph_version.clone())
            .with_param("ids", expected.expected_entity_ids.clone())
            .with_param("labels", crate::graph_storage::falkordb::schema::CHANGE_ENTITY_LABELS.iter().map(|v| (*v).to_owned()).collect::<Vec<_>>())
            .with_param("limit", expected_count.clamp(1, 1_000) as i64)
            .with_timeout(self.query_timeout_ms)
            .execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let mut observed_ids: Vec<String> = Vec::new();
        while let Some(row) = rows.data.next().await {
            let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
            let id: String = row
                .try_get("id")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            observed_ids.push(id);
        }
        let mut expected_ids = expected.expected_entity_ids.clone();
        expected_ids.sort_unstable();
        expected_ids.dedup();
        observed_ids.sort_unstable();
        observed_ids.dedup();
        let relationships = self
            .derived_relationships_for_run(
                &expected.context,
                &expected.analysis_run_id,
                expected.expected_relationships.len().clamp(1, 1_000),
            )
            .await?;
        let mut actual_relationships = relationships;
        actual_relationships
            .sort_by(|a, b| (&a.kind, &a.from, &a.to).cmp(&(&b.kind, &b.from, &b.to)));
        let mut expected_relationships = expected.expected_relationships.clone();
        expected_relationships
            .sort_by(|a, b| (&a.kind, &a.from, &a.to).cmp(&(&b.kind, &b.from, &b.to)));
        expected_relationships
            .dedup_by(|a, b| a.kind == b.kind && a.from == b.from && a.to == b.to);
        let semantic_match =
            observed_ids == expected_ids && actual_relationships == expected_relationships;
        Ok(ChangeGraphVerificationReport {
            context: expected.context.clone(),
            analysis_run_id: expected.analysis_run_id.clone(),
            entity_count: observed_ids.len(),
            relationship_count: actual_relationships.len(),
            semantic_match,
        })
    }

    async fn write_test_graph(
        &self,
        context: &RepositoryContext,
        tests: &[TestEntity],
        relationships: &[TestRelationship],
    ) -> Result<(), StorageError> {
        Self::validate_context(context)?;
        if tests.iter().any(|test| &test.context != context)
            || relationships
                .iter()
                .any(|relationship| &relationship.context != context)
        {
            return Err(StorageError::InvalidContext);
        }
        crate::engines::tie::graph_builder::validate(tests, relationships)
            .map_err(StorageError::Query)?;

        let test_ids: HashSet<&str> = tests.iter().map(|test| test.id.as_str()).collect();
        let relationship_ids: HashSet<&str> = relationships
            .iter()
            .map(|relationship| relationship.id.as_str())
            .collect();
        let graph_name = self.graph_name_for_workspace(&context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;

        for test in tests {
            graph
                .query(
                    "MERGE (n:TestEntity {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) SET n.name = $name, n.path = $path, n.line = $line, n.framework = $framework, n.status = $status, n.kind = $kind, n.workspace_id = $workspace, n.repository_id = $repository, n.snapshot_id = $snapshot, n.graph_version = $graph_version",
                )
                .with_param("id", test.id.clone())
                .with_param("name", test.name.clone())
                .with_param("path", test.path.clone())
                .with_param("line", test.line as i64)
                .with_param("framework", test.framework.clone())
                .with_param("status", format!("{:?}", test.status))
                .with_param("kind", format!("{:?}", test.kind))
                .with_param("workspace", test.context.workspace_id.clone())
                .with_param("repository", test.context.repository_id.clone())
                .with_param("snapshot", test.context.source_snapshot_id.clone())
                .with_param("graph_version", test.context.graph_version.clone())
                .with_timeout(self.query_timeout_ms).execute()
                .await
                .map_err(|error| StorageError::Query(error.to_string()))?;
        }

        for relationship in relationships {
            if relationship.resolution != TestResolution::Resolved {
                continue;
            }
            let Some(target_id) = relationship.entity_id.as_deref() else {
                return Err(StorageError::Query(
                    "resolved TIE relationship is missing its target ID".to_owned(),
                ));
            };
            let relationship_type =
                crate::graph_storage::falkordb::mapping::test_relationship_label(
                    &relationship.kind,
                );
            let target_label = match relationship.target_kind {
                crate::engines::tie::model::TestTargetKind::Code => "CodeEntity",
                crate::engines::tie::model::TestTargetKind::Test => "TestEntity",
            };
            let evidence = relationship.evidence.as_ref();
            let query = format!(
                "MATCH (test:TestEntity {{id: $test_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}}) MATCH (target:{target_label} {{id: $target_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}}) MERGE (test)-[r:{relationship_type} {{id: $relationship_id}}]->(target) SET r.path = $path, r.evidence_path = $path, r.start_line = $start_line, r.end_line = $end_line, r.evidence_snapshot_id = $snapshot, r.evidence_provenance = $provenance, r.snapshot_id = $snapshot, r.workspace_id = $workspace, r.repository_id = $repository, r.graph_version = $graph_version, r.entity_id = $target_id, r.test_id = $test_id, r.kind = $kind, r.target_kind = $target_kind, r.resolution = $resolution"
            );
            graph
                .query(query)
                .with_param("test_id", relationship.test_id.clone())
                .with_param("target_id", target_id.to_owned())
                .with_param("relationship_id", relationship.id.clone())
                .with_param("path", relationship.evidence_path.clone())
                .with_param(
                    "start_line",
                    evidence.map(|value| value.start_line as i64).unwrap_or(0),
                )
                .with_param(
                    "end_line",
                    evidence.map(|value| value.end_line as i64).unwrap_or(0),
                )
                .with_param(
                    "provenance",
                    evidence
                        .map(|value| value.provenance.clone())
                        .unwrap_or_default(),
                )
                .with_param("target_kind", format!("{:?}", relationship.target_kind))
                .with_param("kind", format!("{:?}", relationship.kind))
                .with_param("resolution", format!("{:?}", relationship.resolution))
                .with_param("workspace", relationship.context.workspace_id.clone())
                .with_param("repository", relationship.context.repository_id.clone())
                .with_param("snapshot", relationship.context.source_snapshot_id.clone())
                .with_param("graph_version", relationship.context.graph_version.clone())
                .with_timeout(self.query_timeout_ms)
                .execute()
                .await
                .map_err(|error| StorageError::Query(error.to_string()))?;
        }
        // Delay stale-fact deletion until all replacement facts and resolved
        // relationships have been written. Partial writes therefore leave the
        // previously complete same-snapshot projection intact and retryable.
        graph
            .query(
                "MATCH (n:TestEntity {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) WHERE NOT n.id IN $test_ids DETACH DELETE n",
            )
            .with_param("workspace", context.workspace_id.clone())
            .with_param("repository", context.repository_id.clone())
            .with_param("snapshot", context.source_snapshot_id.clone())
            .with_param("graph_version", context.graph_version.clone())
            .with_param(
                "test_ids",
                test_ids
                    .iter()
                    .map(|id| (*id).to_owned())
                    .collect::<Vec<_>>(),
            )
            .with_timeout(self.query_timeout_ms).execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;

        graph
            .query(
                "MATCH (:TestEntity {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})-[r]->() WHERE type(r) IN $relationship_types AND NOT r.id IN $relationship_ids DELETE r",
            )
            .with_param("workspace", context.workspace_id.clone())
            .with_param("repository", context.repository_id.clone())
            .with_param("snapshot", context.source_snapshot_id.clone())
            .with_param("graph_version", context.graph_version.clone())
            .with_param(
                "relationship_types",
                crate::graph_storage::falkordb::schema::TEST_RELATIONSHIP_LABELS
                    .iter()
                    .map(|value| (*value).to_owned())
                    .collect::<Vec<_>>(),
            )
            .with_param(
                "relationship_ids",
                relationship_ids
                    .iter()
                    .map(|id| (*id).to_owned())
                    .collect::<Vec<_>>(),
            )
            .with_timeout(self.query_timeout_ms).execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;

        Ok(())
    }
}

#[async_trait]
impl MemoryRepository for FalkorDbStorage {
    async fn persist_memory(&self, record: &MemoryRecord) -> Result<MemoryRecord, StorageError> {
        validation::validate_memory(record).map_err(StorageError::Query)?;
        let graph_name = self.graph_name_for_workspace(&record.context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;
        let now = if record.updated_at.is_empty() {
            Utc::now().to_rfc3339()
        } else {
            record.updated_at.clone()
        };
        let payload = serde_json::to_string(record)
            .map_err(|error| StorageError::Query(error.to_string()))?;

        let mut existing = graph.ro_query(
            "MATCH (m:MemoryRecord {id: $id, workspace_id: $workspace, repository_id: $repository})              RETURN m.payload_json AS payload, m.idempotency_key AS idempotency_key, m.version AS version LIMIT 1"
        )
        .with_param("id", record.memory_id.clone())
        .with_param("workspace", record.context.workspace_id.clone())
        .with_param("repository", record.context.repository_id.clone())
        .with_timeout(self.query_timeout_ms).execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
        if let Some(row) = existing.data.next().await {
            let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
            let key: String = row
                .try_get("idempotency_key")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let existing_payload: String = row
                .try_get("payload")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            if key == record.idempotency_key {
                if existing_payload != payload {
                    return Err(StorageError::Query(
                        "memory idempotency key was reused for a different payload".to_owned(),
                    ));
                }
                return serde_json::from_str(&existing_payload)
                    .map_err(|error| StorageError::Query(error.to_string()));
            }
            return Err(StorageError::Query(
                "memory identity already exists with another idempotency key".to_owned(),
            ));
        }

        graph.query(
            "MERGE (m:MemoryRecord {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot}) \
             SET m.owner_id = $owner, m.kind = $kind, m.scope = $scope, m.lifecycle = $lifecycle, m.freshness = $freshness, \
             m.payload_json = $payload, m.graph_version = $graph_version, m.idempotency_key = $idempotency, \
             m.version = $version, m.updated_at = $updated_at, m.created_at = coalesce(m.created_at, $created_at)"
        )
        .with_param("id", record.memory_id.clone())
        .with_param("owner", record.owner_id.clone())
        .with_param("kind", record.kind.clone())
        .with_param("scope", record.scope.clone())
        .with_param("lifecycle", record.lifecycle.clone())
        .with_param("freshness", record.freshness.clone())
        .with_param("payload", payload)
        .with_param("workspace", record.context.workspace_id.clone())
        .with_param("repository", record.context.repository_id.clone())
        .with_param("snapshot", record.context.source_snapshot_id.clone())
        .with_param("graph_version", record.context.graph_version.clone())
        .with_param("idempotency", record.idempotency_key.clone())
        .with_param("version", record.version as i64)
        .with_param("updated_at", now.clone())
        .with_param("created_at", record.created_at.clone())
        .with_timeout(self.query_timeout_ms).execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
        let mut saved = record.clone();
        saved.updated_at = now;
        Ok(saved)
    }

    async fn retrieve_memory(
        &self,
        context: &RepositoryContext,
        scope: &str,
        kind: &str,
        owner_id: &str,
        limit: usize,
    ) -> Result<Vec<MemoryRecord>, StorageError> {
        Self::validate_context(context)?;
        let graph_name = self.graph_name_for_workspace(&context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;
        let mut rows = graph
            .ro_query(
                "MATCH (m:MemoryRecord {workspace_id: $workspace, repository_id: $repository}) \
             WHERE m.scope = $scope AND m.kind = $kind \
               AND (m.snapshot_id = $snapshot OR m.freshness <> 'fresh') \
               AND ((m.scope = 'repository' AND (m.owner_id = '' OR m.owner_id = $owner)) OR (m.scope IN ['user', 'team'] AND m.owner_id = $owner)) \
             RETURN m.payload_json AS payload LIMIT $limit",
            )
            .with_param("workspace", context.workspace_id.clone())
            .with_param("repository", context.repository_id.clone())
            .with_param("scope", scope.to_owned())
            .with_param("kind", kind.to_owned())
            .with_param("snapshot", context.source_snapshot_id.clone())
            .with_param("owner", owner_id.to_owned())
            .with_param("limit", limit.clamp(1, 100) as i64)
            .with_timeout(self.query_timeout_ms).execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let mut records = Vec::new();
        while let Some(row) = rows.data.next().await {
            let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
            let payload: String = row
                .try_get("payload")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let record = serde_json::from_str::<MemoryRecord>(&payload)
                .map_err(|error| StorageError::Query(error.to_string()))?;
            if record.context.source_snapshot_id == context.source_snapshot_id
                || record.freshness != "fresh"
            {
                records.push(record);
            }
        }
        Ok(records)
    }

    async fn persist_handoff(&self, record: &HandoffRecord) -> Result<HandoffRecord, StorageError> {
        validation::validate_handoff(record).map_err(StorageError::Query)?;
        let graph_name = self.graph_name_for_workspace(&record.context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;
        let now = if record.updated_at.is_empty() {
            Utc::now().to_rfc3339()
        } else {
            record.updated_at.clone()
        };
        let payload = serde_json::to_string(record)
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let mut existing = graph
            .ro_query(
                "MATCH (h:HandoffRecord {id: $id, workspace_id: $workspace, repository_id: $repository}) \
                 RETURN h.payload_json AS payload, h.idempotency_key AS idempotency_key, \
                 h.status AS status, h.version AS version LIMIT 1",
            )
            .with_param("id", record.handoff_id.clone())
            .with_param("workspace", record.context.workspace_id.clone())
            .with_param("repository", record.context.repository_id.clone())
            .with_timeout(self.query_timeout_ms).execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;
        if let Some(row) = existing.data.next().await {
            let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
            let existing_key: String = row
                .try_get("idempotency_key")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let existing_payload: String = row
                .try_get("payload")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let mut current = serde_json::from_str::<HandoffRecord>(&existing_payload)
                .map_err(|error| StorageError::Query(error.to_string()))?;
            current.status = row
                .try_get("status")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let version: i64 = row
                .try_get("version")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            current.version = version.max(0) as u64;
            if existing_key == record.idempotency_key {
                if existing_payload != payload {
                    return Err(StorageError::Query(
                        "same idempotency key was reused for a different handoff payload"
                            .to_owned(),
                    ));
                }
                return Ok(current);
            }
            return Err(StorageError::Query(
                "conflicting handoff delivery".to_owned(),
            ));
        }
        graph.query(
            "MERGE (h:HandoffRecord {id: $id, workspace_id: $workspace, repository_id: $repository}) \
             SET h.payload_json = $payload, h.status = $status, h.version = $version, \
             h.snapshot_id = $snapshot, h.graph_version = $graph_version, \
             h.idempotency_key = $idempotency, h.updated_at = $updated_at, h.created_at = coalesce(h.created_at, $created_at)"
        )
        .with_param("id", record.handoff_id.clone())
        .with_param("workspace", record.context.workspace_id.clone())
        .with_param("repository", record.context.repository_id.clone())
        .with_param("snapshot", record.context.source_snapshot_id.clone())
        .with_param("graph_version", record.context.graph_version.clone())
        .with_param("payload", payload)
        .with_param("status", record.status.clone())
        .with_param("version", record.version as i64)
        .with_param("idempotency", record.idempotency_key.clone())
        .with_param("updated_at", now.clone())
        .with_param("created_at", record.created_at.clone())
        .with_timeout(self.query_timeout_ms).execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
        let mut saved = record.clone();
        saved.updated_at = now;
        Ok(saved)
    }

    async fn retrieve_handoff(
        &self,
        context: &RepositoryContext,
        handoff_id: &str,
    ) -> Result<HandoffRecord, StorageError> {
        Self::validate_context(context)?;
        let graph_name = self.graph_name_for_workspace(&context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;
        let mut rows = graph.ro_query(
            "MATCH (h:HandoffRecord {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) \
             RETURN h.payload_json AS payload, h.status AS status, h.version AS version LIMIT 1"
        )
        .with_param("id", handoff_id.to_owned())
        .with_param("workspace", context.workspace_id.clone())
        .with_param("repository", context.repository_id.clone())
        .with_param("snapshot", context.source_snapshot_id.clone())
        .with_param("graph_version", context.graph_version.clone())
        .with_timeout(self.query_timeout_ms).execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
        let Some(row) = rows.data.next().await else {
            return Err(StorageError::Query("handoff not found".to_owned()));
        };
        let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
        let payload: String = row
            .try_get("payload")
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let mut record: HandoffRecord = serde_json::from_str(&payload)
            .map_err(|error| StorageError::Query(error.to_string()))?;
        record.status = row
            .try_get("status")
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let version: i64 = row
            .try_get("version")
            .map_err(|error| StorageError::Query(error.to_string()))?;
        record.version = version.max(0) as u64;
        if record.context.source_snapshot_id != context.source_snapshot_id
            || record.context.graph_version != context.graph_version
        {
            return Err(StorageError::Query(
                "handoff snapshot or graph version is incompatible".to_owned(),
            ));
        }
        Ok(record)
    }

    async fn transition_handoff(
        &self,
        context: &RepositoryContext,
        handoff_id: &str,
        status: &str,
    ) -> Result<HandoffRecord, StorageError> {
        Self::validate_context(context)?;
        if !matches!(status, "claimed" | "consumed" | "expired") {
            return Err(StorageError::Query("invalid handoff transition".to_owned()));
        }
        let now = Utc::now().to_rfc3339();
        let graph_name = self.graph_name_for_workspace(&context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;
        let mut rows = graph
            .query(
                "MATCH (h:HandoffRecord {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) \
                 WHERE (h.status = 'pending' AND ($status = 'claimed' OR $status = 'expired')) \
                    OR (h.status = 'claimed' AND $status = 'consumed') \
                 SET h.status = $status, h.version = h.version + 1, h.updated_at = $updated_at \
                 RETURN h.payload_json AS payload, h.status AS status, h.version AS version",
            )
            .with_param("id", handoff_id.to_owned())
            .with_param("workspace", context.workspace_id.clone())
            .with_param("repository", context.repository_id.clone())
            .with_param("snapshot", context.source_snapshot_id.clone())
            .with_param("graph_version", context.graph_version.clone())
            .with_param("status", status.to_owned())
            .with_param("updated_at", now)
            .with_timeout(self.query_timeout_ms).execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;
        if let Some(row) = rows.data.next().await {
            let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
            let payload: String = row
                .try_get("payload")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let mut record = serde_json::from_str::<HandoffRecord>(&payload)
                .map_err(|error| StorageError::Query(error.to_string()))?;
            record.status = row
                .try_get("status")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            let version: i64 = row
                .try_get("version")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            record.version = version.max(0) as u64;
            return Ok(record);
        }

        let current = self.retrieve_handoff(context, handoff_id).await?;
        if (current.status == "consumed" && status == "consumed")
            || (current.status == "expired" && status == "expired")
        {
            return Ok(current);
        }
        Err(StorageError::Query(format!(
            "handoff transition conflict: {} -> {status}",
            current.status
        )))
    }

    async fn persist_review_context(
        &self,
        record: &ReviewContextRecord,
    ) -> Result<ReviewContextRecord, StorageError> {
        Self::validate_context(&record.context)?;
        let graph_name = self.graph_name_for_workspace(&record.context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;

        graph.query(
            "MERGE (r:ReviewContextRecord {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})              SET r.scope = $scope, r.owner_id = $owner_id, r.session_id = $session, r.episode_id = $episode, r.updated_at = $updated_at"
        )
        .with_param("id", record.context_id.clone())
        .with_param("scope", record.scope.clone())
        .with_param("owner_id", record.owner_id.clone())
        .with_param("session", record.session.session_id.clone())
        .with_param("episode", record.episode.episode_id.clone())
        .with_param("workspace", record.context.workspace_id.clone())
        .with_param("repository", record.context.repository_id.clone())
        .with_param("snapshot", record.context.source_snapshot_id.clone())
        .with_param("graph_version", record.context.graph_version.clone())
        .with_param("updated_at", record.updated_at.clone())
        .with_timeout(self.query_timeout_ms).execute().await.map_err(|error| StorageError::Query(error.to_string()))?;

        let session_payload = serde_json::to_string(&record.session)
            .map_err(|error| StorageError::Query(error.to_string()))?;
        graph.query(
            "MERGE (s:ConversationSession {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) \
             SET s.payload_json = $payload, s.scope = $scope"
        )
        .with_param("id", record.session.session_id.clone())
        .with_param("workspace", record.context.workspace_id.clone())
        .with_param("repository", record.context.repository_id.clone())
        .with_param("snapshot", record.context.source_snapshot_id.clone())
        .with_param("graph_version", record.context.graph_version.clone())
        .with_param("scope", record.scope.clone())
        .with_param("payload", session_payload)
        .with_timeout(self.query_timeout_ms).execute().await.map_err(|error| StorageError::Query(error.to_string()))?;

        let episode_payload = serde_json::to_string(&record.episode)
            .map_err(|error| StorageError::Query(error.to_string()))?;
        graph.query(
            "MERGE (e:ReviewEpisode {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) \
             SET e.payload_json = $payload"
        )
        .with_param("id", record.episode.episode_id.clone())
        .with_param("workspace", record.context.workspace_id.clone())
        .with_param("repository", record.context.repository_id.clone())
        .with_param("snapshot", record.context.source_snapshot_id.clone())
        .with_param("graph_version", record.context.graph_version.clone())
        .with_param("payload", episode_payload)
        .with_timeout(self.query_timeout_ms).execute().await.map_err(|error| StorageError::Query(error.to_string()))?;

        graph.query(
            "MATCH (s:ConversationSession {id: $session, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}), \
             (e:ReviewEpisode {id: $episode, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) \
             MERGE (s)-[:HAS_EPISODE]->(e)"
        )
        .with_param("session", record.session.session_id.clone())
        .with_param("episode", record.episode.episode_id.clone())
        .with_param("workspace", record.context.workspace_id.clone())
        .with_param("repository", record.context.repository_id.clone())
        .with_param("snapshot", record.context.source_snapshot_id.clone())
        .with_param("graph_version", record.context.graph_version.clone())
        .with_timeout(self.query_timeout_ms).execute().await.map_err(|error| StorageError::Query(error.to_string()))?;

        for turn in &record.turns {
            let payload = serde_json::to_string(turn)
                .map_err(|error| StorageError::Query(error.to_string()))?;
            graph.query(
                "MERGE (t:ConversationTurn {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) \
                 SET t.payload_json = $payload \
                 WITH t MATCH (s:ConversationSession {id: $session, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) \
                 MERGE (s)-[:HAS_TURN]->(t)"
            )
            .with_param("id", turn.turn_id.clone())
            .with_param("session", turn.session_id.clone())
            .with_param("workspace", record.context.workspace_id.clone())
            .with_param("repository", record.context.repository_id.clone())
            .with_param("snapshot", record.context.source_snapshot_id.clone())
            .with_param("graph_version", record.context.graph_version.clone())
            .with_param("payload", payload)
            .with_timeout(self.query_timeout_ms).execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
        }

        for finding in &record.findings {
            let payload = serde_json::to_string(finding)
                .map_err(|error| StorageError::Query(error.to_string()))?;
            graph.query(
                "MERGE (f:ReviewFinding {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) \
                 SET f.payload_json = $payload \
                 WITH f MATCH (e:ReviewEpisode {id: $episode, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) \
                 MERGE (e)-[:HAS_FINDING]->(f)"
            )
            .with_param("id", finding.finding_id.clone())
            .with_param("episode", finding.episode_id.clone())
            .with_param("workspace", record.context.workspace_id.clone())
            .with_param("repository", record.context.repository_id.clone())
            .with_param("snapshot", record.context.source_snapshot_id.clone())
            .with_param("graph_version", record.context.graph_version.clone())
            .with_param("payload", payload)
            .with_timeout(self.query_timeout_ms).execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
        }

        Ok(record.clone())
    }

    async fn retrieve_review_context(
        &self,
        context: &RepositoryContext,
        context_id: &str,
    ) -> Result<ReviewContextRecord, StorageError> {
        Self::validate_context(context)?;
        let graph_name = self.graph_name_for_workspace(&context.workspace_id)?;
        let mut graph = self.ready_graph(&graph_name).await?;
        let mut rows = graph
            .ro_query(
                "MATCH (r:ReviewContextRecord {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) \
                 MATCH (s:ConversationSession {id: r.session_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) \
                 RETURN s.payload_json AS payload, r.owner_id AS owner_id, r.episode_id AS episode_id LIMIT 1"
            )
            .with_param("id", context_id.to_owned())
            .with_param("workspace", context.workspace_id.clone())
            .with_param("repository", context.repository_id.clone())
            .with_param("snapshot", context.source_snapshot_id.clone())
            .with_param("graph_version", context.graph_version.clone())
            .with_timeout(self.query_timeout_ms).execute().await.map_err(|error| StorageError::Query(error.to_string()))?;

        let Some(row) = rows.data.next().await else {
            return Err(StorageError::Query("review session not found".to_owned()));
        };
        let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
        let session_payload: String = row
            .try_get("payload")
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let owner_id: String = row
            .try_get::<Option<String>>("owner_id")
            .map_err(|error| StorageError::Query(error.to_string()))?
            .unwrap_or_default();
        let episode_id: String = row
            .try_get("episode_id")
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let session: crate::memory::model::ConversationSession =
            serde_json::from_str(&session_payload)
                .map_err(|error| StorageError::Query(error.to_string()))?;

        let mut turn_rows = graph.ro_query(
            "MATCH (s:ConversationSession {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})-[:HAS_TURN]->(t:ConversationTurn) \
             RETURN t.payload_json AS payload LIMIT 100"
        )
        .with_param("id", session.session_id.clone())
        .with_param("workspace", context.workspace_id.clone())
        .with_param("repository", context.repository_id.clone())
        .with_param("snapshot", context.source_snapshot_id.clone())
        .with_param("graph_version", context.graph_version.clone())
        .with_timeout(self.query_timeout_ms).execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
        let mut turns: Vec<crate::memory::model::ConversationTurn> = Vec::new();
        while let Some(row) = turn_rows.data.next().await {
            let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
            let payload: String = row
                .try_get("payload")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            turns.push(
                serde_json::from_str(&payload)
                    .map_err(|error| StorageError::Query(error.to_string()))?,
            );
        }

        let mut episode_rows = graph.ro_query(
            "MATCH (e:ReviewEpisode {id: $episode, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) \
             RETURN e.payload_json AS payload LIMIT 1"
        )
        .with_param("episode", episode_id.clone())
        .with_param("workspace", context.workspace_id.clone())
        .with_param("repository", context.repository_id.clone())
        .with_param("snapshot", context.source_snapshot_id.clone())
        .with_param("graph_version", context.graph_version.clone())
        .with_timeout(self.query_timeout_ms).execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
        let Some(row) = episode_rows.data.next().await else {
            return Err(StorageError::Query("review episode not found".to_owned()));
        };
        let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
        let episode_payload: String = row
            .try_get("payload")
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let episode: crate::memory::model::ReviewEpisode =
            serde_json::from_str(&episode_payload)
                .map_err(|error| StorageError::Query(error.to_string()))?;

        let mut finding_rows = graph.ro_query(
            "MATCH (e:ReviewEpisode {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})-[:HAS_FINDING]->(f:ReviewFinding) \
             WHERE e.id = $episode RETURN f.payload_json AS payload LIMIT 64"
        )
        .with_param("episode", episode.episode_id.clone())
        .with_param("workspace", context.workspace_id.clone())
        .with_param("repository", context.repository_id.clone())
        .with_param("snapshot", context.source_snapshot_id.clone())
        .with_param("graph_version", context.graph_version.clone())
        .with_timeout(self.query_timeout_ms).execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
        let mut findings = Vec::new();
        while let Some(row) = finding_rows.data.next().await {
            let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
            let payload: String = row
                .try_get("payload")
                .map_err(|error| StorageError::Query(error.to_string()))?;
            findings.push(
                serde_json::from_str(&payload)
                    .map_err(|error| StorageError::Query(error.to_string()))?,
            );
        }

        Ok(ReviewContextRecord {
            context_id: context_id.to_owned(),
            scope: session.scope.clone(),
            owner_id,
            session,
            turns,
            episode,
            findings,
            context: context.clone(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        })
    }
}

pub(super) fn parse_test_relationship(value: String) -> Result<TestRelationshipKind, StorageError> {
    match value.as_str() {
        "TARGETS" => Ok(TestRelationshipKind::Targets),
        "DECLARED_BY" => Ok(TestRelationshipKind::DeclaredBy),
        _ => Err(StorageError::Mapping(format!(
            "unsupported test relationship label: {value}"
        ))),
    }
}

fn parse_test_node_kind(
    value: String,
) -> Result<crate::engines::tie::model::TestNodeKind, StorageError> {
    match value.as_str() {
        "Test" => Ok(crate::engines::tie::model::TestNodeKind::Test),
        "TestSuite" => Ok(crate::engines::tie::model::TestNodeKind::TestSuite),
        "Fixture" => Ok(crate::engines::tie::model::TestNodeKind::Fixture),
        "TestCase" => Ok(crate::engines::tie::model::TestNodeKind::TestCase),
        _ => Err(StorageError::Mapping(format!(
            "unsupported TIE node kind: {value}"
        ))),
    }
}

fn parse_test_status(
    value: String,
) -> Result<crate::engines::tie::model::TestStatus, StorageError> {
    match value.as_str() {
        "Supported" => Ok(crate::engines::tie::model::TestStatus::Supported),
        "Partial" => Ok(crate::engines::tie::model::TestStatus::Partial),
        "Unsupported" => Ok(crate::engines::tie::model::TestStatus::Unsupported),
        _ => Err(StorageError::Mapping(format!(
            "unsupported test status: {value}"
        ))),
    }
}

fn parse_test_resolution(
    value: String,
) -> Result<crate::engines::tie::model::TestResolution, StorageError> {
    match value.as_str() {
        "Resolved" => Ok(crate::engines::tie::model::TestResolution::Resolved),
        "Unresolved" => Ok(crate::engines::tie::model::TestResolution::Unresolved),
        "Ambiguous" => Ok(crate::engines::tie::model::TestResolution::Ambiguous),
        "Unsupported" => Ok(crate::engines::tie::model::TestResolution::Unsupported),
        _ => Err(StorageError::Mapping(format!(
            "unsupported test relationship resolution: {value}"
        ))),
    }
}

fn parse_resolution(
    value: String,
) -> Result<crate::engines::coie::model::ResolutionStatus, StorageError> {
    match value.as_str() {
        "Resolved" => Ok(crate::engines::coie::model::ResolutionStatus::Resolved),
        "Unresolved" => Ok(crate::engines::coie::model::ResolutionStatus::Unresolved),
        "Ambiguous" => Ok(crate::engines::coie::model::ResolutionStatus::Ambiguous),
        "Unsupported" => Ok(crate::engines::coie::model::ResolutionStatus::Unsupported),
        _ => Err(StorageError::Mapping(format!(
            "unsupported graph resolution: {value}"
        ))),
    }
}

fn relationship_type(kind: &RelationshipKind) -> &'static str {
    crate::graph_storage::falkordb::mapping::code_relationship_label(kind)
}

pub(super) fn parse_relationship(value: String) -> Result<RelationshipKind, StorageError> {
    match value.as_str() {
        "HAS_SNAPSHOT" => Ok(RelationshipKind::HasSnapshot),
        "CONTAINS" => Ok(RelationshipKind::Contains),
        "DEFINES" => Ok(RelationshipKind::Defines),
        "IMPORTS" => Ok(RelationshipKind::Imports),
        "CALLS" => Ok(RelationshipKind::Calls),
        _ => Err(StorageError::Mapping(format!(
            "unsupported graph relationship label: {value}"
        ))),
    }
}

fn parse_entity_kind(value: String) -> Result<EntityKind, StorageError> {
    match value.as_str() {
        "Repository" => Ok(EntityKind::Repository),
        "RepositorySnapshot" => Ok(EntityKind::RepositorySnapshot),
        "Directory" => Ok(EntityKind::Directory),
        "File" => Ok(EntityKind::File),
        "Module" => Ok(EntityKind::Module),
        "Package" => Ok(EntityKind::Package),
        "Symbol" => Ok(EntityKind::Symbol),
        _ => Err(StorageError::Mapping(format!(
            "unsupported graph entity kind: {value}"
        ))),
    }
}

#[cfg(test)]
mod phase27_routing_tests {
    use super::resolve_workspace_graph_name;

    #[test]
    fn one_workspace_maps_to_one_stable_physical_graph() {
        let first = resolve_workspace_graph_name("codemind", "workspace-a").unwrap();
        let second = resolve_workspace_graph_name("codemind", "workspace-a").unwrap();
        assert_eq!(first, second);
        assert!(first.starts_with("codemind_"));
    }

    #[test]
    fn different_workspaces_are_physically_isolated() {
        let first = resolve_workspace_graph_name("codemind", "workspace-a").unwrap();
        let second = resolve_workspace_graph_name("codemind", "workspace-b").unwrap();
        assert_ne!(first, second);
    }

    #[test]
    fn untrusted_namespace_and_workspace_values_are_rejected() {
        assert!(resolve_workspace_graph_name("codemind;drop", "workspace-a").is_err());
        assert!(resolve_workspace_graph_name("codemind", "workspace\n-a").is_err());
    }
}

#[cfg(test)]
mod index_initialization_tests {
    use super::is_existing_index_error;

    #[test]
    fn existing_index_responses_are_idempotent_across_falkordb_error_wordings() {
        assert!(is_existing_index_error(
            &"An error occurred while sending the request to Redis: 'id' is already indexed"
        ));
        assert!(is_existing_index_error(
            &"An index on :CodeEntity(id) already exists"
        ));
        assert!(!is_existing_index_error(
            &"permission denied while creating index"
        ));
    }
}
