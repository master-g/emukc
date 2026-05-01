use emukc_cache::Kache;
use emukc_model::kc2::start2::ApiManifest;

use crate::{
    make_list::manifest::{
        DecoderCoverageAssets, PathRules, ResourceCategoriesAsset, ResourceCoverageMode,
        ResourceTemplateFamily, ResourceTemplateInput, ResourceTemplatePlaceholderFormat,
        ResourceTemplateSegmentKind,
    },
    make_list::{CacheList, CacheListAuthorityStage, CacheListMakeStrategy},
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

pub(super) async fn make_manifest_support(
    mst: &ApiManifest,
    cache: &Kache,
    list: &mut CacheList,
    decoder_assets: Option<&DecoderCoverageAssets>,
    categories: Option<&ResourceCategoriesAsset>,
    rules: Option<&PathRules>,
) -> Result<(), CacheListMakingError> {
    let strategy = CacheListMakeStrategy::Manifest;

    let previous = list.set_authority_stage(Some(CacheListAuthorityStage::RuleAuthored));
    if let Some(decoder_assets) = decoder_assets {
        add_decoder_audio_paths(mst, decoder_assets, list);
        add_decoder_ui_paths(decoder_assets, list);
        add_decoder_template_paths(mst, decoder_assets, list);
        let furniture_categories = decoder_furniture_categories(decoder_assets);
        if !furniture_categories.is_empty() {
            furniture::make_decoder_categories(mst, cache, list, &furniture_categories).await?;
        } else if let Some(ui) = decoder_assets.ui_resources.as_ref() {
            furniture::make_decoder_categories(mst, cache, list, &ui.furniture.categories).await?;
        }
    }
    ship::make_manifest_category_extensions(mst, list, rules, categories);
    slot::make_manifest_category_extensions(mst, list, categories);
    slot::make_manifest_plane_extensions(mst, list, rules);
    list.set_authority_stage(previous);

    let previous = list.set_authority_stage(Some(CacheListAuthorityStage::FallbackAuthored));
    bgm::make(mst, &strategy, list).await?;
    furniture::make(mst, cache, &strategy, list).await?;
    gauge::make(cache, &strategy, list).await?;
    map::make(cache, &strategy, list).await?;
    ship::make_manifest_type_extensions(mst, list);
    unversioned::make(list).await?;
    use_item::make(mst, cache, &strategy, list).await?;
    list.set_authority_stage(previous);

    Ok(())
}

fn decoder_template_families(
    decoder_assets: &DecoderCoverageAssets,
) -> impl Iterator<Item = &ResourceTemplateFamily> {
    decoder_assets
        .resource_templates
        .as_ref()
        .into_iter()
        .flat_map(|templates| templates.families.iter())
        .filter(|family| family.coverage_mode != ResourceCoverageMode::Unresolved)
}

fn decoder_template_family<'a>(
    decoder_assets: &'a DecoderCoverageAssets,
    key: &str,
) -> Option<&'a ResourceTemplateFamily> {
    decoder_template_families(decoder_assets).find(|family| family.key == key)
}

fn template_input_available(
    input: &ResourceTemplateInput,
    family: &ResourceTemplateFamily,
    mst: &ApiManifest,
    decoder_assets: &DecoderCoverageAssets,
) -> bool {
    match input {
        ResourceTemplateInput::ManifestMapinfo => !mst.api_mst_mapinfo.is_empty(),
        ResourceTemplateInput::ManifestMapbgm => !mst.api_mst_mapbgm.is_empty(),
        ResourceTemplateInput::ManifestBgm => !mst.api_mst_bgm.is_empty(),
        ResourceTemplateInput::ManifestFurniture => !mst.api_mst_furniture.is_empty(),
        ResourceTemplateInput::ManifestUseitem => !mst.api_mst_useitem.is_empty(),
        ResourceTemplateInput::CacheSourceSoundBucket => false,
        ResourceTemplateInput::DecoderAudio => decoder_assets.audio_resources.is_some(),
        ResourceTemplateInput::DecoderUi => decoder_assets.ui_resources.is_some(),
        ResourceTemplateInput::DecoderTemplateRange => family.range.is_some(),
    }
}

fn can_expand_template(
    family: &ResourceTemplateFamily,
    mst: &ApiManifest,
    decoder_assets: &DecoderCoverageAssets,
) -> bool {
    family.coverage_mode != ResourceCoverageMode::Unresolved
        && family
            .required_inputs
            .iter()
            .all(|input| template_input_available(input, family, mst, decoder_assets))
}

