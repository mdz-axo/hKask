use rustyline::Context;
use rustyline::completion::Completer;
use rustyline::highlight::CmdKind;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use std::borrow::Cow;

use super::commands::SLASH_COMMANDS;

/// A turn in the session: user input and the agent's response.
#[derive(Debug, Clone)]
pub(crate) struct Turn {
    pub(crate) user_input: String,
    pub(crate) agent: String,
    pub(crate) response: String,
}

#[derive(Debug, Clone)]
pub(crate) struct SessionHistory {
    turns: Vec<Turn>,
}

impl SessionHistory {
    pub(super) fn new() -> Self {
        Self { turns: Vec::new() }
    }
    pub(super) fn record(&mut self, user_input: &str, agent: &str, response: &str) {
        self.turns.push(Turn {
            user_input: user_input.to_string(),
            agent: agent.to_string(),
            response: response.to_string(),
        });
    }

    pub(crate) fn turn_count(&self) -> usize {
        self.turns.len()
    }

    /// Iterate turns as (agent, response) pairs for display.
    pub(crate) fn turns_for_display(&self) -> impl Iterator<Item = (&str, &str)> {
        self.turns
            .iter()
            .map(|t| (t.agent.as_str(), t.response.as_str()))
    }

    /// Return the last `n` turns as formatted context text suitable for
    /// prepending to the model's prompt.
    pub(crate) fn recent_context(&self, n: usize) -> String {
        if self.turns.is_empty() {
            return String::new();
        }
        let start = self.turns.len().saturating_sub(n);
        let recent: Vec<String> = self.turns[start..]
            .iter()
            .map(|t| format!("User: {}\n{}: {}", t.user_input, t.agent, t.response))
            .collect();
        format!(
            "[Previous conversation]\n{}\n[/Previous conversation]\n\n",
            recent.join("\n\n")
        )
    }
}

pub(super) struct KaskHelper {
    slash_completions: Vec<String>,
}

impl KaskHelper {
    pub(super) fn new() -> Self {
        let mut slash_completions = Vec::new();
        for cmd in SLASH_COMMANDS {
            slash_completions.push(format!("/{}", cmd.primary));
            for alias in cmd.aliases {
                slash_completions.push(format!("/{}", alias));
            }
        }
        Self { slash_completions }
    }
}

impl Completer for KaskHelper {
    type Candidate = String;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<String>)> {
        if !line.starts_with('/') || pos == 0 {
            return Ok((0, Vec::new()));
        }

        let partial = &line[..pos];
        let matches: Vec<String> = self
            .slash_completions
            .iter()
            .filter(|c| c.starts_with(partial))
            .cloned()
            .collect();

        Ok((0, matches))
    }
}

impl Hinter for KaskHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> Option<String> {
        if !line.starts_with('/') || pos == 0 {
            return None;
        }
        let partial = &line[..pos];
        self.slash_completions
            .iter()
            .find(|c| c.starts_with(partial) && c.len() > partial.len())
            .map(|c| c[partial.len()..].to_string())
    }
}

impl Highlighter for KaskHelper {
    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Cow::Owned(format!("\x1b[2m{}\x1b[0m", hint))
    }

    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        if line.starts_with('/') {
            Cow::Owned(format!("\x1b[1;36m{}\x1b[0m", line))
        } else {
            Cow::Borrowed(line)
        }
    }

    fn highlight_char(&self, line: &str, _pos: usize, _cmd_kind: CmdKind) -> bool {
        line.starts_with('/')
    }
}

impl Validator for KaskHelper {}
impl rustyline::Helper for KaskHelper {}
