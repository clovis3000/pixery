use anyhow::{Context, Result};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{Duration, Instant};

use crate::models::GenerationResult;

const API_BASE: &str = "https://queue.fal.run";
const POLL_INTERVAL_MS: u64 = 1000; // 1 second between polls
const MAX_POLL_ATTEMPTS: u32 = 300; // 5 minutes max (Ultra models queue longer)

/// Model ID mapping for fal.ai models
///
/// # Z-Image Turbo
/// - Text-to-image: `fal-ai/z-image/turbo` ($0.005/MP)
/// - Image-to-image: `fal-ai/z-image/turbo/image-to-image` ($0.005/MP)
/// - With LoRA: `fal-ai/z-image/turbo/lora` ($0.0085/MP)
/// - Max 1 reference image for image-to-image
/// - Parameters: num_images (1-4), num_inference_steps (1-8, default 8)
/// - Image sizes: square, square_hd, portrait_4_3, portrait_16_9, landscape_4_3, landscape_16_9
/// - Note: Only Turbo variant is publicly available. "Z-Image base" is not deployed.
fn resolve_model(model: &str, has_reference: bool) -> &str {
    match model {
        "flux-schnell" => "fal-ai/flux/schnell",
        "flux-pro" | "fal-ai/flux-pro/v1.1" => "fal-ai/flux-pro/v1.1",
        "flux-ultra" | "fal-ai/flux-pro/v1.1-ultra" => "fal-ai/flux-pro/v1.1-ultra",
        "recraft" | "fal-ai/recraft-v3" => "fal-ai/recraft-v3",
        "flux2-turbo" | "fal-ai/flux-2/turbo" => {
            if has_reference {
                "fal-ai/flux-2/turbo/edit"
            } else {
                "fal-ai/flux-2/turbo"
            }
        }
        "flux2-pro" | "fal-ai/flux-2-pro" => {
            if has_reference {
                "fal-ai/flux-2-pro/edit"
            } else {
                "fal-ai/flux-2-pro"
            }
        }
        "flux2-max" | "fal-ai/flux-2-max" => {
            if has_reference {
                "fal-ai/flux-2-max/edit"
            } else {
                "fal-ai/flux-2-max"
            }
        }
        "flux2-hdr" | "fal-ai/flux-2-lora-gallery/hdr-style" => "fal-ai/flux-2-lora-gallery/hdr-style",
        "imagen4" | "fal-ai/imagen4/preview" => "fal-ai/imagen4/preview",
        "imagen4-fast" | "fal-ai/imagen4/preview/fast" => "fal-ai/imagen4/preview/fast",
        "imagen4-ultra" | "fal-ai/imagen4/preview/ultra" => "fal-ai/imagen4/preview/ultra",
        "nano-banana-2" | "fal-ai/nano-banana-2" => "fal-ai/nano-banana-2",
        // Z-Image: route to image-to-image endpoint when reference provided
        "z-image" | "fal-ai/z-image/turbo" | "fal-ai/z-image/turbo/image-to-image" => {
            if has_reference {
                "fal-ai/z-image/turbo/image-to-image"
            } else {
                "fal-ai/z-image/turbo"
            }
        }
        _ => model,
    }
}

#[derive(Serialize)]
struct FalRequest {
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    image_url: Option<String>,
    /// Multiple image URLs for edit endpoints (e.g. FLUX 2 Pro/Max edit)
    #[serde(skip_serializing_if = "Option::is_none")]
    image_urls: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    image_size: Option<ImageSize>,
    /// Aspect ratio for models that use ratio strings (e.g. Imagen 4: "1:1", "16:9")
    #[serde(skip_serializing_if = "Option::is_none")]
    aspect_ratio: Option<String>,
    /// Strength for image-to-image (0.0-1.0, default 0.6)
    /// Higher = more influence from prompt, lower = more from reference
    #[serde(skip_serializing_if = "Option::is_none")]
    strength: Option<f64>,
    /// Resolution for models that use resolution strings (e.g. Nano Banana 2: "1K", "2K")
    #[serde(skip_serializing_if = "Option::is_none")]
    resolution: Option<String>,
}

