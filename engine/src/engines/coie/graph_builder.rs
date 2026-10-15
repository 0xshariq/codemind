use super::model::CodeGraph;

pub fn validate(graph: &CodeGraph) -> bool {
    graph
        .entities
        .iter()
        .all(|entity| entity.context == graph.context)
        && graph
            .relationships
            .iter()
            .all(|relationship| relationship.context == graph.context)
}
