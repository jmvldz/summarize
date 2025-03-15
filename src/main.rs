use anyhow::{anyhow, Result};
use clap::{Parser, ValueEnum};
use comfy_table::{ContentArrangement, Table};
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use thousands::Separable;
use tiktoken_rs::p50k_base;
// use walkdir::WalkDir; // Removed unused import

#[derive(Debug, Default)]
struct TokenReport {
    file_tokens: HashMap<PathBuf, usize>,
    total_tokens: usize,
    // Duration in milliseconds
    duration_ms: u128,
}

impl TokenReport {
    fn new() -> Self {
        Self {
            file_tokens: HashMap::new(),
            total_tokens: 0,
            duration_ms: 0,
        }
    }

    fn add_file(&mut self, path: PathBuf, token_count: usize) {
        self.file_tokens.insert(path, token_count);
        self.total_tokens += token_count;
    }
    
    fn set_duration(&mut self, duration_ms: u128) {
        self.duration_ms = duration_ms;
    }
}

#[derive(Debug, Clone, ValueEnum)]
enum OutputFormat {
    Default,
    Cxml,
    Markdown,
}

#[derive(Debug, Clone, ValueEnum)]
enum TokenizerModel {
    Gemini15Pro,
    Gemini15Flash,
    Gemini20Flash,
    Gemini20FlashLite,
    Gemini20Pro,
    Gemini20ProExp,
    Gemini20ProExp0205,
    Gemini20FlashThinkingExp,
    Gpt35Turbo,
    Gpt4,
    Gpt4Turbo,
    Claude3Sonnet,
    Claude3Opus,
}

impl fmt::Display for TokenizerModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenizerModel::Gemini15Pro => write!(f, "Gemini 1.5 Pro"),
            TokenizerModel::Gemini15Flash => write!(f, "Gemini 1.5 Flash"),
            TokenizerModel::Gemini20Flash => write!(f, "Gemini 2.0 Flash"),
            TokenizerModel::Gemini20FlashLite => write!(f, "Gemini 2.0 Flash-Lite"),
            TokenizerModel::Gemini20Pro => write!(f, "Gemini 2.0 Pro"),
            TokenizerModel::Gemini20ProExp => write!(f, "Gemini 2.0 Pro Exp 02-05"),
            TokenizerModel::Gemini20ProExp0205 => write!(f, "Gemini 2.0 Pro Exp 02-05"),
            TokenizerModel::Gemini20FlashThinkingExp => write!(f, "Gemini 2.0 Flash Thinking Exp"),
            TokenizerModel::Gpt35Turbo => write!(f, "GPT-3.5 Turbo"),
            TokenizerModel::Gpt4 => write!(f, "GPT-4"),
            TokenizerModel::Gpt4Turbo => write!(f, "GPT-4 Turbo"),
            TokenizerModel::Claude3Sonnet => write!(f, "Claude 3 Sonnet"),
            TokenizerModel::Claude3Opus => write!(f, "Claude 3 Opus"),
        }
    }
}

#[derive(Parser, Debug, Clone)]
#[command(
    name = "summarize",
    about = "Concatenate a directory full of files into a single prompt for use with LLMs",
    author = "Original by Simon Willison, Rust port by Claude",
    version
)]
struct Cli {
    /// Paths to files or directories to process
    #[arg(required = false)]
    paths: Vec<PathBuf>,

    /// Only include files with the specified extension(s)
    #[arg(short = 'e', long = "extension")]
    extensions: Vec<String>,

    /// Include files and folders starting with . (hidden files and directories)
    #[arg(long = "include-hidden")]
    include_hidden: bool,

    /// --ignore option only ignores files
    #[arg(long = "ignore-files-only")]
    ignore_files_only: bool,

    /// Ignore .gitignore files and include all files
    #[arg(long = "ignore-gitignore")]
    ignore_gitignore: bool,
    
    /// Exclude version control directories (.git, .svn, .hg)
    #[arg(long = "exclude-vcs", default_value_t = true)]
    exclude_vcs: bool,
    
    /// Include version control directories
    #[arg(long = "include-vcs", default_value_t = false)]
    include_vcs: bool,

    /// List of patterns to ignore
    #[arg(long = "ignore")]
    ignore_patterns: Vec<String>,

    /// Output to a file instead of stdout
    #[arg(short = 'o', long = "output")]
    output_file: Option<PathBuf>,

