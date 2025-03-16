use anyhow::{anyhow, Result};
use reqwest::blocking::Client;

use super::models::{OpenAIMessage, OpenAIRequest, OpenAIResponse};

pub fn summarize_with_openai(
    code_content: &str,
    prompt: &str,
    model: &str,
    api_key: &str,
) -> Result<String> {
    let client = Client::new();

    let request = OpenAIRequest {
        model: model.to_string(),
        messages: vec![
            OpenAIMessage {
                role: "system".to_string(),
                content: prompt.to_string(),
            },
            OpenAIMessage {
                role: "user".to_string(),
                content: code_content.to_string(),
            },
        ],
        temperature: 0.7,
        max_tokens: 4096,
    };

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .send()?
        .json::<OpenAIResponse>()?;

    if response.choices.is_empty() {
        return Err(anyhow!("No response content from OpenAI API"));
    }

    Ok(response.choices[0].message.content.clone())
}
