//! Golden transcript determinism safety net (plan 010 U0).
//!
//! Captures `BattlePacket` / `NightBattlePacket` snapshots for fixed day and
//! night battle setups across seeds 1..=20. These golden files are the
//! determinism safety net for the owned-pass refactor: any change that alters
//! simulation output (RNG draw order, damage, targeting) makes these tests fail,
//! catching unintended behavior drift while phases are rewritten.
//!
//! Snapshots use the `{:#?}` debug rendering rather than JSON: `BattlePacket`
//! and `NightBattlePacket` do not derive `Serialize`, and adding that derive
//! purely for a test would touch production types the refactor (U6) will
//! replace. Debug output is equally deterministic for these integer-only
//! structs.
//!
//! Re-bless after an intentional logic change:
//! `env EMUKC_BLESS_GOLDEN=1 cargo test -p emukc_battle --test golden_transcript`
//!
//! The setups read real ships, their stock equipment and the abyssal data from
//! `.data/codex`, so rebuilding the codex (`make update`, `make gear-bonus-update`,
//! `cargo run -- bootstrap --codex-only`) can turn these red with no code
//! change. After a rebuild, re-bless on `main` in a commit of its own that says
//! the data changed, before any other work; a refactor must never need one.

use std::fs;
use std::path::{Path, PathBuf};

use emukc_battle::{
    AirCorpsInput, AirSquadronInput, AirState, BattleContext, BattleRng, BattleRuntimeShip,
    BattleShipInput, BattleSimulation, BattleType, CombinedFleetRole, CombinedSetup, CombinedType,
    EngagementType, NightBattleInput, any_alive, execute_day, execute_night, execute_sp_midnight,
    night_enemy_deck,
};
use emukc_crypto::rng::GameRng;
use emukc_model::codex::Codex;
use emukc_model::kc2::api::KcApiSlotItem;
use emukc_model::kc2::level::{exp_to_ship_level, ship_level_required_exp};

/// Deterministic RNG mirroring the crate-internal `SeededRng`, which is
/// `#[cfg(test)]` and unreachable from an integration test. Same `GameRng`
/// backend and seeding, so it draws the identical sequence.
struct SeededRng {
    inner: GameRng,
}

impl SeededRng {
    fn new(seed: u64) -> Self {
        Self {
            inner: GameRng::seeded(seed),
        }
    }
}

impl BattleRng for SeededRng {
    fn random_f64_range(&mut self, min: f64, max: f64) -> f64 {
        self.inner.f64_range(min, max)
    }

    fn roll_range_impl(&mut self, min: i64, max: i64) -> i64 {
        self.inner.i64(min..max)
    }
}

/// Number of RNG seeds captured per battle type.
const SEED_COUNT: u64 = 20;

/// Friendly ship master id used across all setups (validated by existing tests).
const FRIENDLY_MST: i64 = 79;
/// Enemy ship master id used across all setups (validated by existing tests).
const ENEMY_MST: i64 = 412;

fn load_codex() -> Codex {
    let mut codex = Codex::load_without_cache_source("../../.data/codex")
        .expect("load codex from .data/codex (run `cargo run -- bootstrap` first)");
    codex.game_cfg.god_mode = false;
    codex.game_cfg.one_hit_kill = false;
    codex
}

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

/// Build a fully-statted ship from a master id, mirroring the crate's internal
/// `sample_ship` test helper (which integration tests cannot reach).
fn sample_ship(codex: &Codex, mst_id: i64, level: i64) -> BattleShipInput {
    let (mut ship, slot_items) =
        codex.new_ship(mst_id).unwrap_or_else(|| panic!("unknown master id {mst_id}"));
    let exp_now = ship_level_required_exp(level);
    let (_, next_exp) = exp_to_ship_level(exp_now);
    ship.api_lv = level;
    ship.api_exp = [exp_now, next_exp, 0];
    codex.cal_ship_status(&mut ship, &slot_items, false).expect("calculate ship status");
    BattleShipInput {
        ship,
        slot_items,
        aircraft_exp: vec![],
        effect_list: vec![0],
        married: false,
    }
}

/// High-firepower attacker so shelling and torpedo phases produce damage.
fn attacker(codex: &Codex) -> BattleShipInput {
    let mut s = sample_ship(codex, FRIENDLY_MST, 99);
    s.ship.api_karyoku[0] = 120;
    s.ship.api_raisou[0] = 120;
    s.ship.api_soukou[0] = 60;
    s.ship.api_nowhp = 60;
    s.ship.api_maxhp = 60;
    s
}

