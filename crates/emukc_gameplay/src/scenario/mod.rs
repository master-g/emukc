//! One-shot scenario / state builder.
//!
//! Puts a fresh profile into a declared target state in a single call, skipping
//! the manual create-account → PvP-for-exp → sortie-to-unlock → repair loop. It
//! composes the existing gameplay operations (ship ops, material ops,
//! fleet ops) plus the KTD-5 direct map clear/unlock setter, so the
//! `battle sim` CLI and the integration tests share one builder.
//!
//! The builder operates over an existing profile (created via the usual
//! sign-up / new-profile / start-game flow); it seeds ship, material, fleet, and
//! map state only.

use emukc_db::sea_orm::TransactionTrait;
use emukc_model::kc2::{KcApiShip, MaterialCategory, level};

/// Slots a ship can carry equipment in, excluding the reinforcement expansion.
const MAX_DECLARED_SLOTS: usize = 5;

use crate::{
    err::GameplayError,
    game::{clear_and_unlock_map_impl, unlock_map_impl},
    gameplay::Ctx,
};

/// A single ship to place in the scenario fleet.
#[derive(Debug, Clone)]
pub struct ShipSpec {
    /// Ship manifest (master) id.
    pub mst_id: i64,
    /// Target level.
    pub level: i64,
    /// Optional current-HP override (e.g., a damaged flagship).
    pub hp: Option<i64>,
    /// Optional fuel override.
    pub fuel: Option<i64>,
    /// Optional ammo override.
    pub ammo: Option<i64>,
    /// Equipment manifest ids, by slot index. Entries past the ship's own slot
    /// count are ignored.
    pub slots: Vec<i64>,
    /// Optional anti-submarine modernisation (`api_kyouka[6]`).
    ///
    /// Modernisation is an *input* to `cal_ship_status`, which is why it can be
    /// declared while a derived stat like `api_taisen[0]` cannot: every write
    /// path ends in a recalculation that would overwrite the derived value.
    pub asw_mod: Option<i64>,
}

impl ShipSpec {
    /// A ship at the given master id and level, with no stat overrides.
    pub fn new(mst_id: i64, level: i64) -> Self {
        Self {
            mst_id,
            level,
            hp: None,
            fuel: None,
            ammo: None,
            slots: Vec::new(),
            asw_mod: None,
        }
    }

    /// Set a current-HP override (builder style).
    #[must_use]
    pub fn with_hp(mut self, hp: i64) -> Self {
        self.hp = Some(hp);
        self
    }

    /// Equip the given manifest ids, slot 0 first (builder style).
    #[must_use]
    pub fn with_slots(mut self, slots: impl IntoIterator<Item = i64>) -> Self {
        self.slots = slots.into_iter().collect();
        self
    }

    /// Set the anti-submarine modernisation value (builder style).
    #[must_use]
    pub fn with_asw_mod(mut self, asw_mod: i64) -> Self {
        self.asw_mod = Some(asw_mod);
        self
    }
}

/// A declarative target state applied over a fresh profile.
#[derive(Debug, Clone, Default)]
pub struct Scenario {
    /// Ships placed into fleet 1, in order.
    pub fleet: Vec<ShipSpec>,
    /// Materials to add.
    pub materials: Vec<(MaterialCategory, i64)>,
    /// Maps to unlock without clearing.
    pub unlock_maps: Vec<i64>,
    /// Maps to mark cleared (in dependency order); each cascades unlock to its
    /// dependents.
    pub clear_maps: Vec<i64>,
}

impl Scenario {
    /// A fresh fleet able to sortie 1-1 (which is unlocked by default).
    pub fn fresh_1_1() -> Self {
        Self {
            fleet: vec![ShipSpec::new(951, 1), ShipSpec::new(951, 1)],
            materials: default_materials(),
            unlock_maps: vec![],
            clear_maps: vec![],
        }
    }

    /// A fleet that opens with anti-submarine fire: Fletcher Mk.II at 99 has
    /// 97 ASW, and 零式水中聴音機 carries it past the destroyer threshold of
    /// 100 while also satisfying the sonar requirement.
    ///
    /// The carrier is there for routing, not for firepower: 4-3's first routing
    /// rule sends any fleet containing a 正規空母 to the one cell off its start
    /// point whose every composition is submarines. Without it the destroyer
    /// count matches a later rule and the fleet is routed to a surface node,
    /// where the phase can never fire.
    pub fn opening_asw() -> Self {
        let mut fleet = vec![ShipSpec::new(AKAGI_MST_ID, 99).with_slots([FIGHTER_MST_ID])];
        fleet.extend(vec![
            ShipSpec::new(FLETCHER_MK2_MST_ID, 99).with_slots([LARGE_SONAR_MST_ID]);
            5
        ]);
        Self {
            fleet,
            materials: default_materials(),
            unlock_maps: vec![],
            // 4-3, because the gate fights exactly one battle and never
            // advances a cell: two of the three cells reachable from its start
            // point field submarines, so opening ASW actually happens. A map
            // with a submarine node deeper in would never be reached.
            clear_maps: vec![11, 12, 13, 14, 21, 22, 23, 24, 31, 32, 33, 34, 41, 42],
        }
    }

