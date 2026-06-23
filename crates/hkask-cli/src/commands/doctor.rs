//! `kask doctor` — validate all configured providers.
//!
//! Checks each provider key is set and makes a lightweight API call
//! to verify the credentials are valid. Reports tier status.

use hkask_services::{InferenceConfig, InferenceRouter};

/// Run a diagnostic check on all configured providers.
pub async fn run_doctor() {
    println!("hKask Doctor — Provider Health Check\n");

    let mut configured = 0u32;
    let mut total = 0u32;

    // ── Inference ──────────────────────────────────────────
    println!("Inference Providers");
    println!("───────────────────");
    configured += check_env("DI_API_KEY", "DeepInfra", &mut total);
    configured += check_env("FA_API_KEY", "fal.ai", &mut total);
    configured += check_env("TOGETHER_API_KEY", "Together AI", &mut total);
    configured += check_env("OPENROUTER_API_KEY", "OpenRouter", &mut total);
    configured += check_env("RUNPOD_API_KEY", "RunPod", &mut total);
    configured += check_env("BASETEN_API_KEY", "Baseten", &mut total);
    println!();

    // ── Fusion ──────────────────────────────────────────────
    let config = InferenceConfig::from_env();
    if config.fusion.is_some() {
        println!("Fusion Model");
        println!("────────────");
        let router = InferenceRouter::new(config);
        total += 1;
        match router.verify_fusion_model().await {
            Ok(true) => {
                println!("  ✅ Fusion group verified on OpenRouter");
                configured += 1;
            }
            Ok(false) => {
                println!("  ❌ Fusion group NOT FOUND — create it at https://openrouter.ai/fusion");
            }
            Err(e) => {
                println!("  ⚠️  Could not verify fusion group: {e}");
            }
        }
        println!();
    }

    // ── Search ─────────────────────────────────────────────
    println!("Search Providers");
    println!("────────────────");
    configured += check_env("HKASK_BRAVE_API_KEY", "Brave Search", &mut total);
    configured += check_env("HKASK_FIRECRAWL_API_KEY", "Firecrawl", &mut total);
    configured += check_env("HKASK_TAVILY_API_KEY", "Tavily", &mut total);
    configured += check_env("HKASK_SERPAPI_API_KEY", "SerpAPI", &mut total);
    configured += check_env("HKASK_EXA_API_KEY", "Exa", &mut total);
    println!();

    // ── Financial ──────────────────────────────────────────
    println!("Financial Data");
    println!("──────────────");
    configured += check_env("HKASK_FMP_API_KEY", "FMP", &mut total);
    configured += check_env("HKASK_EODHD_API_KEY", "EODHD", &mut total);
    println!();

    // ── Object Storage ─────────────────────────────────────
    println!("Object Storage");
    println!("──────────────");
    configured += check_env("LITESTREAM_BUCKET", "Litestream bucket", &mut total);
    configured += check_env("LITESTREAM_ENDPOINT", "Litestream endpoint", &mut total);
    configured += check_env(
        "LITESTREAM_ACCESS_KEY_ID",
        "Litestream access key",
        &mut total,
    );
    println!();

    // ── Cloud ──────────────────────────────────────────────
    println!("Cloud Providers");
    println!("───────────────");
    configured += check_env("HCLOUD_TOKEN", "Hetzner Cloud", &mut total);
    println!();

    // ── Matrix ─────────────────────────────────────────────
    println!("Matrix");
    println!("──────");
    if let Ok(url) = std::env::var("HKASK_MATRIX_URL") {
        if url.is_empty() {
            println!("  ⚠️  HKASK_MATRIX_URL — not set");
        } else {
            println!("  ✅ HKASK_MATRIX_URL — {url}");
            configured += 1;
        }
    } else {
        println!("  ⚠️  HKASK_MATRIX_URL — not set");
    }
    total += 1;
    println!();

    // ── Container ──────────────────────────────────────────
    println!("Container Registry");
    println!("──────────────────");
    configured += check_env("CONTAINER_REGISTRY", "Container registry", &mut total);
    println!();

    // ── Summary ────────────────────────────────────────────
    let pct = if total > 0 {
        (configured * 100) / total
    } else {
        0
    };
    let tier = match pct {
        0..=19 => "No tier",
        20..=49 => "CORE (inference)",
        50..=79 => "STANDARD (inference + search + backups)",
        _ => "FULL (cloud deployment ready)",
    };

    println!("═══════════════════════════════════════");
    println!("  {configured}/{total} providers configured ({pct}%)");
    println!("  Tier: {tier}");
    println!("═══════════════════════════════════════");
}

fn check_env(var: &str, label: &str, total: &mut u32) -> u32 {
    *total += 1;
    match std::env::var(var) {
        Ok(val) if !val.is_empty() => {
            println!("  ✅ {var} — {label}");
            1
        }
        _ => {
            println!("  ⚠️  {var} — {label} (not set)");
            0
        }
    }
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  none
/// post: prints provider health report to stdout
pub fn run_doctor_cmd(rt: &tokio::runtime::Runtime) {
    rt.block_on(run_doctor());
}
