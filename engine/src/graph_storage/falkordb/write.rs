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