    /// Output format
    #[arg(
        short = 'f',
        long = "format",
        value_enum,
        default_value_t = OutputFormat::Default
    )]
    output_format: OutputFormat,

    /// Output in Claude XML format
    #[arg(short = 'c', long = "cxml", conflicts_with = "output_format")]
    cxml: bool,

    /// Output Markdown with fenced code blocks
    #[arg(short = 'm', long = "markdown", conflicts_with = "output_format")]
    markdown: bool,

    /// Add line numbers to the output
    #[arg(short = 'n', long = "line-numbers")]
    line_numbers: bool,

    /// Use NUL character as separator when reading from stdin
    #[arg(short = '0', long = "null")]
    null: bool,

    /// Count tokens instead of outputting content
    #[arg(short = 't', long = "count-tokens")]
    count_tokens: bool,

    /// Tokenization model to use for counting or summarization
    #[arg(
        long = "model",
        value_enum,
        default_value_t = TokenizerModel::Gemini15Flash,
        requires = "count_tokens"
    )]
    tokenizer_model: TokenizerModel,
    
    /// API key for the LLM service
    #[arg(long = "api-key")]
    api_key: Option<String>,
    
    /// Use API key from environment variable
    #[arg(long = "api-key-env", conflicts_with = "api_key")]
    api_key_env: Option<String>,

    /// Show per-file token counts
    #[arg(long = "verbose", requires = "count_tokens")]
    verbose: bool,

    /// Show estimated API costs
    #[arg(long = "show-cost", requires = "count_tokens")]
    show_cost: bool,
    
    /// Only concatenate files without generating a summary
    #[arg(long = "no-summarize")]
    no_summarize: bool,
    
    /// Custom prompt to use when generating a summary
    #[arg(
        long = "prompt", 
        default_value = "You are a senior software engineer reviewing a codebase. Generate a comprehensive overview.md file that explains the purpose, structure, and key components of this codebase. Focus on helping a new developer understand how the codebase is organized and how different parts work together."
    )]
    custom_prompt: String,
    
    /// Output file for the summary
    #[arg(long = "summary-output", default_value = "overview.md")]
    summary_output: PathBuf,
    
    /// List available models from the LLM service
    #[arg(long = "list-models")]
    list_models: bool,
    
    /// Number of threads to use for token counting (0 = use all available cores)
    #[arg(long = "threads", default_value = "0")]
    num_threads: usize,
}

// Maps file extensions to language names for markdown formatting
lazy_static::lazy_static! {
    static ref EXT_TO_LANG: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        m.insert("py", "python");
        m.insert("c", "c");
        m.insert("cpp", "cpp");
        m.insert("h", "c");
        m.insert("hpp", "cpp");
        m.insert("java", "java");
        m.insert("js", "javascript");
        m.insert("ts", "typescript");
        m.insert("html", "html");
        m.insert("css", "css");
        m.insert("xml", "xml");
        m.insert("json", "json");
        m.insert("yaml", "yaml");
        m.insert("yml", "yaml");
        m.insert("sh", "bash");
        m.insert("rb", "ruby");
        m.insert("rs", "rust");
        m.insert("go", "go");
        m.insert("md", "markdown");
        m.insert("toml", "toml");
        m
    };
}

struct Writer {
    file: Option<File>,
    document_index: usize,
}

impl Writer {
    fn new(path: Option<PathBuf>) -> Result<Self> {
        let file = match path {
            Some(p) => Some(File::create(p)?),
            None => None,
        };
        Ok(Self {
            file,
            document_index: 1,
        })
    }

    fn write(&mut self, content: &str) -> Result<()> {
        match &mut self.file {
            Some(f) => {
                writeln!(f, "{}", content)?;
                Ok(())
            }
            None => {
                println!("{}", content);
                Ok(())
            }
        }
    }
}

fn build_globset(patterns: &[String]) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = Glob::new(pattern)?;
        builder.add(glob);
    }
    Ok(builder.build()?)
}

