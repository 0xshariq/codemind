use crate::{contract::generated, source};

pub fn context(value: &generated::RepositoryContext) -> Result<source::RepositoryContext, String> {
    validate_context_values(
        &value.repository_id,
        &value.workspace_id,
        &value.source_snapshot_id,
        &value.graph_version,
    )?;
    Ok(source::RepositoryContext {
        repository_id: value.repository_id.clone(),
        workspace_id: value.workspace_id.clone(),
        source_snapshot_id: value.source_snapshot_id.clone(),
        graph_version: value.graph_version.clone(),
    })
}

/// Validate raw protobuf context values before they enter the engine boundary.
/// The canonical validation rules remain owned by `RepositoryContext`.
pub(crate) fn validate_context_values(
    repository_id: &str,
    workspace_id: &str,
    source_snapshot_id: &str,
    graph_version: &str,
) -> Result<(), String> {
    source::RepositoryContext {
        repository_id: repository_id.to_owned(),
        workspace_id: workspace_id.to_owned(),
        source_snapshot_id: source_snapshot_id.to_owned(),
        graph_version: graph_version.to_owned(),
    }
    .validate()
    .map_err(|error| error.to_string())
}

/// Validate the workspace identity carried by repository-resolution requests.
/// Those requests occur before a snapshot exists, so the other three context
/// fields are intentionally provisional and must not be treated as a complete
/// RepositoryContext yet.
pub(crate) fn workspace_id(value: &str) -> Result<String, String> {
    validate_context_values(
        "repository-request",
        value,
        "snapshot-pending",
        "graph-pending",
    )?;
    Ok(value.to_owned())
}

/// Convert and validate a protobuf code-search envelope into the existing
/// internal CoIE query contract.
pub fn search_code_request(
    value: generated::SearchCodeQuery,
) -> Result<crate::contract::intelligence::CoieQueryRequest, String> {
    let context = context(
        value
            .context
            .as_ref()
            .ok_or_else(|| "context is required".to_owned())?,
    )?;
    let request = crate::contract::intelligence::CoieQueryRequest {
        context,
        kind: crate::contract::intelligence::CoieQueryKind::SearchSymbols,
        query: normalized_text(&value.query, "query", 500)?,
        limit: value.limit.clamp(1, 100) as usize,
    };
    request.validate().map_err(|error| error.to_string())?;
    Ok(request)
}

/// Convert and validate a protobuf relationship envelope into the same CoIE
/// query contract used by engine-side semantic reads.
pub fn symbol_relationship_request(
    value: generated::SymbolRelationshipQuery,
) -> Result<crate::contract::intelligence::CoieQueryRequest, String> {
    let context = context(
        value
            .context
            .as_ref()
            .ok_or_else(|| "context is required".to_owned())?,
    )?;
    let request = crate::contract::intelligence::CoieQueryRequest {
        context,
        kind: crate::contract::intelligence::CoieQueryKind::QueryRelationships,
        query: required_identifier(&value.symbol_id, "symbol_id")?,
        limit: value.limit.clamp(1, 100) as usize,
    };
    request.validate().map_err(|error| error.to_string())?;
    Ok(request)
}

/// Convert a protobuf change-analysis envelope into the established internal
/// ChIE request contract, validating context and depth before engine dispatch.
pub fn change_analysis_request(
    value: generated::ChangeAnalysisQuery,
) -> Result<crate::contract::intelligence::ChangeAnalysisRequest, String> {
    let context = context(
        value
            .context
            .as_ref()
            .ok_or_else(|| "context is required".to_owned())?,
    )?;
    let max_depth = usize::try_from(value.max_depth)
        .map_err(|_| "max_depth must be between 1 and 8".to_owned())?;
    let request = crate::contract::intelligence::ChangeAnalysisRequest {
        context,
        target_entity_id: required_identifier(&value.symbol_id, "symbol_id")?,
        max_depth: validate_change_depth(max_depth)?,
        max_results: crate::contract::result::MAX_RELATIONSHIPS,
    };
    request.validate().map_err(|error| error.to_string())?;
    Ok(request)
}

fn normalized_text(value: &str, field: &str, max_bytes: usize) -> Result<String, String> {
    let normalized = value.trim();
    if normalized.is_empty() || normalized.len() > max_bytes {
        return Err(format!("{field} must contain 1 to {max_bytes} bytes"));
    }
    if normalized.chars().any(char::is_control) {
        return Err(format!("{field} must not contain control characters"));
    }
    Ok(normalized.to_owned())
}

fn required_identifier(value: &str, field: &str) -> Result<String, String> {
    normalized_text(value, field, 512)
}

fn validate_change_depth(max_depth: usize) -> Result<usize, String> {
    if !(1..=8).contains(&max_depth) {
        return Err("max_depth must be between 1 and 8".to_owned());
    }
    Ok(max_depth)
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
    use super::{context, normalized_text, required_identifier, validate_change_depth};
    use crate::contract::generated;

    #[test]
    fn query_text_is_trimmed_bounded_and_rejects_controls() {
        assert_eq!(
            normalized_text("  parser  ", "query", 500).unwrap(),
            "parser"
        );
        assert!(normalized_text("  ", "query", 500).is_err());
        assert!(normalized_text(&"x".repeat(501), "query", 500).is_err());
        assert!(normalized_text("query\nnext", "query", 500).is_err());
    }

    #[test]
    fn opaque_symbol_identifiers_are_trimmed_and_bounded() {
        assert_eq!(
            required_identifier("  symbol:123  ", "symbol_id").unwrap(),
            "symbol:123"
        );
        assert!(required_identifier("  ", "symbol_id").is_err());
        assert!(required_identifier(&"x".repeat(513), "symbol_id").is_err());
    }

    #[test]
    fn change_analysis_depth_accepts_only_supported_bounds() {
        assert_eq!(validate_change_depth(1).unwrap(), 1);
        assert_eq!(validate_change_depth(8).unwrap(), 8);
        assert!(validate_change_depth(0).is_err());
        assert!(validate_change_depth(9).is_err());
    }

    #[test]
    fn context_conversion_rejects_empty_workspace_id() {
        let value = generated::RepositoryContext {
            repository_id: "repository-1".to_owned(),
            workspace_id: String::new(),
            source_snapshot_id: "snapshot-1".to_owned(),
            graph_version: "graph-1".to_owned(),
        };

        let error = context(&value).unwrap_err();
        assert!(error.contains("workspace_id"), "unexpected error: {error}");
    }
}
