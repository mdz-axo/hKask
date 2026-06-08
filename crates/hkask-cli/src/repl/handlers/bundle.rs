//! REPL /bundle handler — skill bundle composition and management

pub(crate) fn handle_bundle(arg1: &str) {
    match arg1 {
        "list" => {
            println!("  \x1b[1mSkill Bundles\x1b[0m");
            println!("  (use \x1b[36mkask bundle list\x1b[0m for full details)");
            println!();
        }
        "off" => {
            println!("  Bundle deactivated.");
            println!();
        }
        "skills" => {
            println!("  \x1b[1mAvailable Skills\x1b[0m");
            println!("  (use \x1b[36mkask bundle skills\x1b[0m for full details)");
            println!();
        }
        "" => {
            println!("  \x1b[1mBundle Commands\x1b[0m");
            println!("    \x1b[36m/bundle SKILL1 SKILL2\x1b[0m  Compose a bundle from skills");
            println!("    \x1b[36m/bundle list\x1b[0m          List all bundles");
            println!("    \x1b[36m/bundle off\x1b[0m           Deactivate current bundle");
            println!("    \x1b[36m/bundle skills\x1b[0m        List available skills");
            println!();
        }
        skills_arg => {
            println!("  Composing bundle from: {}", skills_arg);
            println!(
                "  (use \x1b[36mkask bundle compose SKILL1 SKILL2\x1b[0m for full composition)"
            );
            println!();
        }
    }
}
