pub fn can_transition(from: &str, to: &str) -> bool {
    matches!(
        (from, to),
        ("pending", "claimed")
            | ("claimed", "consumed")
            | ("pending", "expired")
            | ("consumed", "consumed")
            | ("expired", "expired")
    )
}

#[cfg(test)]
mod tests {
    use super::can_transition;

    #[test]
    fn handoff_transitions_cannot_skip_claim_or_rewind() {
        assert!(can_transition("pending", "claimed"));
        assert!(can_transition("claimed", "consumed"));
        assert!(can_transition("pending", "expired"));
        assert!(!can_transition("pending", "consumed"));
        assert!(!can_transition("consumed", "claimed"));
    }
}
