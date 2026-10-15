use crate::engines::chie::engine::ChieEngine;
use crate::engines::chie::model::ChangeAnalysis;
use crate::engines::coie::model::{
    CodeGraph, EntityKind, GraphEntity, GraphRelationship, RelationshipKind,
};
use crate::engines::tie::model::{TestEntity, TestRelationship, TestRelationshipKind};
use crate::graph_storage::port::{GraphStorage, StorageError};
use crate::source::RepositoryContext;
use async_trait::async_trait;
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
