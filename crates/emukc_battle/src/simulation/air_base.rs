//! An air corps' attack on the enemy fleet, flown before the fleets meet.
//!
//! The three stages are those of the carrier air battle in [`super::kouku`]
//! and are kept at its level of detail where that does no harm. The anti-air
//! stage is the source's own, one ship firing on each squadron, because the
//! carrier battle's estimate (the whole fleet's anti-air over 400) empties a
//! squadron against an ordinary six-ship fleet. What is particular to an air
//! corps follows `KC3Kai/kancolle-replay`'s `kcsim.js` (`LBASPhase`,
//! `airstrikeLBAS`) and `kcships.js` (`LandBase.airPower`); the plans
//! `2026-10-08-006` and `2026-10-17-003` list each formula with its line and
//! what was left out.

use emukc_model::{
    codex::Codex,
    kc2::{KcApiSlotItem, KcSlotItemType3, start2::ApiMstSlotitem},
};

use crate::accuracy::{HitOutcome, roll_strike, squadron_proficiency};
use crate::damage::balloon_contact;
use crate::damage::{apply_cap, calculate_defense_power, resolve_damage};
use crate::pt_imp::air_corps_strike_roll;
use crate::random::BattleRng;
use crate::targeting::{choice_with_protect, is_airstrike_attack_type, target_class};
use crate::types::{
    AirCorpsInput, AirState, BattleAirBaseAttack, BattleKouku, BattleKoukuStage1,
    BattleKoukuStage2, BattleKoukuStage3, BattleRuntimeShip, BattleSquadronPlane, DamageCell,
    SquadronFlight, TargetClass,
};

use super::kouku::{
    Contact, apply_plane_losses, attack_plane_from, calculate_fighter_power,
    proficiency_fighter_power, roll_contact, total_plane_count,
};

/// Where an air corps strike stops growing linearly (`lbasDmgCap`).
const DAMAGE_CAP: f64 = 220.0;
/// What a squadron's strength counts for under the square root.
const SLOT_MODIFIER: f64 = 1.8;
/// 陸上攻撃機 and 大型陸上機 strike at this share before the cap…
const LAND_ATTACKER_PRE_CAP: f64 = 0.8;
/// …and 陸上攻撃機 at this multiple after it.
const LAND_ATTACKER_POST_CAP: f64 = 1.8;
/// The B-25, which a PT imp dodges better than other aircraft.
const B25: i64 = 459;
/// A strike on a ship of a combined fleet, either deck, lands harder by this.
const COMBINED_FLEET_POST_CAP: f64 = 1.1;

fn is_land_attacker(type3: i64) -> bool {
    matches!(
        KcSlotItemType3::n(type3),
        Some(KcSlotItemType3::LandBasedAttacker | KcSlotItemType3::LargeLandBasedAircraft)
    )
}

/// Whether a squadron of this equipment attacks ships at sea; the rest only
/// fight for the air, unless they hunt submarines.
fn strikes(type3: i64) -> bool {
    is_land_attacker(type3) || is_airstrike_attack_type(type3)
}

fn is_patrol(type3: i64) -> bool {
    matches!(
        KcSlotItemType3::n(type3),
        Some(KcSlotItemType3::AutoGyro | KcSlotItemType3::AntiSubmarinePatrol)
    )
}

/// What a squadron goes for (`LBASPhase`): ships at sea when it strikes them or
/// is a patrol plane with bombs, submarines when its 対潜 is 7 or more.
/// `None` for a squadron that stays out of the strike.
// ponytail: the source also sends an aircraft with 対潜 7 that is no bomber
// at all (a seaplane scout) after submarines, for no damage; it is left home.
fn strike_targets(mst: &ApiMstSlotitem) -> Option<(bool, bool)> {
    let type3 = mst.api_type[2];
    let surface = strikes(type3) || (is_patrol(type3) && mst.api_baku > 0);
    let submarines = mst.api_tais >= 7;
    ((strikes(type3) || is_patrol(type3)) && (surface || submarines))
        .then_some((surface, submarines))
}

/// What an air corps' earlier waves in this battle leave the next
/// (`LandBase.airStatePrev`, `contactPrev`, cleared by `resetBattle`): the
/// last air state that was not parity, and the last contact made.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct PreviousWave {
    air_state: Option<AirState>,
    contact: Option<f64>,
}

impl PreviousWave {
    /// The contact a wave flying under `air_state` strikes with, and the
    /// aircraft that made it for `api_touch_plane`. Parity tries it under the
    /// last wave's air state; incapability, or parity after it, takes the last
    /// contact made without showing it. `roll` tries contact under a state.
    fn contact(
        &mut self,
        air_state: AirState,
        roll: impl FnOnce(AirState) -> Option<Contact>,
    ) -> (f64, Option<i64>) {
        let state = if air_state == AirState::Parity {
            self.air_state
        } else {
            self.air_state = Some(air_state);
            Some(air_state)
        };
        if let Some(contact) = state.and_then(roll) {
            self.contact = Some(contact.modifier);
            return (contact.modifier, Some(contact.plane));
        }
        let carried = self.contact.filter(|_| state == Some(AirState::Incapability));
        (carried.unwrap_or(1.0), None)
    }
}

