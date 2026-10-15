use chrono::{Duration, Local, NaiveDate};
use serde::{Deserialize, Serialize};

/// Supported image generation providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Gemini,
    Fal,
    OpenAI,
    SelfHosted,
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Provider::Gemini => write!(f, "gemini"),
            Provider::Fal => write!(f, "fal"),
            Provider::OpenAI => write!(f, "openai"),
            Provider::SelfHosted => write!(f, "selfhosted"),
        }
    }
}

impl std::str::FromStr for Provider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "gemini" => Ok(Provider::Gemini),
            "fal" => Ok(Provider::Fal),
            "openai" => Ok(Provider::OpenAI),
            "selfhosted" => Ok(Provider::SelfHosted),
            _ => Err(format!("Unknown provider: {}", s)),
        }
    }
}

/// Available models per provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub provider: Provider,
    pub display_name: String,
    pub cost_per_image: f64,
    /// Max reference images supported (0 = text-to-image only)
    pub max_refs: u32,
}

/// Prompting guide for a model or model family
#[derive(Debug, Clone)]
pub struct PromptingGuide {
    pub model_pattern: &'static str,
    pub style: &'static str,
    pub required_prefix: Option<&'static str>,
    pub structure: &'static str,
    pub tips: &'static str,
    pub avoid: Option<&'static str>,
    pub negative_template: Option<&'static str>,
    pub settings: Option<&'static str>,
    pub example: &'static str,
}

