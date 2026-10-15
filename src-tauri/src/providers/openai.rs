use anyhow::{Context, Result};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::models::GenerationResult;

const API_URL: &str = "https://api.openai.com/v1/images/generations";

/// Model ID mapping
fn resolve_model(model: &str) -> &str {
    match model {
        "dalle" | "dalle3" | "dall-e-3" => "dall-e-3",
        "dalle2" | "dall-e-2" => "dall-e-2",
        "gpt-image" | "gpt-image-1" => "gpt-image-1",
        "gpt-image-2" | "gptimage2" => "gpt-image-2",
        _ => model,
    }
}

/// GPT image models always return base64 and reject `response_format` with a 400
/// ("Unknown parameter") — only the DALL-E models accept it.
fn accepts_response_format(model_id: &str) -> bool {
    !model_id.starts_with("gpt-image")
}

#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    prompt: String,
    n: u32,
    size: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    quality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    style: Option<String>,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    data: Option<Vec<OpenAIImage>>,
    error: Option<OpenAIError>,
}

#[derive(Deserialize)]
struct OpenAIError {
    message: String,
}

#[derive(Deserialize)]
struct OpenAIImage {
    b64_json: Option<String>,
}

fn get_api_key() -> Result<String> {
    std::env::var("OPENAI_API_SECRET_KEY")
        .or_else(|_| std::env::var("OPENAI_API_KEY"))
        .context("OPENAI_API_SECRET_KEY or OPENAI_API_KEY environment variable not set")
}

pub async fn generate(
    model: &str,
    prompt: &str,
    _reference_paths: &[String],
    _negative_prompt: Option<&str>,
    _width: Option<i32>,
    _height: Option<i32>,
) -> Result<GenerationResult> {
    let api_key = get_api_key()?;
    let model_id = resolve_model(model);

    // Note: DALL-E 3 doesn't support reference images
    // We ignore reference_paths for OpenAI

    let request = OpenAIRequest {
        model: model_id.to_string(),
        prompt: prompt.to_string(),
        n: 1,
        size: "1024x1024".to_string(),
        response_format: accepts_response_format(model_id).then(|| "b64_json".to_string()),
        quality: if model_id == "dall-e-3" {
            Some("standard".to_string())
        } else {
            None
        },
        style: if model_id == "dall-e-3" {
            Some("vivid".to_string())
        } else {
            None
        },
    };

    let client = super::client();

    let start = Instant::now();
    let response = client
        .post(API_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await
        .context("Failed to send request to OpenAI API")?;

    let elapsed = start.elapsed().as_secs_f64();

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        anyhow::bail!("OpenAI API error {}: {}", status, text);
    }

    let data: OpenAIResponse = response.json().await.context("Failed to parse OpenAI response")?;

    if let Some(error) = data.error {
        anyhow::bail!("OpenAI API error: {}", error.message);
    }

    // Extract image data
    let b64_data = data
        .data
        .and_then(|images| images.into_iter().next())
        .and_then(|img| img.b64_json)
        .ok_or_else(|| anyhow::anyhow!("No image data in OpenAI response"))?;

    let image_data = base64::engine::general_purpose::STANDARD
        .decode(&b64_data)
        .context("Failed to decode base64 image data")?;

    Ok(GenerationResult {
        image_data,
        seed: None,
        generation_time_seconds: elapsed,
        cost_usd: None, // OpenAI doesn't return token-based billing
    })
}
