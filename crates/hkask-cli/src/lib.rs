//! hKask CLI — Command-line interface

pub mod bootstrap;
pub mod cli;
pub mod commands;
pub mod onboarding;
pub mod repl;
pub mod transcript_viewer;

/// Extract the ElevenLabs voice preset name from a VoiceDesign JSON string.
/// Falls back to "Rachel" if parsing fails.
pub fn voice_preset_from_design(vd_json: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(vd_json) {
        Ok(design) => {
            if let Some(name) = design.get("elevenlabs_voice").and_then(|v| v.as_str()) {
                return name.to_string();
            }
            if let Some(name) = design.get("preset").and_then(|v| v.as_str()) {
                return name.to_string();
            }
            if let Some(name) = design.get("name").and_then(|v| v.as_str()) {
                return name.to_string();
            }
            "custom".to_string()
        }
        Err(_) => "Rachel".to_string(),
    }
}
