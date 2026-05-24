use emukc_model::kc2::start2::{ApiManifest, ApiMstShipgraph};
use emukc_model::thirdparty::CacheSource;

use crate::make_list::{
    CacheList,
    manifest::{
        CacheRuleShipVoiceFormula, CacheRuleShipVoiceRule, CacheRuleSoundBucketRule,
        CacheRuleSoundRules, DecoderCoverageAssets, ResourceCoverageMode, ResourceTemplateInput,
    },
};

pub(super) fn make(
    mst: &ApiManifest,
    cache_source: &Option<CacheSource>,
    sound_rules: &CacheRuleSoundRules,
    decoder_assets: Option<&DecoderCoverageAssets>,
    list: &mut CacheList,
) {
    make_ship_voices(mst, &sound_rules.ship_voices, list);
    make_bucket(&sound_rules.kc9997, cache_source, false, list);
    make_bucket(
        &sound_rules.kc9998,
        cache_source,
        has_cache_source_sound_template(decoder_assets, "sound.kc9998"),
        list,
    );
    make_bucket(&sound_rules.kc9999, cache_source, false, list);
}

fn make_bucket(
    rule: &CacheRuleSoundBucketRule,
    cache_source: &Option<CacheSource>,
    cache_source_authoritative: bool,
    list: &mut CacheList,
) {
    if rule.coverage_mode == ResourceCoverageMode::Unresolved {
        return;
    }

    if rule.coverage_mode == ResourceCoverageMode::Partial
        && cache_source_authoritative
        && let Some(ids) = fallback_bucket_ids(rule.bucket.as_str(), cache_source)
    {
        for voice_id in ids {
            if bucket_id_is_known_missing(rule.bucket.as_str(), *voice_id) {
                continue;
            }
            list.add_unversioned(format!("kcs/sound/kc{}/{voice_id}.mp3", rule.bucket));
        }
        return;
    }

    let fallback_ids = if rule.coverage_mode == ResourceCoverageMode::Partial {
        fallback_bucket_ids(rule.bucket.as_str(), cache_source)
    } else {
        None
    };

    for voice_id in &rule.voice_ids {
        if let Some(ids) = fallback_ids {
            let Ok(voice_id) = u64::try_from(*voice_id) else {
                continue;
            };
            if !ids.contains(&voice_id) {
                continue;
            }
        }
        list.add_unversioned(format!("kcs/sound/kc{}/{voice_id}.mp3", rule.bucket));
    }
}

fn has_cache_source_sound_template(
    decoder_assets: Option<&DecoderCoverageAssets>,
    key: &str,
) -> bool {
    decoder_assets
        .and_then(|assets| assets.resource_templates.as_ref())
        .into_iter()
        .flat_map(|templates| templates.families.iter())
        .any(|family| {
            family.key == key
                && family.coverage_mode != ResourceCoverageMode::Unresolved
                && family.required_inputs.contains(&ResourceTemplateInput::CacheSourceSoundBucket)
        })
}

fn bucket_id_is_known_missing(bucket: &str, voice_id: u64) -> bool {
    bucket == "9998" && super::kc9998::is_missing_id(voice_id)
}

fn fallback_bucket_ids<'a>(
    bucket: &str,
    cache_source: &'a Option<CacheSource>,
) -> Option<&'a Vec<u64>> {
    let source = cache_source.as_ref()?;
    match bucket {
        "9997" => Some(&source.voices.event),
        "9998" => Some(&source.voices.abyssal),
        "9999" => Some(&source.voices.npc),
        _ => None,
    }
}

fn make_ship_voices(mst: &ApiManifest, rule: &CacheRuleShipVoiceRule, list: &mut CacheList) {
    if rule.coverage_mode == ResourceCoverageMode::Unresolved {
        return;
    }

    let Some(formula) = rule.formula.as_ref() else {
        return;
    };

    for graph in mst.api_mst_shipgraph.iter() {
        let Some(sort_no) = graph.api_sortno else {
            continue;
        };
        if sort_no == 0 {
            continue;
        }
        if !graph_satisfies_required_fields(graph, &rule.required_ship_graph_fields) {
            continue;
        }
        let Some(ship_mst) = mst.find_ship(graph.api_id) else {
            continue;
        };

        for voice_id in &rule.base_voice_ids {
            add_ship_voice(list, graph, formula, *voice_id);
        }

        let voice_flag = ship_mst.api_voicef.unwrap_or(0);
        if voice_flag & 1 != 0 {
            for voice_id in &rule.be_left_voice_ids {
                add_ship_voice(list, graph, formula, *voice_id);
            }
        }
        if voice_flag & 4 != 0 {
            for voice_id in &rule.be_left_tired_voice_ids {
                add_ship_voice(list, graph, formula, *voice_id);
            }
        }
        if voice_flag & 2 != 0
            && let (Some(start), Some(count)) =
                (rule.time_signal_start_voice_id, rule.time_signal_voice_count)
        {
            for voice_id in start..start + count {
                add_ship_voice(list, graph, formula, voice_id);
            }
        }
        if rule.special_art_ship_ids.contains(&graph.api_id) {
            for voice_id in &rule.special_voice_ids {
                add_ship_voice(list, graph, formula, *voice_id);
            }
        }
    }
}

