use crate::cli::Cli;
use crate::models::TokenizerModel;
use dotenv::dotenv;

pub fn get_api_key(cli: &Cli) -> Option<String> {
    // Load environment variables from .env file in home directory if it exists
    if let Some(home_dir) = dirs::home_dir() {
        let env_path = home_dir.join(".env");
        if env_path.exists() {
            if let Err(e) = dotenv::from_path(env_path) {
                eprintln!(
                    "Warning: Failed to load .env file from home directory: {}",
                    e
                );
            }
        }
    }

    // Also load from current directory if it exists (this will override home directory values)
    let _ = dotenv();

    if let Some(key) = &cli.api_key {
        return Some(key.clone());
    }

    if let Some(env_var) = &cli.api_key_env {
        return std::env::var(env_var).ok();
    }

    // Try common environment variables for different providers
    match cli.tokenizer_model {
        TokenizerModel::Gemini15Pro
        | TokenizerModel::Gemini15Flash
        | TokenizerModel::Gemini20Flash
        | TokenizerModel::Gemini20FlashLite
        | TokenizerModel::Gemini20Pro
        | TokenizerModel::Gemini20ProExp
        | TokenizerModel::Gemini20ProExp0205
        | TokenizerModel::Gemini20FlashThinkingExp => std::env::var("GOOGLE_API_KEY").ok(),
        TokenizerModel::Gpt35Turbo | TokenizerModel::Gpt4 | TokenizerModel::Gpt4Turbo => {
            std::env::var("OPENAI_API_KEY").ok()
        }
        TokenizerModel::Claude3Sonnet | TokenizerModel::Claude3Opus => {
            std::env::var("ANTHROPIC_API_KEY").ok()
        }
    }
}
