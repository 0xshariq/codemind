use crate::engines::chie::engine::ChieEngine;
use crate::engines::chie::model::ChangeAnalysis;
use crate::engines::coie::model::{
    CodeGraph, EntityKind, GraphEntity, GraphRelationship, RelationshipKind,
};
use crate::engines::tie::model::{TestEntity, TestRelationship, TestRelationshipKind};
use crate::graph_storage::port::{GraphStorage, StorageError};
use crate::memory::model::{HandoffRecord, MemoryRecord, ReviewContextRecord};
use crate::memory::repository::MemoryRepository;
use crate::memory::validation;
use crate::source::RepositoryContext;
use async_trait::async_trait;
use chrono::Utc;
use falkordb::{FalkorAsyncClient, FalkorClientBuilder, FalkorConnectionInfo};
use futures::StreamExt;

pub struct FalkorDbStorage {
    client: FalkorAsyncClient,
    graph_name: String,
}

impl FalkorDbStorage {
    pub async fn connect(url: &str, graph_name: impl Into<String>) -> Result<Self, StorageError> {
        let connection_info = FalkorConnectionInfo::try_from(url)
            .map_err(|error| StorageError::Unavailable(error.to_string()))?;
        let client = FalkorClientBuilder::new_async()
            .with_connection_info(connection_info)
            .build()
            .await
            .map_err(|error| StorageError::Unavailable(error.to_string()))?;
        Ok(Self {
            client,
            graph_name: graph_name.into(),
        })
    }

    fn validate_context(context: &RepositoryContext) -> Result<(), StorageError> {
        if context.workspace_id.trim().is_empty()
            || context.repository_id.trim().is_empty()
            || context.source_snapshot_id.trim().is_empty()
            || context.graph_version.trim().is_empty()
        {
            return Err(StorageError::InvalidContext);
        }
        Ok(())
    }
}