fn should_ignore(path: &Path, ignore_patterns: &[String], ignore_files_only: bool) -> bool {
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

fn add_line_numbers(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let padding = lines.len().to_string().len();

    lines
        .iter()
        .enumerate()
        .map(|(i, line)| format!("{:padding$}  {}", i + 1, line, padding = padding))
        .collect::<Vec<String>>()
        .join("\n")
}

fn print_path(
    writer: &mut Writer,
    path: &Path,
    content: &str,
    format: &OutputFormat,
    line_numbers: bool,
) -> Result<()> {
    match format {
        OutputFormat::Cxml => print_as_xml(writer, path, content, line_numbers),
        OutputFormat::Markdown => print_as_markdown(writer, path, content, line_numbers),
        OutputFormat::Default => print_default(writer, path, content, line_numbers),
    }
}

fn print_default(
    writer: &mut Writer,
    path: &Path,
    content: &str,
    line_numbers: bool,
) -> Result<()> {
    writer.write(&path.to_string_lossy())?;
    writer.write("---")?;
    
    let content_to_write = if line_numbers {
        add_line_numbers(content)
    } else {
        content.to_string()
    };
    
    writer.write(&content_to_write)?;
    writer.write("")?;
    writer.write("---")?;
    Ok(())
}

fn print_as_xml(writer: &mut Writer, path: &Path, content: &str, line_numbers: bool) -> Result<()> {
    writer.write(&format!(
        r#"<document index="{}">"#,
        writer.document_index
    ))?;
    writer.write(&format!(r#"<source>{}</source>"#, path.to_string_lossy()))?;
    writer.write("<document_content>")?;
    
    let content_to_write = if line_numbers {
        add_line_numbers(content)
    } else {
        content.to_string()
    };
    
    writer.write(&content_to_write)?;
    writer.write("</document_content>")?;
    writer.write("</document>")?;
    
    writer.document_index += 1;
    Ok(())
}

fn print_as_markdown(
    writer: &mut Writer,
    path: &Path,
    content: &str,
    line_numbers: bool,
) -> Result<()> {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");
    
    let lang = EXT_TO_LANG.get(extension).copied().unwrap_or("");
    
    // Figure out how many backticks to use
    let mut backticks = "```".to_string();
    while content.contains(&backticks) {
        backticks.push('`');
    }
    
    writer.write(&path.to_string_lossy())?;
    writer.write(&format!("{}{}", backticks, lang))?;
    
    let content_to_write = if line_numbers {
        add_line_numbers(content)
    } else {
        content.to_string()
    };
    
    writer.write(&content_to_write)?;
    writer.write(&backticks)?;
    Ok(())
}

fn read_paths_from_stdin(use_null_separator: bool) -> Result<Vec<PathBuf>> {
    // Check if stdin is a terminal - if it is, don't try to read from it
    if atty::is(atty::Stream::Stdin) {
        return Ok(Vec::new());
    }

    let mut buffer = Vec::new();
    io::stdin().read_to_end(&mut buffer)?;
    
    let content = String::from_utf8_lossy(&buffer);
    
    let paths: Vec<PathBuf> = if use_null_separator {
        content.split('\0')
            .filter(|p| !p.is_empty())
            .map(PathBuf::from)
            .collect()
    } else {
        content.split_whitespace()
            .filter(|p| !p.is_empty())
            .map(PathBuf::from)
            .collect()
    };
    
    Ok(paths)
}

// This function is no longer used - we use WalkBuilder directly
// which handles gitignore files properly

fn get_tokenizer_name(model: &TokenizerModel) -> &'static str {
    match model {
        TokenizerModel::Gemini15Pro => "cl100k_base",    // Approximate with cl100k_base
        TokenizerModel::Gemini15Flash => "cl100k_base",  // Approximate with cl100k_base
        TokenizerModel::Gemini20Flash => "cl100k_base",  // Approximate with cl100k_base
        TokenizerModel::Gemini20FlashLite => "cl100k_base", // Approximate with cl100k_base
        TokenizerModel::Gemini20Pro => "cl100k_base",    // Approximate with cl100k_base
        TokenizerModel::Gemini20ProExp => "cl100k_base", // Approximate with cl100k_base
        TokenizerModel::Gemini20ProExp0205 => "cl100k_base", // Approximate with cl100k_base
        TokenizerModel::Gemini20FlashThinkingExp => "cl100k_base", // Approximate with cl100k_base
        TokenizerModel::Gpt35Turbo => "cl100k_base",     // GPT-3.5-Turbo uses cl100k_base
        TokenizerModel::Gpt4 => "cl100k_base",           // GPT-4 uses cl100k_base
        TokenizerModel::Gpt4Turbo => "cl100k_base",      // GPT-4-Turbo uses cl100k_base
        TokenizerModel::Claude3Sonnet => "p50k_base",    // Approximate with p50k_base
        TokenizerModel::Claude3Opus => "p50k_base",      // Approximate with p50k_base
    }
}

fn get_token_cost(model: &TokenizerModel, _tokens: usize) -> (f64, f64) {
    // (input_cost_per_1k, output_cost_per_1k)
    match model {
        TokenizerModel::Gemini15Pro => (0.0000, 0.0000),  // Estimated
        TokenizerModel::Gemini15Flash => (0.0000, 0.0000),  // Estimated
        TokenizerModel::Gemini20Flash => (0.0000, 0.0000),  // Currently free during preview
        TokenizerModel::Gemini20FlashLite => (0.0000, 0.0000),  // Currently free during preview
        TokenizerModel::Gemini20Pro => (0.0000, 0.0000),  // Currently free during preview
        TokenizerModel::Gemini20ProExp => (0.0000, 0.0000),  // Currently free during preview (experimental)
        TokenizerModel::Gemini20ProExp0205 => (0.0000, 0.0000),  // Currently free during preview (experimental)
        TokenizerModel::Gemini20FlashThinkingExp => (0.0000, 0.0000),  // Currently free during preview (experimental)
        TokenizerModel::Gpt35Turbo => (0.0010, 0.0020),  // $0.0010 per 1k input, $0.0020 per 1k output
        TokenizerModel::Gpt4 => (0.03, 0.06),            // $0.03 per 1k input, $0.06 per 1k output
        TokenizerModel::Gpt4Turbo => (0.01, 0.03),       // $0.01 per 1k input, $0.03 per 1k output
        TokenizerModel::Claude3Sonnet => (0.003, 0.015), // $0.003 per 1k input, $0.015 per 1k output
        TokenizerModel::Claude3Opus => (0.015, 0.075),   // $0.015 per 1k input, $0.075 per 1k output
    }
}

fn count_tokens(text: &str, model: &TokenizerModel) -> usize {
    // Currently we're using tiktoken for all models but in a real-world implementation
    // we'd use different tokenizers for each model family
    match get_tokenizer_name(model) {
        "cl100k_base" => {
            // We use the full path instead of the import to avoid unused import warnings
            let bpe = tiktoken_rs::cl100k_base().unwrap();
            bpe.encode_ordinary(text).len()
        }
        "p50k_base" => {
            let bpe = p50k_base().unwrap();
            bpe.encode_ordinary(text).len()
        }
        _ => {
            // Fallback to p50k_base
            let bpe = p50k_base().unwrap();
            bpe.encode_ordinary(text).len()
        }
    }
}

fn display_token_report(report: &TokenReport, cli: &Cli) -> Result<()> {
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
        println!("Total tokens: {}", report.total_tokens.separate_with_commas());
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
            println!("Time taken: {:.2} seconds ({} tokens/sec)", 
                seconds, 
                tokens_per_second.separate_with_commas());
        } else {
            let minutes = (seconds / 60.0).floor();
            let remaining_seconds = seconds - (minutes * 60.0);
            println!("Time taken: {:.0} min {:.2} sec ({} tokens/sec)", 
                minutes, 
                remaining_seconds,
                tokens_per_second.separate_with_commas());
        }
    }
    
    if cli.show_cost {
        let (input_cost_per_k, output_cost_per_k) = get_token_cost(model, report.total_tokens);
        let input_cost = (report.total_tokens as f64 / 1000.0) * input_cost_per_k;
        
        // Assume a typical response might be about 20% of the input size for cost estimation
        let estimated_output_tokens = (report.total_tokens as f64 * 0.2).round() as usize;
        let output_cost = (estimated_output_tokens as f64 / 1000.0) * output_cost_per_k;
        
        println!("\nEstimated cost ({:?}):", model);
        println!("  Input: ${:.4} ({} tokens @ ${:.4}/1K tokens)", 
            input_cost, 
            report.total_tokens.separate_with_commas(),
            input_cost_per_k
        );
        println!("  Output: ${:.4} (est. {} tokens @ ${:.4}/1K tokens)*", 
            output_cost, 
            estimated_output_tokens.separate_with_commas(),
            output_cost_per_k
        );
        println!("  Total: ${:.4}", input_cost + output_cost);
        println!("\n* Output tokens are estimated at 20% of input tokens");
    }
    
    Ok(())
}