impl PromptingGuide {
    pub fn all() -> Vec<PromptingGuide> {
        vec![
            // Gemini models - prose-based
            PromptingGuide {
                model_pattern: "gemini",
                style: "prose",
                required_prefix: None,
                structure: "Character description → Setting/atmosphere → Art style cues → Technical specs (optional)",
                tips: r#"- Long prose, poetic, allusive descriptions work well
- Full paragraphs welcome - more context = better results
- Era cues: "Dreamcast era", "pre-shader", "Jet Set Radio", "Space Channel 5"
- Style cues: "faceted geometry", "flat cel shading", "Frutiger Aero palette"
- Emotional tone: "optimistic melancholy", "gently futuristic""#,
                avoid: Some("anime, screenshot, visual novel, cute"),
                negative_template: None,
                settings: None,
                example: r#"A young woman with short silver hair and distant eyes, as if hiding something beneath her cheerful exterior. She has the aesthetic of a Sega Dreamcast era 3D character - faceted geometry, flat cel shading, that pre-shader optimism. Her outfit suggests a near-future that never quite arrived: holographic accents on practical clothing. The background is a rain-slicked plaza at twilight, neon signs reflecting in puddles. Frutiger Aero color palette - teals, warm oranges, translucent whites."#,
            },
            // Animagine - strict booru tags
            PromptingGuide {
                model_pattern: "animagine",
                style: "tags",
                required_prefix: None,
                structure: "[count], [character], [series], [rating], [appearance], [outfit], [pose], [expression], [setting], [year], [quality]",
                tips: r#"- STRICT booru tag format - natural language WILL FAIL
- Comma-separated tags only, no sentences
- Use underscores for multi-word tags: long_hair, blue_eyes

RATING TAGS (content safety):
  safe, sensitive, nsfw, explicit

YEAR TAGS (anime era/style):
  year 2005  - early 2000s style (Code Geass, Death Note)
  year 2000  - late 90s style (NGE, Cowboy Bebop adjacent)
  year 2015  - mid 2010s style
  year 2023  - contemporary anime style

SCORE TAGS (quality control, strongest to weakest):
  high score > great score > good score > average score > bad score > low score
  Use lower scores intentionally for rougher/sketchier aesthetics

QUALITY TAGS (add at end):
  masterpiece, best quality, absurdres  - highest quality
  low quality, worst quality            - for negative prompt

RESOLUTIONS (SDXL native):
  Square:    1024x1024 (1:1)
  Portrait:  832x1216 (2:3), 768x1344 (4:7)
  Landscape: 1216x832 (3:2), 1344x768 (7:4)

LIMITATIONS:
  - Hands/fingers often malformed - check outputs carefully
  - No text rendering - don't request text in images
  - Recent characters have lower accuracy (limited training data)
  - Multiple characters need careful prompt engineering
  - Above 1536x1536 degrades quality (trained on SDXL resolution)
  - Style consistency requires explicit style tags"#,
                avoid: Some("natural language, sentences, prose, text in images, resolutions above 1536x1536"),
                negative_template: Some("lowres, bad anatomy, bad hands, text, error, missing finger, extra digits, fewer digits, cropped, worst quality, low quality, low score, bad score, average score, signature, watermark, username, blurry"),
                settings: Some("CFG: 4-7 (5 recommended), Steps: 25-28, Sampler: Euler a"),
                example: "1girl, solo, long silver hair, blue eyes, black hoodie, standing, smile, city background, night, year 2015, safe, masterpiece, high score, great score, absurdres",
            },
            // Pony - hybrid natural + tags
            PromptingGuide {
                model_pattern: "pony",
                style: "hybrid",
                required_prefix: Some("score_9, score_8_up, score_7_up, score_6_up, score_5_up, score_4_up"),
                structure: "[score tags], [source tag], [rating], [description or tags]",
                tips: r#"- CRITICAL: CLIP Skip MUST be 2 or output is garbage (low quality blobs)
- Always start with full score tag chain (training quirk, works better than score_9 alone)
- Natural language capable - understands both prose and tags
- No negative prompts needed in most cases

SOURCE TAGS (style control):
  source_anime    - anime style (add to fight Western art bias)
  source_cartoon  - Western cartoon style
  source_furry    - furry art style
  source_pony     - MLP style

RATING TAGS:
  rating_safe, rating_questionable, rating_explicit

QUALITY NOTES:
  - Avoid extra quality modifiers like "hd" or "masterpiece" - score tags handle this
  - Full score chain works better than abbreviated versions

RESOLUTIONS:
  1024x1024 recommended, flexible with SDXL resolutions

LIMITATIONS:
  - Sometimes generates pseudo-signatures hard to remove even with negatives
  - If signatures persist, try inpainting or V5.5"#,
                avoid: Some("hd, masterpiece, best quality (score tags replace these)"),
                negative_template: Some("source_cartoon, source_furry, source_pony (for anime style)"),
                settings: Some("CFG: 7, Steps: 25, Sampler: Euler a, CLIP Skip: 2 (CRITICAL)"),
                example: "score_9, score_8_up, score_7_up, score_6_up, score_5_up, score_4_up, source_anime, rating_safe, 1girl, silver hair, blue eyes, standing in rain, city night, neon lights reflecting on wet pavement",
            },
            // FLUX 2 HDR Style - LoRA gallery model
            PromptingGuide {
                model_pattern: "flux2-hdr",
                style: "prose",
                required_prefix: None,
                structure: "Just describe your subject — the HDR trigger word (Hyp3rRe4list1c) is auto-prepended",
                tips: r#"- Trigger word 'Hyp3rRe4list1c' is AUTOMATICALLY added — do NOT include it in your prompt
- Simply describe the subject; the model handles dramatic HDR lighting/color
- Best for: product photography, fantasy landscapes, cinematic scenes
- Adjust LoRA scale (0-2) to control HDR intensity: lower = subtle, higher = surreal
- Default 40 inference steps; quality is high out of the box
- Supports standard FLUX aspect ratios"#,
                avoid: Some("including the trigger word manually (it's auto-prepended), overly technical HDR instructions (the model handles this)"),
                negative_template: None,
                settings: Some("Steps: 40 (default), LoRA Scale: 1.0 (default, range 0-2), Guidance: model default"),
                example: "A weathered lighthouse on a rocky cliff at golden hour, waves crashing against the rocks below, dramatic clouds stretching across the sky",
            },
            // FLUX 2 base models (turbo, pro, max)
            PromptingGuide {
                model_pattern: "flux2",
                style: "prose",
                required_prefix: None,
                structure: "Natural language description of the desired image",
                tips: r#"- Natural prose prompting — describe what you want to see
- FLUX 2 has strong text rendering — include exact text in quotes if needed
- Three tiers with different tradeoffs:
  flux2-turbo: Cheapest ($0.008/MP), fast, good for iteration and drafts
  flux2-pro:   Mid-range ($0.03/MP), zero-config, consistent production quality
  flux2-max:   Premium ($0.07/MP), zero-config, highest quality and realism
- Pro and Max have NO tunable parameters (no steps, no guidance) — quality is automatic
- Turbo supports guidance_scale (default 2.5) and inference steps (default 28)
- All support seed for reproducibility
- HEX color codes in prompt can guide palette (e.g. "background in #2D1B69")"#,
                avoid: Some("booru tags (use prose instead), overly short prompts (more detail = better results)"),
                negative_template: None,
                settings: Some("Turbo: Steps 28, Guidance 2.5 (adjustable) | Pro/Max: no user-facing settings"),
                example: "A cozy Japanese coffee shop interior at dusk, warm amber lighting from paper lanterns, a steaming cup of matcha latte on a worn wooden counter, rain visible through the window, soft bokeh lights from the street outside",
            },
            // NoobAI - v-prediction model, different from epsilon models
            PromptingGuide {
                model_pattern: "noobai",
                style: "hybrid",
                required_prefix: Some("masterpiece, best quality, newest, absurdres, highres"),
                structure: "[quality], [count], [character], [series], [artists], [special tags], [general tags]",
                tips: r#"- V-PREDICTION MODEL - different architecture from Pony/Animagine
- Handles both tags and natural language
- CRITICAL: Use Euler or DDIM sampler ONLY - Karras samplers DO NOT WORK

QUALITY TAGS (percentile-based):
  masterpiece     - >95th percentile
  best quality    - 85-95th percentile
  good quality    - 60-85th percentile
  normal quality  - 30-60th percentile
  worst quality   - <30th percentile (for negative)

AESTHETIC TAGS:
  very awa        - top 5% aesthetic scores
  worst aesthetic - bottom 5% (for negative)

DATE/ERA TAGS:
  newest          - 2021-2024 (recommended default)
  recent          - 2018-2020
  mid             - 2014-2017
  early           - 2011-2014
  old             - 2005-2010
  Or use: year 2021, year 2019, etc.

RESOLUTIONS (SDXL, ~1M pixels total):
  Portrait:  832x1216 (optimal), 768x1344
  Square:    1024x1024
  Landscape: 1216x832, 1344x768

LIMITATIONS:
  - NO Karras samplers (v-prediction incompatible)
  - Requires Euler or DDIM only
  - Dynamic CFG 0.2 helps prevent oversaturation"#,
                avoid: Some("Karras samplers, DPM++ Karras, resolutions far from 1M pixels"),
                negative_template: Some("nsfw, worst quality, old, early, low quality, lowres, signature, username, logo, bad hands, mutated hands, mammal, anthro, furry, ambiguous form, feral, semi-anthro"),
                settings: Some("CFG: 4-5, Steps: 28-35, Sampler: Euler (REQUIRED, no Karras), Dynamic CFG: 0.2 (optional)"),
                example: "masterpiece, best quality, newest, absurdres, highres, safe, 1girl, silver hair, blue eyes, black jacket, standing on rooftop, city skyline, sunset, dramatic lighting, year 2023",
            },
        ]
    }

