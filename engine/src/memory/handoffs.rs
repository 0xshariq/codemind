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
