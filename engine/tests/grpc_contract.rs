use codemind_engine::memory::handoffs::can_transition;

#[test]
fn handoff_lifecycle_is_idempotent_but_not_skippable() {
    assert!(can_transition("pending", "claimed"));
    assert!(can_transition("claimed", "consumed"));
    assert!(can_transition("consumed", "consumed"));
    assert!(can_transition("expired", "expired"));
    assert!(!can_transition("pending", "consumed"));
    assert!(!can_transition("consumed", "claimed"));
}
