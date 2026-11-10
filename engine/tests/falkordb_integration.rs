use codemind_engine::contract::generated::{
    CanonicalResult, Completeness, Correlation, Freshness, Outcome, QueryResponse,
    RepositoryContext, query_response,
};
use prost::Message;

#[test]
fn canonical_result_round_trip_preserves_context_and_status() {
    let response = QueryResponse {
        result: Some(query_response::Result::Data(CanonicalResult {
            outcome: Outcome::Partial as i32,
            completeness: Completeness::Truncated as i32,
            freshness: Freshness::Current as i32,
            context: Some(RepositoryContext {
                repository_id: "repo-a".to_owned(),
                workspace_id: "workspace-a".to_owned(),
                source_snapshot_id: "snapshot-a".to_owned(),
                graph_version: "day-1".to_owned(),
            }),
            correlation: Some(Correlation {
                request_id: "request-a".to_owned(),
                trace_id: "trace-a".to_owned(),
                operation_id: "operation-a".to_owned(),
            }),
            data_json: "{\"relationships\":[]}".to_owned(),
            evidence: Vec::new(),
            diagnostics: Vec::new(),
            limitations: vec!["bounded".to_owned()],
        })),
    };
    let decoded = QueryResponse::decode(response.encode_to_vec().as_slice()).unwrap();
    let Some(query_response::Result::Data(result)) = decoded.result else {
        panic!("expected canonical result");
    };
    assert_eq!(result.outcome, Outcome::Partial as i32);
    assert_eq!(result.completeness, Completeness::Truncated as i32);
    assert_eq!(result.context.unwrap().source_snapshot_id, "snapshot-a");
    assert_eq!(result.correlation.unwrap().operation_id, "operation-a");
}
