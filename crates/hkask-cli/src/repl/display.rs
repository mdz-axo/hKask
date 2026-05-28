use super::commands::{find_command, fuzzy_match_command};

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub(super) fn print_banner(agent: &str, template: Option<&str>) {
    let ghost = "\x1b[2;36m";
    let body = "\x1b[1;36m";
    let bright = "\x1b[1;37m";
    let dim = "\x1b[2;37m";
    let gold = "\x1b[1;33m";
    let r = "\x1b[0m";

    let eye_frames: &[&str] = &["center", "right", "center", "left"];

    for (i, gaze) in eye_frames.iter().enumerate() {
        if i > 0 {
            print!("\x1b[10A");
        }

        let eye = match *gaze {
            "right" => format!("{bright}.::{gold}>{bright}:.{r}"),
            "left" => format!("{bright}.:{gold}<{bright}::.{r}"),
            _ => format!("{bright}.:{dim}:{bright}:.{r}"),
        };

        println!("{ghost}  __          {body}    ___________    __    {r}");
        println!("{ghost} /  \\         {body}   /  \\   {eye}   /  \\   {r}");
        println!("{ghost}|    |        {body}  |    |           |    |  {r}");
        println!("{ghost} \\__/         {body}  |    |    KASK   |    |  {r}");
        println!("{ghost}              {body}  |    |           |    |  {r}");
        println!("{ghost}              {body}   \\__/~~~~~~~~~~~\\__/   {r}");
        println!("  {ghost}shadow{r}       {body}    hKask v{VERSION}{r}");
        println!();
        println!("{body}     Planck's Constant of Agent Systems{r}");
        println!();

        std::io::Write::flush(&mut std::io::stdout()).ok();

        if i < eye_frames.len() - 1 {
            std::thread::sleep(std::time::Duration::from_millis(350));
        }
    }

    println!(
        "  \x1b[1mAgent:\x1b[0m \x1b[1m{}\x1b[0m  \x1b[1mTemplate:\x1b[0m \x1b[1m{}\x1b[0m",
        agent,
        template.unwrap_or("auto-select")
    );
    println!(
        "  \x1b[1;36m/help\x1b[0m for commands  \x1b[2m<TAB>\x1b[0m autocomplete  \x1b[2m/quit\x1b[0m exit"
    );
    println!();
}

pub(super) fn print_help() {
    println!();
    println!("\x1b[1mℏKask Commands\x1b[0m");
    println!();

    let categories = [
        ("Session", &["help", "quit", "clear", "history"] as &[&str]),
        ("Agent", &["agent", "agents", "pods"]),
        ("Ensemble", &["into", "ensemble", "filter", "mode", "ask"]),
        ("System", &["status", "tools", "templates", "sovereignty"]),
        (
            "Governance",
            &["escalations", "resolve", "dismiss", "metacognition"],
        ),
    ];

    for (category, cmds) in &categories {
        println!("  \x1b[1;33m{}\x1b[0m", category);
        for &cmd_name in *cmds {
            if let Some(cmd) = find_command(cmd_name) {
                let alias_str = if cmd.aliases.is_empty() {
                    String::new()
                } else {
                    format!(", /{}", cmd.aliases.join(", /"))
                };
                let args_str = if cmd.args.is_empty() {
                    String::new()
                } else {
                    format!(" {}", cmd.args)
                };
                println!(
                    "    \x1b[36m/{}{}\x1b[0m{}  — {}",
                    cmd.primary, args_str, alias_str, cmd.about
                );
            }
        }
        println!();
    }

    println!("  \x1b[2mTip: /help <command> for details on a specific command\x1b[0m");
    println!();
}

pub(super) fn print_command_help(cmd_name: &str) {
    if let Some(cmd) = find_command(cmd_name) {
        println!();
        println!("  \x1b[1;36m/{} {}\x1b[0m", cmd.primary, cmd.args);
        if !cmd.aliases.is_empty() {
            println!("  Aliases: /{}", cmd.aliases.join(", /"));
        }
        println!("  {}", cmd.about);

        match cmd.primary {
            "ensemble" => {
                println!();
                println!("  Subcommands:");
                println!(
                    "    \x1b[36m/ensemble sessions\x1b[0m    — List active ensemble sessions"
                );
                println!(
                    "    \x1b[36m/ensemble create\x1b[0m <id> — Create a new ensemble chat session"
                );
                println!(
                    "    \x1b[36m/ensemble join\x1b[0m <id> <bot> <role> — Register a bot in a session"
                );
                println!(
                    "    \x1b[36m/ensemble send\x1b[0m <id> <msg> — Send a message to a session"
                );
                println!(
                    "    \x1b[36m/ensemble invite\x1b[0m <bot> [role] — Invite agent into current session"
                );
                println!(
                    "    \x1b[36m/ensemble participants\x1b[0m — Show who's in the current session"
                );
                println!();
                println!("  Roles: memory_bot, spandrel_bot, okapi_bot, scholar_bot");
                println!("  Use \x1b[36m/into <session>\x1b[0m to enter ensemble mode");
            }
            "into" => {
                println!();
                println!(
                    "  \x1b[2m/into research-team\x1b[0m  — Enter ensemble session 'research-team'"
                );
                println!(
                    "  \x1b[2m/into\x1b[0m               — Leave ensemble mode, return to single-agent"
                );
                println!();
                println!("  In ensemble mode, messages go to the group. Agents self-select");
                println!("  to speak based on relevance confidence (generative improvisation).");
            }
            "filter" => {
                println!();
                println!("  \x1b[2m/filter\x1b[0m          — Show current participation threshold");
                println!(
                    "  \x1b[2m/filter 0.8\x1b[0m      — Set threshold (0.0-1.0, higher = more selective)"
                );
                println!();
                println!("  Controls how confident an agent must be to speak in ensemble mode.");
                println!(
                    "  Default: 0.75. Increase for focused discussion, decrease for more voices."
                );
            }
            "mode" => {
                println!();
                println!("  \x1b[2m/mode\x1b[0m                     — Show current ensemble mode");
                println!(
                    "  \x1b[2m/mode freeform\x1b[0m            — Agents self-select by relevance (default)"
                );
                println!("  \x1b[2m/mode curator_led\x1b[0m         — Curator picks speakers");
                println!("  \x1b[2m/mode round_robin\x1b[0m         — All agents speak in turn");
            }
            "ask" => {
                println!();
                println!("  \x1b[2m/ask ScholarBot What do you think?\x1b[0m");
                println!();
                println!("  Force a specific agent to respond, bypassing relevance filter.");
            }
            "agent" => {
                println!();
                println!("  \x1b[2m/agent\x1b[0m          — Show current agent");
                println!("  \x1b[2m/agent Russell\x1b[0m  — Switch to Russell");
                println!("  \x1b[2m/agents\x1b[0m         — List all available agents");
            }
            _ => {}
        }
        println!();
    } else {
        let fuzzy = fuzzy_match_command(cmd_name);
        if fuzzy.is_empty() {
            println!("  Unknown command: /{}", cmd_name);
        } else {
            println!("  Unknown command: /{} — did you mean:", cmd_name);
            for cmd in &fuzzy {
                println!("    /{} — {}", cmd.primary, cmd.about);
            }
        }
        println!("  Type /help for available commands.");
    }
}
