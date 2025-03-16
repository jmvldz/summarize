use crate::models::TokenizerModel;
use tiktoken_rs::{cl100k_base, p50k_base};

pub fn get_tokenizer_name(model: &TokenizerModel) -> &'static str {
    match model {
        TokenizerModel::Gemini15Pro => "cl100k_base", // Approximate with cl100k_base
        TokenizerModel::Gemini15Flash => "cl100k_base", // Approximate with cl100k_base
        TokenizerModel::Gemini20Flash => "cl100k_base", // Approximate with cl100k_base
        TokenizerModel::Gemini20FlashLite => "cl100k_base", // Approximate with cl100k_base
        TokenizerModel::Gemini20Pro => "cl100k_base", // Approximate with cl100k_base
        TokenizerModel::Gemini20ProExp => "cl100k_base", // Approximate with cl100k_base
        TokenizerModel::Gemini20ProExp0205 => "cl100k_base", // Approximate with cl100k_base
        TokenizerModel::Gemini20FlashThinkingExp => "cl100k_base", // Approximate with cl100k_base
        TokenizerModel::Gpt35Turbo => "cl100k_base",  // GPT-3.5-Turbo uses cl100k_base
        TokenizerModel::Gpt4 => "cl100k_base",        // GPT-4 uses cl100k_base
        TokenizerModel::Gpt4Turbo => "cl100k_base",   // GPT-4-Turbo uses cl100k_base
        TokenizerModel::Claude3Sonnet => "p50k_base", // Approximate with p50k_base
        TokenizerModel::Claude3Opus => "p50k_base",   // Approximate with p50k_base
    }
}

pub fn get_token_cost(model: &TokenizerModel, _tokens: usize) -> (f64, f64) {
    // (input_cost_per_1k, output_cost_per_1k)
    match model {
        TokenizerModel::Gemini15Pro => (0.0000, 0.0000), // Estimated
        TokenizerModel::Gemini15Flash => (0.0000, 0.0000), // Estimated
        TokenizerModel::Gemini20Flash => (0.0000, 0.0000), // Currently free during preview
        TokenizerModel::Gemini20FlashLite => (0.0000, 0.0000), // Currently free during preview
        TokenizerModel::Gemini20Pro => (0.0000, 0.0000), // Currently free during preview
        TokenizerModel::Gemini20ProExp => (0.0000, 0.0000), // Currently free during preview (experimental)
        TokenizerModel::Gemini20ProExp0205 => (0.0000, 0.0000), // Currently free during preview (experimental)
        TokenizerModel::Gemini20FlashThinkingExp => (0.0000, 0.0000), // Currently free during preview (experimental)
        TokenizerModel::Gpt35Turbo => (0.0010, 0.0020), // $0.0010 per 1k input, $0.0020 per 1k output
        TokenizerModel::Gpt4 => (0.03, 0.06),           // $0.03 per 1k input, $0.06 per 1k output
        TokenizerModel::Gpt4Turbo => (0.01, 0.03),      // $0.01 per 1k input, $0.03 per 1k output
        TokenizerModel::Claude3Sonnet => (0.003, 0.015), // $0.003 per 1k input, $0.015 per 1k output
        TokenizerModel::Claude3Opus => (0.015, 0.075), // $0.015 per 1k input, $0.075 per 1k output
    }
}

pub fn count_tokens(text: &str, model: &TokenizerModel) -> usize {
    // Currently we're using tiktoken for all models but in a real-world implementation
    // we'd use different tokenizers for each model family
    match get_tokenizer_name(model) {
        "cl100k_base" => {
            let bpe = cl100k_base().unwrap();
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