fn decoder_furniture_categories(decoder_assets: &DecoderCoverageAssets) -> Vec<String> {
    let mut categories = std::collections::BTreeSet::<String>::new();
    if let Some(ui) = decoder_assets.ui_resources.as_ref() {
        categories.extend(ui.furniture.categories.iter().cloned());
    }
    for family in decoder_template_families(decoder_assets) {
        if let Some(category) = family.key.strip_prefix("furniture.") {
            categories.insert(category.to_string());
        }
    }
    categories.into_iter().collect()
}

fn add_decoder_template_paths(
    mst: &ApiManifest,
    decoder_assets: &DecoderCoverageAssets,
    list: &mut CacheList,
) {
    if decoder_assets.resource_templates.is_none() {
        return;
    }

    if let Some(family) = decoder_template_family(decoder_assets, "map.base")
        && can_expand_template(family, mst, decoder_assets)
    {
        add_template_map_base_paths(mst, list);
    }
    if let Some(family) = decoder_template_family(decoder_assets, "map.info")
        && can_expand_template(family, mst, decoder_assets)
    {
        add_template_map_info_paths(mst, list);
    }
    if let Some(family) = decoder_template_family(decoder_assets, "gauge.map")
        && can_expand_template(family, mst, decoder_assets)
    {
        add_template_gauge_paths(mst, list);
    }
    if let Some(family) = decoder_template_family(decoder_assets, "bgm.category")
        && can_expand_template(family, mst, decoder_assets)
    {
        add_template_bgm_paths(mst, list);
    }
    if let Some(family) = decoder_template_family(decoder_assets, "area.sally")
        && can_expand_template(family, mst, decoder_assets)
    {
        add_template_area_paths(mst, "sally", decoder_assets, list);
    }
    if let Some(family) = decoder_template_family(decoder_assets, "area.airunit")
        && can_expand_template(family, mst, decoder_assets)
    {
        add_template_area_paths(mst, "airunit", decoder_assets, list);
    }
    if let Some(family) = decoder_template_family(decoder_assets, "area.airunit_extend_confirm")
        && can_expand_template(family, mst, decoder_assets)
    {
        add_template_area_paths(mst, "airunit_extend_confirm", decoder_assets, list);
    }
    for family in decoder_template_families(decoder_assets) {
        if family.required_inputs.contains(&ResourceTemplateInput::DecoderTemplateRange)
            && can_expand_template(family, mst, decoder_assets)
        {
            add_template_range_paths(family, list);
        }
    }
}

fn render_template(
    family: &ResourceTemplateFamily,
    values: &std::collections::BTreeMap<&str, String>,
) -> Option<String> {
    let mut rendered = String::new();
    for segment in &family.path_template {
        match segment.kind {
            ResourceTemplateSegmentKind::Literal => rendered.push_str(&segment.value),
            ResourceTemplateSegmentKind::Placeholder => {
                let raw = values.get(segment.name.as_str())?;
                let value = match segment
                    .format
                    .as_ref()
                    .unwrap_or(&ResourceTemplatePlaceholderFormat::Raw)
                {
                    ResourceTemplatePlaceholderFormat::Number
                    | ResourceTemplatePlaceholderFormat::Raw => raw.clone(),
                    ResourceTemplatePlaceholderFormat::Pad2 => {
                        format!("{:02}", raw.parse::<i64>().ok()?)
                    }
                    ResourceTemplatePlaceholderFormat::Pad3 => {
                        format!("{:03}", raw.parse::<i64>().ok()?)
                    }
                };
                rendered.push_str(&value);
            }
        }
    }
    Some(rendered)
}

fn add_template_map_base_paths(mst: &ApiManifest, list: &mut CacheList) {
    for map in &mst.api_mst_mapinfo {
        let area = format!("{:03}", map.api_maparea_id);
        let no = format!("{:02}", map.api_no);
        list.add_unversioned(format!("kcs2/resources/map/{area}/{no}.png"));
    }
}

fn add_template_map_info_paths(mst: &ApiManifest, list: &mut CacheList) {
    for map in &mst.api_mst_mapinfo {
        let area = format!("{:03}", map.api_maparea_id);
        let no = format!("{:02}", map.api_no);
        list.add_unversioned(format!("kcs2/resources/map/{area}/{no}_image.json"));
        list.add_unversioned(format!("kcs2/resources/map/{area}/{no}_image.png"));
        list.add_unversioned(format!("kcs2/resources/map/{area}/{no}_info.json"));
    }
}