#[allow(dead_code)]
fn process_path_count_tokens(
    path: &Path,
    cli: &Cli,
    report: &mut TokenReport,
) -> Result<()> {
    if path.is_file() {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                let token_count = count_tokens(&content, &cli.tokenizer_model);
                report.add_file(path.to_path_buf(), token_count);
            }
            Err(_) => {
                // Just skip this file but continue with the process
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
                        return !entry.file_type().map_or(false, |ft| ft.is_dir());
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
        if !entry.file_type().map_or(false, |ft| ft.is_file()) {
            continue;
        }
        
        // Gitignore rules are now handled by WalkBuilder
        
        // Check ignore patterns
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
                let token_count = count_tokens(&content, &cli.tokenizer_model);
                report.add_file(entry_path.to_path_buf(), token_count);
            }
            Err(_) => {
                // Skip this file but continue processing others
            }
        }
    }
    
    // Processing is complete
    
    Ok(())
}

fn process_path(
    path: &Path,
    cli: &Cli,
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
                        return !entry.file_type().map_or(false, |ft| ft.is_dir());
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
        if !entry.file_type().unwrap_or_else(|| std::fs::metadata(entry_path).unwrap().file_type()).is_file() {
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
                print_path(writer, entry_path, &content, output_format, cli.line_numbers)?;
            }
            Err(_) => {
                // Skip this file but continue processing others
            }
        }
    }
    
    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
