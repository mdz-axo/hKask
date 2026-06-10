use rustyline::Context;
use rustyline::completion::Completer;
use rustyline::highlight::CmdKind;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use std::borrow::Cow;

use super::commands::SLASH_COMMANDS;

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