/// Response from fal.ai - can be either a queue status or the final result
#[derive(Deserialize, Debug)]
struct FalResponse {
    // Queue status fields
    status: Option<String>,
    response_url: Option<String>,
    // Result fields
    images: Option<Vec<FalImage>>,
    seed: Option<u64>,
    error: Option<String>,
}

#[derive(Deserialize, Debug)]
struct FalImage {
    url: String,
}

fn get_api_key() -> Result<String> {
    std::env::var("FAL_KEY").context("FAL_KEY environment variable not set")
}

/// fal.ai `image_size`: a named preset or exact dimensions. Every image_size endpoint
/// pixery uses accepts `{width, height}` (checked against fal's OpenAPI schemas 2026-10-16),
/// so pass the exact size rather than snapping to the nearest preset.
#[derive(Serialize)]
#[serde(untagged)]
enum ImageSize {
    Named(&'static str),
    Custom { width: i32, height: i32 },
}

fn resolve_image_size(width: Option<i32>, height: Option<i32>) -> ImageSize {
    match (width, height) {
        (Some(width), Some(height)) => ImageSize::Custom { width, height },
        _ => ImageSize::Named("square_hd"),
    }
}

/// Aspect ratios each ratio-string endpoint accepts, from fal's schemas (Imagen 4 publishes
/// none; its list is the set known to work).
fn supported_aspect_ratios(model_id: &str) -> Option<&'static [&'static str]> {
    if model_id.starts_with("fal-ai/imagen4/") {
        Some(&["1:1", "4:3", "3:4", "16:9", "9:16"])
    } else if model_id == "fal-ai/flux-pro/v1.1-ultra" {
        Some(&["21:9", "16:9", "4:3", "3:2", "1:1", "2:3", "3:4", "9:16", "9:21"])
    } else if model_id == "fal-ai/nano-banana-2" {
        Some(&["21:9", "16:9", "3:2", "4:3", "5:4", "1:1", "4:5", "3:4", "2:3", "9:16"])
    } else {
        None
    }
}

