#[cfg(test)]
mod tests {
    use summarize::formatters::add_line_numbers;

    #[test]
    fn test_add_line_numbers() {
        let content = "Line 1\nLine 2\nLine 3";
        let numbered = add_line_numbers(content);

        assert_eq!(numbered, "1  Line 1\n2  Line 2\n3  Line 3");

        // Test larger number padding
        let large_content = (1..=100)
            .map(|i| format!("Line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        let numbered_large = add_line_numbers(&large_content);

        // Check first few lines for proper padding
        let lines: Vec<&str> = numbered_large.lines().collect();
        assert_eq!(lines[0], "  1  Line 1");
        assert_eq!(lines[9], " 10  Line 10");
        assert_eq!(lines[99], "100  Line 100");
    }
}
