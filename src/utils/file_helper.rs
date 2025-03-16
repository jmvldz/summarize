use anyhow::Result;
use atty;
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

pub fn build_globset(patterns: &[String]) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = Glob::new(pattern)?;
        builder.add(glob);
    }
    Ok(builder.build()?)
}

pub fn should_ignore(path: &Path, ignore_patterns: &[String], ignore_files_only: bool) -> bool {
    if ignore_patterns.is_empty() {
        return false;
    }

    // Build a GlobSet from patterns - any errors just cause pattern to be skipped
    let globset = match build_globset(ignore_patterns) {
        Ok(gs) => gs,
        Err(_) => return false,
    };

    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let name_str = name.to_string();

    if globset.is_match(&name_str) {
        return true;
    }

    if !ignore_files_only && path.is_dir() {
        let dir_name = format!("{}/", name);
        if globset.is_match(&dir_name) {
            return true;
        }
    }

    false
}

pub fn read_paths_from_stdin(use_null_separator: bool) -> Result<Vec<PathBuf>> {
    // Check if stdin is a terminal - if it is, don't try to read from it
    if atty::is(atty::Stream::Stdin) {
        return Ok(Vec::new());
    }

    let mut buffer = Vec::new();
    io::stdin().read_to_end(&mut buffer)?;

    let content = String::from_utf8_lossy(&buffer);

    let paths: Vec<PathBuf> = if use_null_separator {
        content
            .split('\0')
            .filter(|p| !p.is_empty())
            .map(PathBuf::from)
            .collect()
    } else {
        content
            .split_whitespace()
            .filter(|p| !p.is_empty())
            .map(PathBuf::from)
            .collect()
    };

    Ok(paths)
}
