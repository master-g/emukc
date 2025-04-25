use emukc_cache::Kache;
use emukc_crypto::SuffixUtils;
use emukc_model::kc2::start2::ApiManifest;

use crate::{
	make_list::{CacheList, CacheListMakeStrategy},
	prelude::CacheListMakingError,
};

mod plain;
mod resources;
mod versioned;

pub(super) async fn make(
	mst: &ApiManifest,
	kache: &Kache,
	strategy: CacheListMakeStrategy,
	list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
	plain::make(kache, list).await?;
	versioned::make(mst, kache, strategy, list).await?;
	resources::make(mst, kache, strategy, list).await?;

	Ok(())
}

fn gen_path(id: i64, padding: u8, folder: &str, category: &str, extension: &str) -> String {
	let id = if padding == 3 {
		format!("{0:03}", id)
	} else {
		format!("{0:04}", id)
	};

	let key = SuffixUtils::create(&id, format!("{folder}_{category}").as_str());
	format!("kcs2/resources/{folder}/{category}/{id}_{key}.{extension}")
}
