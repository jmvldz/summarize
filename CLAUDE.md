# Build/Lint/Test Commands

## Rust Project
- Build: `cargo build`
- Run: `cargo run -- [args]` 
- Test: `cargo test`
- Single test: `cargo test test_name`
- Lint: `cargo clippy`
- Format: `cargo fmt`
- Token counting: `cargo run -- --count-tokens [paths] --threads [num]`

## Python Project
- Run tests: `cd files-to-prompt && python -m pytest`
- Single test: `cd files-to-prompt && python -m pytest tests/test_files_to_prompt.py::test_name`
- Format: `cd files-to-prompt && black .` (requires black)

# Code Style Guidelines

## Rust
- Use idiomatic Rust with Result/Option for error handling
- Follow standard Rust naming: snake_case for functions, CamelCase for types
- Error handling uses anyhow crate
- Group imports by standard library, external crates, then local modules
- Use struct formatting with one field per line for readability
- Use rayon for parallel processing where performance is important
- Use Arc and Mutex for thread-safe shared data access

## Python
- Follow PEP 8 conventions
- Use click for CLI interface
- Use descriptive variable names
- Group imports by standard library, external dependencies, local modules
- Write comprehensive tests using pytest