#[cfg(test)]
mod tests {
    use summarize::models::TokenizerModel;
    use summarize::tokenizers;

    #[test]
    fn test_tokenizer_name() {
        // Test a few different models
        assert_eq!(
            tokenizers::get_tokenizer_name(&TokenizerModel::Gpt35Turbo),
            "cl100k_base"
        );
        assert_eq!(
            tokenizers::get_tokenizer_name(&TokenizerModel::Claude3Sonnet),
            "p50k_base"
        );
    }

    #[test]
    fn test_token_cost() {
        // Test GPT costs
        let (input_cost, output_cost) = tokenizers::get_token_cost(&TokenizerModel::Gpt35Turbo, 0);
        assert_eq!(input_cost, 0.0010);
        assert_eq!(output_cost, 0.0020);

        // Test Claude costs
        let (input_cost, output_cost) = tokenizers::get_token_cost(&TokenizerModel::Claude3Opus, 0);
        assert_eq!(input_cost, 0.015);
        assert_eq!(output_cost, 0.075);
    }

    #[test]
    fn test_token_counting() {
        // Test with a simple string
        let text = "Hello, world! This is a test.";
        let token_count = tokenizers::count_tokens(text, &TokenizerModel::Gpt35Turbo);

        // The exact count may vary depending on the tokenizer implementation
        assert!(token_count > 0);

        // Basic sanity check: longer text should have more tokens
        let longer_text = text.repeat(10);
        let longer_token_count =
            tokenizers::count_tokens(&longer_text, &TokenizerModel::Gpt35Turbo);
        assert!(longer_token_count > token_count);
    }
}