fn add_template_gauge_paths(_mst: &ApiManifest, list: &mut CacheList) {
    for id in gauge::MAP_ID_LIST.iter().chain(gauge::EVENT_MAP_ID_LIST.iter()) {
        list.add_unversioned(format!("kcs2/resources/gauge/{id}.json"));
    }
}

fn add_template_bgm_paths(mst: &ApiManifest, list: &mut CacheList) {
    for id in 1..=5 {
        list.add_unversioned(gen_bgm_path(id, "fanfare"));
    }
    list.add_unversioned(gen_bgm_path(0, "port"));
    for bgm in &mst.api_mst_bgm {
        list.add_unversioned(gen_bgm_path(bgm.api_id, "port"));
    }
    let mut battle_ids = std::collections::BTreeSet::<i64>::new();
    for entry in &mst.api_mst_mapbgm {
        if entry.api_moving_bgm > 0 {
            battle_ids.insert(entry.api_moving_bgm);
        }
        battle_ids.extend(entry.api_map_bgm.iter().copied().filter(|id| *id > 0));
        battle_ids.extend(entry.api_boss_bgm.iter().copied().filter(|id| *id > 0));
    }
    for id in battle_ids {
        list.add_unversioned(gen_bgm_path(id, "battle"));
    }
}

fn add_template_area_paths(
    mst: &ApiManifest,
    family: &str,
    decoder_assets: &DecoderCoverageAssets,
    list: &mut CacheList,
) {
    let area_ids: Vec<String> = if family == "sally" {
        mst.api_mst_mapinfo
            .iter()
            .filter(|map| !map.api_sally_flag.is_empty())
            .map(|map| format!("{:03}", map.api_maparea_id))
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    } else if let Some(ui) = decoder_assets.ui_resources.as_ref() {
        match family {
            "airunit" => ui.area.airunit_ids.ids.to_vec(),
            "airunit_extend_confirm" => ui.area.airunit_extend_confirm_ids.ids.to_vec(),
            _ => mst
                .api_mst_mapinfo
                .iter()
                .map(|map| format!("{:03}", map.api_maparea_id))
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect(),
        }
    } else {
        unversioned::AREA_AIR_UNIT.iter().map(|s| (*s).to_string()).collect()
    };
    for id in area_ids {
        list.add_unversioned(format!("kcs2/resources/area/{family}/{id}.png"));
    }
}

fn add_template_range_paths(family: &ResourceTemplateFamily, list: &mut CacheList) {
    let Some(range) = family.range.as_ref() else {
        return;
    };
    if range.start > range.end {
        return;
    }
    for value in range.start..=range.end {
        let mut values = std::collections::BTreeMap::<&str, String>::new();
        values.insert("voiceId", value.to_string());
        values.insert("worldId", value.to_string());
        values.insert("state", "".to_string());
        if let Some(path) = render_template(family, &values) {
            list.add_unversioned(path);
        }
        if family.key == "worldselect.chinjufu-buttons" {
            values.insert("state", "_off".to_string());
            if let Some(path) = render_template(family, &values) {
                list.add_unversioned(path);
            }
        }
    }
    if family.key == "worldselect.chinjufu-buttons" {
        list.add_unversioned("kcs2/resources/worldselect/btn_chinjyufu_on.png".to_string());
    }
}

