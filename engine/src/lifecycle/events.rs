use crate::contract::events::EngineEvent;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LifecycleEvent {
    Engine(EngineEvent),
}

impl LifecycleEvent {
    pub fn metadata(&self) -> &crate::contract::events::EventMetadata {
        match self {
            Self::Engine(event) => event.metadata(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LifecycleEvent;
    use crate::{
        contract::events::EngineEvent, lifecycle::operation::Operation, source::RepositoryContext,
    };

    #[test]
    fn lifecycle_event_exposes_the_underlying_engine_metadata() {
        let context = RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        };
        let mut operation = Operation::new("op", "request", "trace", context).unwrap();
        let event = operation.begin_stage("coie").unwrap();
        let engine_event = EngineEvent::Progress {
            metadata: event.metadata.clone(),
            engine: "codemind".into(),
            stage: "coie".into(),
            message: "started".into(),
        };
        assert_eq!(
            LifecycleEvent::Engine(engine_event).metadata().operation_id,
            "op"
        );
    }
}