struct GeminiMessage {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Serialize, Deserialize, Debug)]
struct GeminiPart {
    text: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct GeminiRequest {
    contents: Vec<GeminiMessage>,
    generation_config: GeminiConfig,
}

#[derive(Serialize, Deserialize, Debug)]
struct GeminiConfig {
    temperature: f32,
    top_p: f32,
    top_k: u32,
    max_output_tokens: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Serialize, Deserialize, Debug)]
struct GeminiCandidate {
    content: GeminiContent,
}

#[derive(Serialize, Deserialize, Debug)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Serialize, Deserialize, Debug)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Serialize, Deserialize, Debug)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

#[derive(Serialize, Deserialize, Debug)]
struct AnthropicMessage {
    role: String,
    content: Vec<AnthropicContent>,
}

#[derive(Serialize, Deserialize, Debug)]
struct AnthropicContent {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Serialize, Deserialize, Debug)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
}

#[derive(Serialize, Deserialize, Debug)]
struct GeminiListModelsResponse {
    models: Vec<GeminiModel>,
}

#[derive(Serialize, Deserialize, Debug)]
struct GeminiModel {
    name: String,
    version: String,
    #[serde(rename = "displayName")]
    display_name: String,
    description: String,
    #[serde(rename = "inputTokenLimit")]
    input_token_limit: Option<u32>,
    #[serde(rename = "outputTokenLimit")]
    output_token_limit: Option<u32>,
    #[serde(rename = "supportedGenerationMethods")]
    supported_generation_methods: Option<Vec<String>>,
}

fn list_gemini_models(api_key: &str) -> Result<()> {
    let client = Client::new();
    
    // First, get standard models
    let standard_url = format!(
        "https://generativelanguage.googleapis.com/v1/models?key={}",
        api_key
    );
    
    let standard_response_text = client
        .get(&standard_url)
        .send()?
        .text()?;
    
    // Also try to get experimental models
    let experimental_url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models?key={}",
        api_key
    );
    
    let experimental_response_text = client
        .get(&experimental_url)
        .send()
        .map_or_else(|_| "".to_string(), |resp| resp.text().unwrap_or_default());
        
    println!("Available Gemini Models:");
    
    // Parse standard models
    let standard_models = match serde_json::from_str::<GeminiListModelsResponse>(&standard_response_text) {
        Ok(response) => response.models,
        Err(e) => {
            println!("Error parsing standard models API response: {}", e);
            println!("Raw API response: {}", standard_response_text);
            vec![] // Return empty vector to continue with experimental models
        }
    };
    
    // Parse experimental models
    let experimental_models = if !experimental_response_text.is_empty() {
        match serde_json::from_str::<GeminiListModelsResponse>(&experimental_response_text) {
            Ok(response) => response.models,
            Err(e) => {
                println!("Error parsing experimental models API response: {}", e);
                vec![] // Return empty vector
            }
        }
    } else {
        vec![]
    };
    
    // Create a table for better formatting
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec!["Model Name", "Type", "Display Name", "Description", "Supported Methods"]);
    
    // Collect the names of standard models for later comparison
    let standard_model_names: Vec<String> = standard_models
        .iter()
        .map(|m| m.name.split('/').last().unwrap_or(&m.name).to_string())
        .collect();
    
    // Add standard models to table
    for model in &standard_models {
        // Extract the model name without the full path
        let short_name = model.name.split('/').last().unwrap_or(&model.name);
        
        // Check if this is likely an experimental model
        let model_type = if short_name.contains("exp") {
            "Experimental"
        } else {
            "Standard"
        };
        
        // Format supported methods
        let methods = match &model.supported_generation_methods {
            Some(methods) => methods.join(", "),
            None => "N/A".to_string(),
        };
        
        table.add_row(vec![
            short_name.to_string(),
            model_type.to_string(),
            model.display_name.clone(),
            model.description.clone(),
            methods,
        ]);
    }
    
    // Add experimental models to table
    for model in &experimental_models {
        // Extract the model name without the full path
        let short_name = model.name.split('/').last().unwrap_or(&model.name);
        
        // Skip if this model already appears in the standard models list
        if standard_model_names.iter().any(|name| name == short_name) {
            continue;
        }
        
        // Format supported methods
        let methods = match &model.supported_generation_methods {
            Some(methods) => methods.join(", "),
            None => "N/A".to_string(),
        };
        
        table.add_row(vec![
            short_name.to_string(),
            "Experimental".to_string(),
            model.display_name.clone(),
            model.description.clone(),
            methods,
        ]);
    }
    
    println!("{table}");
    
    // Provide information about using experimental models
    println!("\nNote about experimental models:");
    println!("- Experimental models contain 'exp' in their names (e.g. gemini-2.0-pro-exp-02-05)");
    println!("- These models may change or be removed without notice");
    println!("- They are not recommended for production use");
    println!("- Use the --model flag to specify an experimental model for summarization");
    
    Ok(())
}

