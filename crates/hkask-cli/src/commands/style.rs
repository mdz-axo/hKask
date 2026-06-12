//! Style command dispatcher — routes to compose or embed-corpus subcommands

use crate::cli::StyleAction;

/// Run a style subcommand
pub fn run(rt: &tokio::runtime::Runtime, action: StyleAction) {
    match action {
        StyleAction::Compose {
            prompt,
            cognition,
            db,
            passphrase,
            no_validate,
        } => super::compose::run(rt, prompt, cognition, db, passphrase, no_validate),
        StyleAction::EmbedCorpus {
            config,
            db,
            passphrase,
        } => super::embed_corpus::run(rt, config, db, passphrase),
    }
}