#[async_trait]
impl GraphStorage for FalkorDbStorage {
    async fn write_code_graph(&self, graph_data: &CodeGraph) -> Result<(), StorageError> {
        Self::validate_context(&graph_data.context)?;
        let mut graph = self.client.select_graph(&self.graph_name);
        for entity in &graph_data.entities {
            graph
                .query(
                    "MERGE (n:CodeEntity {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) SET n.kind = $kind, n.name = $name, \
                 n.path = $path, n.line = $line, n.workspace_id = $workspace, \
                 n.repository_id = $repository, n.snapshot_id = $snapshot",
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
                .execute()
                .await
                .map_err(|error| StorageError::Query(error.to_string()))?;
        }
        for relationship in &graph_data.relationships {
            let relationship_type = relationship_type(&relationship.kind);
            let query = format!(
                "MATCH (a:CodeEntity {{id: $from, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}}), (b:CodeEntity {{id: $to, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}}) \
                 MERGE (a)-[r:{relationship_type} {{id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}}]->(b)"
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
                .execute()
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
        Self::validate_context(context)?;
        let mut graph = self.client.select_graph(&self.graph_name);
        let mut result = graph.ro_query(
            "MATCH (a:CodeEntity {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})-[r]->(b:CodeEntity {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) \
             WHERE r.workspace_id = $workspace AND r.repository_id = $repository AND r.snapshot_id = $snapshot AND r.graph_version = $graph_version \
             RETURN r.id AS relationship_id, type(r) AS relationship_type, a.id AS from_id, b.id AS to_id \
             LIMIT $limit",
        )
        .with_param("id", entity_id.to_owned())
        .with_param("workspace", context.workspace_id.clone())
        .with_param("repository", context.repository_id.clone())
        .with_param("snapshot", context.source_snapshot_id.clone())
        .with_param("graph_version", context.graph_version.clone())
        .with_param("limit", limit.min(1000) as i64)
        .execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
        let mut relationships = Vec::new();
        while let Some(row) = result.data.next().await {
            let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
            relationships.push(GraphRelationship {
                id: row
                    .try_get("relationship_id")
                    .map_err(|error| StorageError::Query(error.to_string()))?,
                kind: parse_relationship(
                    row.try_get("relationship_type")
                        .map_err(|error| StorageError::Query(error.to_string()))?,
                ),
                from: row
                    .try_get("from_id")
                    .map_err(|error| StorageError::Query(error.to_string()))?,
                to: row
                    .try_get("to_id")
                    .map_err(|error| StorageError::Query(error.to_string()))?,
                context: context.clone(),
            });
        }
        Ok(relationships)
    }

    async fn analyze_change(
        &self,
        context: &RepositoryContext,
        entity_id: &str,
        max_depth: usize,
    ) -> Result<ChangeAnalysis, StorageError> {
        Self::validate_context(context)?;
        let mut graph = self.client.select_graph(&self.graph_name);
        let mut rows = graph.ro_query(
            "MATCH (n:CodeEntity) WHERE n.workspace_id = $workspace AND n.repository_id = $repository AND n.snapshot_id = $snapshot AND n.graph_version = $graph_version RETURN n.id AS id, n.kind AS kind, n.name AS name, n.path AS path, n.line AS line",
        )
        .with_param("workspace", context.workspace_id.clone())
        .with_param("repository", context.repository_id.clone())
        .with_param("snapshot", context.source_snapshot_id.clone())
        .with_param("graph_version", context.graph_version.clone())
        .execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
        let mut entities = Vec::new();
        while let Some(row) = rows.data.next().await {
            let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
            entities.push(GraphEntity {
                id: row
                    .try_get("id")
                    .map_err(|error| StorageError::Query(error.to_string()))?,
                kind: match row
                    .try_get::<String>("kind")
                    .map_err(|error| StorageError::Query(error.to_string()))?
                    .as_str()
                {
                    "File" => EntityKind::File,
                    _ => EntityKind::Symbol,
                },
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
            });
        }
        let mut relationship_rows = graph.ro_query(
            "MATCH (a:CodeEntity)-[r]->(b:CodeEntity) WHERE r.workspace_id = $workspace AND r.repository_id = $repository AND r.snapshot_id = $snapshot AND r.graph_version = $graph_version RETURN r.id AS relationship_id, type(r) AS relationship_type, a.id AS from_id, b.id AS to_id",
        )
        .with_param("workspace", context.workspace_id.clone())
        .with_param("repository", context.repository_id.clone())
        .with_param("snapshot", context.source_snapshot_id.clone())
        .with_param("graph_version", context.graph_version.clone())
        .execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
        let mut relationships = Vec::new();
        while let Some(row) = relationship_rows.data.next().await {
            let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
            relationships.push(GraphRelationship {
                id: row
                    .try_get("relationship_id")
                    .map_err(|error| StorageError::Query(error.to_string()))?,
                kind: parse_relationship(
                    row.try_get("relationship_type")
                        .map_err(|error| StorageError::Query(error.to_string()))?,
                ),
                from: row
                    .try_get("from_id")
                    .map_err(|error| StorageError::Query(error.to_string()))?,
                to: row
                    .try_get("to_id")
                    .map_err(|error| StorageError::Query(error.to_string()))?,
                context: context.clone(),
            });
        }
        let graph_data = CodeGraph {
            context: context.clone(),
            entities,
            relationships,
            diagnostics: Vec::new(),
        };
        ChieEngine {
            max_depth: max_depth.clamp(1, 8),
        }
        .analyze(&graph_data, &[], &[], entity_id)
        .map_err(StorageError::Query)
    }

    async fn write_test_graph(
        &self,
        tests: &[TestEntity],
        relationships: &[TestRelationship],
    ) -> Result<(), StorageError> {
        for test in tests {
            Self::validate_context(&test.context)?;
            let mut graph = self.client.select_graph(&self.graph_name);
            graph
                .query(
                    "MERGE (n:TestEntity {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) SET n.name = $name, n.path = $path, \
                 n.line = $line, n.framework = $framework, n.workspace_id = $workspace, \
                 n.repository_id = $repository, n.snapshot_id = $snapshot",
                )
                .with_param("id", test.id.clone())
                .with_param("name", test.name.clone())
                .with_param("path", test.path.clone())
                .with_param("line", test.line as i64)
                .with_param("framework", test.framework.clone())
                .with_param("workspace", test.context.workspace_id.clone())
                .with_param("repository", test.context.repository_id.clone())
                .with_param("snapshot", test.context.source_snapshot_id.clone())
                .with_param("graph_version", test.context.graph_version.clone())
                .execute()
                .await
                .map_err(|error| StorageError::Query(error.to_string()))?;
        }
        for relationship in relationships {
            Self::validate_context(&relationship.context)?;
            let mut graph = self.client.select_graph(&self.graph_name);
            if relationship.kind != TestRelationshipKind::Targets {
                continue;
            }
            let target = relationship.target_symbol_name.clone().unwrap_or_default();
            graph.query(
                "MATCH (test:TestEntity {id: $test_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) \
                 MATCH (target:CodeEntity {name: $target, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) \
                 MERGE (test)-[r:TARGETS {path: $path, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}]->(target)"
            )
            .with_param("test_id", relationship.test_id.clone())
            .with_param("path", relationship.evidence_path.clone())
            .with_param("workspace", relationship.context.workspace_id.clone())
            .with_param("repository", relationship.context.repository_id.clone())
            .with_param("snapshot", relationship.context.source_snapshot_id.clone())
            .with_param("graph_version", relationship.context.graph_version.clone())
            .with_param("target", target)
            .execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;
        }
        Ok(())
    }
}

#[async_trait]
impl MemoryRepository for FalkorDbStorage {
    async fn persist_memory(&self, record: &MemoryRecord) -> Result<MemoryRecord, StorageError> {
        validation::validate_memory(record).map_err(StorageError::Query)?;
        let mut graph = self.client.select_graph(&self.graph_name);
        let now = if record.updated_at.is_empty() {
            Utc::now().to_rfc3339()
        } else {
            record.updated_at.clone()
        };
        let payload = serde_json::to_string(record)
            .map_err(|error| StorageError::Query(error.to_string()))?;
        graph.query(
            "MERGE (m:MemoryRecord {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot}) \
             SET m.kind = $kind, m.scope = $scope, m.lifecycle = $lifecycle, m.freshness = $freshness, \
             m.payload_json = $payload, m.graph_version = $graph_version, m.idempotency_key = $idempotency, \
             m.version = $version, m.updated_at = $updated_at, m.created_at = coalesce(m.created_at, $created_at)"
        )
        .with_param("id", record.memory_id.clone())
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
        .execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
        let mut saved = record.clone();
        saved.updated_at = now;
        Ok(saved)
    }

    async fn retrieve_memory(
        &self,
        context: &RepositoryContext,
        scope: &str,
        kind: &str,
        limit: usize,
    ) -> Result<Vec<MemoryRecord>, StorageError> {
        Self::validate_context(context)?;
        let mut graph = self.client.select_graph(&self.graph_name);
        let mut rows = graph
            .ro_query(
                "MATCH (m:MemoryRecord {workspace_id: $workspace, repository_id: $repository}) \
             WHERE m.scope = $scope AND m.kind = $kind \
             RETURN m.payload_json AS payload LIMIT $limit",
            )
            .with_param("workspace", context.workspace_id.clone())
            .with_param("repository", context.repository_id.clone())
            .with_param("scope", scope.to_owned())
            .with_param("kind", kind.to_owned())
            .with_param("limit", limit.clamp(1, 100) as i64)
            .execute()
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
        let mut graph = self.client.select_graph(&self.graph_name);
        let now = if record.updated_at.is_empty() {
            Utc::now().to_rfc3339()
        } else {
            record.updated_at.clone()
        };
        let payload = serde_json::to_string(record)
            .map_err(|error| StorageError::Query(error.to_string()))?;
        graph.query(
            "MERGE (h:HandoffRecord {id: $id, workspace_id: $workspace, repository_id: $repository}) \
             SET h.payload_json = $payload, h.status = $status, h.version = $version, \
             h.idempotency_key = $idempotency, h.updated_at = $updated_at, h.created_at = coalesce(h.created_at, $created_at)"
        )
        .with_param("id", record.handoff_id.clone())
        .with_param("workspace", record.context.workspace_id.clone())
        .with_param("repository", record.context.repository_id.clone())
        .with_param("payload", payload)
        .with_param("status", record.status.clone())
        .with_param("version", record.version as i64)
        .with_param("idempotency", record.idempotency_key.clone())
        .with_param("updated_at", now.clone())
        .with_param("created_at", record.created_at.clone())
        .execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
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
        let mut graph = self.client.select_graph(&self.graph_name);
        let mut rows = graph.ro_query(
            "MATCH (h:HandoffRecord {id: $id, workspace_id: $workspace, repository_id: $repository}) RETURN h.payload_json AS payload LIMIT 1"
        )
        .with_param("id", handoff_id.to_owned())
        .with_param("workspace", context.workspace_id.clone())
        .with_param("repository", context.repository_id.clone())
        .execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
        let Some(row) = rows.data.next().await else {
            return Err(StorageError::Query("handoff not found".to_owned()));
        };
        let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
        let payload: String = row
            .try_get("payload")
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let record: HandoffRecord = serde_json::from_str(&payload)
            .map_err(|error| StorageError::Query(error.to_string()))?;
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
        if !matches!(status, "claimed" | "consumed" | "expired") {
            return Err(StorageError::Query("invalid handoff transition".to_owned()));
        }
        let current = self.retrieve_handoff(context, handoff_id).await?;
        if (current.status == "consumed" && status == "consumed")
            || (current.status == "expired" && status == "expired")
        {
            return Ok(current);
        }
        let valid = matches!(
            (current.status.as_str(), status),
            ("pending", "claimed") | ("claimed", "consumed") | ("pending", "expired")
        );
        if !valid {
            return Err(StorageError::Query(format!(
                "handoff cannot transition from {} to {status}",
                current.status
            )));
        }
        let mut next = current;
        next.status = status.to_owned();
        next.version += 1;
        next.updated_at = Utc::now().to_rfc3339();
        self.persist_handoff(&next).await
    }

    async fn persist_review_context(
        &self,
        record: &ReviewContextRecord,
    ) -> Result<ReviewContextRecord, StorageError> {
        let mut graph = self.client.select_graph(&self.graph_name);
        Self::validate_context(&record.context)?;
        let payload = serde_json::to_string(record)
            .map_err(|error| StorageError::Query(error.to_string()))?;
        graph
            .query(
                "MERGE (r:ReviewContextRecord {id: $id, workspace_id: $workspace, repository_id: $repository}) \
                 SET r.payload_json = $payload, r.snapshot_id = $snapshot, r.graph_version = $graph_version, \
                 r.updated_at = $updated_at",
            )
            .with_param("id", record.context_id.clone())
            .with_param("workspace", record.context.workspace_id.clone())
            .with_param("repository", record.context.repository_id.clone())
            .with_param("snapshot", record.context.source_snapshot_id.clone())
            .with_param("graph_version", record.context.graph_version.clone())
            .with_param("payload", payload)
            .with_param("updated_at", record.updated_at.clone())
            .execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;
        Ok(record.clone())
    }

    async fn retrieve_review_context(
        &self,
        context: &RepositoryContext,
        context_id: &str,
    ) -> Result<ReviewContextRecord, StorageError> {
        Self::validate_context(context)?;
        let mut graph = self.client.select_graph(&self.graph_name);
        let mut rows = graph
            .ro_query(
                "MATCH (r:ReviewContextRecord {id: $id, workspace_id: $workspace, repository_id: $repository}) \
                 RETURN r.payload_json AS payload LIMIT 1",
            )
            .with_param("id", context_id.to_owned())
            .with_param("workspace", context.workspace_id.clone())
            .with_param("repository", context.repository_id.clone())
            .execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let Some(row) = rows.data.next().await else {
            return Err(StorageError::Query("review context not found".to_owned()));
        };
        let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
        let payload: String = row
            .try_get("payload")
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let record = serde_json::from_str::<ReviewContextRecord>(&payload)
            .map_err(|error| StorageError::Query(error.to_string()))?;
        if record.context.source_snapshot_id != context.source_snapshot_id
            || record.context.graph_version != context.graph_version
        {
            return Err(StorageError::Query(
                "review context snapshot or graph version is incompatible".to_owned(),
            ));
        }
        Ok(record)
    }
}

fn relationship_type(kind: &RelationshipKind) -> &'static str {
    match kind {
        RelationshipKind::Defines => "DEFINES",
        RelationshipKind::Imports => "IMPORTS",
        RelationshipKind::Calls => "CALLS",
    }
}

fn parse_relationship(value: String) -> RelationshipKind {
    match value.as_str() {
        "IMPORTS" => RelationshipKind::Imports,
        "CALLS" => RelationshipKind::Calls,
        _ => RelationshipKind::Defines,
    }
}