fn get_api_key(cli: &Cli) -> Option<String> {
    if let Some(key) = &cli.api_key {
        return Some(key.clone());
    }
    
    if let Some(env_var) = &cli.api_key_env {
        return std::env::var(env_var).ok();
    }
    
    // Try common environment variables for different providers
    match cli.tokenizer_model {
        TokenizerModel::Gemini15Pro | 
        TokenizerModel::Gemini15Flash |
        TokenizerModel::Gemini20Flash |
        TokenizerModel::Gemini20FlashLite |
        TokenizerModel::Gemini20Pro |
        TokenizerModel::Gemini20ProExp |
        TokenizerModel::Gemini20ProExp0205 |
        TokenizerModel::Gemini20FlashThinkingExp => std::env::var("GOOGLE_API_KEY").ok(),
        TokenizerModel::Gpt35Turbo | 
        TokenizerModel::Gpt4 |
        TokenizerModel::Gpt4Turbo => std::env::var("OPENAI_API_KEY").ok(),
        TokenizerModel::Claude3Sonnet |
        TokenizerModel::Claude3Opus => std::env::var("ANTHROPIC_API_KEY").ok(),
    }
}

fn summarize_with_llm(code_content: &str, prompt: &str, model: &TokenizerModel, api_key: &str) -> Result<String> {
    println!("Attempting to summarize with model: {}", model);
    
    match model {
        TokenizerModel::Gemini15Pro => {
            println!("Using Gemini 1.5 Pro model");
            summarize_with_gemini(code_content, prompt, "gemini-1.5-pro", api_key)
        },
        TokenizerModel::Gemini15Flash => {
            println!("Using Gemini 1.5 Flash model");
            summarize_with_gemini(code_content, prompt, "gemini-1.5-flash", api_key)
        },
        TokenizerModel::Gemini20Flash => {
            println!("Using Gemini 2.0 Flash model");
            summarize_with_gemini(code_content, prompt, "gemini-2.0-flash", api_key)
        },
        TokenizerModel::Gemini20FlashLite => {
            println!("Using Gemini 2.0 Flash-Lite model");
            summarize_with_gemini(code_content, prompt, "gemini-2.0-flash-lite", api_key)
        },
        TokenizerModel::Gemini20Pro => {
            println!("Using Gemini 2.0 Pro model");
            summarize_with_gemini(code_content, prompt, "gemini-2.0-pro", api_key)
        },
        TokenizerModel::Gemini20ProExp => {
            println!("Using Gemini 2.0 Pro Exp 02-05 model");
            summarize_with_gemini(code_content, prompt, "gemini-2.0-pro-exp-02-05", api_key)
        },
        TokenizerModel::Gemini20ProExp0205 => {
            println!("Using Gemini 2.0 Pro Exp 02-05 model");
            summarize_with_gemini(code_content, prompt, "gemini-2.0-pro-exp-02-05", api_key)
        },
        TokenizerModel::Gemini20FlashThinkingExp => {
            println!("Using Gemini 2.0 Flash Thinking Exp model");
            summarize_with_gemini(code_content, prompt, "gemini-2.0-flash-thinking-exp", api_key)
        },
        TokenizerModel::Gpt35Turbo | TokenizerModel::Gpt4 | TokenizerModel::Gpt4Turbo => {
            let model_name = match model {
                TokenizerModel::Gpt35Turbo => "gpt-3.5-turbo",
                TokenizerModel::Gpt4 => "gpt-4",
                TokenizerModel::Gpt4Turbo => "gpt-4-turbo",
                _ => unreachable!(),
            };
            println!("Using OpenAI model: {}", model_name);
            summarize_with_openai(code_content, prompt, model_name, api_key)
        },
        TokenizerModel::Claude3Sonnet | TokenizerModel::Claude3Opus => {
            let model_name = match model {
                TokenizerModel::Claude3Sonnet => "claude-3-sonnet-20240229",
                TokenizerModel::Claude3Opus => "claude-3-opus-20240229",
                _ => unreachable!(),
            };
            println!("Using Anthropic model: {}", model_name);
            summarize_with_anthropic(code_content, prompt, model_name, api_key)
        }
    }
}

