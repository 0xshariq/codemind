pub mod conversions;
pub mod progress;
pub mod server;

#[cfg(test)]
mod level2_tests {
    use super::conversions;

    #[test]
    fn protobuf_repository_context_values_are_validated_before_dispatch() {
        assert!(
            conversions::validate_context_values("repo-1", "workspace-1", "snapshot-1", "graph-1",)
                .is_ok()
        );

        let empty_workspace =
            conversions::validate_context_values("repo-1", "  ", "snapshot-1", "graph-1")
                .unwrap_err();
        assert!(empty_workspace.contains("workspace_id"));

        let control_character =
            conversions::validate_context_values("repo-1", "workspace-1", "snapshot\n1", "graph-1")
                .unwrap_err();
        assert!(control_character.contains("source_snapshot_id"));

        let oversized = "x".repeat(513);
        let too_long =
            conversions::validate_context_values("repo-1", &oversized, "snapshot-1", "graph-1")
                .unwrap_err();
        assert!(too_long.contains("workspace_id"));
    }

    #[test]
    fn repository_resolution_validates_workspace_without_requiring_a_snapshot() {
        assert_eq!(
            conversions::workspace_id("workspace-1").unwrap(),
            "workspace-1"
        );
        assert!(
            conversions::workspace_id("")
                .unwrap_err()
                .contains("workspace_id")
        );
        assert!(
            conversions::workspace_id("bad\nworkspace")
                .unwrap_err()
                .contains("workspace_id")
        );
    }
}
