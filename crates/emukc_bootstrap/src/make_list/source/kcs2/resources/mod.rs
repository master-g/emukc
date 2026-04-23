use emukc_cache::Kache;
use emukc_model::kc2::start2::ApiManifest;

use crate::{
    make_list::manifest::{DecoderCoverageAssets, PathRules, ResourceCategoriesAsset},
    make_list::{CacheList, CacheListMakeStrategy},
    prelude::CacheListMakingError,
};

mod bgm;
mod furniture;
mod gauge;
mod map;
pub(crate) mod ship;
pub(crate) mod slot;
mod unversioned;
mod use_item;

fn gen_bgm_path(id: i64, category: &str) -> String {
    super::gen_path(id, 3, "bgm", category, "mp3")
}

pub(super) async fn make(
    mst: &ApiManifest,
    cache: &Kache,
    strategy: &CacheListMakeStrategy,
    list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
    bgm::make(mst, strategy, list).await?;
    furniture::make(mst, cache, strategy, list).await?;
    gauge::make(cache, strategy, list).await?;
    map::make(cache, strategy, list).await?;
    ship::make(mst, cache, strategy, list).await?;
    slot::make(mst, cache, strategy, list).await?;
    unversioned::make(list).await?;
    use_item::make(mst, cache, strategy, list).await?;

    Ok(())
}

pub(super) async fn make_manifest_support(
    mst: &ApiManifest,
    cache: &Kache,
    list: &mut CacheList,
    decoder_assets: Option<&DecoderCoverageAssets>,
    categories: Option<&ResourceCategoriesAsset>,
    rules: Option<&PathRules>,
) -> Result<(), CacheListMakingError> {
    let strategy = CacheListMakeStrategy::Manifest;

    if let Some(decoder_assets) = decoder_assets {
        add_decoder_audio_paths(decoder_assets, list);
        add_decoder_ui_paths(decoder_assets, list);
    }
    bgm::make(mst, &strategy, list).await?;
    furniture::make(mst, cache, &strategy, list).await?;
    gauge::make(cache, &strategy, list).await?;
    map::make(cache, &strategy, list).await?;
    ship::make_manifest_category_extensions(mst, list, rules, categories);
    ship::make_manifest_type_extensions(mst, list);
    slot::make_manifest_category_extensions(mst, list, categories);
    slot::make_manifest_plane_extensions(mst, list, rules);
    unversioned::make(list).await?;
    use_item::make(mst, cache, &strategy, list).await?;

    Ok(())
}

fn add_decoder_audio_paths(decoder_assets: &DecoderCoverageAssets, list: &mut CacheList) {
    let Some(audio) = decoder_assets.audio_resources.as_ref() else {
        return;
    };

    for id in &audio.se_ids.ids {
        list.add_unversioned(format!("kcs2/resources/se/{id}.mp3"));
    }
    for id in &audio.bgm.fanfare_ids.ids {
        list.add_unversioned(gen_bgm_path(*id, "fanfare"));
    }
    for id in &audio.bgm.port_ids.ids {
        list.add_unversioned(gen_bgm_path(*id, "port"));
    }
    for id in &audio.bgm.battle_ids.ids {
        list.add_unversioned(gen_bgm_path(*id, "battle"));
    }
    for stem in &audio.voice.tutorial_voice_stems {
        list.add_unversioned(format!("kcs2/resources/voice/tutorial/{stem}.mp3"));
    }
    for file in &audio.voice.explicit_files {
        list.add_unversioned(format!("kcs2/resources/voice/{file}"));
    }
}

fn add_decoder_ui_paths(decoder_assets: &DecoderCoverageAssets, list: &mut CacheList) {
    let Some(ui) = decoder_assets.ui_resources.as_ref() else {
        return;
    };

    for file in &ui.map.default_files.files {
        list.add_unversioned(format!("kcs2/resources/map/{file}"));
    }
    for file in &ui.map.event_files.files {
        list.add_unversioned(format!("kcs2/resources/map/{file}"));
    }
    for id in &ui.use_item.card_ids.ids {
        list.add_unversioned(format!("kcs2/resources/useitem/card/{id}.png"));
    }
    for id in &ui.use_item.underline_ids.ids {
        list.add_unversioned(format!("kcs2/resources/useitem/card_/{id}.png"));
    }
    for id in &ui.area.sally_ids.ids {
        list.add_unversioned(format!("kcs2/resources/area/sally/{id}.png"));
    }
    for id in &ui.area.airunit_ids.ids {
        list.add_unversioned(format!("kcs2/resources/area/airunit/{id}.png"));
    }
    for id in &ui.area.airunit_extend_confirm_ids.ids {
        list.add_unversioned(format!("kcs2/resources/area/airunit_extend_confirm/{id}.png"));
    }
    for file in &ui.world_select.files {
        list.add_unversioned(format!("kcs2/resources/worldselect/{file}"));
    }
    for path in &ui.furniture.explicit_paths {
        list.add_unversioned(format!("kcs2/{path}"));
    }
}
