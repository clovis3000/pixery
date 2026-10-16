Providers
==========

API integration layer for external image generation services. Each provider has distinct quirks that aren't evident from code alone.

Gemini
----------

**DO NOT** assume model IDs are stable — Gemini renames models frequently. When calls start failing, check the Python backup at `~/tools/imagen-python-backup/` for last-known-working IDs. The `_ => model` fallback silently passes broken IDs through, so failures surface as API errors, not Rust errors.

**DO NOT** wrap base64 in data URIs — Gemini expects bare base64 strings with MIME type passed separately.

Seeds are not returned by the Gemini API (always NULL in database).

fal.ai
----------

**DO NOT** ignore 202 or 400 responses during polling — both can mean "still processing":
- 202 = queued, keep polling
- 400 with "still in progress" message = also keep polling (not an error)

The current 2-minute timeout (`MAX_POLL_ATTEMPTS * 500ms`) works for most models but slower ones may need adjustment.

**DO NOT** pass multiple references to Z-Image Turbo — max 1 reference for image-to-image. Code uses `.first()` but doesn't validate; extra refs are silently dropped.

**Z-Image endpoint routing**: `resolve_model()` routes to different API endpoints based on `has_reference`:
- `false` → `fal-ai/z-image/turbo` (text-to-image)
- `true` → `fal-ai/z-image/turbo/image-to-image`

This polymorphism is intentional but non-obvious. The `strength` parameter only applies to image-to-image; it's silently ignored for text-to-image.

**DO NOT** snap sizes to fal's named presets (`landscape_4_3` etc.) — pixery's 3:2/2:3 ratios match none of them, and the old threshold matcher silently fell back to square (fixed 2026-10-16). Send exact `{width, height}`.

**DO NOT** send `image_size` to a ratio-string endpoint — Imagen 4, Nano Banana 2 and FLUX 1.1 Pro Ultra ignore it and use their default ratio (Ultra did this unnoticed until 2026-10-16). A new fal model needs its schema checked (`https://fal.ai/api/openapi/queue/openapi.json?endpoint_id=<id>`) and, if ratio-based, an entry in `supported_aspect_ratios()`.

**DO** wrap base64 as data URIs — fal.ai expects `data:{mime};base64,{encoded}` format, unlike Gemini/OpenAI.

OpenAI
----------

**DO NOT** expect reference support — DALL-E 3, gpt-image-1 and gpt-image-2 ignore the `reference_paths` parameter entirely. No warning is logged; references are silently dropped. The parameter exists for interface consistency across providers. (gpt-image-2 supports `/v1/images/edits` upstream; pixery doesn't call it.)

**DO NOT** send `response_format` to `gpt-image-*` models — the API returns 400 "Unknown parameter: 'response_format'" (verified against gpt-image-2, 2026-10-15). GPT image models always return base64; only DALL-E accepts the param. `accepts_response_format()` gates it by model-ID prefix, so a future `gpt-image-*` model inherits the right behaviour.

**DO NOT** wrap base64 in data URIs — OpenAI expects bare base64 strings, like Gemini.

Seeds are not returned by the OpenAI API (always NULL in database).

Cross-Provider
----------

**Base64 format inconsistency** is intentional per each API's contract:
- fal.ai: `data:{mime};base64,{encoded}`
- Gemini: bare base64 + separate MIME type
- OpenAI: bare base64

A new provider must check which format that API expects.

**Seed availability varies**: fal.ai returns seeds, Gemini and OpenAI don't. Frontend can't assume seed will be populated.

**Image format detection** in `archive.rs` defaults to PNG when format is undetectable. If a provider returns an unusual format (WEBP, AVIF), verify the MIME type matches what gets saved.
