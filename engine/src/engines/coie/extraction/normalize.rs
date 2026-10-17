pub fn identifier(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let mut characters = value.chars();
    let first = characters.next()?;
    if !(first == '_' || first.is_alphabetic()) {
        return None;
    }
    characters
        .all(|character| character == '_' || character.is_alphanumeric())
        .then_some(value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::identifier;

    #[test]
    fn identifier_accepts_rust_like_names_and_rejects_invalid_tokens() {
        assert_eq!(identifier(" _name2 "), Some("_name2".into()));
        assert!(identifier("").is_none());
        assert!(identifier("2name").is_none());
        assert!(identifier("name-with-dash").is_none());
    }
}
