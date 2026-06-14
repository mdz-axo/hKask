//! Embedded Jinja2 prompt templates for media tools.
//!
//! Templates are compiled into the binary at build time.
//! Rendered at runtime with tool parameters via minijinja.

use minijinja::Environment;
use std::collections::HashMap;

/// Initialize the template environment with all media prompt templates.
pub fn create_env() -> Environment<'static> {
    let mut env = Environment::new();
    env.add_template("tag_faces", TAG_FACES).ok();
    env.add_template("tag_objects", TAG_OBJECTS).ok();
    env.add_template("tag_colors", TAG_COLORS).ok();
    env.add_template("tag_composition", TAG_COMPOSITION).ok();
    env.add_template("describe_scene", DESCRIBE_SCENE).ok();
    env.add_template("classify_style", CLASSIFY_STYLE).ok();
    env.add_template("caption", CAPTION).ok();
    env.add_template(
        "voice_design",
        include_str!("../manifests/media/voice_design.j2"),
    )
    .ok();
    env.add_template("video_caption", VIDEO_CAPTION).ok();
    env
}

/// Render a template with string key-value variables.
pub fn render(env: &Environment, name: &str, vars: &HashMap<&str, &str>) -> Result<String, String> {
    let tmpl = env
        .get_template(name)
        .map_err(|e| format!("Template '{}' not found: {}", name, e))?;
    tmpl.render(vars)
        .map_err(|e| format!("Render error for '{}': {}", name, e))
}

// ── Embedded templates ──────────────────────────────────────────────────────

const TAG_FACES: &str = r#"Analyze this image and detect all visible human faces.

{% if detail_level == "basic" %}
For each face, provide position and estimated age range.
Return ONLY a JSON array. Each element: face_index, age_range, position.
{% else %}
For each face, provide:
1. Estimated age range (e.g., "25-35", "child", "elderly")
2. Apparent gender presentation
3. Notable features (glasses, beard, expression, hair color/style)
4. Position in image (e.g., "left third", "center-right", "foreground")
5. Approximate face size relative to image (small / medium / large)

Return ONLY a JSON array. Each element: face_index, age_range, gender_presentation, features, position, size.
{% endif %}"#;

const TAG_OBJECTS: &str = r#"Analyze this image and detect all visible objects.

{% if detail_level == "detailed" %}
For each object, provide:
1. Object name (be specific — e.g., "golden retriever" not "dog")
2. Bounding box description (e.g., "upper-left", "center", "lower-right")
3. Confidence level (high / medium / low)
4. Brief description of appearance (color, size, condition, distinctive features)
{% else %}
List each object with its name and general location in the image.
{% endif %}

Limit to the {{ max_objects }} most prominent objects.

Return ONLY a JSON array. Each element: name, location, confidence{% if detail_level == "detailed" %}, description{% endif %}."#;

const TAG_COLORS: &str = r##"Analyze this image and identify its color palette.

For each dominant color, provide:
1. Color name (e.g., "crimson", "teal", "warm amber")
2. Hex code (e.g., "#FF5733")
3. Approximate percentage of image area
4. Role in composition (primary, secondary, accent, background)

Also describe:
- Overall palette style (monochromatic, complementary, analogous, triadic, warm, cool, neutral)
- Color temperature (warm-dominant, cool-dominant, balanced)
- Saturation level (vibrant, muted, desaturated)

Limit to the {{ max_colors }} most dominant colors.

Return ONLY a JSON object with these fields:
- colors: array of {name, hex, percentage, role}
- palette_style: string
- temperature: string
- saturation: string"##;

const TAG_COMPOSITION: &str = r#"Analyze the photographic composition of this image.

Evaluate these compositional elements:
1. Focal point — what draws the eye first? Where is it positioned?
2. Rule of thirds — does the composition follow it? Describe the grid placement.
3. Leading lines — are there lines guiding the viewer's eye? Describe them.
4. Depth of field — shallow or deep? What is in focus vs blurred?
5. Perspective — eye-level, low angle, high angle, bird's eye, worm's eye?
6. Framing — natural frames (doorways, arches, foliage) within the image?
7. Symmetry — is the composition symmetrical, asymmetrical, or balanced?
8. Negative space — how is empty space used?

Return ONLY a JSON object with these fields:
- focal_point: string
- rule_of_thirds: string
- leading_lines: string
- depth_of_field: string
- perspective: string
- framing: string
- symmetry: string
- negative_space: string"#;

const DESCRIBE_SCENE: &str = r#"{% if style == "descriptive" %}
Describe this image in detail. Cover the subject, setting, lighting, colors, composition, mood, and any notable details. Write 2-4 sentences.

{% elif style == "artistic" %}
Write an artistic, evocative description of this image. Use poetic language and focus on mood, emotion, and aesthetic quality. Write 2-3 sentences.

{% elif style == "technical" %}
Provide a technical description of this image. Note the photographic/compositional elements: focal point, depth of field, lighting conditions, color palette, perspective, and any post-processing effects visible. Write 2-4 sentences.

{% elif style == "alt_text" %}
Write concise alt text for this image suitable for accessibility. Describe only what is visually present — no interpretation. Keep to 1-2 sentences, max 125 characters.

{% endif %}

Return ONLY the description text. No markdown, no preamble, no labels."#;

const CLASSIFY_STYLE: &str = r#"Analyze this image and classify its photographic style.

{% if categories %}
Classify into these categories (an image can belong to multiple): {{ categories }}
{% else %}
Evaluate these dimensions:
- Genre: portrait, landscape, street, macro, architecture, documentary, abstract, still life, wildlife, fashion, food, sports, aerial, underwater, astrophotography
- Era/Style: contemporary, vintage, film-grain, HDR, minimalist, maximalist, surreal, photorealistic, painterly, noir, pastel, neon, grunge
- Technique: long exposure, bokeh, tilt-shift, double exposure, infrared, black-and-white, sepia, cross-processed
{% endif %}

For each matching category, provide:
- category: string
- confidence: number (0.0 to 1.0)

Return ONLY a JSON array. Each element: category, confidence."#;

const CAPTION: &str = r#"{% if style == "descriptive" %}
Describe this image in detail. Cover the subject, setting, lighting, colors, composition, mood, and any notable details. Write 2-4 sentences.

{% elif style == "artistic" %}
Write an artistic, evocative caption for this image. Use poetic language and focus on mood, emotion, and aesthetic quality. Write 2-3 sentences.

{% elif style == "technical" %}
Provide a technical description of this image. Note the photographic/compositional elements: focal point, depth of field, lighting conditions, color palette, perspective, and any post-processing effects visible. Write 2-4 sentences.

{% elif style == "alt_text" %}
Write concise alt text for this image suitable for accessibility. Describe only what is visually present — no interpretation. Keep to 1-2 sentences, max 125 characters.

{% endif %}

Return ONLY the caption text. No markdown, no preamble, no labels."#;

const VIDEO_CAPTION: &str = r#"You are viewing keyframes extracted from a short video clip, shown in chronological order.

{% if style == "descriptive" %}
Describe what happens in this video. Cover the subject, action, setting, and any notable visual elements. Write 3-5 sentences covering the full sequence.
{% elif style == "summary" %}
Write a concise 1-2 sentence summary of what this video shows.
{% elif style == "hashtags" %}
Generate 5-10 relevant hashtags for this video content. Each should start with # and be a single concept. Focus on discoverability.
{% endif %}

Return ONLY the text. No markdown, no preamble, no labels."#;