/// The ships a squadron may be sent at, before かばう: those afloat of the
/// kinds it goes for; against a combined fleet the main deck 45 times in a
/// hundred, else the escort deck, unless one of them has none; and only the
/// submarines, when it hunts them and there are any.
fn strike_candidates(
    codex: &Codex,
    rng: &mut impl BattleRng,
    enemy: &[BattleRuntimeShip],
    (surface, submarines): (bool, bool),
) -> Vec<usize> {
    let is_submarine = |index: usize| target_class(codex, &enemy[index]).is_submarine();
    let afloat: Vec<usize> = (0..enemy.len())
        .filter(|&index| {
            enemy[index].is_alive()
                && if is_submarine(index) {
                    submarines
                } else {
                    surface
                }
        })
        .collect();
    let (escort, main): (Vec<usize>, Vec<usize>) =
        afloat.into_iter().partition(|&index| enemy[index].is_escort_deck());
    let targets = if escort.is_empty() {
        main
    } else if main.is_empty() || rng.random_f64_range(0.0, 1.0) >= 0.45 {
        escort
    } else {
        main
    };
    let subs: Vec<usize> = targets.iter().copied().filter(|&index| is_submarine(index)).collect();
    if submarines && !subs.is_empty() {
        subs
    } else {
        targets
    }
}

/// 制空値 of an air corps on a sortie: each squadron's anti-air, with a land
/// fighter's interception counted at one and a half, times the root of its
/// strength.
fn fighter_power(codex: &Codex, corps: &AirCorpsInput) -> i64 {
    corps
        .squadrons
        .iter()
        .filter(|squadron| squadron.count > 0)
        .filter_map(|squadron| {
            let mst = codex.find::<ApiMstSlotitem>(&squadron.mst_id).ok()?;
            let mut base = mst.api_tyku as f64;
            if KcSlotItemType3::n(mst.api_type[2]) == Some(KcSlotItemType3::LocalFighter) {
                // 迎撃 is carried in the evasion field.
                base += mst.api_houk as f64 * 1.5;
            }
            let bonus = proficiency_fighter_power(mst.api_type[2], Some(squadron.alv), true);
            Some((base * (squadron.count as f64).sqrt() + bonus).floor() as i64)
        })
        .sum()
}

/// The figure a squadron strikes this target with: 対潜 against a submarine,
/// otherwise bombs or torpedoes, depending on what it flies and whether the
/// target stands on land.
fn strike_stat(mst: &ApiMstSlotitem, target: TargetClass) -> f64 {
    if target.is_submarine() {
        return mst.api_tais.max(0) as f64;
    }
    let on_land = target == TargetClass::Installation;
    let bombs = mst.api_baku.max(0) as f64;
    let torpedoes = mst.api_raig.max(0) as f64;
    if is_land_attacker(mst.api_type[2]) {
        if on_land {
            bombs
        } else {
            torpedoes
        }
    } else if KcSlotItemType3::n(mst.api_type[2])
        == Some(KcSlotItemType3::CarrierBasedTorpedoBomber)
    {
        if on_land {
            (torpedoes / 2.0).floor()
        } else {
            torpedoes
        }
    } else {
        bombs
    }
}

/// What an enemy ship's fixed shot takes of its own and its fleet's anti-air.
pub(super) const ENEMY_FLAT_SHOT: f64 = 0.1875;

/// 加重対空 of a ship: for an enemy the root of her anti-air, for a friendly
/// ship half of what she has without equipment, plus her anti-air equipment
/// weighted by kind (`Ship.weightedAntiAir`, `kcships.js:1619`).
pub(super) fn weighted_anti_air(codex: &Codex, ship: &BattleRuntimeShip) -> i64 {
    let anti_air = ship.ship.api_taiku[0].max(0) as f64;
    let equipped = |item: &KcApiSlotItem| {
        codex.find::<ApiMstSlotitem>(&item.api_slotitem_id).map_or(0, |mst| mst.api_tyku)
    };
    let mut weighted = if ship.is_friendly {
        (anti_air - ship.slot_items.iter().map(equipped).sum::<i64>() as f64).max(0.0) / 2.0
    } else {
        anti_air.sqrt()
    };
    for item in &ship.slot_items {
        let Ok(mst) = codex.find::<ApiMstSlotitem>(&item.api_slotitem_id) else {
            continue;
        };
        weighted += mst.api_tyku as f64
            * match anti_air_kind(mst) {
                AntiAirKind::HighAngle => 2.0,
                AntiAirKind::Gun => 3.0,
                AntiAirKind::AirRadar => 1.5,
                AntiAirKind::Type3Shell | AntiAirKind::LargeGun | AntiAirKind::Other => 0.0,
            };
    }
    weighted.floor() as i64
}

