use emukc_cache::Kache;
use emukc_model::codex::Codex;

use crate::{
    make_list::{CacheList, CacheListAuthorityStage, CacheListMakeStrategy, manifest},
    prelude::CacheListMakingError,
};

mod kc9997;
mod kc9998;
mod kc9999;
mod purchase;
mod sound_rules;
mod voice;

pub(super) async fn make(
    codex: &Codex,
    cache: &Kache,
    strategy: CacheListMakeStrategy,
    rules_bundle: Option<&manifest::DecoderRulesBundle>,
    list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
    if strategy == CacheListMakeStrategy::Manifest {
        let previous = list.set_authority_stage(Some(CacheListAuthorityStage::FallbackAuthored));
        kc9997::make(&codex.cache_source, list).await?;
        kc9998::make(&codex.cache_source, list, &strategy);
        kc9999::make(&codex.cache_source, list).await?;
        purchase::make(&codex.manifest, list);
        voice::make(&codex.manifest, cache, &strategy, list).await?;
        list.set_authority_stage(previous);
        return Ok(());
    }

    let sound_rules = rules_bundle.map(|rules_bundle| &rules_bundle.cache_rules.sound_rules);
    if let Some(rules_bundle) = rules_bundle {
        let previous = list.set_authority_stage(Some(CacheListAuthorityStage::RuleAuthored));
        sound_rules::make(
            &codex.manifest,
            &codex.cache_source,
            &rules_bundle.cache_rules.sound_rules,
            Some(&rules_bundle.decoder_assets),
            list,
        );
        list.set_authority_stage(previous);
    }

    let previous = list.set_authority_stage(Some(CacheListAuthorityStage::FallbackAuthored));
    if !has_complete_sound_rule(sound_rules.map(|rules| &rules.kc9997)) {
        kc9997::make(&codex.cache_source, list).await?;
    }
    if !has_complete_sound_rule(sound_rules.map(|rules| &rules.kc9998)) {
        kc9998::make(&codex.cache_source, list, &strategy);
    }
    if !has_complete_sound_rule(sound_rules.map(|rules| &rules.kc9999)) {
        kc9999::make(&codex.cache_source, list).await?;
    }
    purchase::make(&codex.manifest, list);
    if !has_complete_ship_voice_rule(sound_rules) {
        voice::make(&codex.manifest, cache, &strategy, list).await?;
    }
    list.set_authority_stage(previous);

    Ok(())
}

fn has_complete_sound_rule(rule: Option<&manifest::CacheRuleSoundBucketRule>) -> bool {
    rule.is_some_and(|rule| rule.coverage_mode == manifest::ResourceCoverageMode::ObservedComplete)
}

