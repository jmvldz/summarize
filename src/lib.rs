use anyhow::Result;
use comfy_table::{ContentArrangement, Table};
use ignore::WalkBuilder;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
// Directly use tempfile::NamedTempFile instead of importing the crate
use thousands::Separable;

pub mod cli;
pub mod formatters;
pub mod llm;
pub mod models;
pub mod tokenizers;
pub mod utils;

use crate::formatters::{print_path, Writer};
use crate::models::{OutputFormat, TokenReport};
use crate::utils::should_ignore;

pub fn display_token_report(report: &TokenReport, cli: &cli::Cli) -> Result<()> {
    let model = &cli.tokenizer_model;

    if cli.verbose {
        let mut table = Table::new();
        table.set_content_arrangement(ContentArrangement::Dynamic);
        table.set_header(vec!["File", "Tokens"]);

        // Sort entries by path for consistent output
        let mut entries: Vec<_> = report.file_tokens.iter().collect();
        entries.sort_by(|a, b| a.0.cmp(b.0));

        for (path, tokens) in entries {
            table.add_row(vec![
                path.to_string_lossy().to_string(),
                tokens.separate_with_commas(),
            ]);
        }

        // Add total row
        table.add_row(vec![
            "TOTAL".to_string(),
            report.total_tokens.separate_with_commas(),
        ]);

        println!("{table}");
    } else {
        println!(
            "Total tokens: {}",
            report.total_tokens.separate_with_commas()
        );
    }

    println!("Files processed: {}", report.file_tokens.len());

    // Format the duration in a human-readable way
    if report.duration_ms > 0 {
        let seconds = report.duration_ms as f64 / 1000.0;
        let tokens_per_second = if seconds > 0.0 {
            (report.total_tokens as f64 / seconds).round() as usize
        } else {
            0
        };

        if seconds < 60.0 {
            println!(
                "Time taken: {:.2} seconds ({} tokens/sec)",
                seconds,
                tokens_per_second.separate_with_commas()
            );
        } else {
            let minutes = (seconds / 60.0).floor();
            let remaining_seconds = seconds - (minutes * 60.0);
            println!(
                "Time taken: {:.0} min {:.2} sec ({} tokens/sec)",
                minutes,
                remaining_seconds,
                tokens_per_second.separate_with_commas()
            );
        }
    }

    if cli.show_cost {
        let (input_cost_per_k, output_cost_per_k) =
            tokenizers::get_token_cost(model, report.total_tokens);
        let input_cost = (report.total_tokens as f64 / 1000.0) * input_cost_per_k;

        // Assume a typical response might be about 20% of the input size for cost estimation
        let estimated_output_tokens = (report.total_tokens as f64 * 0.2).round() as usize;
        let output_cost = (estimated_output_tokens as f64 / 1000.0) * output_cost_per_k;

        println!("\nEstimated cost ({:?}):", model);
        println!(
            "  Input: ${:.4} ({} tokens @ ${:.4}/1K tokens)",
            input_cost,
            report.total_tokens.separate_with_commas(),
            input_cost_per_k
        );
        println!(
            "  Output: ${:.4} (est. {} tokens @ ${:.4}/1K tokens)*",
            output_cost,
            estimated_output_tokens.separate_with_commas(),
            output_cost_per_k
        );
        println!("  Total: ${:.4}", input_cost + output_cost);
        println!("\n* Output tokens are estimated at 20% of input tokens");
    }

    Ok(())
}

