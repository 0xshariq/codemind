pub fn stable_merge_key(
    entity_id: &str,
    workspace_id: &str,
    snapshot_id: &str,
) -> Result<String, String> {
    if entity_id.trim().is_empty()
        || workspace_id.trim().is_empty()
        || snapshot_id.trim().is_empty()
    {
        return Err("graph write identity components are required".to_owned());
    }
    Ok(format!("{workspace_id}:{snapshot_id}:{entity_id}"))
}

#[cfg(test)]
mod tests {
    use super::stable_merge_key;

    #[test]
    fn stable_merge_key_separates_workspace_snapshot_and_entity_identity() {
        assert_eq!(
            stable_merge_key("entity", "workspace", "snapshot").unwrap(),
            "workspace:snapshot:entity"
        );
        assert!(stable_merge_key("", "workspace", "snapshot").is_err());
    }
}

pub fn history_stable_merge_key(
    workspace_id: &str,
    repository_id: &str,
    identity: &str,
) -> Result<String, String> {
    if workspace_id.trim().is_empty()
        || repository_id.trim().is_empty()
        || identity.trim().is_empty()
    {
        return Err("history graph identity components are required".to_owned());
    }
    Ok(format!("{workspace_id}:{repository_id}:{identity}"))
}

#[cfg(test)]
mod history_tests {
    use super::history_stable_merge_key;

    #[test]
    fn history_identity_is_repository_scoped_not_snapshot_scoped() {
        assert_eq!(
            history_stable_merge_key("workspace", "repo", "commit:path").unwrap(),
            "workspace:repo:commit:path"
        );
        assert!(history_stable_merge_key("", "repo", "commit").is_err());
    }
}
