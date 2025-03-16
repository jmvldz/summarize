use clap::Parser;
use std::path::PathBuf;

use crate::models::{OutputFormat, TokenizerModel};

#[derive(Parser, Debug, Clone)]
#[command(
    name = "summarize",
    about = "Concatenate a directory full of files into a single prompt for use with LLMs",
    author = "Original by Simon Willison, Rust port by Claude",
    version
)]
pub struct Cli {
    /// Paths to files or directories to process
    #[arg(required = false)]
    pub paths: Vec<PathBuf>,

    /// Only include files with the specified extension(s)
    #[arg(short = 'e', long = "extension")]
    pub extensions: Vec<String>,

    /// Include files and folders starting with . (hidden files and directories)
    #[arg(long = "include-hidden")]
    pub include_hidden: bool,

    /// --ignore option only ignores files
    #[arg(long = "ignore-files-only")]
    pub ignore_files_only: bool,

    /// Ignore .gitignore files and include all files
    #[arg(long = "ignore-gitignore")]
    pub ignore_gitignore: bool,

    /// Exclude version control directories (.git, .svn, .hg)
    #[arg(long = "exclude-vcs", default_value_t = true)]
    pub exclude_vcs: bool,

    /// Include version control directories
    #[arg(long = "include-vcs", default_value_t = false)]
    pub include_vcs: bool,

    /// List of patterns to ignore
    #[arg(long = "ignore")]
    pub ignore_patterns: Vec<String>,

    /// Output to a file instead of stdout
    #[arg(short = 'o', long = "output")]
    pub output_file: Option<PathBuf>,

    /// Output format
    #[arg(
        short = 'f',
        long = "format",
        value_enum,
        default_value_t = OutputFormat::Default
    )]
    pub output_format: OutputFormat,

    /// Output in Claude XML format
    #[arg(short = 'c', long = "cxml", conflicts_with = "output_format")]
    pub cxml: bool,

    /// Output Markdown with fenced code blocks
    #[arg(short = 'm', long = "markdown", conflicts_with = "output_format")]
    pub markdown: bool,

    /// Add line numbers to the output
    #[arg(short = 'n', long = "line-numbers")]
    pub line_numbers: bool,

    /// Use NUL character as separator when reading from stdin
    #[arg(short = '0', long = "null")]
    pub null: bool,

    /// Count tokens instead of outputting content
    #[arg(short = 't', long = "count-tokens")]
    pub count_tokens: bool,

    /// Tokenization model to use for counting or summarization
    #[arg(
        long = "model",
        value_enum,
        default_value_t = TokenizerModel::Gemini15Flash,
        requires = "count_tokens"
    )]
    pub tokenizer_model: TokenizerModel,

    /// API key for the LLM service
    #[arg(long = "api-key")]
    pub api_key: Option<String>,

    /// Use API key from environment variable
    #[arg(long = "api-key-env", conflicts_with = "api_key")]
    pub api_key_env: Option<String>,

    /// Show per-file token counts
    #[arg(long = "verbose", requires = "count_tokens")]
    pub verbose: bool,

    /// Show estimated API costs
    #[arg(long = "show-cost", requires = "count_tokens")]
    pub show_cost: bool,

    /// Only concatenate files without generating a summary
    #[arg(long = "no-summarize")]
    pub no_summarize: bool,

    /// Custom prompt to use when generating a summary
    #[arg(
        long = "prompt",
        default_value = "You are a senior software engineer reviewing a codebase. Generate a comprehensive overview.md file that explains the purpose, structure, and key components of this codebase. Focus on helping a new developer understand how the codebase is organized and how different parts work together."
    )]
    pub custom_prompt: String,

    /// Output file for the summary
    #[arg(long = "summary-output", default_value = "overview.md")]
    pub summary_output: PathBuf,

    /// List available models from the LLM service
    #[arg(long = "list-models")]
    pub list_models: bool,

    /// Number of threads to use for token counting (0 = use all available cores)
    #[arg(long = "threads", default_value = "0")]
    pub num_threads: usize,
}
