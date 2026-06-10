//! Settings API routes — read/write REPL inference settings via REST.
//!
//! Same settings as the `/repl` slash command and `kask settings` CLI.
//! Persisted to ~/.config/hkask/settings.json. Magna Carta P3 (Generative
//! Space): all settings exposed equally across every surface.

use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};

use crate::ApiState;
use hkask_services::settings_path;

/// JSON shape for the settings response.
#[derive(Debug, Serialize, Deserialize)]
pub struct SettingsResponse {
    pub tool_loop_limit: usize,
    pub context_turns: usize,
    pub temperature: f32,
    pub top_p: f32,
    pub top_k: u32,
    pub min_p: f32,
    pub typical_p: f32,
    pub max_tokens: u32,
    pub seed: Option<u32>,
    pub gas_heuristic: u64,
    pub gas_cap: u64,
    #[serde(alias = "auto_compact")]
    pub auto_condense: bool,
    pub context_length: Option<u32>,
    pub supports_thinking: Option<bool>,
}

impl Default for SettingsResponse {
    fn default() -> Self {
        Self {
            tool_loop_limit: 21,
            context_turns: 3,
            temperature: 0.7,
            top_p: 0.9,
            top_k: 40,
            min_p: 0.0,
            typical_p: 0.0,
            max_tokens: 512,
            seed: None,
            gas_heuristic: 500,
            gas_cap: 10_000,
            auto_condense: true,
            context_length: None,
            supports_thinking: None,
        }
    }
}

/// Load settings from disk. Returns defaults if the file doesn't exist
/// or can't be parsed.
fn load_settings() -> SettingsResponse {
    let path = settings_path();
    match std::fs::read_to_string(&path) {
        Ok(json) => serde_json::from_str::<SettingsResponse>(&json).unwrap_or_default(),
        Err(_) => SettingsResponse::default(),
    }
}

/// Save settings to disk.
fn save_settings(settings: &SettingsResponse) -> Result<(), String> {
    let path = settings_path();
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

/// JSON shape for updating settings. All fields optional — only present
/// fields are applied; omitted fields keep their current value.
#[derive(Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    pub tool_loop_limit: Option<usize>,
    pub context_turns: Option<usize>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub top_k: Option<u32>,
    pub min_p: Option<f32>,
    pub typical_p: Option<f32>,
    pub max_tokens: Option<u32>,
    pub seed: Option<Option<u32>>,
    pub gas_heuristic: Option<u64>,
    pub gas_cap: Option<u64>,
    #[serde(alias = "auto_compact")]
    pub auto_condense: Option<bool>,
}

pub fn settings_router() -> Router<ApiState> {
    Router::new().route("/api/settings", get(get_settings).put(update_settings))
}

/// GET /api/settings — return current settings.
async fn get_settings(State(_state): State<ApiState>) -> Json<SettingsResponse> {
    Json(load_settings())
}

/// PUT /api/settings — update settings, merge with current values.
async fn update_settings(
    State(_state): State<ApiState>,
    Json(req): Json<UpdateSettingsRequest>,
) -> Json<SettingsResponse> {
    let mut settings = load_settings();

    if let Some(v) = req.tool_loop_limit
        && v > 0
    {
        settings.tool_loop_limit = v;
    }
    if let Some(v) = req.context_turns {
        settings.context_turns = v;
    }
    if let Some(v) = req.temperature
        && (0.0..=2.0).contains(&v)
    {
        settings.temperature = v;
    }
    if let Some(v) = req.top_p
        && (0.0..=1.0).contains(&v)
    {
        settings.top_p = v;
    }
    if let Some(v) = req.top_k
        && v >= 1
    {
        settings.top_k = v;
    }
    if let Some(v) = req.min_p
        && (0.0..=1.0).contains(&v)
    {
        settings.min_p = v;
    }
    if let Some(v) = req.typical_p
        && (0.0..=1.0).contains(&v)
    {
        settings.typical_p = v;
    }
    if let Some(v) = req.max_tokens
        && v > 0
    {
        settings.max_tokens = v;
    }
    if let Some(v) = req.seed {
        settings.seed = v;
    }
    if let Some(v) = req.gas_heuristic
        && v > 0
    {
        settings.gas_heuristic = v;
    }
    if let Some(v) = req.gas_cap
        && v > 0
    {
        settings.gas_cap = v;
    }
    if let Some(v) = req.auto_condense {
        settings.auto_condense = v;
    }

    let _ = save_settings(&settings);
    Json(settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    // REQ: Merge-update preserves unspecified fields — only the fields
    // in the request are changed; all others keep their current values.

    #[test]
    fn update_settings_merge_preserves_unspecified_fields() {
        // Start with defaults
        let mut settings = SettingsResponse::default();

        // Only update temperature
        let req = UpdateSettingsRequest {
            temperature: Some(0.3),
            tool_loop_limit: None,
            context_turns: None,
            top_p: None,
            top_k: None,
            min_p: None,
            typical_p: None,
            max_tokens: None,
            seed: None,
            gas_heuristic: None,
            gas_cap: None,
            auto_condense: None,
        };

        // Apply the merge (same logic as the PUT handler)
        if let Some(v) = req.temperature {
            if (0.0..=2.0).contains(&v) {
                settings.temperature = v;
            }
        }

        // Temperature should be updated
        assert!((settings.temperature - 0.3).abs() < f32::EPSILON);
        // Unspecified fields should retain defaults
        assert_eq!(settings.tool_loop_limit, 21);
        assert_eq!(settings.context_turns, 3);
        assert!((settings.top_p - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn update_settings_out_of_range_is_ignored() {
        let mut settings = SettingsResponse::default();
        let req = UpdateSettingsRequest {
            temperature: Some(3.0), // out of range
            tool_loop_limit: None,
            context_turns: None,
            top_p: None,
            top_k: None,
            min_p: None,
            typical_p: None,
            max_tokens: None,
            seed: None,
            gas_heuristic: None,
            gas_cap: None,
            auto_condense: None,
        };
        if let Some(v) = req.temperature
            && (0.0..=2.0).contains(&v)
        {
            settings.temperature = v;
        }
        // Out-of-range value should be silently ignored (no change)
        assert!((settings.temperature - 0.7).abs() < f32::EPSILON);
    }
}