pub fn process_path(
    path: &Path,
    cli: &cli::Cli,
    writer: &mut Writer,
    output_format: &OutputFormat,
) -> Result<()> {
    if path.is_file() {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                print_path(writer, path, &content, output_format, cli.line_numbers)?;
            }
            Err(_) => {
                // Skip this file silently
            }
        }
        return Ok(());
    }

    // Process a directory using WalkBuilder, which properly handles .gitignore files
    let mut builder = WalkBuilder::new(path);

    // Configure the builder based on CLI options
    builder.follow_links(true);

    // Control whether to respect .gitignore files
    builder.git_ignore(!cli.ignore_gitignore);
    builder.git_global(!cli.ignore_gitignore);

    // Handle hidden files
    builder.hidden(!cli.include_hidden);

    // Handle version control directories
    if cli.exclude_vcs && !cli.include_vcs {
        // Ignore .git directories
        if cli.ignore_gitignore {
            // The git_ignore setting already skips .git directories,
            // but if we've disabled git_ignore, we need to add it manually
            builder.filter_entry(|entry| {
                let path = entry.path();
                let file_name = path.file_name();
                if let Some(name) = file_name {
                    // Skip .git, .svn, and .hg directories
                    if name == ".git" || name == ".svn" || name == ".hg" {
                        return !entry.file_type().is_some_and(|ft| ft.is_dir());
                    }
                }
                true
            });
        }
    }

    // Process the entries using the walker
    let walker = builder.build();

    for result in walker {
        let entry = match result {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        let entry_path = entry.path();

        // Skip directories, we only want files
        if !entry
            .file_type()
            .unwrap_or_else(|| std::fs::metadata(entry_path).unwrap().file_type())
            .is_file()
        {
            continue;
        }

        // Check custom ignore patterns
        if should_ignore(entry_path, &cli.ignore_patterns, cli.ignore_files_only) {
            continue;
        }

        // Check extensions
        if !cli.extensions.is_empty() {
            let extension = entry_path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("");

            if !cli.extensions.iter().any(|ext| ext == extension) {
                continue;
            }
        }

        // Process file
        match std::fs::read_to_string(entry_path) {
            Ok(content) => {
                print_path(
                    writer,
                    entry_path,
                    &content,
                    output_format,
                    cli.line_numbers,
                )?;
            }
            Err(_) => {
                // Skip this file but continue processing others
            }
        }
    }

    Ok(())
}

pub fn collect_file_contents(
    paths: &[PathBuf],
    cli: &cli::Cli,
    output_format: &OutputFormat,
) -> Result<String> {
    // Create a temporary file to store the output
    let temp_file = tempfile::NamedTempFile::new()?;
    let temp_path = temp_file.path().to_path_buf();

    // Create a writer that writes to our temp file
    let mut writer = Writer::new(Some(temp_path.clone()))?;

    // Start XML document if needed
    if matches!(output_format, OutputFormat::Cxml) {
        writer.write("<documents>")?;
    }

    // Process each path
    for path in paths {
        process_path(path, cli, &mut writer, output_format)?;
    }

    // End XML document if needed
    if matches!(output_format, OutputFormat::Cxml) {
        writer.write("</documents>")?;
    }

    // Read the file contents
    let content = std::fs::read_to_string(temp_path)?;

    Ok(content)
}