    /// A gunnery fleet: two main guns plus a secondary form 連撃 at night, and
    /// the seaplane recon lets the day artillery-spotting cut-ins fire.
    ///
    /// The fighter carrier is what makes the day half reachable: artillery
    /// spotting only resolves under air superiority or supremacy, which a
    /// battleship line cannot win on its own.
    pub fn gunnery_cutin() -> Self {
        let mut fleet = vec![ShipSpec::new(AKAGI_MST_ID, 99).with_slots([
            FIGHTER_MST_ID,
            FIGHTER_MST_ID,
            FIGHTER_MST_ID,
            FIGHTER_MST_ID,
        ])];
        fleet.extend(vec![
            ShipSpec::new(NAGATO_MST_ID, 99).with_slots([
                LARGE_MAIN_GUN_MST_ID,
                LARGE_MAIN_GUN_MST_ID,
                SECONDARY_GUN_MST_ID,
                SEAPLANE_RECON_MST_ID,
            ]);
            5
        ]);
        Self {
            fleet,
            materials: default_materials(),
            unlock_maps: vec![],
            clear_maps: vec![11, 12, 13, 14],
        }
    }

    /// A carrier fleet able to form the 空母カットイン (fighter + dive bomber +
    /// torpedo bomber).
    pub fn carrier_cutin() -> Self {
        Self {
            fleet: vec![
                ShipSpec::new(AKAGI_MST_ID, 99).with_slots([
                    FIGHTER_MST_ID,
                    DIVE_BOMBER_MST_ID,
                    TORPEDO_BOMBER_MST_ID,
                    FIGHTER_MST_ID,
                ]);
                6
            ],
            materials: default_materials(),
            unlock_maps: vec![],
            clear_maps: vec![11, 12, 13, 14],
        }
    }

    /// A leveled six-ship fleet with maps 1-1..1-4 cleared, so 2-1 (the
    /// mid-boss area) becomes sortie-able through the full prerequisite chain.
    pub fn leveled_for_mid_boss() -> Self {
        Self {
            fleet: vec![ShipSpec::new(951, 30); 6],
            materials: default_materials(),
            unlock_maps: vec![],
            clear_maps: vec![11, 12, 13, 14],
        }
    }
}

/// Fletcher Mk.II: the highest-ASW destroyer in the manifest (97 at level 99).
const FLETCHER_MK2_MST_ID: i64 = 629;
/// 零式水中聴音機 (大型ソナー, +11 ASW).
const LARGE_SONAR_MST_ID: i64 = 132;
/// 長門: four slots, so two main guns plus a secondary and a seaplane fit.
const NAGATO_MST_ID: i64 = 80;
/// 35.6cm連装砲.
const LARGE_MAIN_GUN_MST_ID: i64 = 7;
/// 15.5cm三連装副砲.
const SECONDARY_GUN_MST_ID: i64 = 12;
/// 零式水上偵察機, the spotter the day artillery cut-ins need.
const SEAPLANE_RECON_MST_ID: i64 = 25;
/// 赤城.
const AKAGI_MST_ID: i64 = 83;
/// 零式艦戦21型.
const FIGHTER_MST_ID: i64 = 20;
/// 九九式艦爆.
const DIVE_BOMBER_MST_ID: i64 = 23;
/// 九七式艦攻.
const TORPEDO_BOMBER_MST_ID: i64 = 16;

/// A named scenario preset plus its default sortie target.
///
/// Enumerable so the `battle sim` CLI and the sim→validate gate iterate one
/// shared list — adding a preset to [`PRESETS`] is automatically picked up by
/// both, keeping them in sync by construction.
pub struct Preset {
    /// The preset name used on the CLI (`--scenario <name>`).
    pub name: &'static str,
    /// Builds the scenario state.
    pub build: fn() -> Scenario,
    /// Default sortie target map area id.
    pub maparea: i64,
    /// Default sortie target map info no.
    pub mapinfo: i64,
}

impl Preset {
    /// Find a preset by name.
    pub fn lookup(name: &str) -> Option<&'static Preset> {
        PRESETS.iter().find(|preset| preset.name == name)
    }
}