fn summarize_with_gemini(code_content: &str, prompt: &str, model_name: &str, api_key: &str) -> Result<String> {
    let client = Client::new();
    
    let full_prompt = format!("{}\n\nHere's the codebase:\n\n{}", prompt, code_content);
    
    let request = GeminiRequest {
        contents: vec![GeminiMessage {
            role: "user".to_string(),
            parts: vec![GeminiPart { text: full_prompt }],
        }],
        generation_config: GeminiConfig {
            temperature: 0.7,
            top_p: 0.95,
            top_k: 40,
            max_output_tokens: 8192,
        },
    };
    
    // For experimental models, use v1beta endpoint, otherwise use v1
    // Handle both model formats: full path (models/...) or just model name
    let api_version = if model_name.contains("exp") { "v1beta" } else { "v1" };
    
    // Check if model_name already contains "models/" prefix
    let model_path = if model_name.starts_with("models/") {
        // Extract just the model name part after "models/"
        model_name.split('/').skip(1).collect::<Vec<&str>>().join("/")
    } else {
        model_name.to_string()
    };
    
    let url = format!(
        "https://generativelanguage.googleapis.com/{}/models/{}:generateContent?key={}",
        api_version, model_path, api_key
    );
    
    println!("Using API URL: {}", url.replace(api_key, "[REDACTED]"));
    
    // Send the request with timeout and error handling
    let response = match client.post(&url).json(&request).send() {
        Ok(resp) => resp,
        Err(e) => {
            return Err(anyhow!("Error sending request to Gemini API: {}", e));
        }
    };
    
    // Check status code first
    if !response.status().is_success() {
        println!("API Error - Status Code: {}", response.status());
    }
    
    // Get response text with error handling
    let response_text = match response.text() {
        Ok(text) => text,
        Err(e) => {
            return Err(anyhow!("Error reading Gemini API response: {}", e));
        }
    };
    
    // Print response for debugging in case of errors
    if response_text.is_empty() {
        println!("API Response: [Empty response]");
    } else {
        println!("API Response: {}", &response_text);
    }
    
    // Check if response contains an error about model not found
    if response_text.contains("NOT_FOUND") && response_text.contains("is not found") {
        // Handle model not found case specifically
        println!("\nError: The specified model '{}' was not found.", model_name);
        println!("To see a list of available models, run:");
        println!("  summarize --list-models --api-key YOUR_API_KEY");
        return Err(anyhow!("Model not found: {}", model_name));
    }
    
    let response: GeminiResponse = match serde_json::from_str(&response_text) {
        Ok(resp) => resp,
        Err(e) => {
            return Err(anyhow!("Error parsing Gemini API response: {}. Response: {}", e, response_text));
        }
    };
    
    if response.candidates.is_empty() || response.candidates[0].content.parts.is_empty() {
        return Err(anyhow!("No response content from Gemini API"));
    }
    
    Ok(response.candidates[0].content.parts[0].text.clone())
}

fn summarize_with_openai(code_content: &str, prompt: &str, model: &str, api_key: &str) -> Result<String> {
    let client = Client::new();
    
    let request = OpenAIRequest {
        model: model.to_string(),
        messages: vec![
            OpenAIMessage {
                role: "system".to_string(),
                content: prompt.to_string(),
            },
            OpenAIMessage {
                role: "user".to_string(),
                content: code_content.to_string(),
            },
        ],
        temperature: 0.7,
        max_tokens: 4096,
    };
    
    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .send()?
        .json::<OpenAIResponse>()?;
    
    if response.choices.is_empty() {
        return Err(anyhow!("No response content from OpenAI API"));
    }
    
    Ok(response.choices[0].message.content.clone())
}

fn summarize_with_anthropic(code_content: &str, prompt: &str, model: &str, api_key: &str) -> Result<String> {
    let client = Client::new();
    
    let request = AnthropicRequest {
        model: model.to_string(),
        messages: vec![
            AnthropicMessage {
                role: "user".to_string(),
                content: vec![
                    AnthropicContent {
                        content_type: "text".to_string(),
                        text: format!("{}\n\nHere's the codebase:\n\n{}", prompt, code_content),
                    },
                ],
            },
        ],
        max_tokens: 4096,
        temperature: 0.7,
    };
    
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&request)
        .send()?
        .json::<AnthropicResponse>()?;
    
    if response.content.is_empty() {
        return Err(anyhow!("No response content from Anthropic API"));
    }
    
    Ok(response.content[0].text.clone())
}