pub fn process_token_count(cli: &cli::Cli) -> Result<()> {
    // Initialize token report
    let mut report = TokenReport::new();

    // Configure thread pool if specified
    if cli.num_threads > 0 {
        rayon::ThreadPoolBuilder::new()
            .num_threads(cli.num_threads)
            .build_global()
            .unwrap_or_else(|e| {
                eprintln!("Warning: Failed to configure thread pool: {}", e);
                eprintln!("Using default thread pool configuration");
            });
        println!("Using {} threads for token counting", cli.num_threads);
    } else {
        println!("Using all available CPU cores for token counting");
    }

    // Start the timer for token counting
    let start_time = Instant::now();

    // Process files in parallel using a thread-safe approach
    let paths = cli.paths.clone();
    let cli_arc = Arc::new(cli.clone());

    // Create a thread-safe collection to hold results
    let shared_results: Arc<Mutex<HashMap<PathBuf, usize>>> = Arc::new(Mutex::new(HashMap::new()));

    // Initial discovery phase - collect all files to process
    let mut all_files = Vec::new();

    println!("Discovering files to process...");
    let discovery_progress = ProgressBar::new_spinner();
    discovery_progress.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    discovery_progress.set_message("Scanning directories...");

    for path in &paths {
        if path.is_file() {
            all_files.push(path.clone());
            discovery_progress.set_message(format!("Found {} files", all_files.len()));
            continue;
        }

        // Process a directory using WalkBuilder, which properly handles .gitignore files
        let mut builder = WalkBuilder::new(path);

        // Configure the builder based on CLI options
        builder.follow_links(true);

        // Control whether to respect .gitignore files
        builder.git_ignore(!cli_arc.ignore_gitignore);
        builder.git_global(!cli_arc.ignore_gitignore);

        // Handle hidden files
        builder.hidden(!cli_arc.include_hidden);

        // Handle version control directories
        if cli_arc.exclude_vcs && !cli_arc.include_vcs {
            // Ignore .git directories
            if !cli_arc.ignore_gitignore {
                // The git_ignore setting already skips .git directories,
                // but if we've disabled git_ignore, we need to add it manually
                builder.filter_entry(|entry| {
                    let path = entry.path();
                    let file_name = path.file_name();
                    if let Some(name) = file_name {
                        // Skip .git, .svn, and .hg directories
                        if name == ".git" || name == ".svn" || name == ".hg" {
                            return !entry.file_type().is_some_and(|ft| ft.is_dir());
                        }
                    }
                    true
                });
            }
        }

        // Process the entries using the walker
        let walker = builder.build();

        for result in walker {
            match result {
                Ok(entry) => {
                    let entry_path = entry.path();
                    discovery_progress.set_message(format!(
                        "Scanning: {} (found {} files)",
                        entry_path.display(),
                        all_files.len()
                    ));

                    // Skip directories, we only want files
                    if !entry
                        .file_type()
                        .unwrap_or_else(|| std::fs::metadata(entry_path).unwrap().file_type())
                        .is_file()
                    {
                        continue;
                    }

                    // Check custom ignore patterns
                    if should_ignore(
                        entry_path,
                        &cli_arc.ignore_patterns,
                        cli_arc.ignore_files_only,
                    ) {
                        continue;
                    }

                    // Check extensions
                    if !cli_arc.extensions.is_empty() {
                        let extension = entry_path
                            .extension()
                            .and_then(|ext| ext.to_str())
                            .unwrap_or("");

                        if !cli_arc.extensions.iter().any(|ext| ext == extension) {
                            continue;
                        }
                    }

                    // Add valid file to our collection
                    all_files.push(entry_path.to_path_buf());

                    // Update progress message
                    if all_files.len() % 100 == 0 {
                        discovery_progress.set_message(format!("Found {} files", all_files.len()));
                    }
                }
                Err(_) => continue,
            }
        }
    }

    discovery_progress.finish_with_message(format!("Found {} files to process", all_files.len()));

    // Process all files with a progress bar
    println!("Counting tokens in {} files...", all_files.len());
    let progress = ProgressBar::new(all_files.len() as u64);
    progress.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} files ({percent}%) {msg}")
        .unwrap()
        .progress_chars("#>-"));

    // Use a counter to track total tokens
    let token_counter = Arc::new(Mutex::new(0usize));

    // Process each file in parallel
    all_files.par_iter().for_each(|file_path| {
        match std::fs::read_to_string(file_path) {
            Ok(content) => {
                let token_count = tokenizers::count_tokens(&content, &cli_arc.tokenizer_model);

                // Update the shared results
                if let Ok(mut results) = shared_results.lock() {
                    results.insert(file_path.clone(), token_count);
                }

                // Update token counter and progress
                if let Ok(mut counter) = token_counter.lock() {
                    *counter += token_count;
                    progress.set_message(format!("{} tokens", counter.separate_with_commas()));
                }

                progress.inc(1);
            }
            Err(_) => {
                // Skip this file silently but still update progress
                progress.inc(1);
            }
        }
    });

    progress.finish_with_message(format!("Processed {} files", all_files.len()));

    // Add all results to the report
    let final_results = Arc::try_unwrap(shared_results)
        .expect("Failed to retrieve results")
        .into_inner()
        .expect("Failed to unlock results");

    for (path, token_count) in final_results {
        report.add_file(path, token_count);
    }

    // Calculate and store the duration
    let duration = start_time.elapsed();
    report.set_duration(duration.as_millis());

    // Display token counting results
    display_token_report(&report, cli)?;

    Ok(())
}
