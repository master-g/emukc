//! Sim→validate auto-gate (plan 2026-06-15-002 U1, R1/R7).
//!
//! For every registered scenario preset, this runs the simulation across a
//! bounded seed set and asserts the serialized `SortieBattleResponse` passes the
//! client-derived day-battle protocol rules (`validate_day_battle_response`) with
//! zero error-severity findings. It converts the previously manual `battle
//! validate` CLI check into an automatic gate: a protocol-level sim↔client drift
//! (missing field, wrong shape, invalid enemy id) now fails this test instead of
//! passing silently until a human remembers to run the CLI.
//!
//! Protocol conformance only — this does NOT assert numerical/behavioral
//! equivalence (damage correctness, attack-type triggers). See the plan's
//! scope-honesty note.
//!
//! Codex-gated, fail-loud per the repo convention (R7): a missing `.data/codex`
//! panics with the bootstrap prerequisite rather than silently skipping.

use emukc_bootstrap::prelude::{
    load_repo_battle_knowledge_assets, validate_day_battle_response, validate_night_battle_response,
};
use emukc_crypto::rng;
use emukc_db::{prelude::new_mem_db, sea_orm::DbConn};
use emukc_gameplay::prelude::*;
use emukc_model::codex::Codex;

/// Bounded RNG seed set. Seeds vary hit/miss/damage rolls within each preset's
/// attack-type path; they do not change which attack-type variant triggers (that
/// is equipment-dependent — see plan R1). Asserting protocol conformance holds
/// for every outcome, so a fixed seed list keeps the gate deterministic.
const SEEDS: &[u64] = &[1, 2, 3, 5, 8, 13];

async fn mock_context() -> (DbConn, Codex) {
    let db = new_mem_db().await.unwrap();
    let codex = Codex::load_without_cache_source("../../.data/codex")
        .expect("load codex from ../../.data/codex (run `cargo run -- bootstrap` first)");
    (db, codex)
}

async fn new_profile(context: &(DbConn, Codex)) -> i64 {
    let account = context.sign_up("sim-gate", "1234567").await.unwrap();
    let profile = context.new_profile(&account.access_token.token, "sim-gate").await.unwrap();
    let session =
        context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
    session.profile.id
}

#[tokio::test]
async fn every_preset_day_battle_passes_protocol_validation() {
    let assets = load_repo_battle_knowledge_assets().unwrap();

    for preset in PRESETS {
        let context = mock_context().await;
        let pid = new_profile(&context).await;
        apply_scenario(&context, pid, &(preset.build)())
            .await
            .unwrap_or_else(|e| panic!("preset {}: apply_scenario: {e:?}", preset.name));

        // Snapshot the post-scenario fleet so each seed starts from identical
        // state — accumulated sortie damage would otherwise make a run depend on
        // seed history rather than on (preset, seed) alone.
        let baseline = context.get_ships(pid).await.unwrap();

        for &seed in SEEDS {
            for ship in &baseline {
                context.update_ship(ship).await.unwrap();
            }
            rng::seed(seed);

            // start_sortie's defensive cleanup discards any stale sortie/battle
            // state left from the previous seed's run.
            context.start_sortie(pid, 1, preset.maparea, preset.mapinfo).await.unwrap_or_else(
                |e| panic!("preset {} seed {seed}: start_sortie: {e:?}", preset.name),
            );
            let battle = context.sortie_battle(pid, 1).await.unwrap_or_else(|e| {
                panic!("preset {} seed {seed}: sortie_battle: {e:?}", preset.name)
            });

            let report =
                validate_day_battle_response(&context.1.manifest, &battle, &assets).unwrap();
            assert!(
                !report.has_errors(),
                "preset {} seed {seed} produced day-battle protocol errors: {:#?}",
                preset.name,
                report.findings,
            );
            assert!(
                !report.expected_resources.is_empty(),
                "preset {} seed {seed}: validator inferred no expected resources",
                preset.name,
            );
        }
        rng::reseed_from_entropy();
    }
}