fn graph_satisfies_required_fields(graph: &ApiMstShipgraph, required_fields: &[String]) -> bool {
    required_fields.iter().all(|field| match field.as_str() {
        "api_battle_n" => graph.api_battle_n.is_some(),
        "api_boko_d" => graph.api_boko_d.is_some(),
        _ => true,
    })
}

fn add_ship_voice(
    list: &mut CacheList,
    graph: &ApiMstShipgraph,
    formula: &CacheRuleShipVoiceFormula,
    voice_id: i64,
) {
    let resolved_voice_id = calc_voice_id(graph.api_id, voice_id, formula);
    let version = get_voice_version(graph, voice_id);
    list.add(format!("kcs/sound/kc{}/{}.mp3", graph.api_filename, resolved_voice_id), version);
}

fn calc_voice_id(ship_id: i64, voice_id: i64, formula: &CacheRuleShipVoiceFormula) -> i64 {
    if voice_id <= formula.max_formula_voice_id {
        let Some(diff) = formula.voice_diffs.get((voice_id - 1) as usize) else {
            return voice_id;
        };
        formula.base
            + formula.multiplier * (ship_id + formula.ship_id_offset) * diff % formula.modulo
    } else {
        voice_id
    }
}

fn get_voice_version(graph: &ApiMstShipgraph, voice_id: i64) -> i64 {
    let index = if voice_id == 2 || voice_id == 3 {
        2
    } else {
        1
    };
    graph.api_version.get(index).and_then(|value| value.parse().ok()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use emukc_model::kc2::start2::{ApiManifest, ApiMstShip, ApiMstShipgraph};

    use super::*;
    use crate::make_list::manifest::{CacheRuleSoundRules, ResourceCoverageMode};

    fn make_manifest() -> ApiManifest {
        ApiManifest {
            api_mst_ship: vec![ApiMstShip {
                api_id: 1,
                api_aftershipid: Some("2".to_string()),
                api_voicef: Some(7),
                api_ctype: 1,
                api_name: "Test".to_string(),
                api_slot_num: 0,
                api_soku: 10,
                api_sort_id: 1,
                api_stype: 2,
                api_yomi: "test".to_string(),
                ..Default::default()
            }],
            api_mst_shipgraph: vec![ApiMstShipgraph {
                api_id: 1,
                api_sortno: Some(1),
                api_battle_n: Some([0, 0]),
                api_boko_d: Some([0, 0]),
                api_filename: "foo".to_string(),
                api_version: vec!["0".to_string(), "11".to_string(), "22".to_string()],
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[test]
    fn test_make_ship_voices_from_rule() {
        let mut list = CacheList::new();
        let mut rules = CacheRuleSoundRules::default();
        rules.ship_voices.coverage_mode = ResourceCoverageMode::ObservedComplete;
        rules.ship_voices.formula = Some(CacheRuleShipVoiceFormula {
            base: 100000,
            multiplier: 17,
            ship_id_offset: 7,
            modulo: 99173,
            max_formula_voice_id: 53,
            voice_diffs: vec![2475],
        });
        rules.ship_voices.required_ship_graph_fields =
            vec!["api_battle_n".to_string(), "api_boko_d".to_string()];
        rules.ship_voices.base_voice_ids = vec![1];
        rules.ship_voices.be_left_voice_ids = vec![29];
        rules.ship_voices.be_left_tired_voice_ids = vec![129];
        rules.ship_voices.time_signal_start_voice_id = Some(30);
        rules.ship_voices.time_signal_voice_count = Some(2);
        rules.ship_voices.special_art_ship_ids = vec![1];
        rules.ship_voices.special_voice_ids = vec![900];

        make(&make_manifest(), &None, &rules, None, &mut list);
        let paths = list.into_path_set();

        assert!(paths.contains("kcs/sound/kcfoo/139081.mp3"));
        assert!(paths.contains("kcs/sound/kcfoo/29.mp3"));
        assert!(paths.contains("kcs/sound/kcfoo/129.mp3"));
        assert!(paths.contains("kcs/sound/kcfoo/30.mp3"));
        assert!(paths.contains("kcs/sound/kcfoo/31.mp3"));
        assert!(paths.contains("kcs/sound/kcfoo/900.mp3"));
    }

    #[test]
    fn test_make_bucket_rule() {
        let mut list = CacheList::new();
        let mut rules = CacheRuleSoundRules::default();
        rules.kc9999.coverage_mode = ResourceCoverageMode::Partial;
        rules.kc9999.bucket = "9999".to_string();
        rules.kc9999.voice_ids = vec![11, 12, 308];

        let cache_source = Some(CacheSource {
            voices: emukc_model::thirdparty::VoiceCacheSource {
                npc: vec![11, 12, 308],
                ..Default::default()
            },
        });

        make(&ApiManifest::default(), &cache_source, &rules, None, &mut list);
        let paths = list.into_path_set();

        assert!(paths.contains("kcs/sound/kc9999/11.mp3"));
        assert!(paths.contains("kcs/sound/kc9999/12.mp3"));
        assert!(paths.contains("kcs/sound/kc9999/308.mp3"));
    }
}
