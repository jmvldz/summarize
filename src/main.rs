use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use thousands::Separable;

use summarize::cli::Cli;
use summarize::llm::{list_gemini_models, summarize_with_llm};
use summarize::models::OutputFormat;
use summarize::tokenizers;
use summarize::utils::{get_api_key, read_paths_from_stdin};
use summarize::{collect_file_contents, process_token_count};

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

    // If no paths provided, use the current directory
    if cli.paths.is_empty() {
        cli.paths.push(PathBuf::from("."));
    }

    // If API key is needed, try to retrieve it
    let api_key = get_api_key(&cli);

    // For token counting, show a note if API key is missing but only for cost estimates
    if cli.count_tokens && cli.show_cost && api_key.is_none() {
        println!("Note: No API key found. Cost estimates are based on published rates only.");
        println!("To set an API key, use --api-key or --api-key-env options\n");
    }

    // Special handling for token counting mode
    if cli.count_tokens {
        return process_token_count(&cli);
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
        eprintln!(
            "Please provide an API key with --api-key or set the appropriate environment variable."
        );
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
    let token_count = tokenizers::count_tokens(&content, &cli.tokenizer_model);

    println!(
        "Input size: {} bytes ({} KB, {:.2} MB)",
        input_size_bytes.separate_with_commas(),
        input_size_kb.separate_with_commas(),
        input_size_mb as f64
    );
    println!(
        "Estimated token count: {}",
        token_count.separate_with_commas()
    );

    // Get summary from LLM
    let summary = summarize_with_llm(&content, &cli.custom_prompt, &cli.tokenizer_model, &api_key)?;

    // Write summary to file
    std::fs::write(&cli.summary_output, summary)?;

    println!("Summary written to {}", cli.summary_output.display());

    Ok(())
}