/// 艦隊防空 of a fleet: each ship afloat adds her equipment's anti-air
/// weighted by kind (`Fleet.fleetAntiAir`, `kcships.js:57`). The formation's
/// modifier is left out.
pub(super) fn fleet_anti_air(codex: &Codex, fleet: &[BattleRuntimeShip]) -> i64 {
    fleet
        .iter()
        .filter(|ship| ship.is_alive())
        .map(|ship| {
            ship.slot_items
                .iter()
                .filter_map(|item| codex.find::<ApiMstSlotitem>(&item.api_slotitem_id).ok())
                .map(|mst| {
                    mst.api_tyku as f64
                        * match anti_air_kind(mst) {
                            AntiAirKind::HighAngle => 0.35,
                            AntiAirKind::AirRadar => 0.4,
                            AntiAirKind::Type3Shell => 0.6,
                            AntiAirKind::LargeGun => 0.25,
                            AntiAirKind::Gun | AntiAirKind::Other => 0.2,
                        }
                })
                .sum::<f64>()
                .floor() as i64
        })
        .sum()
}

/// How a piece of equipment counts towards anti-air fire.
enum AntiAirKind {
    /// 高角砲 and 高射装置.
    HighAngle,
    /// 対空機銃.
    Gun,
    /// 対空電探.
    AirRadar,
    /// 三式弾.
    Type3Shell,
    /// 大口径主砲.
    LargeGun,
    Other,
}

fn anti_air_kind(mst: &ApiMstSlotitem) -> AntiAirKind {
    // 高角砲 is told by its icon; the rest by equipment type.
    if mst.api_type[3] == 16 {
        return AntiAirKind::HighAngle;
    }
    match mst.api_type[2] {
        36 => AntiAirKind::HighAngle,
        21 => AntiAirKind::Gun,
        12 | 13 if mst.api_tyku >= 2 => AntiAirKind::AirRadar,
        18 => AntiAirKind::Type3Shell,
        3 => AntiAirKind::LargeGun,
        _ => AntiAirKind::Other,
    }
}

/// Attack power of one squadron before the target's armour:
/// `25 + stat × √(1.8 × strength)`, a land attacker's taken at 0.8 (against a
/// submarine at 0.7 to 1 for 対潜 10 or more, else 0.35 to 0.8), times the
/// land scouts' `scouting`, capped at 220, times `contact`, 陸上攻撃機's
/// then multiplied by 1.8, and against a ship of a combined fleet by 1.1.
fn strike_power(
    rng: &mut impl BattleRng,
    mst: &ApiMstSlotitem,
    count: i64,
    target: TargetClass,
    scouting: f64,
    contact: f64,
    combined: bool,
) -> f64 {
    let stat = strike_stat(mst, target);
    let pre = if target.is_submarine() {
        if stat >= 10.0 {
            rng.random_f64_range(0.7, 1.0)
        } else {
            rng.random_f64_range(0.35, 0.8)
        }
    } else if is_land_attacker(mst.api_type[2]) {
        LAND_ATTACKER_PRE_CAP
    } else {
        1.0
    };
    let power = (25.0 + stat * (SLOT_MODIFIER * count as f64).sqrt()) * pre * scouting;
    let mut power = apply_cap(power, DAMAGE_CAP) as f64 * contact;
    if KcSlotItemType3::n(mst.api_type[2]) == Some(KcSlotItemType3::LandBasedAttacker) {
        power *= LAND_ATTACKER_POST_CAP;
    }
    if combined {
        power *= COMBINED_FLEET_POST_CAP;
    }
    power
}