// Collects all file contents into a single string
fn collect_file_contents(paths: &[PathBuf], cli: &Cli, output_format: &OutputFormat) -> Result<String> {
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

fn main() -> Result<()> {
    let mut cli = Cli::parse();
    
    // Handle list-models flag first
    if cli.list_models {
        // Get API key for Gemini
        let api_key = if let Some(key) = &cli.api_key {
            key.clone()
        } else if let Some(env_var) = &cli.api_key_env {
            std::env::var(env_var).ok().unwrap_or_default()
        } else {
            std::env::var("GOOGLE_API_KEY").ok().unwrap_or_default()
        };
        
        if api_key.is_empty() {
            eprintln!("Error: No API key found. An API key is required to list models.");
            eprintln!("Please provide a Gemini API key with --api-key or set the GOOGLE_API_KEY environment variable.");
            std::process::exit(1);
        }
        
        return list_gemini_models(&api_key);
    }
    
    // Read paths from stdin if available
    let mut stdin_paths = read_paths_from_stdin(cli.null)?;
    
    // Combine paths from arguments and stdin
    cli.paths.append(&mut stdin_paths);
    
    if cli.paths.is_empty() {
        eprintln!("Error: No paths provided. Provide at least one path as an argument or via stdin.");
        std::process::exit(1);
    }
    
    // If API key is needed, try to retrieve it
    let api_key = get_api_key(&cli);
    
    // For token counting, show a note if API key is missing but only for cost estimates
    if cli.count_tokens && cli.show_cost {
        if api_key.is_none() {
            println!("Note: No API key found. Cost estimates are based on published rates only.");
            println!("To set an API key, use --api-key or --api-key-env options\n");
        }
    }
    
    // We don't need to check for the API key for the summarization case here
    // because we'll do that later when we actually need it
    
    // Special handling for token counting mode
    if cli.count_tokens {
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
        discovery_progress.set_style(ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap());
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
                                return !entry.file_type().map_or(false, |ft| ft.is_dir());
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
                        discovery_progress.set_message(format!("Scanning: {} (found {} files)", 
                            entry_path.display(), all_files.len()));
                        
                        // Skip directories, we only want files
                        if !entry.file_type().unwrap_or_else(|| std::fs::metadata(entry_path).unwrap().file_type()).is_file() {
                            continue;
                        }
                        
                        // Check custom ignore patterns
                        if should_ignore(entry_path, &cli_arc.ignore_patterns, cli_arc.ignore_files_only) {
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
                    },
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
                    let token_count = count_tokens(&content, &cli_arc.tokenizer_model);
                    
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
        display_token_report(&report, &cli)?;
        
        return Ok(());
    }
    
    // Determine output format
    let output_format = if cli.cxml {
        OutputFormat::Cxml
    } else if cli.markdown {
        OutputFormat::Markdown
    } else {
        cli.output_format.clone()
    };
    
    // Collect all file contents
    let content = collect_file_contents(&cli.paths, &cli, &output_format)?;
    
    // Output concatenated content to file if requested
    if let Some(output_file) = &cli.output_file {
        // Write to file
        std::fs::write(output_file, &content)?;
        println!("Concatenated content written to {}", output_file.display());
    }
    
    // If no-summarize, just output content to stdout if no output file specified
    if cli.no_summarize {
        if cli.output_file.is_none() {
            // Write to stdout
            print!("{}", content);
        }
        return Ok(());
    }
    
    // Default behavior: send the content to the LLM for summarization
    
    // Check for API key again since we need it for summarization
    if api_key.is_none() {
        eprintln!("Error: No API key found. An API key is required for summarization.");
        eprintln!("Please provide an API key with --api-key or set the appropriate environment variable.");
        eprintln!("Use --no-summarize to only concatenate files without generating a summary.");
        std::process::exit(1);
    }
    
    println!("Summarizing codebase with {} model...", cli.tokenizer_model);
    
    // Get the API key
    let api_key = api_key.unwrap();
    
    // Log input size information
    let input_size_bytes = content.len();
    let input_size_kb = input_size_bytes / 1024;
    let input_size_mb = input_size_kb / 1024;
    let token_count = count_tokens(&content, &cli.tokenizer_model);
    
    println!("Input size: {} bytes ({} KB, {:.2} MB)", 
             input_size_bytes.separate_with_commas(), 
             input_size_kb.separate_with_commas(), 
             input_size_mb as f64);
    println!("Estimated token count: {}", token_count.separate_with_commas());
    
    // Get summary from LLM
    let summary = summarize_with_llm(&content, &cli.custom_prompt, &cli.tokenizer_model, &api_key)?;
    
    // Write summary to file
    std::fs::write(&cli.summary_output, summary)?;
    
    println!("Summary written to {}", cli.summary_output.display());
    
    Ok(())
}