/// The enumerable registry of scenario presets: the single source of truth for
/// both the `battle sim` CLI and the sim→validate gate test.
pub const PRESETS: &[Preset] = &[
    Preset {
        name: "fresh_1_1",
        build: Scenario::fresh_1_1,
        maparea: 1,
        mapinfo: 1,
    },
    Preset {
        name: "leveled_for_mid_boss",
        build: Scenario::leveled_for_mid_boss,
        maparea: 2,
        mapinfo: 1,
    },
    Preset {
        name: "opening_asw",
        build: Scenario::opening_asw,
        maparea: 4,
        mapinfo: 3,
    },
    Preset {
        name: "gunnery_cutin",
        build: Scenario::gunnery_cutin,
        maparea: 2,
        mapinfo: 1,
    },
    Preset {
        name: "carrier_cutin",
        build: Scenario::carrier_cutin,
        maparea: 2,
        mapinfo: 1,
    },
];

fn default_materials() -> Vec<(MaterialCategory, i64)> {
    vec![
        (MaterialCategory::Fuel, 10000),
        (MaterialCategory::Ammo, 10000),
        (MaterialCategory::Steel, 10000),
        (MaterialCategory::Bauxite, 10000),
        (MaterialCategory::Bucket, 100),
    ]
}

/// Apply a scenario to an existing profile, returning the created ship ids (in
/// fleet order).
///
/// Ships, materials, and fleet assignment go through the public gameplay trait
/// methods; map unlock/clear goes through the KTD-5 minimal setter in a single
/// transaction.
pub async fn apply_scenario(
    ctx: &Ctx,
    profile_id: i64,
    scenario: &Scenario,
) -> Result<Vec<i64>, GameplayError> {
    if !scenario.materials.is_empty() {
        ctx.add_material(profile_id, &scenario.materials).await?;
    }

    let mut ship_ids = Vec::with_capacity(scenario.fleet.len());
    for spec in &scenario.fleet {
        let mut ship = ctx.add_ship(profile_id, spec.mst_id).await?;
        apply_ship_spec(&mut ship, spec);
        ctx.update_ship(&ship).await?;

        // Equip after the level is in place: both write paths end in
        // `cal_ship_status`, and only the last one sees the final level and
        // loadout together, so only its derived stats are the real ones.
        let slot_count = (ship.api_slotnum.max(0) as usize).min(MAX_DECLARED_SLOTS);
        for (slot_idx, mst_id) in
            spec.slots.iter().copied().take(slot_count).enumerate().filter(|(_, id)| *id > 0)
        {
            let item = ctx.add_slot_item(profile_id, mst_id, 0, 0).await?;
            ctx.set_slot_item(ship.api_id, slot_idx as i64, item.api_id).await?;
        }

        // A current-HP override has to land after the last recalculation: that
        // pass rewrites `api_maxhp` and `api_nowhp` from the final loadout.
        if let Some(hp) = spec.hp {
            let mut equipped = ctx
                .find_ship(ship.api_id)
                .await?
                .ok_or_else(|| GameplayError::EntryNotFound(format!("ship {}", ship.api_id)))?;
            equipped.api_nowhp = hp;
            ctx.update_ship(&equipped).await?;
        }

        ship_ids.push(ship.api_id);
    }

    if !ship_ids.is_empty() {
        let mut slots = [-1_i64; 6];
        for (slot, id) in slots.iter_mut().zip(ship_ids.iter()) {
            *slot = *id;
        }
        ctx.update_fleet_ships(profile_id, 1, &slots).await?;
    }

    if !scenario.unlock_maps.is_empty() || !scenario.clear_maps.is_empty() {
        let codex = &ctx.codex;
        let tx = ctx.db.begin().await?;
        for &map_id in &scenario.unlock_maps {
            unlock_map_impl(&tx, codex, profile_id, map_id).await?;
        }
        for &map_id in &scenario.clear_maps {
            clear_and_unlock_map_impl(&tx, codex, profile_id, map_id).await?;
        }
        tx.commit().await?;
    }

    Ok(ship_ids)
}

/// Set level (and a consistent exp triple), modernisation, and any HP/fuel/ammo
/// overrides on a freshly-added ship.
///
/// Derived combat stats are not set here and must not be: every write path ends
/// in `cal_ship_status`, which recomputes them from level, modernisation, and
/// equipment. Those three are the only levers a scenario has.
fn apply_ship_spec(ship: &mut KcApiShip, spec: &ShipSpec) {
    if spec.level > 1 {
        let exp_now = level::ship_level_required_exp(spec.level);
        let (_, next_exp) = level::exp_to_ship_level(exp_now);
        ship.api_lv = spec.level;
        ship.api_exp = [exp_now, next_exp, 0];
    }
    if let Some(asw_mod) = spec.asw_mod {
        ship.api_kyouka[6] = asw_mod;
    }
    if let Some(hp) = spec.hp {
        ship.api_nowhp = hp;
    }
    if let Some(fuel) = spec.fuel {
        ship.api_fuel = fuel;
    }
    if let Some(ammo) = spec.ammo {
        ship.api_bull = ammo;
    }
}