    /// Find guide for a model ID (matches by prefix)
    pub fn for_model(model_id: &str) -> Option<PromptingGuide> {
        Self::all()
            .into_iter()
            .find(|g| model_id.starts_with(g.model_pattern))
    }

    /// Format guide for CLI output
    pub fn format(&self) -> String {
        let mut out = String::new();

        out.push_str(&format!("PROMPTING GUIDE: {}\n", self.model_pattern.to_uppercase()));
        out.push_str(&format!("Style: {}\n\n", self.style));

        if let Some(prefix) = self.required_prefix {
            out.push_str(&format!("REQUIRED PREFIX:\n{}\n\n", prefix));
        }

        out.push_str(&format!("STRUCTURE:\n{}\n\n", self.structure));
        out.push_str(&format!("TIPS:\n{}\n\n", self.tips));

        if let Some(avoid) = self.avoid {
            out.push_str(&format!("AVOID:\n{}\n\n", avoid));
        }

        if let Some(neg) = self.negative_template {
            out.push_str(&format!("NEGATIVE PROMPT TEMPLATE:\n{}\n\n", neg));
        }

        if let Some(settings) = self.settings {
            out.push_str(&format!("RECOMMENDED SETTINGS:\n{}\n\n", settings));
        }

        out.push_str(&format!("EXAMPLE:\n{}\n", self.example));

        out
    }
}

