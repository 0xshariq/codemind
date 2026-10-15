pub fn is_test_declaration(line: &str) -> bool {
    line.contains("#[test]")
}
