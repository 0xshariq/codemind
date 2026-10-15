pub fn normalize_path(path: &str) -> Option<String> {
    let path = path.trim().replace('\\', "/");
    (!path.is_empty() && !path.starts_with("../") && path != "..").then_some(path)
}
