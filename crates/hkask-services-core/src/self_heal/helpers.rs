//! Self-healing helper functions — env value resolution and LLM response parsing.

use std::process::Command;

use super::types::{EnvValueSource, HealInstruction};

pub(super) fn resolve_env_value(source: &EnvValueSource) -> Result<String, String> {
    match source {
        EnvValueSource::Literal(v) => Ok(v.clone()),
        EnvValueSource::FromFile(p) => {
            if !p.exists() {
                return Ok(String::new());
            }
            std::fs::read_to_string(p)
                .map(|s| s.lines().next().unwrap_or("").trim().to_string())
                .map_err(|e| format!("Read {}: {}", p.display(), e))
        }
        EnvValueSource::FromCommand(cmd) => {
            let out = Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .output()
                .map_err(|e| format!("{}: {}", cmd, e))?;
            Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
        }
        EnvValueSource::FirstOf(sources) => {
            for s in sources {
                if let Ok(v) = resolve_env_value(s)
                    && !v.is_empty()
                {
                    return Ok(v);
                }
            }
            Ok(String::new())
        }
    }
}

pub(super) fn parse_llm_response(raw: &str) -> Result<Vec<HealInstruction>, String> {
    let t = raw.trim();
    if let Ok(v) = serde_json::from_str::<Vec<HealInstruction>>(t) {
        return Ok(v);
    }
    if let Ok(obj) = serde_json::from_str::<serde_json::Value>(t)
        && let Some(arr) = obj.get("actions").and_then(|v| v.as_array())
    {
        return arr
            .iter()
            .map(|v| serde_json::from_value(v.clone()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("{}", e));
    }
    for fence in &["```json", "```"] {
        if let Some(start) = t.find(fence) {
            let after = &t[start + fence.len()..];
            if let Some(end) = after.find("```")
                && let Ok(v) = serde_json::from_str::<Vec<HealInstruction>>(&after[..end])
            {
                return Ok(v);
            }
        }
    }
    Err(format!("Not valid JSON. Got: {}", &t[..t.len().min(200)]))
}