fn add_decoder_audio_paths(
    mst: &ApiManifest,
    decoder_assets: &DecoderCoverageAssets,
    list: &mut CacheList,
) {
    let Some(audio) = decoder_assets.audio_resources.as_ref() else {
        return;
    };

    for id in &audio.se_ids.ids {
        if !unversioned::is_default_se_id(*id) {
            continue;
        }
        list.add_unversioned(format!("kcs2/resources/se/{id}.mp3"));
    }
    for id in &audio.bgm.fanfare_ids.ids {
        if !(1..=5).contains(id) {
            continue;
        }
        list.add_unversioned(gen_bgm_path(*id, "fanfare"));
    }
    for id in &audio.bgm.port_ids.ids {
        if !mst.api_mst_bgm.iter().any(|bgm| bgm.api_id == *id) {
            continue;
        }
        list.add_unversioned(gen_bgm_path(*id, "port"));
    }
    for id in &audio.bgm.battle_ids.ids {
        if *id <= 0 {
            continue;
        }
        list.add_unversioned(gen_bgm_path(*id, "battle"));
    }
    for stem in &audio.voice.tutorial_voice_stems {
        if !unversioned::is_default_tutorial_voice_stem(stem) {
            continue;
        }
        list.add_unversioned(format!("kcs2/resources/voice/tutorial/{stem}.mp3"));
    }
    for file in &audio.voice.explicit_files {
        if !unversioned::is_default_voice_file(file) {
            continue;
        }
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
        if !use_item::is_default_card_id(id) {
            continue;
        }
        list.add_unversioned(format!("kcs2/resources/useitem/card/{id}.png"));
    }
    for id in &ui.use_item.underline_ids.ids {
        if !use_item::is_default_underline_id(id) {
            continue;
        }
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
        if !unversioned::is_default_world_select_file(file) {
            continue;
        }
        list.add_unversioned(format!("kcs2/resources/worldselect/{file}"));
    }
    for path in &ui.furniture.explicit_paths {
        list.add_unversioned(format!("kcs2/{path}"));
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use emukc_model::kc2::start2::{ApiManifest, ApiMstBgm, ApiMstMapbgm, ApiMstMapinfo};

    use crate::make_list::manifest::{
        ResourceTemplateDomain, ResourceTemplateFamily, ResourceTemplateInput,
        ResourceTemplateProvenance, ResourceTemplateRange, ResourceTemplateSegment,
        ResourceTemplatesAsset,
    };
    use crate::make_list::{CacheList, CacheListAuthorityStage};

    use super::*;

    #[test]
    fn decoder_ui_paths_take_authority_over_fallback_duplicates() {
        let ui_resources = serde_json::from_value(json!({
            "version": 1,
            "generatedAt": "2026-01-01T00:00:00Z",
            "scriptVersion": "6.2.8.0",
            "useItem": {
                "cardIds": { "coverageMode": "partial", "ids": ["073", "999"] },
                "underlineIds": { "coverageMode": "partial", "ids": ["075", "999"] }
            },
            "worldSelect": {
                "files": ["btn_chinjyufu1_off.png"]
            }
        }))
        .unwrap();
        let decoder_assets = DecoderCoverageAssets {
            ui_resources: Some(ui_resources),
            ..Default::default()
        };
        let mut list = CacheList::new();

        let previous = list.set_authority_stage(Some(CacheListAuthorityStage::RuleAuthored));
        add_decoder_ui_paths(&decoder_assets, &mut list);
        list.set_authority_stage(Some(CacheListAuthorityStage::FallbackAuthored));
        list.add_unversioned("kcs2/resources/useitem/card/073.png".to_string());
        list.add_unversioned("kcs2/resources/worldselect/btn_chinjyufu1_off.png".to_string());
        list.set_authority_stage(previous);

        let output = list.into_path_build_output();
        assert!(
            output
                .diagnostics
                .rule_authored_paths
                .contains(&"kcs2/resources/useitem/card/073.png".to_string())
        );
        assert!(
            !output
                .diagnostics
                .fallback_authored_paths
                .contains(&"kcs2/resources/useitem/card/073.png".to_string())
        );
        assert!(
            !output
                .diagnostics
                .fallback_authored_paths
                .contains(&"kcs2/resources/worldselect/btn_chinjyufu1_off.png".to_string())
        );
        assert!(!output.paths.contains("kcs2/resources/useitem/card/999.png"));
        assert!(!output.paths.contains("kcs2/resources/useitem/card_/999.png"));
    }

    fn template_family(key: &str, domain: ResourceTemplateDomain) -> ResourceTemplateFamily {
        ResourceTemplateFamily {
            key: key.to_string(),
            domain,
            output_prefix: "kcs2/resources/test".to_string(),
            path_template: vec![
                ResourceTemplateSegment {
                    kind: ResourceTemplateSegmentKind::Literal,
                    value: "kcs2/resources/voice/titlecall_1/".to_string(),
                    name: String::new(),
                    format: None,
                },
                ResourceTemplateSegment {
                    kind: ResourceTemplateSegmentKind::Placeholder,
                    value: String::new(),
                    name: "voiceId".to_string(),
                    format: Some(ResourceTemplatePlaceholderFormat::Pad3),
                },
                ResourceTemplateSegment {
                    kind: ResourceTemplateSegmentKind::Literal,
                    value: ".mp3".to_string(),
                    name: String::new(),
                    format: None,
                },
            ],
            required_inputs: vec![ResourceTemplateInput::DecoderTemplateRange],
            coverage_mode: ResourceCoverageMode::ObservedComplete,
            provenance: ResourceTemplateProvenance {
                module_ids: vec!["93788".to_string()],
                module_names: vec!["AppInitializeTask".to_string()],
            },
            completeness_blockers: Vec::new(),
            range: Some(ResourceTemplateRange {
                start: 1,
                end: 2,
                pad: Some(3),
            }),
        }
    }

    #[test]
    fn decoder_templates_expand_manifest_and_range_inputs_as_rule_authored() {
        let mut list = CacheList::new();
        let mst = ApiManifest {
            api_mst_mapinfo: vec![ApiMstMapinfo {
                api_maparea_id: 1,
                api_no: 2,
                api_sally_flag: vec![1],
                ..Default::default()
            }],
            api_mst_bgm: vec![ApiMstBgm {
                api_id: 101,
                api_name: "Port".to_string(),
            }],
            api_mst_mapbgm: vec![ApiMstMapbgm {
                api_moving_bgm: 7,
                api_map_bgm: vec![8],
                api_boss_bgm: vec![9],
                ..Default::default()
            }],
            ..Default::default()
        };
        let decoder_assets = DecoderCoverageAssets {
            resource_templates: Some(ResourceTemplatesAsset {
                families: vec![
                    ResourceTemplateFamily {
                        key: "map.base".to_string(),
                        domain: ResourceTemplateDomain::Map,
                        coverage_mode: ResourceCoverageMode::ObservedComplete,
                        required_inputs: vec![ResourceTemplateInput::ManifestMapinfo],
                        ..Default::default()
                    },
                    ResourceTemplateFamily {
                        key: "map.info".to_string(),
                        domain: ResourceTemplateDomain::Map,
                        coverage_mode: ResourceCoverageMode::Partial,
                        required_inputs: vec![ResourceTemplateInput::ManifestMapinfo],
                        ..Default::default()
                    },
                    ResourceTemplateFamily {
                        key: "bgm.category".to_string(),
                        domain: ResourceTemplateDomain::Bgm,
                        coverage_mode: ResourceCoverageMode::ObservedComplete,
                        required_inputs: vec![
                            ResourceTemplateInput::ManifestBgm,
                            ResourceTemplateInput::ManifestMapbgm,
                        ],
                        ..Default::default()
                    },
                    template_family("voice.titlecall_1", ResourceTemplateDomain::Voice),
                ],
                ..Default::default()
            }),
            ..Default::default()
        };

        let previous = list.set_authority_stage(Some(CacheListAuthorityStage::RuleAuthored));
        add_decoder_template_paths(&mst, &decoder_assets, &mut list);
        list.set_authority_stage(previous);

        let output = list.into_path_build_output();
        assert!(output.paths.contains("kcs2/resources/map/001/02.png"));
        assert!(output.paths.contains("kcs2/resources/map/001/02_image.json"));
        assert!(output.paths.contains(&gen_bgm_path(101, "port")));
        assert!(output.paths.contains(&gen_bgm_path(7, "battle")));
        assert!(output.paths.contains("kcs2/resources/voice/titlecall_1/001.mp3"));
        assert!(
            output
                .diagnostics
                .rule_authored_paths
                .contains(&"kcs2/resources/map/001/02.png".to_string())
        );
    }

    #[test]
    fn decoder_map_base_template_does_not_claim_sidecars_without_info_family() {
        let mut list = CacheList::new();
        let mst = ApiManifest {
            api_mst_mapinfo: vec![ApiMstMapinfo {
                api_maparea_id: 1,
                api_no: 2,
                ..Default::default()
            }],
            ..Default::default()
        };
        let decoder_assets = DecoderCoverageAssets {
            resource_templates: Some(ResourceTemplatesAsset {
                families: vec![ResourceTemplateFamily {
                    key: "map.base".to_string(),
                    domain: ResourceTemplateDomain::Map,
                    coverage_mode: ResourceCoverageMode::ObservedComplete,
                    required_inputs: vec![ResourceTemplateInput::ManifestMapinfo],
                    ..Default::default()
                }],
                ..Default::default()
            }),
            ..Default::default()
        };

        let previous = list.set_authority_stage(Some(CacheListAuthorityStage::RuleAuthored));
        add_decoder_template_paths(&mst, &decoder_assets, &mut list);
        list.set_authority_stage(previous);

        let output = list.into_path_build_output();
        assert!(output.paths.contains("kcs2/resources/map/001/02.png"));
        assert!(!output.paths.contains("kcs2/resources/map/001/02_image.json"));
        assert!(!output.paths.contains("kcs2/resources/map/001/02_info.json"));
    }

    #[test]
    fn decoder_template_missing_manifest_input_leaves_family_fallback_authored() {
        let decoder_assets = DecoderCoverageAssets {
            resource_templates: Some(ResourceTemplatesAsset {
                families: vec![ResourceTemplateFamily {
                    key: "map.base".to_string(),
                    domain: ResourceTemplateDomain::Map,
                    coverage_mode: ResourceCoverageMode::ObservedComplete,
                    required_inputs: vec![ResourceTemplateInput::ManifestMapinfo],
                    provenance: ResourceTemplateProvenance {
                        module_ids: vec!["map-loader".to_string()],
                        module_names: vec!["MapLoader".to_string()],
                    },
                    ..Default::default()
                }],
                ..Default::default()
            }),
            ..Default::default()
        };
        let mut list = CacheList::new();

        let previous = list.set_authority_stage(Some(CacheListAuthorityStage::RuleAuthored));
        add_decoder_template_paths(&ApiManifest::default(), &decoder_assets, &mut list);
        list.set_authority_stage(Some(CacheListAuthorityStage::FallbackAuthored));
        list.add_unversioned("kcs2/resources/map/001/01.png".to_string());
        list.set_authority_stage(previous);

        let output = list.into_path_build_output();
        assert!(
            !output
                .diagnostics
                .rule_authored_paths
                .contains(&"kcs2/resources/map/001/01.png".to_string())
        );
        assert!(
            output
                .diagnostics
                .fallback_authored_paths
                .contains(&"kcs2/resources/map/001/01.png".to_string())
        );
    }

    #[test]
    fn template_area_paths_use_decoder_observed_ids_for_airunit() {
        let ui_resources = serde_json::from_value(json!({
            "version": 1,
            "generatedAt": "2026-01-01T00:00:00Z",
            "scriptVersion": "6.2.8.0",
            "area": {
                "airunitIds": { "ids": ["006", "007"] },
                "airunitExtendConfirmIds": { "ids": ["006"] }
            }
        }))
        .unwrap();
        let decoder_assets = DecoderCoverageAssets {
            ui_resources: Some(ui_resources),
            ..Default::default()
        };
        let mst = ApiManifest {
            api_mst_mapinfo: vec![
                ApiMstMapinfo {
                    api_maparea_id: 1,
                    api_no: 1,
                    api_sally_flag: vec![1],
                    ..Default::default()
                },
                ApiMstMapinfo {
                    api_maparea_id: 6,
                    api_no: 1,
                    api_sally_flag: vec![],
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let mut list = CacheList::new();
        add_template_area_paths(&mst, "airunit", &decoder_assets, &mut list);

        let paths: Vec<&str> = list.items.iter().map(|i| i.path.as_str()).collect();
        assert!(paths.iter().any(|p| p.contains("airunit/006.png")));
        assert!(paths.iter().any(|p| p.contains("airunit/007.png")));
        assert!(
            !paths.iter().any(|p| p.contains("airunit/001.png")),
            "area 001 should not produce airunit paths"
        );

        let mut list2 = CacheList::new();
        add_template_area_paths(&mst, "airunit_extend_confirm", &decoder_assets, &mut list2);

        let paths2: Vec<&str> = list2.items.iter().map(|i| i.path.as_str()).collect();
        assert!(paths2.iter().any(|p| p.contains("airunit_extend_confirm/006.png")));
        assert!(
            !paths2.iter().any(|p| p.contains("airunit_extend_confirm/001.png")),
            "area 001 should not produce airunit_extend_confirm paths"
        );
    }

    #[test]
    fn template_gauge_paths_only_produce_known_gauge_ids() {
        let mst = ApiManifest {
            api_mst_mapinfo: vec![
                ApiMstMapinfo {
                    api_maparea_id: 1,
                    api_no: 1,
                    ..Default::default()
                },
                ApiMstMapinfo {
                    api_maparea_id: 1,
                    api_no: 5,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let mut list = CacheList::new();
        add_template_gauge_paths(&mst, &mut list);

        let paths: Vec<&str> = list.items.iter().map(|i| i.path.as_str()).collect();
        // 00105 is in MAP_ID_LIST — should be present
        assert!(paths.iter().any(|p| p.contains("gauge/00105.json")));
        // 00101 is NOT in any gauge list — should be absent
        assert!(
            !paths.iter().any(|p| p.contains("gauge/00101.json")),
            "regular map 1-1 should not produce gauge paths"
        );
    }
}
