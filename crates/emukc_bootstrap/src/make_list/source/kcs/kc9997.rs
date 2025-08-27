use emukc_model::thirdparty::CacheSource;

use crate::{make_list::CacheList, prelude::CacheListMakingError};

pub(super) async fn make(
	cache_source: &Option<CacheSource>,
	list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
	if let Some(source) = cache_source {
		source.voices.event.iter().for_each(|id| {
			list.add_unversioned(format!("kcs/sound/kc9997/{id}.mp3"));
		});
	}

	Ok(())
}
