use anyhow::{anyhow, Result};
use comfy_table::{ContentArrangement, Table};
use reqwest::blocking::Client;

use super::models::{
    GeminiConfig, GeminiListModelsResponse, GeminiMessage, GeminiPart, GeminiRequest,
    GeminiResponse,
};

pub fn summarize_with_gemini(
    code_content: &str,
    prompt: &str,
    model_name: &str,
    api_key: &str,
) -> Result<String> {
    let client = Client::new();

    let full_prompt = format!("{}\n\nHere's the codebase:\n\n{}", prompt, code_content);

    let request = GeminiRequest {
        contents: vec![GeminiMessage {
            role: "user".to_string(),
            parts: vec![GeminiPart { text: full_prompt }],
        }],
        generation_config: GeminiConfig {
            temperature: 0.7,
            top_p: 0.95,
            top_k: 40,
            max_output_tokens: 8192,
        },
    };

    // For experimental models, use v1beta endpoint, otherwise use v1
    // Handle both model formats: full path (models/...) or just model name
    let api_version = if model_name.contains("exp") {
        "v1beta"
    } else {
        "v1"
    };

    // Check if model_name already contains "models/" prefix
    let model_path = if model_name.starts_with("models/") {
        // Extract just the model name part after "models/"
        model_name
            .split('/')
            .skip(1)
            .collect::<Vec<&str>>()
            .join("/")
    } else {
        model_name.to_string()
    };

    let url = format!(
        "https://generativelanguage.googleapis.com/{}/models/{}:generateContent?key={}",
        api_version, model_path, api_key
    );

    println!("Using API URL: {}", url.replace(api_key, "[REDACTED]"));

    // Send the request with timeout and error handling
    let response = match client.post(&url).json(&request).send() {
        Ok(resp) => resp,
        Err(e) => {
            return Err(anyhow!("Error sending request to Gemini API: {}", e));
        }
    };

    // Check status code first
    if !response.status().is_success() {
        println!("API Error - Status Code: {}", response.status());
    }

    // Get response text with error handling
    let response_text = match response.text() {
        Ok(text) => text,
        Err(e) => {
            return Err(anyhow!("Error reading Gemini API response: {}", e));
        }
    };

    // Print response for debugging in case of errors
    if response_text.is_empty() {
        println!("API Response: [Empty response]");
    } else {
        println!("API Response: {}", &response_text);
    }

    // Check if response contains an error about model not found
    if response_text.contains("NOT_FOUND") && response_text.contains("is not found") {
        // Handle model not found case specifically
        println!(
            "\nError: The specified model '{}' was not found.",
            model_name
        );
        println!("To see a list of available models, run:");
        println!("  summarize --list-models --api-key YOUR_API_KEY");
        return Err(anyhow!("Model not found: {}", model_name));
    }

    let response: GeminiResponse = match serde_json::from_str(&response_text) {
        Ok(resp) => resp,
        Err(e) => {
            return Err(anyhow!(
                "Error parsing Gemini API response: {}. Response: {}",
                e,
                response_text
            ));
        }
    };

    if response.candidates.is_empty() || response.candidates[0].content.parts.is_empty() {
        return Err(anyhow!("No response content from Gemini API"));
    }

    Ok(response.candidates[0].content.parts[0].text.clone())
}

pub fn list_gemini_models(api_key: &str) -> Result<()> {
    let client = Client::new();

    // First, get standard models
    let standard_url = format!(
        "https://generativelanguage.googleapis.com/v1/models?key={}",
        api_key
    );

    let standard_response_text = client.get(&standard_url).send()?.text()?;

    // Also try to get experimental models
    let experimental_url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models?key={}",
        api_key
    );

    let experimental_response_text = client
        .get(&experimental_url)
        .send()
        .map_or_else(|_| "".to_string(), |resp| resp.text().unwrap_or_default());

    println!("Available Gemini Models:");

    // Parse standard models
    let standard_models =
        match serde_json::from_str::<GeminiListModelsResponse>(&standard_response_text) {
            Ok(response) => response.models,
            Err(e) => {
                println!("Error parsing standard models API response: {}", e);
                println!("Raw API response: {}", standard_response_text);
                vec![] // Return empty vector to continue with experimental models
            }
        };

    // Parse experimental models
    let experimental_models = if !experimental_response_text.is_empty() {
        match serde_json::from_str::<GeminiListModelsResponse>(&experimental_response_text) {
            Ok(response) => response.models,
            Err(e) => {
                println!("Error parsing experimental models API response: {}", e);
                vec![] // Return empty vector
            }
        }
    } else {
        vec![]
    };

    // Create a table for better formatting
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![
        "Model Name",
        "Type",
        "Display Name",
        "Description",
        "Supported Methods",
    ]);

    // Collect the names of standard models for later comparison
    let standard_model_names: Vec<String> = standard_models
        .iter()
        .map(|m| m.name.split('/').last().unwrap_or(&m.name).to_string())
        .collect();

    // Add standard models to table
    for model in &standard_models {
        // Extract the model name without the full path
        let short_name = model.name.split('/').last().unwrap_or(&model.name);

        // Check if this is likely an experimental model
        let model_type = if short_name.contains("exp") {
            "Experimental"
        } else {
            "Standard"
        };

        // Format supported methods
        let methods = match &model.supported_generation_methods {
            Some(methods) => methods.join(", "),
            None => "N/A".to_string(),
        };

        table.add_row(vec![
            short_name.to_string(),
            model_type.to_string(),
            model.display_name.clone(),
            model.description.clone(),
            methods,
        ]);
    }

    // Add experimental models to table
    for model in &experimental_models {
        // Extract the model name without the full path
        let short_name = model.name.split('/').last().unwrap_or(&model.name);

        // Skip if this model already appears in the standard models list
        if standard_model_names.iter().any(|name| name == short_name) {
            continue;
        }

        // Format supported methods
        let methods = match &model.supported_generation_methods {
            Some(methods) => methods.join(", "),
            None => "N/A".to_string(),
        };

        table.add_row(vec![
            short_name.to_string(),
            "Experimental".to_string(),
            model.display_name.clone(),
            model.description.clone(),
            methods,
        ]);
    }

    println!("{table}");

    // Provide information about using experimental models
    println!("\nNote about experimental models:");
    println!("- Experimental models contain 'exp' in their names (e.g. gemini-2.0-pro-exp-02-05)");
    println!("- These models may change or be removed without notice");
    println!("- They are not recommended for production use");
    println!("- Use the --model flag to specify an experimental model for summarization");

    Ok(())
}