fn has_complete_ship_voice_rule(rules: Option<&manifest::CacheRuleSoundRules>) -> bool {
    rules.is_some_and(|rules| {
        rules.ship_voices.coverage_mode == manifest::ResourceCoverageMode::ObservedComplete
    })
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use emukc_cache::Kache;
    use emukc_model::{
        codex::Codex,
        thirdparty::{CacheSource, VoiceCacheSource},
    };
    use serde_json::json;

    use super::*;
    use crate::make_list::{
        CacheList,
        manifest::{self, ResourceCoverageMode},
    };

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn tmp_dir(name: &str) -> PathBuf {
        let dir = repo_root().join(".data/tmp").join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).unwrap();
        }
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn make_kache(name: &str) -> Kache {
        let dir = tmp_dir(name);
        Kache::builder()
            .with_cache_root(dir.clone())
            .with_db_path(dir.join("kache.redb").to_string_lossy().into_owned())
            .with_gadgets_cdn("http://invalid.local/".to_string())
            .with_content_cdn("http://invalid.local/".to_string())
            .build()
            .unwrap()
    }

    fn write_bucket_rules_file(
        path: &std::path::Path,
        coverage_mode: ResourceCoverageMode,
        bucket: &str,
        voice_ids: &[i64],
    ) {
        fs::write(
            path,
            serde_json::to_string_pretty(&json!({
                "version": 1,
                "generatedAt": "2026-04-25T00:00:00Z",
                "scriptVersion": "6.2.8.0",
                "resourceManifest": {
                    "version": 2,
                    "generatedAt": "2026-04-25T00:00:00Z",
                    "summary": {},
                    "pathRules": null,
                    "entries": []
                },
                "soundRules": {
                    format!("kc{bucket}"): {
                        "coverageMode": coverage_mode,
                        "kind": "sound_bucket",
                        "bucket": bucket,
                        "voiceIds": voice_ids,
                        "hasDynamicVoiceIds": coverage_mode == ResourceCoverageMode::Partial,
                        "moduleIds": [],
                        "moduleNames": []
                    }
                }
            }))
            .unwrap(),
        )
        .unwrap();
    }

    #[tokio::test]
    async fn test_rules_complete_sound_bucket_skips_matching_fallback() {
        let mut codex = Codex {
            cache_source: Some(CacheSource {
                voices: VoiceCacheSource {
                    npc: vec![11],
                    ..Default::default()
                },
            }),
            ..Default::default()
        };
        codex.manifest = Default::default();

        let rules_dir = tmp_dir("kcs-rules-complete-sound-bucket-rules");
        let rules_path = rules_dir.join("cache_rules.json");
        write_bucket_rules_file(&rules_path, ResourceCoverageMode::ObservedComplete, "9999", &[11]);
        let rules_bundle = manifest::load_cache_rules_bundle_from_path(&rules_path).unwrap();

        let mut list = CacheList::new();
        make(
            &codex,
            &make_kache("kcs-rules-complete-sound-bucket"),
            CacheListMakeStrategy::Rules,
            Some(&rules_bundle),
            &mut list,
        )
        .await
        .unwrap();

        let path = "kcs/sound/kc9999/11.mp3";
        assert_eq!(list.items.iter().filter(|item| item.path == path).count(), 1);

        let output = list.into_path_build_output();
        assert!(output.diagnostics.rule_authored_paths.contains(&path.to_string()));
        assert!(!output.diagnostics.fallback_authored_paths.contains(&path.to_string()));
    }

    #[tokio::test]
    async fn test_rules_partial_sound_bucket_keeps_fallback_attribution() {
        let mut codex = Codex {
            cache_source: Some(CacheSource {
                voices: VoiceCacheSource {
                    npc: vec![11, 12],
                    ..Default::default()
                },
            }),
            ..Default::default()
        };
        codex.manifest = Default::default();

        let rules_dir = tmp_dir("kcs-rules-partial-sound-bucket-rules");
        let rules_path = rules_dir.join("cache_rules.json");
        write_bucket_rules_file(&rules_path, ResourceCoverageMode::Partial, "9999", &[11]);
        let rules_bundle = manifest::load_cache_rules_bundle_from_path(&rules_path).unwrap();

        let mut list = CacheList::new();
        make(
            &codex,
            &make_kache("kcs-rules-partial-sound-bucket"),
            CacheListMakeStrategy::Rules,
            Some(&rules_bundle),
            &mut list,
        )
        .await
        .unwrap();

        let output = list.into_path_build_output();
        assert!(
            output.diagnostics.rule_authored_paths.contains(&"kcs/sound/kc9999/11.mp3".to_string())
        );
        assert!(
            output
                .diagnostics
                .fallback_authored_paths
                .contains(&"kcs/sound/kc9999/12.mp3".to_string())
        );
    }

    #[tokio::test]
    async fn test_rules_partial_kc9998_uses_cache_source_template_membership() {
        let mut codex = Codex {
            cache_source: Some(CacheSource {
                voices: VoiceCacheSource {
                    abyssal: vec![11, 12],
                    ..Default::default()
                },
            }),
            ..Default::default()
        };
        codex.manifest = Default::default();

        let rules_dir = tmp_dir("kcs-rules-partial-kc9998-cache-source-template");
        let rules_path = rules_dir.join("cache_rules.json");
        write_bucket_rules_file(&rules_path, ResourceCoverageMode::Partial, "9998", &[11]);
        fs::write(
            rules_dir.join("resource_templates.json"),
            serde_json::to_string_pretty(&json!({
                "version": 1,
                "generatedAt": "2026-04-25T00:00:00Z",
                "scriptVersion": "6.2.8.0",
                "families": [{
                    "key": "sound.kc9998",
                    "domain": "sound",
                    "outputPrefix": "kcs/sound/kc9998",
                    "pathTemplate": [],
                    "requiredInputs": ["cache-source.sound-bucket"],
                    "coverageMode": "partial",
                    "provenance": {}
                }],
                "unresolvedFamilies": []
            }))
            .unwrap(),
        )
        .unwrap();
        let rules_bundle = manifest::load_cache_rules_bundle_from_path(&rules_path).unwrap();

        let mut list = CacheList::new();
        make(
            &codex,
            &make_kache("kcs-rules-partial-kc9998-cache-source-template"),
            CacheListMakeStrategy::Rules,
            Some(&rules_bundle),
            &mut list,
        )
        .await
        .unwrap();

        let output = list.into_path_build_output();
        assert!(
            output.diagnostics.rule_authored_paths.contains(&"kcs/sound/kc9998/11.mp3".to_string())
        );
        assert!(
            output.diagnostics.rule_authored_paths.contains(&"kcs/sound/kc9998/12.mp3".to_string())
        );
        assert!(
            !output
                .diagnostics
                .fallback_authored_paths
                .contains(&"kcs/sound/kc9998/12.mp3".to_string())
        );
    }
}
