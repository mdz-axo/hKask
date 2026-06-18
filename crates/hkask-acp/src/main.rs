//! hkask-acp binary entry point.
//!
//! The library crate re-exports the public API; this binary just calls main().

use hkask_acp::main_impl;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    Ok(main_impl::run().await?)
}
