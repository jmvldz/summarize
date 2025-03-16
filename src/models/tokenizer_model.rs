use clap::ValueEnum;
use std::fmt;

#[derive(Debug, Clone, ValueEnum)]
pub enum TokenizerModel {
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
