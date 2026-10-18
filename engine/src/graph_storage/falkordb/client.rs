use crate::engines::coie::model::{
    CodeGraph, EntityKind, GraphEntity, GraphRelationship, RelationshipKind,
};
use crate::engines::tie::model::{
    TestEntity, TestRelationship, TestRelationshipKind, TestResolution,
};
use crate::graph_storage::port::{GraphStorage, StorageError};
use crate::memory::model::{HandoffRecord, MemoryRecord, ReviewContextRecord};
use crate::memory::repository::MemoryRepository;
use crate::memory::validation;
use crate::source::RepositoryContext;
use async_trait::async_trait;
use chrono::Utc;
use falkordb::{FalkorAsyncClient, FalkorClientBuilder, FalkorConnectionInfo};
use futures::StreamExt;
use std::collections::HashSet;

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
        context.validate().map_err(|_| StorageError::InvalidContext)
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
        let mut graph = self.client.select_graph(&self.graph_name);
        let pattern = if incoming {
            "MATCH (a:CodeEntity {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})-[r]->(b:CodeEntity {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})"
        } else {
            "MATCH (a:CodeEntity {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})-[r]->(b:CodeEntity {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})"
        };
        let query = format!(
            "{pattern} WHERE r.workspace_id = $workspace AND r.repository_id = $repository AND r.snapshot_id = $snapshot AND r.graph_version = $graph_version AND type(r) IN ['CONTAINS', 'DEFINES', 'IMPORTS', 'CALLS'] RETURN r.id AS relationship_id, type(r) AS relationship_type, a.id AS from_id, b.id AS to_id,                 r.resolution AS resolution, r.evidence_path AS evidence_path, r.evidence_start_line AS evidence_start_line,                 r.evidence_end_line AS evidence_end_line, r.evidence_snapshot_id AS evidence_snapshot_id, r.evidence_provenance AS evidence_provenance LIMIT $limit"
        );
        let mut result = graph
            .ro_query(&query)
            .with_param("id", entity_id.to_owned())
            .with_param("workspace", context.workspace_id.clone())
            .with_param("repository", context.repository_id.clone())
            .with_param("snapshot", context.source_snapshot_id.clone())
            .with_param("graph_version", context.graph_version.clone())
            .with_param("limit", limit as i64)
            .execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;

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
                )?,
                from: row
                    .try_get("from_id")
                    .map_err(|error| StorageError::Query(error.to_string()))?,
                to: row
                    .try_get("to_id")
                    .map_err(|error| StorageError::Query(error.to_string()))?,
                context: context.clone(),
                resolution: parse_resolution(
                    row.try_get("resolution")
                        .unwrap_or_else(|_| "Resolved".to_owned()),
                )?,
                evidence: {
                    let path: String = row.try_get("evidence_path").unwrap_or_default();
                    if path.is_empty() {
                        None
                    } else {
                        Some(crate::engines::coie::model::SourceEvidence {
                            path,
                            start_line: row.try_get::<i64>("evidence_start_line").unwrap_or(0)
                                as u32,
                            end_line: row.try_get::<i64>("evidence_end_line").unwrap_or(0) as u32,
                            snapshot_id: row.try_get("evidence_snapshot_id").unwrap_or_default(),
                            provenance: row.try_get("evidence_provenance").unwrap_or_default(),
                        })
                    }
                },
            });
        }
        Ok(relationships)
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
        let mut graph = self.client.select_graph(&self.graph_name);

        match graph
            .ro_query(
                "MATCH (o:OperationRecord {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})              RETURN o.payload_json AS payload LIMIT 1",
            )
            .with_param("id", operation.operation_id.clone())
            .with_param("workspace", operation.context.workspace_id.clone())
            .with_param("repository", operation.context.repository_id.clone())
            .with_param("snapshot", operation.context.source_snapshot_id.clone())
            .with_param("graph_version", operation.context.graph_version.clone())
            .execute()
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
        .execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
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
        let mut graph = self.client.select_graph(&self.graph_name);
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
        .execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
        Ok(())
    }

    async fn retrieve_operation(
        &self,
        context: &RepositoryContext,
        operation_id: &str,
    ) -> Result<crate::contract::operation::OperationRecord, StorageError> {
        Self::validate_context(context)?;
        let mut graph = self.client.select_graph(&self.graph_name);
        let mut rows = graph.ro_query(
            "MATCH (o:OperationRecord {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) \
             RETURN o.payload_json AS payload LIMIT 1"
        )
        .with_param("id", operation_id.to_owned())
        .with_param("workspace", context.workspace_id.clone())
        .with_param("repository", context.repository_id.clone())
        .with_param("snapshot", context.source_snapshot_id.clone())
        .with_param("graph_version", context.graph_version.clone())
        .execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
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
        let mut graph = self.client.select_graph(&self.graph_name);
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
        .execute().await.map_err(|error| StorageError::Query(error.to_string()))?;

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

    async fn write_code_graph(&self, graph_data: &CodeGraph) -> Result<(), StorageError> {
        Self::validate_context(&graph_data.context)?;
        let mut graph = self.client.select_graph(&self.graph_name);
        let entity_ids: HashSet<&str> = graph_data
            .entities
            .iter()
            .map(|entity| entity.id.as_str())
            .collect();
        let relationship_ids: HashSet<&str> = graph_data
            .relationships
            .iter()
            .map(|relationship| relationship.id.as_str())
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
            if !entity_ids.contains(relationship.from.as_str())
                || !entity_ids.contains(relationship.to.as_str())
            {
                return Err(StorageError::Query(
                    "graph relationship references an entity absent from the code graph".to_owned(),
                ));
            }
        }
        graph
            .query(
                "MATCH (n:CodeEntity {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) WHERE NOT n.id IN $entity_ids DETACH DELETE n",
            )
            .with_param("workspace", graph_data.context.workspace_id.clone())
            .with_param("repository", graph_data.context.repository_id.clone())
            .with_param("snapshot", graph_data.context.source_snapshot_id.clone())
            .with_param("graph_version", graph_data.context.graph_version.clone())
            .with_param("entity_ids", entity_ids.iter().map(|id| (*id).to_owned()).collect::<Vec<_>>())
            .execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;
        graph
            .query(
                "MATCH (:CodeEntity {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})-[r]->(:CodeEntity {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) WHERE type(r) IN ['CONTAINS', 'DEFINES', 'IMPORTS', 'CALLS'] AND NOT r.id IN $relationship_ids DELETE r",
            )
            .with_param("workspace", graph_data.context.workspace_id.clone())
            .with_param("repository", graph_data.context.repository_id.clone())
            .with_param("snapshot", graph_data.context.source_snapshot_id.clone())
            .with_param("graph_version", graph_data.context.graph_version.clone())
            .with_param("relationship_ids", relationship_ids.iter().map(|id| (*id).to_owned()).collect::<Vec<_>>())
            .execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;
        for entity in &graph_data.entities {
            let merge_key = crate::graph_storage::falkordb::write::stable_merge_key(
                &entity.id,
                &entity.context.workspace_id,
                &entity.context.source_snapshot_id,
            )
            .map_err(StorageError::Query)?;
            graph
                .query(
                    "MERGE (n:CodeEntity {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) SET n.kind = $kind, n.name = $name, \
                 n.path = $path, n.line = $line, n.workspace_id = $workspace, \
                 n.repository_id = $repository, n.snapshot_id = $snapshot, n.graph_version = $graph_version,                  n.resolution = $resolution, n.evidence_path = $evidence_path, n.evidence_start_line = $evidence_start_line,                  n.evidence_end_line = $evidence_end_line, n.evidence_snapshot_id = $evidence_snapshot_id, n.evidence_provenance = $evidence_provenance, n.merge_key = $merge_key",
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
                .with_param("resolution", format!("{:?}", entity.resolution))
                .with_param("evidence_path", entity.evidence.as_ref().map(|e| e.path.clone()).unwrap_or_default())
                .with_param("evidence_start_line", entity.evidence.as_ref().map(|e| e.start_line as i64).unwrap_or(0))
                .with_param("evidence_end_line", entity.evidence.as_ref().map(|e| e.end_line as i64).unwrap_or(0))
                .with_param("evidence_snapshot_id", entity.evidence.as_ref().map(|e| e.snapshot_id.clone()).unwrap_or_default())
                .with_param("evidence_provenance", entity.evidence.as_ref().map(|e| e.provenance.clone()).unwrap_or_default())
                .execute()
                .await
                .map_err(|error| StorageError::Query(error.to_string()))?;
        }
        for relationship in &graph_data.relationships {
            let relationship_type = relationship_type(&relationship.kind);
            let query = format!(
                "MATCH (a:CodeEntity {{id: $from, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}}), (b:CodeEntity {{id: $to, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}}) \
                 MERGE (a)-[r:{relationship_type} {{id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}}]->(b)                  SET r.resolution = $resolution, r.evidence_path = $evidence_path, r.evidence_start_line = $evidence_start_line,                      r.evidence_end_line = $evidence_end_line, r.evidence_snapshot_id = $evidence_snapshot_id, r.evidence_provenance = $evidence_provenance"
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
                .with_param("resolution", format!("{:?}", relationship.resolution))
                .with_param(
                    "evidence_path",
                    relationship
                        .evidence
                        .as_ref()
                        .map(|e| e.path.clone())
                        .unwrap_or_default(),
                )
                .with_param(
                    "evidence_start_line",
                    relationship
                        .evidence
                        .as_ref()
                        .map(|e| e.start_line as i64)
                        .unwrap_or(0),
                )
                .with_param(
                    "evidence_end_line",
                    relationship
                        .evidence
                        .as_ref()
                        .map(|e| e.end_line as i64)
                        .unwrap_or(0),
                )
                .with_param(
                    "evidence_snapshot_id",
                    relationship
                        .evidence
                        .as_ref()
                        .map(|e| e.snapshot_id.clone())
                        .unwrap_or_default(),
                )
                .with_param(
                    "evidence_provenance",
                    relationship
                        .evidence
                        .as_ref()
                        .map(|e| e.provenance.clone())
                        .unwrap_or_default(),
                )
                .execute()
                .await
                .map_err(|error| StorageError::Query(error.to_string()))?;
        }
        Ok(())
    }

    async fn persist_change_analysis(
        &self,
        change_set: &crate::engines::chie::ChangeSet,
        change_record: &crate::engines::chie::ChangeRecord,
        analysis_run: &crate::engines::chie::AnalysisRun,
        analysis: &crate::engines::chie::ChangeAnalysis,
    ) -> Result<(), StorageError> {
        Self::validate_context(&analysis.context)?;
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
        let impact_id = format!("impact:{}", analysis_run.id);
        let risk_id = format!("risk:{}", analysis_run.id);
        let mut graph = self.client.select_graph(&self.graph_name);

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
            .execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;

        graph
            .query(
                "MERGE (n:ChangeRecord {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) SET n.payload_json = $payload",
            )
            .with_param("id", format!("change-record:{}", change_record.change_set_id))
            .with_param("workspace", analysis.context.workspace_id.clone())
            .with_param("repository", analysis.context.repository_id.clone())
            .with_param("snapshot", analysis.context.source_snapshot_id.clone())
            .with_param("graph_version", analysis.context.graph_version.clone())
            .with_param("payload", change_record_payload)
            .execute()
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
            .execute()
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
            .execute()
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
                .execute()
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
                .execute()
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
            .execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;

        graph
            .query(
                "MATCH (set:ChangeSet {id: $set_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}), (record:ChangeRecord {id: $record_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) MERGE (set)-[:CONTAINS_CHANGE]->(record)",
            )
            .with_param("set_id", change_set.id.clone())
            .with_param("record_id", format!("change-record:{}", change_record.change_set_id))
            .with_param("workspace", analysis.context.workspace_id.clone())
            .with_param("repository", analysis.context.repository_id.clone())
            .with_param("snapshot", analysis.context.source_snapshot_id.clone())
            .with_param("graph_version", analysis.context.graph_version.clone())
            .execute()
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
            .execute()
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
                .execute()
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
                .execute()
                .await
                .map_err(|error| StorageError::Query(error.to_string()))?;
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
                .execute()
                .await
                .map_err(|error| StorageError::Query(error.to_string()))?;
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
        let mut graph = self.client.select_graph(&self.graph_name);
        let mut rows = graph
            .ro_query(
                "MATCH (run:AnalysisRun {id: $run_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})-[r1:PRODUCES|ANALYZES|HAS_RISK|RECOMMENDS]->(target) RETURN type(r1) AS kind, run.id AS from_id, target.id AS to_id UNION MATCH (impact:ImpactAnalysis {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})-[r2:AFFECTS|SUPPORTED_BY]->(target) MATCH (run:AnalysisRun {id: $run_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})-[:PRODUCES]->(impact) RETURN type(r2) AS kind, impact.id AS from_id, target.id AS to_id UNION MATCH (set:ChangeSet {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})-[r3:CONTAINS_CHANGE]->(target) MATCH (run:AnalysisRun {id: $run_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})-[:ANALYZES]->(set) RETURN type(r3) AS kind, set.id AS from_id, target.id AS to_id LIMIT $limit",
            )
            .with_param("run_id", analysis_run_id.to_owned())
            .with_param("workspace", context.workspace_id.clone())
            .with_param("repository", context.repository_id.clone())
            .with_param("snapshot", context.source_snapshot_id.clone())
            .with_param("graph_version", context.graph_version.clone())
            .with_param("limit", limit.clamp(1, 200) as i64)
            .execute()
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
        let mut graph = self.client.select_graph(&self.graph_name);
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
            .execute()
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
        let mut graph = self.client.select_graph(&self.graph_name);
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
            .execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;

        let mut relationships = Vec::new();
        while let Some(row) = rows.data.next().await {
            let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
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
                resolution: parse_resolution(
                    row.try_get("resolution")
                        .unwrap_or_else(|_| "Resolved".to_owned()),
                )?,
                evidence: {
                    let path: String = row.try_get("evidence_path").unwrap_or_default();
                    if path.is_empty() {
                        None
                    } else {
                        Some(crate::engines::coie::model::SourceEvidence {
                            path,
                            start_line: row.try_get::<i64>("evidence_start_line").unwrap_or(0)
                                as u32,
                            end_line: row.try_get::<i64>("evidence_end_line").unwrap_or(0) as u32,
                            snapshot_id: row.try_get("evidence_snapshot_id").unwrap_or_default(),
                            provenance: row.try_get("evidence_provenance").unwrap_or_default(),
                        })
                    }
                },
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
        let limit = limit.clamp(1, 100);
        let mut graph = self.client.select_graph(&self.graph_name);

        let mut relationships = Vec::new();
        if !target_entity_ids.is_empty() {
            let mut rows = graph
                .ro_query(
                    "MATCH (t:TestEntity {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})-[r:TARGETS|DECLARED_BY]->(e:CodeEntity {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) WHERE e.id IN $targets RETURN r.id AS relationship_id, t.id AS test_id, e.id AS entity_id, type(r) AS relationship_type, r.evidence_path AS evidence_path, r.evidence_start_line AS start_line, r.evidence_end_line AS end_line, r.evidence_snapshot_id AS evidence_snapshot_id, r.evidence_provenance AS evidence_provenance, r.resolution AS resolution LIMIT $limit",
                )
                .with_param("workspace", context.workspace_id.clone())
                .with_param("repository", context.repository_id.clone())
                .with_param("snapshot", context.source_snapshot_id.clone())
                .with_param("graph_version", context.graph_version.clone())
                .with_param("targets", target_entity_ids.to_vec())
                .with_param("limit", limit as i64)
                .execute()
                .await
                .map_err(|error| StorageError::Query(error.to_string()))?;

            while let Some(row) = rows.data.next().await {
                let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
                relationships.push(TestRelationship {
                    id: row
                        .try_get("relationship_id")
                        .map_err(|error| StorageError::Query(error.to_string()))?,
                    kind: match row
                        .try_get::<String>("relationship_type")
                        .unwrap_or_else(|_| "TARGETS".to_owned())
                        .as_str()
                    {
                        "DECLARED_BY" => TestRelationshipKind::DeclaredBy,
                        _ => TestRelationshipKind::Targets,
                    },
                    test_id: row
                        .try_get("test_id")
                        .map_err(|error| StorageError::Query(error.to_string()))?,
                    target_symbol_name: None,
                    entity_id: Some(
                        row.try_get("entity_id")
                            .map_err(|error| StorageError::Query(error.to_string()))?,
                    ),
                    evidence_path: row.try_get("evidence_path").unwrap_or_default(),
                    evidence: {
                        let path: String = row.try_get("evidence_path").unwrap_or_default();
                        if path.is_empty() {
                            None
                        } else {
                            Some(crate::engines::coie::model::SourceEvidence {
                                path,
                                start_line: row.try_get::<i64>("start_line").unwrap_or(0) as u32,
                                end_line: row.try_get::<i64>("end_line").unwrap_or(0) as u32,
                                snapshot_id: row
                                    .try_get("evidence_snapshot_id")
                                    .unwrap_or_default(),
                                provenance: row.try_get("evidence_provenance").unwrap_or_default(),
                            })
                        }
                    },
                    resolution: parse_test_resolution(
                        row.try_get("resolution")
                            .unwrap_or_else(|_| "Resolved".to_owned()),
                    )?,
                    context: context.clone(),
                });
            }
        }

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
                    "MATCH (t:TestEntity {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) WHERE t.id IN $test_ids RETURN t.id AS id, t.name AS name, t.path AS path, t.line AS line, t.framework AS framework, t.status AS status LIMIT $limit",
                )
                .with_param("workspace", context.workspace_id.clone())
                .with_param("repository", context.repository_id.clone())
                .with_param("snapshot", context.source_snapshot_id.clone())
                .with_param("graph_version", context.graph_version.clone())
                .with_param("test_ids", test_ids)
                .with_param("limit", limit as i64)
                .execute()
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
                            .unwrap_or_else(|_| "Supported".to_owned()),
                    )?,
                    context: context.clone(),
                });
            }
        }

        Ok(crate::graph_storage::port::StoredTestGraph {
            tests,
            relationships,
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
        if tests.iter().any(|test| test.context != *context)
            || relationships
                .iter()
                .any(|relationship| relationship.context != *context)
        {
            return Err(StorageError::InvalidContext);
        }
        let test_ids: HashSet<&str> = tests.iter().map(|test| test.id.as_str()).collect();
        let relationship_ids: HashSet<&str> = relationships
            .iter()
            .map(|relationship| relationship.id.as_str())
            .collect();
        let mut graph = self.client.select_graph(&self.graph_name);
        graph
            .query(
                "MATCH (n:TestEntity {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) WHERE NOT n.id IN $test_ids DETACH DELETE n",
            )
            .with_param("workspace", context.workspace_id.clone())
            .with_param("repository", context.repository_id.clone())
            .with_param("snapshot", context.source_snapshot_id.clone())
            .with_param("graph_version", context.graph_version.clone())
            .with_param("test_ids", test_ids.iter().map(|id| (*id).to_owned()).collect::<Vec<_>>())
            .execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;
        graph
            .query(
                "MATCH (:TestEntity {workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version})-[r]->() WHERE type(r) IN ['TARGETS', 'DECLARED_BY'] AND NOT r.id IN $relationship_ids DELETE r",
            )
            .with_param("workspace", context.workspace_id.clone())
            .with_param("repository", context.repository_id.clone())
            .with_param("snapshot", context.source_snapshot_id.clone())
            .with_param("graph_version", context.graph_version.clone())
            .with_param("relationship_ids", relationship_ids.iter().map(|id| (*id).to_owned()).collect::<Vec<_>>())
            .execute()
            .await
            .map_err(|error| StorageError::Query(error.to_string()))?;

        for test in tests {
            graph
                .query(
                    "MERGE (n:TestEntity {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) SET n.name = $name, n.path = $path, \
                 n.line = $line, n.framework = $framework, n.status = $status, n.workspace_id = $workspace, \
                 n.repository_id = $repository, n.snapshot_id = $snapshot",
                )
                .with_param("id", test.id.clone())
                .with_param("name", test.name.clone())
                .with_param("path", test.path.clone())
                .with_param("line", test.line as i64)
                .with_param("framework", test.framework.clone())
                .with_param("status", format!("{:?}", test.status))
                .with_param("workspace", test.context.workspace_id.clone())
                .with_param("repository", test.context.repository_id.clone())
                .with_param("snapshot", test.context.source_snapshot_id.clone())
                .with_param("graph_version", test.context.graph_version.clone())
                .execute()
                .await
                .map_err(|error| StorageError::Query(error.to_string()))?;
        }
        for relationship in relationships {
            let relationship_id = relationship.id.clone();
            match relationship.kind {
                TestRelationshipKind::Targets | TestRelationshipKind::DeclaredBy => {
                    if relationship.resolution != TestResolution::Resolved {
                        continue;
                    }
                    let Some(entity_id) = relationship.entity_id.as_deref() else {
                        return Err(StorageError::Query(
                            "resolved TIE relationship is missing its canonical entity ID"
                                .to_owned(),
                        ));
                    };
                    let relationship_type = match relationship.kind {
                        TestRelationshipKind::Targets => "TARGETS",
                        TestRelationshipKind::DeclaredBy => "DECLARED_BY",
                    };
                    let evidence = relationship.evidence.as_ref();
                    let query = format!(
                        "MATCH (test:TestEntity {{id: $test_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}}) \
                         MATCH (target:CodeEntity {{id: $entity_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}}) \
                         MERGE (test)-[r:{relationship_type} {{id: $relationship_id}}]->(target) \
                         SET r.path = $path, r.evidence_path = $path, r.start_line = $start_line, r.end_line = $end_line, r.evidence_snapshot_id = $snapshot, r.evidence_provenance = $provenance, r.snapshot_id = $snapshot, r.workspace_id = $workspace, r.repository_id = $repository, r.graph_version = $graph_version, r.resolution = $resolution"
                    );
                    graph
                        .query(query)
                        .with_param("test_id", relationship.test_id.clone())
                        .with_param("entity_id", entity_id.to_owned())
                        .with_param("relationship_id", relationship_id)
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
                        .with_param("resolution", format!("{:?}", relationship.resolution))
                        .with_param("workspace", relationship.context.workspace_id.clone())
                        .with_param("repository", relationship.context.repository_id.clone())
                        .with_param("snapshot", relationship.context.source_snapshot_id.clone())
                        .with_param("graph_version", relationship.context.graph_version.clone())
                        .execute()
                        .await
                        .map_err(|error| StorageError::Query(error.to_string()))?;
                }
            }
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

        let mut existing = graph.ro_query(
            "MATCH (m:MemoryRecord {id: $id, workspace_id: $workspace, repository_id: $repository})              RETURN m.payload_json AS payload, m.idempotency_key AS idempotency_key, m.version AS version LIMIT 1"
        )
        .with_param("id", record.memory_id.clone())
        .with_param("workspace", record.context.workspace_id.clone())
        .with_param("repository", record.context.repository_id.clone())
        .execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
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
        owner_id: &str,
        limit: usize,
    ) -> Result<Vec<MemoryRecord>, StorageError> {
        Self::validate_context(context)?;
        let mut graph = self.client.select_graph(&self.graph_name);
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
        let mut existing = graph
            .ro_query(
                "MATCH (h:HandoffRecord {id: $id, workspace_id: $workspace, repository_id: $repository}) \
                 RETURN h.payload_json AS payload, h.idempotency_key AS idempotency_key, \
                 h.status AS status, h.version AS version LIMIT 1",
            )
            .with_param("id", record.handoff_id.clone())
            .with_param("workspace", record.context.workspace_id.clone())
            .with_param("repository", record.context.repository_id.clone())
            .execute()
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
            "MATCH (h:HandoffRecord {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) \
             RETURN h.payload_json AS payload, h.status AS status, h.version AS version LIMIT 1"
        )
        .with_param("id", handoff_id.to_owned())
        .with_param("workspace", context.workspace_id.clone())
        .with_param("repository", context.repository_id.clone())
        .with_param("snapshot", context.source_snapshot_id.clone())
        .with_param("graph_version", context.graph_version.clone())
        .execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
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
        if !matches!(status, "claimed" | "consumed" | "expired") {
            return Err(StorageError::Query("invalid handoff transition".to_owned()));
        }
        let now = Utc::now().to_rfc3339();
        let mut graph = self.client.select_graph(&self.graph_name);
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
            .execute()
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
        let mut graph = self.client.select_graph(&self.graph_name);
        Self::validate_context(&record.context)?;

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
        .execute().await.map_err(|error| StorageError::Query(error.to_string()))?;

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
        .execute().await.map_err(|error| StorageError::Query(error.to_string()))?;

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
        .execute().await.map_err(|error| StorageError::Query(error.to_string()))?;

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
        .execute().await.map_err(|error| StorageError::Query(error.to_string()))?;

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
            .execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
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
            .execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
        }

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
                "MATCH (r:ReviewContextRecord {id: $id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) \
                 MATCH (s:ConversationSession {id: r.session_id, workspace_id: $workspace, repository_id: $repository, snapshot_id: $snapshot, graph_version: $graph_version}) \
                 RETURN s.payload_json AS payload, r.owner_id AS owner_id, r.episode_id AS episode_id LIMIT 1"
            )
            .with_param("id", context_id.to_owned())
            .with_param("workspace", context.workspace_id.clone())
            .with_param("repository", context.repository_id.clone())
            .with_param("snapshot", context.source_snapshot_id.clone())
            .with_param("graph_version", context.graph_version.clone())
            .execute().await.map_err(|error| StorageError::Query(error.to_string()))?;

        let Some(row) = rows.data.next().await else {
            return Err(StorageError::Query("review session not found".to_owned()));
        };
        let row = row.map_err(|error| StorageError::Query(error.to_string()))?;
        let session_payload: String = row
            .try_get("payload")
            .map_err(|error| StorageError::Query(error.to_string()))?;
        let owner_id: String = row.try_get("owner_id").unwrap_or_default();
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
        .execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
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
        .execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
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
        .execute().await.map_err(|error| StorageError::Query(error.to_string()))?;
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

