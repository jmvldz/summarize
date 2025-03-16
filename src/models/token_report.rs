use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct TokenReport {
    pub file_tokens: HashMap<PathBuf, usize>,
    pub total_tokens: usize,
    // Duration in milliseconds
    pub duration_ms: u128,
}

impl TokenReport {
    pub fn new() -> Self {
        Self {
            file_tokens: HashMap::new(),
            total_tokens: 0,
            duration_ms: 0,
        }
    }

    pub fn add_file(&mut self, path: PathBuf, token_count: usize) {
        self.file_tokens.insert(path, token_count);
        self.total_tokens += token_count;
    }

    pub fn set_duration(&mut self, duration_ms: u128) {
        self.duration_ms = duration_ms;
    }
}