/// Pick the supported ratio closest to width/height (compared in log space, so 3:2 vs 4:3
/// and 2:3 vs 3:4 are judged symmetrically).
fn resolve_aspect_ratio(width: Option<i32>, height: Option<i32>, supported: &[&'static str]) -> String {
    let (Some(w), Some(h)) = (width, height) else {
        return "1:1".to_string();
    };
    let target = (w as f64 / h as f64).ln();
    let ratio_of = |r: &str| {
        let (a, b) = r.split_once(':').unwrap();
        (a.parse::<f64>().unwrap() / b.parse::<f64>().unwrap()).ln()
    };
    supported
        .iter()
        .min_by(|a, b| (ratio_of(a) - target).abs().total_cmp(&(ratio_of(b) - target).abs()))
        .unwrap_or(&"1:1")
        .to_string()
}

pub async fn generate(
    model: &str,
    prompt: &str,
    reference_paths: &[String],
    _negative_prompt: Option<&str>,
    width: Option<i32>,
    height: Option<i32>,
) -> Result<GenerationResult> {
    let api_key = get_api_key()?;
    let has_reference = !reference_paths.is_empty();
    let model_id = resolve_model(model, has_reference);

    let is_edit = model_id.ends_with("/edit");

    // Build image_url (singular) for Z-Image image-to-image
    let image_url = if !is_edit {
        if let Some(ref_path) = reference_paths.first() {
            let path = Path::new(ref_path);
            let data = std::fs::read(path).context("Failed to read reference image")?;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
            let mime = super::mime_type(path);
            Some(format!("data:{};base64,{}", mime, b64))
        } else {
            None
        }
    } else {
        None
    };

    // Build image_urls (plural) for FLUX 2 edit endpoints
    let image_urls = if is_edit && !reference_paths.is_empty() {
        let urls: Result<Vec<String>> = reference_paths
            .iter()
            .map(|ref_path| {
                let path = Path::new(ref_path);
                let data = std::fs::read(path).context("Failed to read reference image")?;
                let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                let mime = super::mime_type(path);
                Ok(format!("data:{};base64,{}", mime, b64))
            })
            .collect();
        Some(urls?)
    } else {
        None
    };

    // Set strength for Z-Image image-to-image only (0.6 default balances prompt vs reference)
    let strength = if has_reference && model_id.contains("image-to-image") {
        Some(0.6)
    } else {
        None
    };

    let aspect_ratios = supported_aspect_ratios(model_id);
    let request = FalRequest {
        prompt: prompt.to_string(),
        image_url,
        image_urls,
        image_size: if aspect_ratios.is_some() { None } else { Some(resolve_image_size(width, height)) },
        aspect_ratio: aspect_ratios.map(|supported| resolve_aspect_ratio(width, height, supported)),
        strength,
        resolution: if model_id == "fal-ai/nano-banana-2" { Some("1K".to_string()) } else { None },
    };

    let url = format!("{}/{}", API_BASE, model_id);
    let client = super::client();

    let start = Instant::now();
    let response = client
        .post(&url)
        .header("Authorization", format!("Key {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .timeout(std::time::Duration::from_secs(300)) // 5 minutes - Ultra models can be slow
        .send()
        .await
        .context("Failed to send request to fal.ai API")?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        anyhow::bail!("fal.ai API error {}: {}", status, text);
    }

    let mut data: FalResponse = response.json().await.context("Failed to parse fal.ai response")?;

    if let Some(error) = &data.error {
        anyhow::bail!("fal.ai API error: {}", error);
    }

    // Handle queue-based response - poll until complete
    if data.status.as_deref() == Some("IN_QUEUE") || data.status.as_deref() == Some("IN_PROGRESS") {
        let response_url = data
            .response_url
            .ok_or_else(|| anyhow::anyhow!("Queue response missing response_url"))?;

        for attempt in 0..MAX_POLL_ATTEMPTS {
            tokio::time::sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;

            let poll_response = client
                .get(&response_url)
                .header("Authorization", format!("Key {}", api_key))
                .timeout(Duration::from_secs(30))
                .send()
                .await
                .context("Failed to poll fal.ai queue")?;

            let poll_status = poll_response.status();
            if !poll_status.is_success() {
                // 202 means still processing
                if poll_status.as_u16() == 202 {
                    continue;
                }
                // 400 with "still in progress" also means keep waiting
                if poll_status.as_u16() == 400 {
                    let text = poll_response.text().await.unwrap_or_default();
                    if text.contains("still in progress") {
                        continue;
                    }
                    anyhow::bail!("fal.ai poll error {}: {}", poll_status, text);
                }
                let text = poll_response.text().await.unwrap_or_default();
                anyhow::bail!("fal.ai poll error {}: {}", poll_status, text);
            }

            data = poll_response.json().await.context("Failed to parse poll response")?;

            if let Some(error) = &data.error {
                anyhow::bail!("fal.ai API error: {}", error);
            }

            // Check if we have images now
            if data.images.is_some() {
                break;
            }

            // Still in queue
            if data.status.as_deref() == Some("IN_QUEUE")
                || data.status.as_deref() == Some("IN_PROGRESS")
            {
                continue;
            }

            // Unknown status with no images
            if attempt == MAX_POLL_ATTEMPTS - 1 {
                anyhow::bail!("Timeout waiting for fal.ai generation");
            }
        }
    }

    // Get image URL from response
    let image_info = data
        .images
        .and_then(|images| images.into_iter().next())
        .ok_or_else(|| anyhow::anyhow!("No images in fal.ai response"))?;

    // Fetch the actual image
    let image_response = client
        .get(&image_info.url)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .context("Failed to fetch image from fal.ai")?;

    let elapsed = start.elapsed().as_secs_f64();

    if !image_response.status().is_success() {
        anyhow::bail!("Failed to fetch image: {}", image_response.status());
    }

    let image_data = image_response
        .bytes()
        .await
        .context("Failed to read image bytes")?
        .to_vec();

    Ok(GenerationResult {
        image_data,
        seed: data.seed.map(|s| s.to_string()),
        generation_time_seconds: elapsed,
        cost_usd: None, // fal.ai doesn't return token-based billing
    })
}
