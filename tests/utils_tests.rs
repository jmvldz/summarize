#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use summarize::utils::should_ignore;

    #[test]
    fn test_should_ignore() {
        // Test ignoring a specific file
        let path = PathBuf::from("file.log");
        let ignore_patterns = vec!["*.log".to_string()];
        assert!(should_ignore(&path, &ignore_patterns, false));

        // Test ignoring a directory
        let path = PathBuf::from("node_modules");
        let ignore_patterns = vec!["node_modules/".to_string()];
        // For a directory, this would return true if the path is actually a directory
        // We need to mock a bit here since we're just passing a path string
        assert!(should_ignore(&path, &ignore_patterns, false) == false);

        // Test file extension
        let path = PathBuf::from("test.js");
        let ignore_patterns = vec!["*.js".to_string()];
        assert!(should_ignore(&path, &ignore_patterns, false));

        // Test empty patterns
        let path = PathBuf::from("file.txt");
        let ignore_patterns: Vec<String> = vec![];
        assert!(!should_ignore(&path, &ignore_patterns, false));
    }
}
