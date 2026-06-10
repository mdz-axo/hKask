//! Settings API routes — read/write REPL inference settings via REST.
//!
//! Same settings as the `/repl` slash command and `kask settings` CLI.
//! Persisted to ~/.config/hkask/settings.json. Magna Carta P3 (Generative
//! Space): all settings exposed equally across every surface.

use axum::{Json, Router, extract::State, routing::get};
use serde::{Deserialize, Serialize};

use crate::ApiState;

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
    pub auto_compact: bool,
    pub context_length: Option<u32>,
    pub supports_thinking: Option<bool>,
}

impl From<hkask_cli::repl::handlers::ReplSettings> for SettingsResponse {
    fn from(s: hkask_cli::repl::handlers::ReplSettings) -> Self {
        Self {
            tool_loop_limit: s.tool_loop_limit,
            context_turns: s.context_turns,
            temperature: s.temperature,
            top_p: s.top_p,
            top_k: s.top_k,
            min_p: s.min_p,
            typical_p: s.typical_p,
            max_tokens: s.max_tokens,
            seed: s.seed,
            gas_heuristic: s.gas_heuristic,
            gas_cap: s.gas_cap,
            auto_compact: s.auto_compact,
            context_length: s.model_meta.as_ref().map(|m| m.context_length),
            supports_thinking: s.model_meta.as_ref().map(|m| m.supports_thinking),
        }
    }
}

/// JSON shape for updating settings.
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
    pub auto_compact: Option<bool>,
}

pub fn settings_router() -> Router<ApiState> {
    Router::new().route("/api/settings", get(get_settings).put(update_settings))
}

/// GET /api/settings — return current settings.
async fn get_settings(State(_state): State<ApiState>) -> Json<SettingsResponse> {
    let settings = hkask_cli::commands::settings::load_settings();
    Json(SettingsResponse::from(settings))
}

/// PUT /api/settings — update settings, merge with current values.
async fn update_settings(
    State(_state): State<ApiState>,
    Json(req): Json<UpdateSettingsRequest>,
) -> Json<SettingsResponse> {
    let mut settings = hkask_cli::commands::settings::load_settings();

    if let Some(v) = req.tool_loop_limit {
        if v > 0 {
            settings.tool_loop_limit = v;
        }
    }
    if let Some(v) = req.context_turns {
        settings.context_turns = v;
    }
    if let Some(v) = req.temperature {
        if (0.0..=2.0).contains(&v) {
            settings.temperature = v;
        }
    }
    if let Some(v) = req.top_p {
        if (0.0..=1.0).contains(&v) {
            settings.top_p = v;
        }
    }
    if let Some(v) = req.top_k {
        if v >= 1 {
            settings.top_k = v;
        }
    }
    if let Some(v) = req.min_p {
        if (0.0..=1.0).contains(&v) {
            settings.min_p = v;
        }
    }
    if let Some(v) = req.typical_p {
        if (0.0..=1.0).contains(&v) {
            settings.typical_p = v;
        }
    }
    if let Some(v) = req.max_tokens {
        if v > 0 {
            settings.max_tokens = v;
        }
    }
    if let Some(v) = req.seed {
        settings.seed = v;
    }
    if let Some(v) = req.gas_heuristic {
        if v > 0 {
            settings.gas_heuristic = v;
        }
    }
    if let Some(v) = req.gas_cap {
        if v > 0 {
            settings.gas_cap = v;
        }
    }
    if let Some(v) = req.auto_compact {
        settings.auto_compact = v;
    }

    // Persist
    let path = hkask_cli::repl::handlers::repl_settings::settings_path();
    if let Ok(json) = serde_json::to_string_pretty(&settings) {
        let _ = std::fs::write(&path, json);
    }

    Json(SettingsResponse::from(settings))
}
