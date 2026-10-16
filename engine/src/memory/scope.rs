use crate::source::RepositoryContext;

pub fn allows(record_scope: &str, requested_scope: &str, context: &RepositoryContext) -> bool {
    record_scope == requested_scope
        && matches!(record_scope, "user" | "team" | "repository")
        && !context.workspace_id.is_empty()
}