fn parse_test_status(
    value: String,
) -> Result<crate::engines::tie::model::TestStatus, StorageError> {
    match value.as_str() {
        "Supported" => Ok(crate::engines::tie::model::TestStatus::Supported),
        "Partial" => Ok(crate::engines::tie::model::TestStatus::Partial),
        "Unsupported" => Ok(crate::engines::tie::model::TestStatus::Unsupported),
        _ => Err(StorageError::Query(format!(
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
        _ => Err(StorageError::Query(format!(
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
        _ => Err(StorageError::Query(format!(
            "unsupported graph resolution: {value}"
        ))),
    }
}

fn relationship_type(kind: &RelationshipKind) -> &'static str {
    crate::graph_storage::falkordb::mapping::code_relationship_label(kind)
}

fn parse_relationship(value: String) -> Result<RelationshipKind, StorageError> {
    match value.as_str() {
        "CONTAINS" => Ok(RelationshipKind::Contains),
        "DEFINES" => Ok(RelationshipKind::Defines),
        "IMPORTS" => Ok(RelationshipKind::Imports),
        "CALLS" => Ok(RelationshipKind::Calls),
        _ => Err(StorageError::Query(format!(
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
        _ => Err(StorageError::Query(format!(
            "unsupported graph entity kind: {value}"
        ))),
    }
}