impl ModelInfo {
    pub fn all() -> Vec<ModelInfo> {
        vec![
            // Gemini models - support multiple reference images
            ModelInfo {
                id: "gemini-flash".into(),
                provider: Provider::Gemini,
                display_name: "Gemini 2.5 Flash".into(),
                cost_per_image: 0.039,
                max_refs: 10,
            },
            ModelInfo {
                id: "gemini-pro".into(),
                provider: Provider::Gemini,
                display_name: "Gemini 3 Pro".into(),
                cost_per_image: 0.134,
                max_refs: 10,
            },
            // fal.ai models - text-to-image only (no ref support)
            ModelInfo {
                id: "fal-ai/flux/schnell".into(),
                provider: Provider::Fal,
                display_name: "FLUX Schnell".into(),
                cost_per_image: 0.003,
                max_refs: 0,
            },
            ModelInfo {
                id: "fal-ai/flux-pro/v1.1".into(),
                provider: Provider::Fal,
                display_name: "FLUX Pro 1.1".into(),
                cost_per_image: 0.05,
                max_refs: 0,
            },
            ModelInfo {
                id: "fal-ai/flux-pro/v1.1-ultra".into(),
                provider: Provider::Fal,
                display_name: "FLUX Pro 1.1 Ultra".into(),
                cost_per_image: 0.06,
                max_refs: 0,
            },
            ModelInfo {
                id: "fal-ai/recraft-v3".into(),
                provider: Provider::Fal,
                display_name: "Recraft V3".into(),
                cost_per_image: 0.04,
                max_refs: 0,
            },
            ModelInfo {
                id: "flux2-turbo".into(),
                provider: Provider::Fal,
                display_name: "FLUX 2 Turbo".into(),
                cost_per_image: 0.008,
                max_refs: 4,
            },
            ModelInfo {
                id: "flux2-pro".into(),
                provider: Provider::Fal,
                display_name: "FLUX 2 Pro".into(),
                cost_per_image: 0.03,
                max_refs: 4,
            },
            ModelInfo {
                id: "flux2-max".into(),
                provider: Provider::Fal,
                display_name: "FLUX 2 Max".into(),
                cost_per_image: 0.07,
                max_refs: 4,
            },
            ModelInfo {
                id: "flux2-hdr".into(),
                provider: Provider::Fal,
                display_name: "FLUX 2 HDR Style".into(),
                cost_per_image: 0.021,
                max_refs: 0,
            },
            ModelInfo {
                id: "imagen4".into(),
                provider: Provider::Fal,
                display_name: "Imagen 4 (Preview)".into(),
                cost_per_image: 0.04,
                max_refs: 0,
            },
            ModelInfo {
                id: "imagen4-fast".into(),
                provider: Provider::Fal,
                display_name: "Imagen 4 Fast".into(),
                cost_per_image: 0.04,
                max_refs: 0,
            },
            ModelInfo {
                id: "imagen4-ultra".into(),
                provider: Provider::Fal,
                display_name: "Imagen 4 Ultra".into(),
                cost_per_image: 0.06,
                max_refs: 0,
            },
            ModelInfo {
                id: "nano-banana-2".into(),
                provider: Provider::Fal,
                display_name: "Nano Banana 2".into(),
                cost_per_image: 0.08,
                max_refs: 0,
            },
            // Z-Image Turbo: $0.005/MP. Routes to image-to-image endpoint when ref provided.
            // Max 1 reference image.
            ModelInfo {
                id: "fal-ai/z-image/turbo".into(),
                provider: Provider::Fal,
                display_name: "Z-Image Turbo".into(),
                cost_per_image: 0.005,
                max_refs: 1,
            },
            // OpenAI models - text-to-image only
            ModelInfo {
                id: "dall-e-3".into(),
                provider: Provider::OpenAI,
                display_name: "DALL-E 3".into(),
                cost_per_image: 0.04,
                max_refs: 0,
            },
            ModelInfo {
                id: "gpt-image-1".into(),
                provider: Provider::OpenAI,
                display_name: "GPT Image 1".into(),
                cost_per_image: 0.02,
                max_refs: 0,
            },
            // Token-billed ($30/1M image output); quality is left on `auto`, so this is the
            // ~medium-quality 1024x1024 estimate (low ≈ $0.006, high ≈ $0.21).
            ModelInfo {
                id: "gpt-image-2".into(),
                provider: Provider::OpenAI,
                display_name: "GPT Image 2".into(),
                cost_per_image: 0.05,
                max_refs: 0,
            },
            // Self-hosted models - requires SELFHOSTED_API_URL or GUI settings
            // IP-Adapter supports 1 reference image
            ModelInfo {
                id: "animagine".into(),
                provider: Provider::SelfHosted,
                display_name: "Animagine XL 4.0 (Local)".into(),
                cost_per_image: 0.0,
                max_refs: 1,
            },
            ModelInfo {
                id: "pony".into(),
                provider: Provider::SelfHosted,
                display_name: "Pony Diffusion V6 (Local)".into(),
                cost_per_image: 0.0,
                max_refs: 1,
            },
            ModelInfo {
                id: "noobai".into(),
                provider: Provider::SelfHosted,
                display_name: "NoobAI XL (Local)".into(),
                cost_per_image: 0.0,
                max_refs: 1,
            },
        ]
    }

