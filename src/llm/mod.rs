mod anthropic;
mod gemini;
mod models;
mod openai;

pub use anthropic::summarize_with_anthropic;
pub use gemini::{list_gemini_models, summarize_with_gemini};
pub use models::*;
pub use openai::summarize_with_openai;

use crate::models::TokenizerModel;
use anyhow::Result;

pub fn summarize_with_llm(
    code_content: &str,
    prompt: &str,
    model: &TokenizerModel,
    api_key: &str,
) -> Result<String> {
    println!("Attempting to summarize with model: {}", model);

    match model {
        TokenizerModel::Gemini15Pro => {
            println!("Using Gemini 1.5 Pro model");
            summarize_with_gemini(code_content, prompt, "gemini-1.5-pro", api_key)
        }
        TokenizerModel::Gemini15Flash => {
            println!("Using Gemini 1.5 Flash model");
            summarize_with_gemini(code_content, prompt, "gemini-1.5-flash", api_key)
        }
        TokenizerModel::Gemini20Flash => {
            println!("Using Gemini 2.0 Flash model");
            summarize_with_gemini(code_content, prompt, "gemini-2.0-flash", api_key)
        }
        TokenizerModel::Gemini20FlashLite => {
            println!("Using Gemini 2.0 Flash-Lite model");
            summarize_with_gemini(code_content, prompt, "gemini-2.0-flash-lite", api_key)
        }
        TokenizerModel::Gemini20Pro => {
            println!("Using Gemini 2.0 Pro model");
            summarize_with_gemini(code_content, prompt, "gemini-2.0-pro", api_key)
        }
        TokenizerModel::Gemini20ProExp => {
            println!("Using Gemini 2.0 Pro Exp 02-05 model");
            summarize_with_gemini(code_content, prompt, "gemini-2.0-pro-exp-02-05", api_key)
        }
        TokenizerModel::Gemini20ProExp0205 => {
            println!("Using Gemini 2.0 Pro Exp 02-05 model");
            summarize_with_gemini(code_content, prompt, "gemini-2.0-pro-exp-02-05", api_key)
        }
        TokenizerModel::Gemini20FlashThinkingExp => {
            println!("Using Gemini 2.0 Flash Thinking Exp model");
            summarize_with_gemini(
                code_content,
                prompt,
                "gemini-2.0-flash-thinking-exp",
                api_key,
            )
        }
        TokenizerModel::Gpt35Turbo | TokenizerModel::Gpt4 | TokenizerModel::Gpt4Turbo => {
            let model_name = match model {
                TokenizerModel::Gpt35Turbo => "gpt-3.5-turbo",
                TokenizerModel::Gpt4 => "gpt-4",
                TokenizerModel::Gpt4Turbo => "gpt-4-turbo",
                _ => unreachable!(),
            };
            println!("Using OpenAI model: {}", model_name);
            summarize_with_openai(code_content, prompt, model_name, api_key)
        }
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
