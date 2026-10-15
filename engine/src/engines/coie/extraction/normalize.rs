pub fn identifier(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() || !value.chars().next()?.is_ascii_alphabetic() {
        return None;
    }
    if value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        Some(value.to_owned())
    } else {
        None
    }
}
