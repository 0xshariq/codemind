use crate::{contract::generated, source};

pub fn context(value: &generated::RepositoryContext) -> source::RepositoryContext {
    source::RepositoryContext {
        repository_id: value.repository_id.clone(),
        workspace_id: value.workspace_id.clone(),
        source_snapshot_id: value.source_snapshot_id.clone(),
        graph_version: value.graph_version.clone(),
    }
}

pub fn context_proto(value: &source::RepositoryContext) -> generated::RepositoryContext {
    generated::RepositoryContext {
        repository_id: value.repository_id.clone(),
        workspace_id: value.workspace_id.clone(),
        source_snapshot_id: value.source_snapshot_id.clone(),
        graph_version: value.graph_version.clone(),
    }
}

pub fn canonical_json<T: serde::Serialize>(
    value: &crate::contract::result::CanonicalResult<T>,
) -> Result<String, String> {
    value.validate().map_err(|error| error.to_string())?;
    serde_json::to_string(value).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::result::{CanonicalResult, Completeness, Correlation, Freshness, Outcome};

    #[test]
    fn context_conversion_is_lossless_and_canonical_json_validates() {
        let repository_context = source::RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        };
        let proto = context_proto(&repository_context);
        assert_eq!(context(&proto), repository_context);
        let result = CanonicalResult::new(
            repository_context,
            Correlation {
                request_id: "request".into(),
                trace_id: "trace".into(),
                operation_id: "operation".into(),
            },
            Outcome::Success,
            Completeness::Complete,
            Freshness::Current,
            vec!["value"],
        );
        assert!(canonical_json(&result).is_ok());
    }
}
