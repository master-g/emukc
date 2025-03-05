use anyhow::Result;

use crate::state;
use emukc_internal::prelude::crawl;

/// Crawl from CDN
pub(super) async fn exec(state: &state::State) -> Result<()> {
	crawl(&state.codex.manifest, &state.kache).await?;
	Ok(())
}