/// The gate is not vacuously green: a deliberately-corrupted payload (invalid
/// enemy ship + slot ids) must fail validation, mirroring the manual
/// `sortie_battle_validation_reports_invalid_enemy_ids` check but driven through
/// the registry so the gate's teeth are proven on the same path it guards.
#[tokio::test]
async fn gate_bites_on_corrupted_payload() {
    let assets = load_repo_battle_knowledge_assets().unwrap();
    let preset = &PRESETS[0];

    let context = mock_context().await;
    let pid = new_profile(&context).await;
    apply_scenario(&context, pid, &(preset.build)()).await.unwrap();

    rng::seed(SEEDS[0]);
    context.start_sortie(pid, 1, preset.maparea, preset.mapinfo).await.unwrap();
    let battle = context.sortie_battle(pid, 1).await.unwrap();
    rng::reseed_from_entropy();

    let mut raw = serde_json::to_value(&battle).unwrap();
    raw["api_ship_ke"][0] = serde_json::json!(999999);
    raw["api_eSlot"][0][0] = serde_json::json!(888888);

    let report = validate_day_battle_response(&context.1.manifest, &raw, &assets).unwrap();
    assert!(report.has_errors(), "corrupted payload must fail the gate");
}

/// Night-path gate (plan 2026-06-15-002 U2): drive a real night battle for every
/// registered preset and validate the emitted `SortieNightBattleResponse` with
/// `validate_night_battle_response`, mirroring the day gate.
///
/// The night packet is produced via `sortie_sp_midnight_battle` (the night-only
/// sortie entry), NOT the day→midnight branch. The day→midnight path requires the
/// flagship to survive and *fail* to clear the enemy — a rare RNG branch that is
/// not reliably reachable in 1-1 within a bounded seed set (the existing CLI
/// `seed_search_finds_night_and_reproduces` test, which hunts that same branch,
/// is itself flaky on the current sim). `sortie_sp_midnight_battle` exercises the
/// identical `simulate_night` → `build_night_response` path deterministically, so
/// it is the reliable live-sim night packet for this protocol gate. Each preset
/// is run across the same bounded seed set as the day gate; every produced night
/// packet must pass protocol validation with zero error findings.
#[tokio::test]
async fn every_preset_night_battle_passes_protocol_validation() {
    let assets = load_repo_battle_knowledge_assets().unwrap();

    for preset in PRESETS {
        let context = mock_context().await;
        let pid = new_profile(&context).await;
        apply_scenario(&context, pid, &(preset.build)())
            .await
            .unwrap_or_else(|e| panic!("preset {}: apply_scenario: {e:?}", preset.name));

        let baseline = context.get_ships(pid).await.unwrap();

        for &seed in SEEDS {
            // Discard any stale sortie state from the previous seed; restore the
            // fleet so each seed starts from identical state.
            context.clear_sortie_state_if_any(pid).await;
            for ship in &baseline {
                context.update_ship(ship).await.unwrap();
            }
            rng::seed(seed);

            context.start_sortie(pid, 1, preset.maparea, preset.mapinfo).await.unwrap_or_else(
                |e| panic!("preset {} seed {seed}: start_sortie: {e:?}", preset.name),
            );
            let night = context.sortie_sp_midnight_battle(pid, 1).await.unwrap_or_else(|e| {
                panic!("preset {} seed {seed}: sortie_sp_midnight_battle: {e:?}", preset.name)
            });

            let report =
                validate_night_battle_response(&context.1.manifest, &night, &assets).unwrap();
            assert!(
                !report.has_errors(),
                "preset {} seed {seed} produced night-battle protocol errors: {:#?}",
                preset.name,
                report.findings,
            );
            assert!(
                night.api_hougeki.is_some(),
                "preset {} seed {seed}: night battle emitted no night shelling",
                preset.name,
            );
        }
        rng::reseed_from_entropy();
    }
}
