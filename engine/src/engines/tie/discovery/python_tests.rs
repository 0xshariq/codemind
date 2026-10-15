pub fn is_test_declaration(line: &str) -> bool {
    line.trim_start().starts_with("def test_") || line.contains("unittest.TestCase")
}
