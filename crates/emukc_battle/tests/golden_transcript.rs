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

use std::fs;
use std::path::{Path, PathBuf};

use emukc_battle::{
    BattleContext, BattleRng, BattleRuntimeShip, BattleShipInput, BattleType, EngagementType,
    NightBattleInput, simulate_day, simulate_night,
};
use emukc_crypto::rng::GameRng;
use emukc_model::codex::Codex;
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
    Codex::load_without_cache_source("../../.data/codex")
        .expect("load codex from .data/codex (run `cargo run -- bootstrap` first)")
}

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

/// Build a fully-statted ship from a master id, mirroring the crate's internal
/// `sample_ship` test helper (which integration tests cannot reach).
fn sample_ship(codex: &Codex, mst_id: i64, level: i64) -> BattleShipInput {
    let (mut ship, slot_items) = codex.new_ship(mst_id).expect("known master id");
    let exp_now = ship_level_required_exp(level);
    let (_, next_exp) = exp_to_ship_level(exp_now);
    ship.api_lv = level;
    ship.api_exp = [exp_now, next_exp, 0];
    codex.cal_ship_status(&mut ship, &slot_items, false).expect("calculate ship status");
    BattleShipInput {
        ship,
        slot_items,
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
    BattleContext {
        battle_type: BattleType::Normal,
        is_sortie: true,
        friendly_formation_id: 1,
        enemy_formation_id: 1,
        engagement: EngagementType::SameCourse,
        friend_ships: vec![attacker(codex), attacker(codex)],
        enemy_ships: vec![target(codex), target(codex)],
    }
}

fn night_input(codex: &Codex) -> NightBattleInput {
    NightBattleInput {
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
        let snapshot = format!("{:#?}", simulate_day(&codex, day_context(&codex), &mut rng).packet);

        // Same seed must reproduce a byte-identical packet.
        let mut rng_repeat = SeededRng::new(seed);
        let repeat =
            format!("{:#?}", simulate_day(&codex, day_context(&codex), &mut rng_repeat).packet);
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
            format!("{:#?}", simulate_night(&codex, night_input(&codex), &mut rng).packet);

        // Same seed must reproduce a byte-identical packet.
        let mut rng_repeat = SeededRng::new(seed);
        let repeat =
            format!("{:#?}", simulate_night(&codex, night_input(&codex), &mut rng_repeat).packet);
        assert_eq!(snapshot, repeat, "night battle not deterministic for seed {seed}");

        check_golden(&format!("night_seed_{seed:02}"), &snapshot);
    }
}
