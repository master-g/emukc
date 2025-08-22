use anyhow::Result;
use emukc::cache::NoVersion;

use crate::cfg::AppConfig;

pub(super) async fn exec(config: &AppConfig) -> Result<()> {
	let state = crate::state::State::new(config, false).await?;

	let kache = state.kache.clone();

	kache.get("kcs/sound/kcwjcrloeyiyxw/158288.mp3", 13).await?;
	kache.get("kcs/sound/kcojkgkujsenly/168525.mp3", 27).await?;
	kache.get("kcs2/resources/ship/character_full/0404_3736.png", NoVersion).await?;
	kache.get("gadget_html5/js/kcs_const.js", NoVersion).await?;

	Ok(())
}
