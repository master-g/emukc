use emukc_model::thirdparty::CacheSource;

use crate::{make_list::CacheList, prelude::CacheListMakingError};

pub(super) async fn make(
	cache_source: &Option<CacheSource>,
	list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
	if let Some(source) = cache_source {
		source.voices.npc.iter().for_each(|id| {
			let p = format!("kcs/sound/kc9999/{id}.mp3");
			list.add_unversioned(p);
		});
	}

	Ok(())
}