    pub fn find(model_id: &str) -> Option<ModelInfo> {
        Self::all().into_iter().find(|m| m.id == model_id)
    }

    pub fn provider_for_model(model_id: &str) -> Option<Provider> {
        Self::find(model_id).map(|m| m.provider)
    }
}

/// A single image generation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Generation {
    pub id: i64,
    pub slug: String,
    pub prompt: String,
    pub model: String,
    pub provider: String,
    pub timestamp: String,
    pub date: String,
    pub image_path: String,
    pub thumb_path: Option<String>,
    pub generation_time_seconds: Option<f64>,
    pub cost_estimate_usd: Option<f64>,
    pub seed: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub file_size: Option<i64>,
    pub parent_id: Option<i64>,
    pub starred: bool,
    pub created_at: String,
    pub trashed_at: Option<String>,
    pub title: Option<String>,
    pub negative_prompt: Option<String>,
    pub tags: Vec<String>,
    pub references: Vec<Reference>,
    pub collection_names: Vec<String>,
}

/// Parameters for generating a new image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateParams {
    pub prompt: String,
    pub model: String,
    pub tags: Vec<String>,
    pub reference_paths: Vec<String>,
    #[serde(default)]
    pub reference_source_ids: Vec<Option<i64>>,
    pub copy_to: Option<String>,
    pub negative_prompt: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
}

/// Reference image (deduplicated by hash)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reference {
    pub id: i64,
    pub hash: String,
    pub path: String,
    pub created_at: String,
    pub source_generation_id: Option<i64>,
}

/// Tag with usage count
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagCount {
    pub name: String,
    pub count: i64,
}

/// Cost summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostSummary {
    pub total_usd: f64,
    pub by_model: Vec<(String, f64)>,
    pub by_day: Vec<(String, f64)>,
    pub count: i64,
}