/// Lightly-armored target so attacks land and ships can sink across seeds.
fn target(codex: &Codex) -> BattleShipInput {
    let mut s = sample_ship(codex, ENEMY_MST, 30);
    s.ship.api_karyoku[0] = 20;
    s.ship.api_raisou[0] = 10;
    s.ship.api_soukou[0] = 10;
    s.ship.api_nowhp = 40;
    s.ship.api_maxhp = 40;
    s
}

fn day_context(codex: &Codex) -> BattleContext {
    BattleContext::head_on(
        BattleType::Normal,
        true,
        vec![attacker(codex), attacker(codex)],
        vec![target(codex), target(codex)],
    )
}

fn night_input(codex: &Codex) -> NightBattleInput {
    NightBattleInput {
        special_attacks_made: Default::default(),
        friendly: vec![
            BattleRuntimeShip::new(attacker(codex), true, true),
            BattleRuntimeShip::new(attacker(codex), true, true),
        ],
        enemy: vec![
            BattleRuntimeShip::new(target(codex), false, true),
            BattleRuntimeShip::new(target(codex), false, true),
        ],
        friendly_formation_id: 1,
        enemy_formation_id: 1,
        engagement: EngagementType::SameCourse,
        air_state: None,
        night_start: false,
    }
}

/// Compare a snapshot against its golden file, or rewrite it when blessing.
fn check_golden(name: &str, actual: &str) {
    let path = golden_dir().join(format!("{name}.txt"));
    if std::env::var_os("EMUKC_BLESS_GOLDEN").is_some() {
        fs::create_dir_all(golden_dir()).expect("create golden dir");
        fs::write(&path, actual).expect("write golden file");
        return;
    }
    let expected = fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "missing golden {}. Regenerate with \
             `env EMUKC_BLESS_GOLDEN=1 cargo test -p emukc_battle --test golden_transcript`",
            path.display()
        )
    });
    assert_eq!(
        actual, expected,
        "golden mismatch for {name}: simulation output changed. \
         If this is an intentional logic change, re-bless with EMUKC_BLESS_GOLDEN=1."
    );
}

#[test]
fn day_battle_golden_transcripts() {
    let codex = load_codex();
    for seed in 1..=SEED_COUNT {
        let mut rng = SeededRng::new(seed);
        let snapshot = format!("{:#?}", execute_day(&codex, day_context(&codex), &mut rng).packet);

        // Same seed must reproduce a byte-identical packet.
        let mut rng_repeat = SeededRng::new(seed);
        let repeat =
            format!("{:#?}", execute_day(&codex, day_context(&codex), &mut rng_repeat).packet);
        assert_eq!(snapshot, repeat, "day battle not deterministic for seed {seed}");

        check_golden(&format!("day_seed_{seed:02}"), &snapshot);
    }
}

#[test]
fn night_battle_golden_transcripts() {
    let codex = load_codex();
    for seed in 1..=SEED_COUNT {
        let mut rng = SeededRng::new(seed);
        let snapshot =
            format!("{:#?}", execute_night(&codex, night_input(&codex), &mut rng).packet);

        // Same seed must reproduce a byte-identical packet.
        let mut rng_repeat = SeededRng::new(seed);
        let repeat =
            format!("{:#?}", execute_night(&codex, night_input(&codex), &mut rng_repeat).packet);
        assert_eq!(snapshot, repeat, "night battle not deterministic for seed {seed}");

        check_golden(&format!("night_seed_{seed:02}"), &snapshot);
    }
}

// ---------------------------------------------------------------------------
// Fleet-shape coverage: combined fleets, enemy combined fleets, night-start
// and practice. Real ships with their stock equipment, so the air, opening
// ASW, torpedo, cut-in and night phases all have something to do. Each
// snapshot holds the packets and outcomes, never the runtime ships, whose
// bookkeeping is not battle output.
// ---------------------------------------------------------------------------

