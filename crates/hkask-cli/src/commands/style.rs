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
        StyleAction::Discover {
            author_name,
            max_works,
            output_dir,
            cache_dir,
            serpapi_key,
            no_transcripts,
            no_web,
            no_curate,
            search_terms,
            no_methods,
        } => super::discover::run(
            rt,
            author_name,
            max_works,
            output_dir,
            cache_dir,
            serpapi_key,
            !no_transcripts,
            !no_web,
            !no_curate,
            search_terms,
            !no_methods,
        ),
    }
}
