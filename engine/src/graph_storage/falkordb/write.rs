pub fn stable_merge_key(entity_id: &str, workspace_id: &str, snapshot_id: &str) -> String {
    format!("{workspace_id}:{snapshot_id}:{entity_id}")
}
