use anyhow::{anyhow, Result};
use reqwest::blocking::Client;

use super::models::{AnthropicContent, AnthropicMessage, AnthropicRequest, AnthropicResponse};

pub fn summarize_with_anthropic(
    code_content: &str,
    prompt: &str,
    model: &str,
    api_key: &str,
) -> Result<String> {
    let client = Client::new();

    let request = AnthropicRequest {
        model: model.to_string(),
        messages: vec![AnthropicMessage {
            role: "user".to_string(),
            content: vec![AnthropicContent {
                content_type: "text".to_string(),
                text: format!("{}\n\nHere's the codebase:\n\n{}", prompt, code_content),
            }],
        }],
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