/// Fly one attack of `corps` against `enemy`, taking the losses out of both. `own_balloons` is
/// how many barrage balloons the player's fleet flies at this cell.
pub(crate) fn simulate_air_base_attack(
    codex: &Codex,
    corps: &mut AirCorpsInput,
    enemy: &mut [BattleRuntimeShip],
    enemy_formation_id: i64,
    previous: &mut PreviousWave,
    own_balloons: i64,
    rng: &mut impl BattleRng,
) -> BattleAirBaseAttack {
    let api_squadron_plane = corps
        .squadrons
        .iter()
        .filter(|squadron| squadron.count > 0)
        .map(|squadron| BattleSquadronPlane {
            api_mst_id: squadron.mst_id,
            api_count: squadron.count,
        })
        .collect();
    let set_out: Vec<i64> = corps.squadrons.iter().map(|squadron| squadron.count).collect();

    let friend_planes: i64 = corps.squadrons.iter().map(|squadron| squadron.count).sum();
    let enemy_planes = total_plane_count(codex, enemy);
    let air_state =
        AirState::from_power(fighter_power(codex, corps), calculate_fighter_power(codex, enemy));

    // Stage 1: the fight for the air, every squadron losing the same share.
    let (f_min, f_max) = air_state.stage1_friendly_loss_ratio();
    let (e_min, e_max) = air_state.stage1_enemy_loss_ratio();
    let f_ratio = rng.random_f64_range(f_min, f_max);
    let e_ratio = rng.random_f64_range(e_min, e_max);
    let mut stage1_f_lost = 0;
    for squadron in &mut corps.squadrons {
        let lost = (squadron.count as f64 * f_ratio).floor() as i64;
        squadron.count -= lost;
        stage1_f_lost += lost;
    }
    let stage1_e_lost = (enemy_planes as f64 * e_ratio).floor() as i64;
    apply_plane_losses(codex, enemy, stage1_e_lost);

    // Contact, and the land scouts that steer the strike (`contactModLB`).
    let planes: Vec<(i64, i64)> =
        corps.squadrons.iter().map(|squadron| (squadron.mst_id, squadron.count)).collect();
    let (contact, touch_plane) =
        previous.contact(air_state, |state| roll_contact(codex, rng, &planes, state));
    // The fleets' balloons bend the contact multiplier the strikes take (`LBASPhase`).
    let struck_contact =
        contact * balloon_contact(own_balloons, enemy.first().map_or(0, |ship| ship.balloons));
    let scouting = corps
        .squadrons
        .iter()
        .filter_map(|squadron| codex.find::<ApiMstSlotitem>(&squadron.mst_id).ok())
        .filter(|mst| KcSlotItemType3::n(mst.api_type[2]) == Some(KcSlotItemType3::LandBasedRecon))
        .map(|mst| {
            if mst.api_houm >= 3 {
                1.15
            } else {
                1.12
            }
        })
        .fold(1.0, f64::max);

    // Stage 2: each squadron that comes to strike is fired on by one enemy
    // ship, then picks its target from what it goes for.
    let fleet_aa = fleet_anti_air(codex, enemy);
    let mut stage2_f_count = 0;
    let mut stage2_f_lost = 0;
    let mut attacks = Vec::new();
    for (index, squadron) in corps.squadrons.iter_mut().enumerate() {
        let Some(kinds) =
            codex.find::<ApiMstSlotitem>(&squadron.mst_id).ok().and_then(strike_targets)
        else {
            continue;
        };
        if squadron.count <= 0 {
            continue;
        }
        let afloat: Vec<usize> = enemy
            .iter()
            .enumerate()
            .filter(|(_, ship)| ship.is_alive())
            .map(|(index, _)| index)
            .collect();
        let Some(pick) = rng.choose_index(afloat.len()) else {
            break;
        };
        let ship_aa = weighted_anti_air(codex, &enemy[afloat[pick]]);
        // Two shots, each landing half the time: one takes a share of the
        // squadron, the other a fixed number.
        let mut lost = 0;
        if rng.roll_range(0, 2) == 0 {
            lost += squadron.count * ship_aa / 200;
        }
        if rng.roll_range(0, 2) == 0 {
            lost += ((ship_aa + fleet_aa) as f64 * ENEMY_FLAT_SHOT).floor() as i64;
        }
        let lost = lost.min(squadron.count);
        stage2_f_count += squadron.count;
        stage2_f_lost += lost;
        squadron.count -= lost;
        if squadron.count <= 0 {
            continue;
        }
        let candidates = strike_candidates(codex, rng, enemy, kinds);
        if candidates.is_empty() {
            continue;
        }
        if let Some((target, shown)) =
            choice_with_protect(codex, rng, enemy, &candidates, 0, false, enemy_formation_id)
        {
            attacks.push((index, target, shown));
        }
    }

    // Stage 3: every target is drawn, then each strike lands; one sent at a
    // ship sunk before it arrives is wasted.
    let mut api_edam = vec![DamageCell::Plain(0); enemy.len()];
    let mut api_erai_flag = vec![0_i64; enemy.len()];
    let mut api_ebak_flag = vec![0_i64; enemy.len()];
    let mut api_ecl_flag = vec![0_i64; enemy.len()];
    let struck = !attacks.is_empty();
    for (index, target, shown) in attacks {
        let squadron = &corps.squadrons[index];
        let Ok(mst) = codex.find::<ApiMstSlotitem>(&squadron.mst_id) else {
            continue;
        };
        let class = target_class(codex, &enemy[target]);
        let land_attacker = is_land_attacker(mst.api_type[2]);
        // A land-based strike lands 90 times in a hundred plus 7 for each
        // point of the aircraft's 命中, and the target dodges it less well
        // than it dodges anything else: 0.86 of its evasion, 0.68 in a
        // combined fleet (`airstrikeLBAS`, `lbasEvaMod*`). A PT imp is hit 0.95 as often, 0.85
        // by the B-25.
        let planes = squadron_proficiency(mst.api_type[2], squadron.alv, squadron.exp);
        let combined = enemy[target].enemy_deck.is_some();
        let pt = class == TargetClass::PtBoat;
        let pt_accuracy = match (pt, mst.api_id) {
            (false, _) => 1.0,
            (true, B25) => 0.85,
            (true, _) => 0.95,
        };
        let outcome = roll_strike(
            codex,
            rng,
            &enemy[target],
            (90.0 + 7.0 * mst.api_houm as f64) * pt_accuracy,
            if combined {
                0.68
            } else {
                0.86
            },
            planes,
        );
        if outcome == HitOutcome::Critical {
            api_ecl_flag[target] = 1;
        }
        // A PT imp rolls its own multiplier on a hit.
        let special = if pt && outcome != HitOutcome::Miss {
            air_corps_strike_roll(rng)
        } else {
            1.0
        };
        let power = outcome.power_with(
            strike_power(rng, mst, squadron.count, class, scouting, struck_contact, combined)
                * special,
            planes.critical_damage,
        );
        let defense = calculate_defense_power(rng, enemy[target].ship.api_soukou[0]);
        let damage = resolve_damage(rng, power, defense, enemy[target].hp(), 1.0);
        api_edam[target].accumulate(enemy[target].take_hit(rng, damage, shown).cell);
        // A land attacker drops bombs on what stands on land and torpedoes on
        // what floats; the client draws the one whose flag is set.
        let torpedo = matches!(class, TargetClass::SurfaceShip | TargetClass::PtBoat)
            && (land_attacker
                || KcSlotItemType3::n(mst.api_type[2])
                    == Some(KcSlotItemType3::CarrierBasedTorpedoBomber));
        if torpedo {
            api_erai_flag[target] = 1;
        } else {
            api_ebak_flag[target] = 1;
        }
    }

    BattleAirBaseAttack {
        api_base_id: corps.base_rid,
        api_stage_flag: [1, i64::from(stage2_f_count > 0), i64::from(struck)],
        api_squadron_plane,
        flights: corps
            .squadrons
            .iter()
            .zip(set_out)
            .map(|(squadron, before)| SquadronFlight {
                squadron_id: squadron.squadron_id,
                before,
                after: squadron.count,
            })
            .collect(),
        kouku: BattleKouku {
            api_plane_from: [Vec::new(), attack_plane_from(codex, enemy)],
            api_stage1: BattleKoukuStage1 {
                api_f_count: friend_planes,
                api_f_lostcount: stage1_f_lost,
                api_e_count: enemy_planes,
                api_e_lostcount: stage1_e_lost,
                api_disp_seiku: air_state.api_disp_seiku(),
                api_touch_plane: [touch_plane.unwrap_or(-1), -1],
            },
            api_stage2: BattleKoukuStage2 {
                api_f_count: stage2_f_count,
                api_f_lostcount: stage2_f_lost,
                api_e_count: 0,
                api_e_lostcount: 0,
                api_air_fire: None,
            },
            api_stage3: BattleKoukuStage3 {
                api_frai: Vec::new(),
                api_erai: Vec::new(),
                api_fbak: Vec::new(),
                api_ebak: Vec::new(),
                api_frai_flag: Vec::new(),
                api_erai_flag,
                api_fbak_flag: Vec::new(),
                api_ebak_flag,
                api_fcl_flag: Vec::new(),
                api_ecl_flag,
                api_fdam: Vec::new(),
                api_edam,
                api_f_sp_list: Vec::new(),
                api_e_sp_list: vec![None; enemy.len()],
            },
            api_stage3_combined: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use crate::types::AirSquadronInput;
    use emukc_model::kc2::types::KcShipType;

    /// 一式陸攻: 雷装 10, 爆装 12, 対空 2.
    const LAND_ATTACKER: i64 = 169;
    /// 雷電: 対空 6, 迎撃 2.
    const LOCAL_FIGHTER: i64 = 175;

    fn corps(squadrons: &[(i64, i64)]) -> AirCorpsInput {
        AirCorpsInput {
            base_rid: 1,
            waves: 1,
            squadrons: (1..)
                .zip(squadrons)
                .map(|(squadron_id, &(mst_id, count))| AirSquadronInput {
                    squadron_id,
                    mst_id,
                    count,
                    alv: 0,
                    exp: None,
                })
                .collect(),
        }
    }

    /// One attack in line formation, the first of its battle.
    fn fly(
        codex: &Codex,
        corps: &mut AirCorpsInput,
        enemy: &mut [BattleRuntimeShip],
        rng: &mut impl BattleRng,
    ) -> BattleAirBaseAttack {
        simulate_air_base_attack(codex, corps, enemy, 1, &mut PreviousWave::default(), 0, rng)
    }

    fn lone_enemy(codex: &Codex, ship_type: KcShipType) -> Vec<BattleRuntimeShip> {
        let mut enemy = sample_ship(codex, first_ship_mst_by_type(codex, ship_type), 1);
        enemy.ship.api_nowhp = 500;
        enemy.ship.api_maxhp = 500;
        enemy.ship.api_soukou[0] = 0;
        enemy.ship.api_taiku[0] = 0;
        enemy.ship.api_onslot = [0; 5];
        vec![BattleRuntimeShip::new(enemy, false, true)]
    }

    #[test]
    fn a_land_attacker_strikes_at_the_sources_figures() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mst = codex.manifest.find_slotitem(LAND_ATTACKER).unwrap();
        assert_eq!((mst.api_raig, mst.api_baku), (10, 12), "the figures below assume these");

        let mut rng = crate::random::SeededRng::new(1);
        let mut power = |count, target, scouting, contact| {
            strike_power(&mut rng, mst, count, target, scouting, contact, false)
        };
        let sea = TargetClass::SurfaceShip;

        // 25 + 10 × √32.4 = 81.9, × 0.8 = 65.5 → 65, × 1.8.
        assert!((power(18, sea, 1.0, 1.0) - 117.0).abs() < 1e-9);
        // On land it bombs: 25 + 12 × √32.4 = 93.3, × 0.8 = 74.6 → 74, × 1.8.
        assert!((power(18, TargetClass::Installation, 1.0, 1.0) - 133.2).abs() < 1e-9);
        // Fewer aircraft strike less: 25 + 10 × √1.8 = 38.4, × 0.8 = 30.7 → 30, × 1.8.
        assert!((power(1, sea, 1.0, 1.0) - 54.0).abs() < 1e-9);
        // A land scout before the cap: 81.9 × 0.8 × 1.15 = 75.4 → 75, × 1.8.
        assert!((power(18, sea, 1.15, 1.0) - 135.0).abs() < 1e-9);
        // Contact after it: 65 × 1.2 × 1.8.
        assert!((power(18, sea, 1.0, 1.2) - 140.4).abs() < 1e-9);
    }

    #[test]
    fn a_strike_on_a_combined_fleet_lands_a_tenth_harder() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mst = codex.manifest.find_slotitem(LAND_ATTACKER).unwrap();
        let mut rng = crate::random::SeededRng::new(1);
        let sea = TargetClass::SurfaceShip;
        // 65 after the cap, × 1.8, then × 1.1.
        let single = strike_power(&mut rng, mst, 18, sea, 1.0, 1.0, false);
        let combined = strike_power(&mut rng, mst, 18, sea, 1.0, 1.0, true);
        assert!((single - 117.0).abs() < 1e-9);
        assert!((combined - 128.7).abs() < 1e-9);
    }

    #[test]
    fn the_cap_bends_a_strike_at_220() {
        // 300 over the cap keeps √80 of the excess.
        assert_eq!(apply_cap(300.0, DAMAGE_CAP), 228);
    }

    #[test]
    fn an_air_corps_counts_a_land_fighters_interception() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let fighter = codex.manifest.find_slotitem(LOCAL_FIGHTER).unwrap();
        let expected_fighter = ((fighter.api_tyku as f64 + fighter.api_houk as f64 * 1.5)
            * 18.0_f64.sqrt())
        .floor() as i64;
        // 一式陸攻's 対空 2 × √18 = 8.
        assert_eq!(
            fighter_power(&codex, &corps(&[(LAND_ATTACKER, 18), (LOCAL_FIGHTER, 18)])),
            8 + expected_fighter
        );
    }

    #[test]
    fn proficiency_raises_an_air_corps_fighter_power() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let fighter = codex.manifest.find_slotitem(LOCAL_FIGHTER).unwrap();
        let mut corps = corps(&[(LAND_ATTACKER, 18), (LOCAL_FIGHTER, 18)]);
        corps.squadrons[0].alv = 7;
        corps.squadrons[1].alv = 7;
        // 一式陸攻 gets the root alone from a base: 2 x sqrt(18) + sqrt(12) = 11.9.
        // 雷電 gets the fighters' 22 on top of it.
        let expected_fighter = ((fighter.api_tyku as f64 + fighter.api_houk as f64 * 1.5)
            * 18.0_f64.sqrt()
            + 12.0_f64.sqrt()
            + 22.0)
            .floor() as i64;
        assert_eq!(fighter_power(&codex, &corps), 11 + expected_fighter);
    }

    #[test]
    fn a_proficient_squadron_strikes_critically() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let criticals = |alv: i64| {
            (0..200)
                .filter(|&seed| {
                    let mut enemy = lone_enemy(&codex, KcShipType::DD);
                    let mut corps = corps(&[(LAND_ATTACKER, 18)]);
                    corps.squadrons[0].alv = alv;
                    let mut rng = crate::random::SeededRng::new(seed);
                    let attack = fly(&codex, &mut corps, &mut enemy, &mut rng);
                    attack.kouku.api_stage3.api_ecl_flag[0] == 1
                })
                .count()
        };
        assert_eq!(criticals(0), 0, "no proficiency, no critical");
        assert!(criticals(7) > 0, "8 in a hundred at full proficiency");
    }

    #[test]
    fn a_strike_damages_one_enemy_and_reports_what_flew() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut enemy = lone_enemy(&codex, KcShipType::DD);
        let mut corps = corps(&[(LAND_ATTACKER, 18), (LOCAL_FIGHTER, 18)]);
        let mut rng = crate::random::SeededRng::new(9);

        let attack = fly(&codex, &mut corps, &mut enemy, &mut rng);

        assert_eq!(attack.api_base_id, 1);
        assert_eq!(attack.api_squadron_plane.len(), 2);
        assert_eq!(attack.api_squadron_plane[0].api_count, 18);
        assert_eq!(attack.api_stage_flag, [1, 1, 1]);
        assert_eq!(attack.kouku.api_stage1.api_f_count, 36);
        assert_eq!(
            attack.kouku.api_stage2.api_f_count - attack.kouku.api_stage2.api_f_lostcount,
            corps.squadrons[0].count,
            "only the bombers are fired on"
        );
        assert_eq!(attack.kouku.api_stage3.api_erai_flag, [1], "torpedoes on a ship afloat");
        assert_eq!(attack.kouku.api_stage3.api_ebak_flag, [0]);
        assert!(enemy[0].hp() < 500, "the destroyer was hit");
        assert_eq!(
            attack
                .flights
                .iter()
                .map(|flight| (flight.squadron_id, flight.after))
                .collect::<Vec<_>>(),
            [(1, corps.squadrons[0].count), (2, corps.squadrons[1].count)],
            "what is left is what the caller writes back"
        );
    }

    /// Six ordinary ships must not empty a squadron: each squadron is fired on
    /// by one of them, not by the sum.
    #[test]
    fn a_fleets_anti_air_fire_takes_a_few_aircraft_not_the_squadron() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut worst = 0;
        for seed in 0..50 {
            let mut enemy: Vec<BattleRuntimeShip> =
                (0..6).flat_map(|_| lone_enemy(&codex, KcShipType::CA)).collect();
            for ship in &mut enemy {
                ship.ship.api_taiku[0] = 60;
                ship.slot_items.clear();
            }
            let mut corps = corps(&[(LAND_ATTACKER, 18)]);
            let mut rng = crate::random::SeededRng::new(seed);
            let attack = fly(&codex, &mut corps, &mut enemy, &mut rng);
            worst = worst.max(attack.kouku.api_stage2.api_f_lostcount);
        }
        // √60 = 7: at most 18 × 7 / 200 = 0 by share and 7 × 0.1875 = 1 fixed.
        assert!(worst <= 1, "anti-air fire took {worst} of a squadron");
    }

    #[test]
    fn fighters_alone_fight_for_the_air_and_strike_nothing() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut enemy = lone_enemy(&codex, KcShipType::DD);
        let mut corps = corps(&[(LOCAL_FIGHTER, 18)]);
        let mut rng = crate::random::SeededRng::new(7);

        let attack = fly(&codex, &mut corps, &mut enemy, &mut rng);

        assert_eq!(attack.api_stage_flag, [1, 0, 0]);
        assert_eq!(enemy[0].hp(), 500);
    }

    /// 東海(九〇一空): 対潜 11, 雷装 0.
    const TOKAI: i64 = 270;

    #[test]
    fn contact_carries_over_from_the_earlier_waves() {
        let made = Contact {
            modifier: 1.17,
            plane: 54,
        };
        let mut previous = PreviousWave::default();
        // Each wave: its air state, what a roll would make, the state the
        // roll is tried under and what the wave strikes with.
        for (state, roll, tried, expected) in [
            (AirState::Parity, Some(made), None, (1.0, None)),
            (AirState::Supremacy, Some(made), Some(AirState::Supremacy), (1.17, Some(54))),
            (AirState::Parity, None, Some(AirState::Supremacy), (1.0, None)),
            (AirState::Incapability, None, Some(AirState::Incapability), (1.17, None)),
            (AirState::Parity, None, Some(AirState::Incapability), (1.17, None)),
            (AirState::Denial, None, Some(AirState::Denial), (1.0, None)),
        ] {
            let mut seen = None;
            let got = previous.contact(state, |under| {
                seen = Some(under);
                roll
            });
            assert_eq!((seen, got), (tried, expected), "{state:?}");
        }
    }

    #[test]
    fn a_combined_fleets_main_deck_draws_45_in_a_hundred() {
        use crate::combined::CombinedFleetRole;

        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let seeds = 400;
        let main: i64 = (0..seeds)
            .map(|seed| {
                let mut enemy: Vec<BattleRuntimeShip> =
                    [CombinedFleetRole::Main, CombinedFleetRole::Escort]
                        .into_iter()
                        .flat_map(|role| {
                            lone_enemy(&codex, KcShipType::DD)
                                .into_iter()
                                .map(move |ship| ship.in_enemy_deck(role))
                        })
                        .collect();
                let mut corps = corps(&[(LAND_ATTACKER, 18)]);
                let mut rng = crate::random::SeededRng::new(seed);
                let flags =
                    fly(&codex, &mut corps, &mut enemy, &mut rng).kouku.api_stage3.api_erai_flag;
                assert_eq!(flags[0] + flags[1], 1, "one strike, at one deck");
                flags[0]
            })
            .sum();
        let share = main as f64 / seeds as f64;
        assert!((0.38..0.52).contains(&share), "the main deck drew {share}");
    }

    #[test]
    fn an_anti_submarine_plane_goes_for_the_submarine() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        for seed in 0..30 {
            let fleet = || {
                let mut enemy = lone_enemy(&codex, KcShipType::DD);
                enemy.extend(lone_enemy(&codex, KcShipType::SS));
                enemy
            };
            let flown = |mst_id| {
                let mut enemy = fleet();
                let mut rng = crate::random::SeededRng::new(seed);
                let stage3 =
                    fly(&codex, &mut corps(&[(mst_id, 18)]), &mut enemy, &mut rng).kouku.api_stage3;
                [stage3.api_erai_flag, stage3.api_ebak_flag]
            };
            assert_eq!(flown(TOKAI), [vec![0, 0], vec![0, 1]], "東海 bombs the submarine");
            assert_eq!(flown(LAND_ATTACKER), [vec![1, 0], vec![0, 0]], "一式陸攻 leaves her be");
        }
    }

    /// With no submarine about, 東海 strikes a ship with the 25 the formula
    /// starts from: her torpedoes count for nothing, but she still attacks.
    #[test]
    fn tokai_strikes_a_ship_at_the_base_power_when_no_submarine_is_about() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mst = codex.manifest.find_slotitem(TOKAI).unwrap();
        assert_eq!(mst.api_raig, 0, "the figure below assumes no torpedoes");
        let mut rng = crate::random::SeededRng::new(1);
        // 25 × 0.8 = 20, × 1.8.
        let power = strike_power(&mut rng, mst, 18, TargetClass::SurfaceShip, 1.0, 1.0, false);
        assert!((power - 36.0).abs() < 1e-9);

        let hit = (0..20).any(|seed| {
            let mut enemy = lone_enemy(&codex, KcShipType::DD);
            let mut rng = crate::random::SeededRng::new(seed);
            let attack = fly(&codex, &mut corps(&[(TOKAI, 18)]), &mut enemy, &mut rng);
            assert_eq!(attack.kouku.api_stage3.api_erai_flag, [1]);
            enemy[0].hp() < 500
        });
        assert!(hit, "東海 never landed a strike");
    }

    #[test]
    fn a_flagship_can_be_covered_from_a_strike() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let covered = (0..60).any(|seed| {
            let mut enemy: Vec<BattleRuntimeShip> =
                (0..3).flat_map(|_| lone_enemy(&codex, KcShipType::DD)).collect();
            let mut rng = crate::random::SeededRng::new(seed);
            let attack = fly(&codex, &mut corps(&[(LAND_ATTACKER, 18)]), &mut enemy, &mut rng);
            attack.kouku.api_stage3.api_edam[1..]
                .iter()
                .any(|cell| matches!(cell, DamageCell::Shielded(_)))
        });
        assert!(covered, "no ship ever covered her flagship");
    }

    /// Every target is drawn before any strike lands: two squadrons may both
    /// go for a ship about to sink, and the one that arrives second is wasted
    /// on her instead of turning on the ship left afloat. Drawn as each strike
    /// lands, the second would find only the flagship.
    #[test]
    fn every_target_is_drawn_before_the_first_strike() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let seeds = 400;
        let wasted = (0..seeds)
            .filter(|&seed| {
                let mut enemy = lone_enemy(&codex, KcShipType::DD);
                let mut sinking = lone_enemy(&codex, KcShipType::DD);
                // Too damaged to cover her flagship, and easy to hit.
                sinking[0].ship.api_nowhp = 1;
                sinking[0].current_hp = 1;
                sinking[0].ship.api_kaihi[0] = 0;
                enemy.extend(sinking);
                let mut rng = crate::random::SeededRng::new(seed);
                let mut corps = corps(&[(LAND_ATTACKER, 18), (LAND_ATTACKER, 18)]);
                let stage3 = fly(&codex, &mut corps, &mut enemy, &mut rng).kouku.api_stage3;
                // Both squadrons came through the anti-air fire, so both
                // strikes went at her.
                let both_flew = corps.squadrons.iter().all(|squadron| squadron.count > 0);
                both_flew && !enemy[1].is_alive() && stage3.api_erai_flag == [0, 1]
            })
            .count();
        // Both pick her a quarter of the time; she sinks unless both miss.
        assert!(
            wasted as u64 * 10 > seeds,
            "both went for the sinking ship {wasted} times in {seeds}"
        );
    }

    /// Pointed at one cell twice, an air corps attacks twice, the second time
    /// with what it had before the first: its losses are made good in between.
    #[test]
    fn the_second_wave_flies_with_what_the_first_set_out_with() {
        use crate::types::{BattleContext, BattleType};

        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut enemy = sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::BB), 1);
        enemy.ship.api_nowhp = 5000;
        enemy.ship.api_maxhp = 5000;
        enemy.ship.api_taiku[0] = 200;
        let friend = sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::DD), 1);

        let mut context =
            BattleContext::head_on(BattleType::Normal, true, vec![friend], vec![enemy]);
        context.air_corps = vec![AirCorpsInput {
            waves: 2,
            ..corps(&[(LAND_ATTACKER, 18)])
        }];
        let mut rng = crate::random::SeededRng::new(7);

        let packet = crate::execute_day(&codex, context, &mut rng).packet;

        assert_eq!(packet.air_base_attack.len(), 2);
        let after_first = packet.air_base_attack[0].flights[0].after;
        assert!(after_first < 18, "anti-air fire took some");
        assert_eq!(packet.air_base_attack[0].flights[0].before, 18);
        assert_eq!(packet.air_base_attack[1].flights[0].before, 18);
        assert_eq!(packet.air_base_attack[1].api_squadron_plane[0].api_count, 18);
    }
}
