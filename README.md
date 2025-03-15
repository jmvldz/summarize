# summarize

A command-line tool that concatenates a directory of code files into a prompt for use with LLMs, with an option to generate a comprehensive codebase overview.

## Features

- Recursively process directories of source code
- Filter files by extension or pattern
- Respect .gitignore files by default
- Output in plain text, Markdown, or Claude XML format
- Generate line numbers for each file
- Count tokens for different LLM models (GPT, Claude, Gemini)
- Generate comprehensive codebase overviews with AI assistance
- Parallel processing for performance

## Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/summarize.git
cd summarize

# Build the project
cargo build --release

# The binary will be in target/release/summarize
```

## Usage

```bash
# Basic usage
summarize /path/to/your/codebase

# Only include specific file extensions
summarize /path/to/your/codebase -e js -e ts

# Output to a file instead of stdout
summarize /path/to/your/codebase -o output.txt

# Output in markdown format
summarize /path/to/your/codebase -m

# Count tokens (estimating for Claude 3 Sonnet)
summarize /path/to/your/codebase --count-tokens --model Claude3Sonnet

# Generate a codebase overview
summarize /path/to/your/codebase --summary-output overview.md

# Custom prompt for overview generation
summarize /path/to/your/codebase --prompt "Analyze this codebase and explain its architecture"
```

## Command-Line Options

```
Usage: summarize [OPTIONS] [PATHS]...

Arguments:
  [PATHS]...  Paths to files or directories to process

Options:
  -e, --extension <EXTENSIONS>            Only include files with the specified extension(s)
      --include-hidden                    Include files and folders starting with . (hidden files and directories)
      --ignore-files-only                 --ignore option only ignores files
      --ignore-gitignore                  Ignore .gitignore files and include all files
      --exclude-vcs                       Exclude version control directories (.git, .svn, .hg) [default: true]
      --include-vcs                       Include version control directories [default: false]
      --ignore <IGNORE_PATTERNS>          List of patterns to ignore
  -o, --output <OUTPUT_FILE>              Output to a file instead of stdout
  -f, --format <OUTPUT_FORMAT>            Output format [default: default] [possible values: default, cxml, markdown]
  -c, --cxml                              Output in Claude XML format
  -m, --markdown                          Output Markdown with fenced code blocks
  -n, --line-numbers                      Add line numbers to the output
  -0, --null                              Use NUL character as separator when reading from stdin
  -t, --count-tokens                      Count tokens instead of outputting content
      --model <TOKENIZER_MODEL>           Tokenization model to use for counting or summarization [default: gemini15flash]
      --api-key <API_KEY>                 API key for the LLM service
      --api-key-env <API_KEY_ENV>         Use API key from environment variable
      --verbose                           Show per-file token counts
      --show-cost                         Show estimated API costs
      --no-summarize                      Only concatenate files without generating a summary
      --prompt <CUSTOM_PROMPT>            Custom prompt to use when generating a summary
      --summary-output <SUMMARY_OUTPUT>   Output file for the summary [default: overview.md]
      --list-models                       List available models from the LLM service
      --threads <NUM_THREADS>             Number of threads to use for token counting (0 = use all available cores) [default: 0]
  -h, --help                              Print help
  -V, --version                           Print version
```

## Supported Models

- GPT Models: GPT-3.5 Turbo, GPT-4, GPT-4 Turbo
- Claude Models: Claude 3 Sonnet, Claude 3 Opus
- Gemini Models: Gemini 1.5 Pro, Gemini 1.5 Flash, Gemini 2.0 Pro, Gemini 2.0 Flash

## Environment Variables

The tool looks for API keys in the following environment variables:

- `GOOGLE_API_KEY` - For Gemini models
- `OPENAI_API_KEY` - For GPT models
- `ANTHROPIC_API_KEY` - For Claude models

## License

Apache-2.0