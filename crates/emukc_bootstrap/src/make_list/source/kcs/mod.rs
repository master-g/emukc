use emukc_cache::Kache;
use emukc_model::codex::Codex;

use crate::{
	make_list::{CacheList, CacheListMakeStrategy},
	prelude::CacheListMakingError,
};

mod kc9997;
mod kc9998;
mod kc9999;
mod purchase;
mod voice;

pub(super) async fn make(
	codex: &Codex,
	cache: &Kache,
	strategy: CacheListMakeStrategy,
	list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
	kc9997::make(&codex.cache_source, list).await?;
	kc9998::make(&codex.cache_source, list, strategy);
	kc9999::make(&codex.cache_source, list).await?;
	purchase::make(&codex.manifest, list);
	voice::make(&codex.manifest, cache, strategy, list).await?;
	Ok(())
}
