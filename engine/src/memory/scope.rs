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
mod level1_tests {
    use super::AccessContext;

    #[test]
    fn access_context_requires_principal_workspace_and_supported_scope() {
        assert!(AccessContext::new("user-1", "user", "workspace-1").is_ok());
        assert!(AccessContext::new("team-1", "team", "workspace-1").is_ok());
        assert!(AccessContext::new("user-1", "repository", "workspace-1").is_ok());

        assert!(AccessContext::new("", "user", "workspace-1").is_err());
        assert!(AccessContext::new("user-1", "user", " ").is_err());
        assert!(AccessContext::new("user-1", "global", "workspace-1").is_err());
    }

    #[test]
    fn access_context_validation_rejects_mutated_invalid_values() {
        let mut access = AccessContext::new("user-1", "user", "workspace-1").unwrap();
        assert!(access.validate().is_ok());

        access.requested_scope = "global".into();
        assert!(access.validate().is_err());
    }
}