/// Query filters for listing generations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListFilter {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub tags: Option<Vec<String>>,
    pub exclude_tags: Option<Vec<String>>,
    pub model: Option<String>,
    pub starred_only: bool,
    pub search: Option<String>,
    pub since: Option<String>,
    pub collection_id: Option<i64>,
    pub show_trashed: bool,
    pub uncategorized: bool,
}

/// Result of image generation from a provider
#[derive(Debug)]
pub struct GenerationResult {
    pub image_data: Vec<u8>,
    pub seed: Option<String>,
    pub generation_time_seconds: f64,
    /// Actual cost from API (token-based), if available. Takes precedence over estimate.
    pub cost_usd: Option<f64>,
}

/// Job status for generation tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobStatus::Pending => write!(f, "pending"),
            JobStatus::Running => write!(f, "running"),
            JobStatus::Completed => write!(f, "completed"),
            JobStatus::Failed => write!(f, "failed"),
        }
    }
}

impl std::str::FromStr for JobStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(JobStatus::Pending),
            "running" => Ok(JobStatus::Running),
            "completed" => Ok(JobStatus::Completed),
            "failed" => Ok(JobStatus::Failed),
            _ => Err(format!("Unknown job status: {}", s)),
        }
    }
}

/// Source of a generation job
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobSource {
    Cli,
    Gui,
}

impl std::fmt::Display for JobSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobSource::Cli => write!(f, "cli"),
            JobSource::Gui => write!(f, "gui"),
        }
    }
}

impl std::str::FromStr for JobSource {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "cli" => Ok(JobSource::Cli),
            "gui" => Ok(JobSource::Gui),
            _ => Err(format!("Unknown job source: {}", s)),
        }
    }
}

/// A generation job record for tracking in-flight generations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: i64,
    pub status: JobStatus,
    pub model: String,
    pub prompt: String,
    pub tags: Option<Vec<String>>,
    pub source: JobSource,
    pub ref_count: i32,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub generation_id: Option<i64>,
    pub error: Option<String>,
}

/// Resolve a user-friendly aspect ratio name to pixel dimensions (SDXL native ~1M pixels)
pub fn resolve_aspect_ratio(ratio: &str) -> Option<(i32, i32)> {
    match ratio {
        "square" | "1:1" => Some((1024, 1024)),
        "portrait" | "2:3" => Some((832, 1216)),
        "landscape" | "3:2" => Some((1216, 832)),
        "wide" | "16:9" => Some((1344, 768)),
        "tall" | "9:16" => Some((768, 1344)),
        "4:3" => Some((1152, 896)),
        "3:4" => Some((896, 1152)),
        _ => None,
    }
}

/// A named collection (project folder) for grouping generations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub count: i64,
}

/// Parse a "since" string (e.g., "7d", "30d", "today", "all") into a date string.
/// Returns None for "all" or missing input.
pub fn parse_since(since: &str) -> Result<Option<String>, String> {
    if since == "all" {
        return Ok(None);
    }

    let now = Local::now().date_naive();

    if since == "today" {
        return Ok(Some(now.format("%Y-%m-%d").to_string()));
    }

    if since.ends_with('d') {
        let days: i64 = since[..since.len() - 1]
            .parse()
            .map_err(|_| "Invalid days format".to_string())?;
        let date = now - Duration::days(days);
        return Ok(Some(date.format("%Y-%m-%d").to_string()));
    }

    if since.ends_with('w') {
        let weeks: i64 = since[..since.len() - 1]
            .parse()
            .map_err(|_| "Invalid weeks format".to_string())?;
        let date = now - Duration::weeks(weeks);
        return Ok(Some(date.format("%Y-%m-%d").to_string()));
    }

    // Try parsing as a date
    if let Ok(date) = NaiveDate::parse_from_str(since, "%Y-%m-%d") {
        return Ok(Some(date.format("%Y-%m-%d").to_string()));
    }

    Err("Invalid since format. Use 'today', '7d', '2w', or 'YYYY-MM-DD'".to_string())
}