/// 長門改, 陸奥改, 赤城改, 加賀改, 利根改二, 妙高改二.
const SURFACE_AND_CARRIERS: [i64; 6] = [275, 276, 277, 278, 188, 319];
/// 五十鈴改二, 大井改二, 北上改二, 夕立改二, 時雨改二, 綾波改二.
const ESCORTS: [i64; 6] = [141, 118, 119, 144, 145, 195];
/// 長門改, 赤城改, 五十鈴改二, 大井改二, 北上改二, 夕立改二.
const SINGLE_FLEET: [i64; 6] = [275, 277, 141, 118, 119, 144];
/// 金剛改二, 比叡改二, 翔鶴改二, 瑞鶴改二, 時雨改二, 伊58改.
const PRACTICE_RIVAL: [i64; 6] = [149, 150, 461, 462, 145, 399];
/// 戦艦ル級, 空母ヲ級, 重巡リ級, 雷巡チ級, 潜水カ級, 駆逐イ級.
const ENEMY_SINGLE: [i64; 6] = [1511, 1512, 1509, 1508, 1530, 1501];
/// 戦艦タ級, 空母ヲ級, 軽母ヌ級, 重巡リ級, 戦艦ル級, 駆逐イ級.
const ENEMY_MAIN: [i64; 6] = [1541, 1512, 1510, 1509, 1511, 1501];
/// 軽巡ホ級, 雷巡チ級, 潜水ヨ級, 駆逐イ級 ×3.
const ENEMY_ESCORT: [i64; 6] = [1505, 1508, 1531, 1501, 1501, 1501];

/// 四式水中聴音機, 三式水中探信儀, 三式爆雷投射機: what lets 五十鈴改二 and
/// 夕立改二 open with ASW.
const ASW_LOADOUT: [i64; 3] = [149, 47, 45];

fn ships(codex: &Codex, ids: &[i64], level: i64) -> Vec<BattleShipInput> {
    ids.iter()
        .map(|&id| {
            let mut ship = sample_ship(codex, id, level);
            if matches!(id, 141 | 144) {
                ship.ship.api_slot = [1, 2, 3, -1, -1];
                ship.slot_items = (1..)
                    .zip(ASW_LOADOUT)
                    .map(|(api_id, mst_id)| KcApiSlotItem {
                        api_id,
                        api_slotitem_id: mst_id,
                        api_locked: 0,
                        api_level: 0,
                        api_alv: None,
                    })
                    .collect();
                codex
                    .cal_ship_status(&mut ship.ship, &ship.slot_items, false)
                    .expect("calculate ship status");
            }
            ship
        })
        .collect()
}

/// Abyssal ships, built from the enemy data as a sortie builds them.
fn abyssal(codex: &Codex, ids: &[i64]) -> Vec<BattleShipInput> {
    ids.iter()
        .map(|&id| {
            let (mut ship, slot_items) = codex.new_enemy_ship(id).expect("known abyssal id");
            ship.api_lv = 1;
            // Sturdy enough that most of them see the night.
            ship.api_maxhp *= 4;
            ship.api_nowhp = ship.api_maxhp;
            BattleShipInput {
                ship,
                slot_items,
                aircraft_exp: vec![],
                effect_list: vec![0],
                married: false,
            }
        })
        .collect()
}

fn fleet_context(
    codex: &Codex,
    is_sortie: bool,
    friend: &[i64],
    enemy: &[i64],
    formations: (i64, i64),
) -> BattleContext {
    let mut context = BattleContext::head_on(
        BattleType::Normal,
        is_sortie,
        ships(codex, friend, 99),
        if is_sortie {
            abyssal(codex, enemy)
        } else {
            ships(codex, enemy, 99)
        },
    );
    (context.friendly_formation_id, context.enemy_formation_id) = formations;
    context
}

fn friendly_combined(codex: &Codex, combined_type: CombinedType, formation: i64) -> BattleContext {
    let mut context =
        fleet_context(codex, true, &SURFACE_AND_CARRIERS, &ENEMY_SINGLE, (formation, 1));
    context.combined = Some(CombinedSetup {
        combined_type,
        escort_ships: ships(codex, &ESCORTS, 99),
    });
    context
}

fn enemy_combined(codex: &Codex) -> BattleContext {
    let mut context = fleet_context(codex, true, &SINGLE_FLEET, &ENEMY_MAIN, (1, 14));
    context.enemy_escort_ships = abyssal(codex, &ENEMY_ESCORT);
    context
}

/// A single-fleet sortie with two land attack corps, the second pointed here
/// twice. A friendly combined fleet flies no air corps.
fn single_with_air_corps(codex: &Codex) -> BattleContext {
    let mut context = fleet_context(codex, true, &SINGLE_FLEET, &ENEMY_SINGLE, (1, 1));
    context.air_corps = vec![
        AirCorpsInput {
            base_rid: 1,
            waves: 1,
            squadrons: (1..=4).map(|id| land_attackers(id, 169)).collect(),
        },
        AirCorpsInput {
            base_rid: 2,
            waves: 2,
            squadrons: (1..=4).map(|id| land_attackers(id, 187)).collect(),
        },
    ];
    context
}

