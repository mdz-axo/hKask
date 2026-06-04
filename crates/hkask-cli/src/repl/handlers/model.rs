//! REPL /model handler — model listing, switching, and fuzzy search

pub(crate) fn handle_model(
    arg1: &str,
    rt: &tokio::runtime::Handle,
    state: &mut super::super::ReplState,
) {
    use hkask_templates::search_okapi_models;

    if arg1.is_empty() || arg1.eq_ignore_ascii_case("list") {
        if arg1.eq_ignore_ascii_case("list") {
            let matches = rt.block_on(search_okapi_models(&state.okapi_config, ""));
            if matches.is_empty() {
                println!("  No models found — Okapi may be unreachable.");
            } else {
                println!("  \x1b[1mAvailable models ({}):\x1b[0m", matches.len());
                println!("  {:<30} {:<12} {:<15} SIZE", "NAME", "FAMILY", "PARAMS");
                println!("  {}", "-".repeat(70));
                for m in &matches {
                    let family = m
                        .details
                        .as_ref()
                        .and_then(|d| d.family.as_deref())
                        .unwrap_or("-");
                    let params = m
                        .details
                        .as_ref()
                        .and_then(|d| d.parameter_size.as_deref())
                        .unwrap_or("-");
                    let size_str = m
                        .size
                        .map(|s| format!("{:.1} GB", s as f64 / 1_073_741_824.0))
                        .unwrap_or_else(|| "-".to_string());
                    println!(
                        "  \x1b[36m{:<30}\x1b[0m {:<12} {:<15} {}",
                        m.name, family, params, size_str
                    );
                }
                println!();
                println!("  Use \x1b[36m/model <name>\x1b[0m to switch to a specific model");
            }
        } else {
            println!("  Current model: \x1b[1m{}\x1b[0m", state.current_model);
            println!(
                "  Use \x1b[36m/model <name>\x1b[0m to switch, \x1b[36m/model <query>\x1b[0m to search"
            );
        }
    } else {
        let matches = rt.block_on(search_okapi_models(&state.okapi_config, arg1));

        if matches.is_empty() {
            state.current_model = arg1.to_string();
            println!("  Model set to: \x1b[1m{}\x1b[0m", state.current_model);
            println!("  \x1b[2m(Okapi unreachable — model name stored for next inference)\x1b[0m");
        } else if matches.len() == 1 {
            state.current_model = matches[0].name.clone();
            println!("  Model set to: \x1b[1m{}\x1b[0m", state.current_model);
            if let Some(ref details) = matches[0].details {
                if let Some(ref fam) = details.family {
                    println!("  Family: {}", fam);
                }
                if let Some(ref params) = details.parameter_size {
                    println!("  Parameters: {}", params);
                }
                if let Some(ref quant) = details.quantization_level {
                    println!("  Quantization: {}", quant);
                }
            }
        } else {
            println!(
                "  \x1b[1mModels matching '\x1b[36m{}\x1b[0m\x1b[1m' ({}):\x1b[0m",
                arg1,
                matches.len()
            );
            println!("  {:<30} {:<12} {:<15} SIZE", "NAME", "FAMILY", "PARAMS");
            println!("  {}", "-".repeat(70));
            for m in &matches {
                let family = m
                    .details
                    .as_ref()
                    .and_then(|d| d.family.as_deref())
                    .unwrap_or("-");
                let params = m
                    .details
                    .as_ref()
                    .and_then(|d| d.parameter_size.as_deref())
                    .unwrap_or("-");
                let size_str = m
                    .size
                    .map(|s| format!("{:.1} GB", s as f64 / 1_073_741_824.0))
                    .unwrap_or_else(|| "-".to_string());
                println!(
                    "  \x1b[36m{:<30}\x1b[0m {:<12} {:<15} {}",
                    m.name, family, params, size_str
                );
            }
            println!();
            println!("  Use \x1b[36m/model <name>\x1b[0m to switch to a specific model");
        }
    }
    println!();
}
