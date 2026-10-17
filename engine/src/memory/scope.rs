use crate::source::RepositoryContext;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessContext {
    pub principal_id: String,
    pub requested_scope: String,
    pub workspace_id: String,
}

impl AccessContext {
    pub fn new(
        principal_id: impl Into<String>,
        requested_scope: impl Into<String>,
        workspace_id: impl Into<String>,
    ) -> Result<Self, String> {
        let access = Self {
            principal_id: principal_id.into(),
            requested_scope: requested_scope.into(),
            workspace_id: workspace_id.into(),
        };
        access.validate()?;
        Ok(access)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.principal_id.trim().is_empty()
            || self.workspace_id.trim().is_empty()
            || !matches!(
                self.requested_scope.as_str(),
                "user" | "team" | "repository"
            )
        {
            return Err("access context is incomplete or has an unsupported scope".to_owned());
        }
        Ok(())
    }

    pub fn allows_record(
        &self,
        record_scope: &str,
        owner_id: &str,
        context: &RepositoryContext,
    ) -> bool {
        self.validate().is_ok()
            && self.workspace_id == context.workspace_id
            && self.requested_scope == record_scope
            && match record_scope {
                "user" | "team" => owner_id == self.principal_id,
                "repository" => owner_id.is_empty() || owner_id == context.repository_id,
                _ => false,
            }
    }
}

pub fn allows(record_scope: &str, requested_scope: &str, context: &RepositoryContext) -> bool {
    record_scope == requested_scope
        && matches!(record_scope, "user" | "team" | "repository")
        && !context.workspace_id.is_empty()
}

#[cfg(test)]
mod tests {
    use super::AccessContext;
    use crate::source::RepositoryContext;

    fn context() -> RepositoryContext {
        RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        }
    }

    #[test]
    fn access_context_enforces_principal_and_workspace_for_user_scope() {
        let access = AccessContext::new("user-1", "user", "workspace").unwrap();
        assert!(access.allows_record("user", "user-1", &context()));
        assert!(!access.allows_record("user", "user-2", &context()));
        assert!(!access.allows_record("team", "user-1", &context()));
    }
}