fn land_attackers(squadron_id: i64, mst_id: i64) -> AirSquadronInput {
    AirSquadronInput {
        squadron_id,
        mst_id,
        count: 18,
        alv: 7,
        level: 0,
        exp: None,
    }
}

/// The night after a day battle, fought as a sortie fights it: 第2艦隊 alone for
/// a combined fleet, and against one deck of an enemy combined fleet picked by
/// the escort fleet's condition.
fn night_after(day: &BattleSimulation, formations: (i64, i64)) -> NightBattleInput {
    let night_start = day.friendly.iter().take_while(|ship| ship.is_main_deck()).count();
    let enemy = match day.enemy.iter().position(BattleRuntimeShip::is_escort_deck) {
        None => day.enemy.clone(),
        Some(start) => {
            let escort_hps: Vec<(i64, i64)> =
                day.enemy[start..].iter().map(|ship| (ship.hp(), ship.ship.api_maxhp)).collect();
            match night_enemy_deck(&escort_hps, any_alive(&day.enemy[..start])) {
                CombinedFleetRole::Main => day.enemy[..start].to_vec(),
                CombinedFleetRole::Escort => day.enemy[start..].to_vec(),
            }
        }
    };
    NightBattleInput {
        special_attacks_made: Default::default(),
        friendly: day.friendly[night_start..].to_vec(),
        enemy,
        friendly_formation_id: formations.0,
        enemy_formation_id: formations.1,
        engagement: EngagementType::SameCourse,
        air_state: day
            .packet
            .kouku
            .as_ref()
            .and_then(|kouku| AirState::from_api_disp_seiku(kouku.api_stage1.api_disp_seiku)),
        night_start: false,
    }
}

/// A day battle and the night after it, as one snapshot.
fn day_and_night(codex: &Codex, context: BattleContext, rng: &mut SeededRng) -> String {
    let formations = (context.friendly_formation_id, context.enemy_formation_id);
    let day = execute_day(codex, context, rng);
    let night = execute_night(codex, night_after(&day, formations), rng);
    format!("{:#?}\n{:#?}\n{:#?}\n{:#?}", day.packet, day.outcome, night.packet, night.outcome)
}

/// Run `battle` for every seed, check it reproduces, and compare it to
/// `<name>_seed_NN`.
fn check_seeds(name: &str, battle: impl Fn(&mut SeededRng) -> String) {
    for seed in 1..=SEED_COUNT {
        let snapshot = battle(&mut SeededRng::new(seed));
        assert_eq!(
            snapshot,
            battle(&mut SeededRng::new(seed)),
            "{name} not deterministic for seed {seed}"
        );
        check_golden(&format!("{name}_seed_{seed:02}"), &snapshot);
    }
}

#[test]
fn single_fleet_golden_transcripts() {
    let codex = load_codex();
    check_seeds("single", |rng| day_and_night(&codex, single_with_air_corps(&codex), rng));
}

#[test]
fn carrier_task_force_golden_transcripts() {
    let codex = load_codex();
    check_seeds("ctf", |rng| {
        day_and_night(&codex, friendly_combined(&codex, CombinedType::CarrierTaskForce, 11), rng)
    });
}

#[test]
fn surface_task_force_golden_transcripts() {
    let codex = load_codex();
    check_seeds("stf", |rng| {
        day_and_night(&codex, friendly_combined(&codex, CombinedType::SurfaceTaskForce, 14), rng)
    });
}

#[test]
fn transport_escort_golden_transcripts() {
    let codex = load_codex();
    check_seeds("te", |rng| {
        day_and_night(&codex, friendly_combined(&codex, CombinedType::TransportEscort, 12), rng)
    });
}

#[test]
fn enemy_combined_golden_transcripts() {
    let codex = load_codex();
    check_seeds("ec", |rng| day_and_night(&codex, enemy_combined(&codex), rng));
}

#[test]
fn sp_midnight_golden_transcripts() {
    let codex = load_codex();
    check_seeds("sp_midnight", |rng| {
        let sim = execute_sp_midnight(
            &codex,
            friendly_combined(&codex, CombinedType::CarrierTaskForce, 14),
            rng,
        );
        let main_deck: Vec<i64> = sim.main_deck.iter().map(BattleRuntimeShip::hp).collect();
        format!("{main_deck:?}\n{:#?}\n{:#?}", sim.night.packet, sim.night.outcome)
    });
}

#[test]
fn practice_golden_transcripts() {
    let codex = load_codex();
    check_seeds("practice", |rng| {
        day_and_night(
            &codex,
            fleet_context(&codex, false, &SINGLE_FLEET, &PRACTICE_RIVAL, (1, 1)),
            rng,
        )
    });
}
