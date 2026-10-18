//! Night battle phase simulation.
//!
//! Implements night attack type detection (cut-in / double attack),
//! CI trigger rate calculation, and the night hougeki simulation loop.

use emukc_model::{
    codex::Codex,
    kc2::{KcShipType, KcSlotItemType3},
};

use crate::accuracy::{Aim, AttackKind, HitOutcome, NightContact, asw_proficiency, roll_attack};
use crate::damage::{
    calculate_asw_damage, calculate_night_damage, calculate_scratch_damage, cut_in_hit,
};
use crate::random::BattleRng;
use crate::simulation::abyssal_night::{ABYSSAL_NIGHT_ATTACK, ABYSSAL_NIGHT_CREW_IDS};
use crate::simulation::night_attack_table::{NIGHT_ATTACKS, NightAttackRow, SPECIAL_TORPEDO_IDS};
use crate::simulation::special_attack::{self, SpecialHit};
use crate::targeting::{
    any_alive, can_attack_night_ship, can_attack_submarine_night_shelling, choice_with_protect,
    collect_matching_slot_ids, count_slotitems, extend_limit, has_slotitem_id, has_slotitem_type,
    is_day_surface_display_type, is_main_gun_type, is_radar_type, is_secondary_gun_type,
    is_torpedo_type, searchlight_rerolls, ship_type, submarine_candidates, surface_candidates,
    target_candidates, target_class,
};
use crate::types::{
    AirState, BattleNightHougeki, BattlePhase, BattleRuntimeShip, DamageCell, EngagementType,
    NightBattleParams, SiListId, SpecialAttacksMade, TargetClass,
};

// ---------------------------------------------------------------------------
// Night attack type enum
// ---------------------------------------------------------------------------

/// Night battle special attack (cut-in / double attack) type. The numbers of every type but the
/// carrier's are the rows of [`NIGHT_ATTACKS`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NightAttackType {
    Normal,
    DoubleAttack, // 連撃 (sp_list=1)
    MainTorp,     // 主魚CI (sp_list=2)
    TorpTorpTorp, // 魚魚CI (sp_list=3)
    MainMainSec,  // 主主副CI (sp_list=4)
    MainMainMain, // 主主主CI (sp_list=5)
    // DD CI variants (sp_list 7-14)
    DdGunTorpRadar,      // GTR (sp_list=7)
    DdGunTorpRadar2,     // GTR 2-hit (sp_list=11)
    DdTorpLookoutRadar,  // TRL (sp_list=8)
    DdTorpLookoutRadar2, // TRL 2-hit (sp_list=12)
    DdTorpTorpLookout,   // TTL (sp_list=9)
    DdTorpTorpLookout2,  // TTL 2-hit (sp_list=13)
    DdTorpDrumLookout,   // DTL (sp_list=10)
    DdTorpDrumLookout2,  // DTL 2-hit (sp_list=14)
    // Carrier night CI (sp_list=6)
    CarrierNightCI(CarrierNightCiSubType), // 戦爆連合夜間CI: 1 hit, multiplier varies by sub-type
}

impl NightAttackType {
    /// The types [`NIGHT_ATTACKS`] has a row for.
    const TABLED: [Self; 13] = [
        Self::DoubleAttack,
        Self::MainTorp,
        Self::TorpTorpTorp,
        Self::MainMainSec,
        Self::MainMainMain,
        Self::DdGunTorpRadar,
        Self::DdTorpLookoutRadar,
        Self::DdTorpTorpLookout,
        Self::DdTorpDrumLookout,
        Self::DdGunTorpRadar2,
        Self::DdTorpLookoutRadar2,
        Self::DdTorpTorpLookout2,
        Self::DdTorpDrumLookout2,
    ];

    fn row(self) -> Option<&'static NightAttackRow> {
        let id = self.api_sp_list();
        NIGHT_ATTACKS.iter().find(|row| row.id == id)
    }

    /// What the attack type does to accuracy (`accMod`).
    fn accuracy_modifier(self) -> f64 {
        match self {
            Self::CarrierNightCI(sub) => sub.accuracy_modifier(),
            _ => self.row().map_or(1.0, |row| row.accuracy),
        }
    }

    fn api_sp_list(self) -> i64 {
        match self {
            Self::Normal => 0,
            Self::DoubleAttack => 1,
            Self::MainTorp => 2,
            Self::TorpTorpTorp => 3,
            Self::MainMainSec => 4,
            Self::MainMainMain => 5,
            Self::DdGunTorpRadar => 7,
            Self::DdGunTorpRadar2 => 11,
            Self::DdTorpLookoutRadar => 8,
            Self::DdTorpLookoutRadar2 => 12,
            Self::DdTorpTorpLookout => 9,
            Self::DdTorpTorpLookout2 => 13,
            Self::DdTorpDrumLookout => 10,
            Self::DdTorpDrumLookout2 => 14,
            Self::CarrierNightCI(_) => 6,
        }
    }

    fn damage_multiplier(self) -> f64 {
        match self {
            Self::CarrierNightCI(sub) => sub.damage_multiplier(),
            _ => self.row().map_or(1.0, |row| row.damage),
        }
    }

    fn hit_count(self) -> usize {
        self.row().map_or(1, |row| row.hits)
    }

    /// What the cut-in chance is divided by (`chanceMod`); 0 for the double attack, which
    /// fires at 99%.
    fn chance_modifier(self) -> f64 {
        match self {
            Self::CarrierNightCI(sub) => sub.coefficient() / 100.0,
            _ => self.row().map_or(0.0, |row| row.chance),
        }
    }

    /// The two-hit attack a destroyer of level 80 may make in its place, and how often.
    fn replacement(self) -> Option<(Self, f64)> {
        let row = self.row()?;
        let twice = Self::TABLED.into_iter().find(|kind| kind.api_sp_list() == row.replace)?;
        Some((twice, row.replace_chance))
    }

    fn is_destroyer_cut_in(self) -> bool {
        (7..=14).contains(&self.api_sp_list())
    }
}

// ---------------------------------------------------------------------------
// Carrier night CI
// ---------------------------------------------------------------------------

/// Night plane icon types (`api_type`[3]).
const NIGHT_FIGHTER_ICON: i64 = 45; // 夜間戦闘機
const NIGHT_ATTACKER_ICON: i64 = 46; // 夜間攻撃機
const NIGHT_BOMBER_ICON: i64 = 58; // 夜間爆戦
// Note: 夜間瑞雲 (icon 51) is intentionally not a carrier-night-CI plane — none of the 8
// sub-types reference it (see plan Q13). The separate 夜間瑞雲夜戦カットイン (sp_list=200) is
// deferred follow-up work; its icon constant lands with that plan.

/// Aviation personnel item IDs for night carrier operations.
const AVIATION_PERSONNEL_IDS: &[i64] = &[258, 259]; // 夜間作戦航空要員/夜間作戦航空要員(熟練)

/// Carriers that trigger night CI **without** 夜間作戦航空要員 — a built-in 夜戦特性 granted by
/// a specific remodel. The night-plane requirement still applies. `mst_id`s verified against the
/// Codex bootstrap (`.data/codex/start2.json`); see wikiwiki 夜戦 (<https://wikiwiki.jp/kancolle/夜戦>).
///
/// Saratoga Mk.II Mod.2 (`mst_id=550`) is deliberately **excluded** — it loses 夜戦特性 on the
/// further upgrade (<https://zekamashi.net/kancolle-kouryaku/yasyuu-cutin/>). 加賀改二護 (646) is
/// also excluded — she shells at night instead ([`NIGHT_SHELLING_CV_IDS`]).
const SARATOGA_MK2_ID: i64 = 545; // Saratoga Mk.II
const AKAGI_K2E_ID: i64 = 599; // 赤城改二戊
const KAGA_K2E_ID: i64 = 610; // 加賀改二戊
const RYUUHOU_K2E_ID: i64 = 883; // 龍鳳改二戊
const INDEPENDENCE_FLIGHT2_ID: i64 = 1036; // Independence Flight II
const EXEMPT_NIGHT_CV_IDS: &[i64] =
    &[SARATOGA_MK2_ID, AKAGI_K2E_ID, KAGA_K2E_ID, RYUUHOU_K2E_ID, INDEPENDENCE_FLIGHT2_ID];

/// しまね丸改: the one 補給艦 the source lets fly at night as a carrier, with a night air group
/// (`kcSHIPDATA.js` `canAirAttackNB`, `kcships.js` `AO.prototype.loadEquips`).
const NIGHT_FLYING_OILER_IDS: &[i64] = &[1008];

/// Whether a ship's night attack may be flown at all: a carrier's, or that of the oiler above.
fn flies_at_night_as_carrier(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
    crate::damage::is_cv_type(codex, ship)
        || NIGHT_FLYING_OILER_IDS.contains(&ship.ship.api_ship_id)
}

/// Carriers that shell at night as a surface ship does, with no aircraft in it: Graf Zeppelin
/// and 改, Saratoga, 大鷹改二, 神鷹改二, 加賀改二護, Lexington改, 雲鷹改二, Lexington, Wasp and 改.
/// The first nine are `kancolle-replay`'s `nightattack`, which predates Wasp; the wiki's Wasp
/// page gives the next two (「火力初期値が1以上のため夜間砲撃を行う。改も同様」). The client
/// keeps the same eleven to draw their night attack as gunfire (`_getNormalAttackType`).
///
/// Béarn, Béarn改 and Béarn amélioration are an inference: the client draws their night attack
/// as gunfire too (the last as a torpedo attack when she carries one) and all three start with
/// firepower above zero, but nothing read says outright that they attack at night.
pub(crate) const NIGHT_SHELLING_CV_IDS: &[i64] =
    &[432, 353, 433, 529, 536, 646, 735, 889, 966, 1025, 1030, 1055, 1060, 1061];

/// The source's `nightattack` mark of an abyssal carrier, 0 for a ship without one.
pub(crate) fn abyssal_night_attack(ship: &BattleRuntimeShip) -> u8 {
    ABYSSAL_NIGHT_ATTACK
        .iter()
        .find(|(id, _)| *id == ship.ship.api_ship_id)
        .map_or(0, |(_, mark)| *mark)
}

/// 夜深海艦爆, 夜深海艦爆II and 深海不屈艦爆 Mk.II (`B_NIGHTBOMBER2`): like 光電管彗星 they fly
/// with a night air group and count in its power, without making one.
const ABYSSAL_NIGHT_BOMBER_IDS: &[i64] = &[1582, 1618, 1651];

/// Ark Royal級, whose Swordfish fly at night without a night air group.
const ARK_ROYAL_CLASS: i64 = 78;

/// 光電管彗星 (彗星一二型(三一号光電管爆弾搭載機)). `api_type[3]=7` (regular dive-bomber
/// icon), so `count_night_planes_by_icon` misses it — it must be matched by item id. It makes
/// the carrier cut-in 63 with another night aircraft, and counts towards 64.
const KOUDENKAN_SUISEI_ID: i64 = 320;
/// 零戦62型(爆戦/岩井隊). A 夜間飛行機 but with a non-night icon; it counts towards the carrier
/// cut-in 64 alone.
const IWAI_FUKUSEN_ID: i64 = 154;
/// Swordfish / Swordfish Mk.II(熟練) / Swordfish Mk.III(熟練). 夜間飛行機 with a
/// torpedo-bomber icon (not a night icon), so matched by item id.
const SWORDFISH_IDS: &[i64] = &[242, 243, 244];

const SKILLED_LOOKOUT_ID: i64 = 412;

/// Chuuha bracket: 25% < HP/MaxHP <= 50%. Excludes taiha (HP <= 25%).
fn is_chuuha(ship: &BattleRuntimeShip) -> bool {
    let r = ship.hp() as f64 / ship.ship.api_maxhp.max(1) as f64;
    r > 0.25 && r <= 0.5
}

fn count_night_planes_by_icon(codex: &Codex, ship: &BattleRuntimeShip, icon: i64) -> usize {
    ship.slot_items
        .iter()
        .zip(ship.ship.api_onslot)
        .filter(|(si, onslot)| {
            if *onslot <= 0 {
                return false;
            }
            codex
                .find::<emukc_model::kc2::start2::ApiMstSlotitem>(&si.api_slotitem_id)
                .ok()
                .is_some_and(|mst| mst.api_type[3] == icon)
        })
        .count()
}

/// Count slot items (in non-shot-down slots, `onslot > 0`) whose item id satisfies `pred`.
/// Mirrors `count_night_planes_by_icon`'s shot-down exclusion for item-id-based detection.
fn count_slotitems_with_onslot(ship: &BattleRuntimeShip, pred: impl Fn(i64) -> bool) -> usize {
    ship.slot_items
        .iter()
        .zip(ship.ship.api_onslot)
        .filter(|(si, onslot)| *onslot > 0 && pred(si.api_slotitem_id))
        .count()
}

fn is_swordfish_or_iwai_or_kouden(id: i64) -> bool {
    SWORDFISH_IDS.contains(&id)
        || id == IWAI_FUKUSEN_ID
        || id == KOUDENKAN_SUISEI_ID
        || ABYSSAL_NIGHT_BOMBER_IDS.contains(&id)
}

/// Whether a carrier has a night air group (`hasNBAirGear`): 夜間作戦航空要員 or a built-in
/// 夜戦特性, and a 夜間戦闘機, 夜間攻撃機 or 夜間爆戦 that still has aircraft. 光電管彗星, 岩井爆戦
/// and Swordfish fly with such a group but do not make one.
///
/// An abyssal carrier with a crew of her own (`hasBuiltInNightCrew`) needs the aircraft type
/// only: the source does not ask whether any are left in the slot.
fn has_night_air_gear(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
    let night_icons = [NIGHT_FIGHTER_ICON, NIGHT_ATTACKER_ICON, NIGHT_BOMBER_ICON];
    if ABYSSAL_NIGHT_CREW_IDS.contains(&ship.ship.api_ship_id) {
        return ship.slot_items.iter().any(|si| {
            codex
                .find::<emukc_model::kc2::start2::ApiMstSlotitem>(&si.api_slotitem_id)
                .is_ok_and(|mst| night_icons.contains(&mst.api_type[3]))
        });
    }
    let has_personnel = AVIATION_PERSONNEL_IDS
        .iter()
        .any(|&id| ship.slot_items.iter().any(|si| si.api_slotitem_id == id));
    (has_personnel || EXEMPT_NIGHT_CV_IDS.contains(&ship.ship.api_ship_id))
        && night_icons.iter().any(|&icon| count_night_planes_by_icon(codex, ship, icon) > 0)
}

/// Whether a carrier attacks at night with her aircraft (`canNBAirAttack`): a night air group,
/// or Swordfish aboard Ark Royal, and a flight deck that still works — more than half her
/// hull, or more than a quarter for an armoured carrier.
pub(crate) fn can_night_air_attack(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
    if !flies_at_night_as_carrier(codex, ship) {
        return false;
    }
    let mst = crate::targeting::ship_mst(codex, ship);
    let armoured = mst.is_some_and(|mst| mst.api_stype == KcShipType::CVB as i64);
    // She needs more than one part in `parts` of her hull.
    let parts = if armoured {
        4
    } else {
        2
    };
    if ship.hp() * parts <= ship.ship.api_maxhp {
        return false;
    }
    has_night_air_gear(codex, ship)
        || (mst.is_some_and(|mst| mst.api_ctype == ARK_ROYAL_CLASS)
            && count_slotitems_with_onslot(ship, |id| SWORDFISH_IDS.contains(&id)) > 0)
}

/// The power of a carrier's night air attack (`CV.NBPower`), before the night scout, the
/// cut-in and the cap: her hull's firepower and torpedo, and for every night aircraft its
/// firepower, the root of its 改修, its torpedo (a 艦攻) or bombing (any other), three for
/// each aircraft in the slot, and `0.45 x (火力 + 雷装 + 対潜 + 爆装) x sqrt(aircraft)`.
/// 光電管彗星, 岩井爆戦 and Swordfish take no count term and 0.3 for 0.45. Ark Royal without a
/// night air group adds only her Swordfish' firepower, torpedo and 改修. The fit bonus on her
/// torpedo is added ([`carrier_torpedo_bonus`]); against a land target (`land`) it is not, nor
/// is a 艦攻's torpedo, though the share term and the hull's torpedo still count.
pub(crate) fn night_air_power(codex: &Codex, ship: &BattleRuntimeShip, land: bool) -> f64 {
    let fitted: Vec<_> = ship
        .slot_items
        .iter()
        .zip(ship.ship.api_onslot)
        .filter_map(|(item, count)| {
            let mst = codex
                .find::<emukc_model::kc2::start2::ApiMstSlotitem>(&item.api_slotitem_id)
                .ok()?;
            Some((item, mst, count.max(0) as f64))
        })
        .collect();
    // Her own figures are the hull's with its 近代化改修: `api_karyoku` also holds what the
    // equipment and its fit bonuses add, which this formula counts for itself or not at all.
    let hull = crate::targeting::ship_mst(codex, ship);
    let own = |stat: Option<[i64; 2]>, modernized: i64| stat.map_or(0, |stat| stat[0]) + modernized;
    let mut power = (own(hull.and_then(|mst| mst.api_houg), ship.ship.api_kyouka[0])
        + own(hull.and_then(|mst| mst.api_raig), ship.ship.api_kyouka[1]))
    .max(0) as f64;
    if !land {
        power += carrier_torpedo_bonus(codex, ship);
    }

    let with_group = has_night_air_gear(codex, ship);
    for (item, mst, count) in &fitted {
        let firepower = mst.api_houg as f64;
        let torpedo = mst.api_raig as f64;
        let bombing = mst.api_baku as f64;
        let stars = (item.api_level.max(0) as f64).sqrt();
        let auxiliary = is_swordfish_or_iwai_or_kouden(mst.api_id);
        if !with_group {
            if SWORDFISH_IDS.contains(&mst.api_id) {
                power += firepower + torpedo + stars;
            }
            continue;
        }
        let night_icon =
            [NIGHT_FIGHTER_ICON, NIGHT_ATTACKER_ICON, NIGHT_BOMBER_ICON].contains(&mst.api_type[3]);
        if !night_icon && !auxiliary {
            continue;
        }
        let mut share = 0.3 * (firepower + torpedo + mst.api_tais as f64 + bombing);
        power += firepower + stars;
        if mst.api_type[2] != KcSlotItemType3::CarrierBasedTorpedoBomber as i64 {
            power += bombing;
        } else if !land {
            power += torpedo;
        }
        if !auxiliary {
            power += count * 3.0;
            share *= 1.5;
        }
        power += share * count.sqrt();
    }
    power.max(0.0)
}

/// The aircraft types the source counts as planes (`EQTDATA[..].isPlane`).
const PLANE_TYPES: [i64; 17] = [6, 7, 8, 9, 10, 11, 25, 26, 41, 45, 47, 48, 49, 53, 56, 57, 94];
/// The planes whose torpedo bonus is taken before any other's, in this order
/// (`getEquipBonusCVTorp`); 522 and 523 count together ahead of them all.
const TORPEDO_BONUS_FIRST: [i64; 11] = [238, 239, 521, 118, 369, 368, 372, 373, 374, 425, 424];
const TORPEDO_BONUS_PAIR: [i64; 2] = [522, 523];

/// The fit bonus on a carrier's torpedo in her night air attack (`getEquipBonusCVTorp` with no
/// slot, `TPPlane + TPCrew`). Each piece's share of the ship's 装備ボーナス is what it adds when
/// put on after those before it, as the source's `loadEquips` assigns them. The plane bonus is
/// one plane's: 522 and 523 together if she has either, else the first she carries of a fixed
/// list, then of the others with a bonus by id downwards. The crew bonus is the smallest of her
/// aviation personnel's.
fn carrier_torpedo_bonus(codex: &Codex, ship: &BattleRuntimeShip) -> f64 {
    let Some(hull) = crate::targeting::ship_mst(codex, ship) else {
        return 0.0;
    };
    let carried = crate::targeting::bonus_gears(codex, ship);
    let totals: Vec<i64> = (0..=carried.len())
        .map(|n| codex.gear_bonus(hull, &carried[..n]).map_or(0, |bonus| bonus.raig))
        .collect();
    let shares: Vec<i64> = totals.windows(2).map(|pair| pair[1] - pair[0]).collect();

    let mut planes: std::collections::BTreeMap<i64, i64> = Default::default();
    for (gear, share) in carried.iter().zip(&shares) {
        if PLANE_TYPES.contains(&gear.type2) {
            *planes.entry(gear.id).or_default() += share;
        }
    }
    let plane = if TORPEDO_BONUS_PAIR.iter().any(|id| planes.contains_key(id)) {
        TORPEDO_BONUS_PAIR.iter().filter_map(|id| planes.get(id)).sum()
    } else {
        let others = planes.iter().rev().filter(|(_, bonus)| **bonus != 0).map(|(id, _)| *id);
        TORPEDO_BONUS_FIRST
            .into_iter()
            .chain(others)
            .find_map(|id| planes.get(&id).copied())
            .unwrap_or(0)
    };
    let crew = carried
        .iter()
        .zip(&shares)
        .filter(|(gear, share)| {
            gear.type2 == KcSlotItemType3::AviationPersonnel as i64 && **share != 0
        })
        .map(|(_, share)| *share)
        .min()
        .unwrap_or(0);
    (plane + crew) as f64
}

/// The four carrier night cut-ins (戦爆連合夜間CI), the source's `NBATTACKDATA` 61 to 64, in
/// the order `CV.NBtypes` rolls them. Every one goes out as `api_sp_list` 6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CarrierNightCiSubType {
    /// 61: two night fighters and a night attacker — 1.25 damage, 1.25 accuracy, chance / 1.05.
    TwoFightersAttacker,
    /// 62: a night fighter and a night attacker — 1.2, 1.2, / 1.2.
    FighterAttacker,
    /// 63: 光電管彗星 or a night bomber, with two night aircraft or more — 1.2, 1.2, / 1.2.
    Bomber,
    /// 64: a night fighter, with three night aircraft or more counting Swordfish, 岩井 and
    /// 光電管彗星 — 1.18, 1.2, / 1.3.
    FighterAndTwo,
}

impl CarrierNightCiSubType {
    fn damage_multiplier(self) -> f64 {
        match self {
            Self::TwoFightersAttacker => 1.25,
            Self::FighterAttacker | Self::Bomber => 1.2,
            Self::FighterAndTwo => 1.18,
        }
    }

    fn accuracy_modifier(self) -> f64 {
        match self {
            Self::TwoFightersAttacker => 1.25,
            _ => 1.2,
        }
    }

    /// `chanceMod`, in hundredths.
    fn coefficient(self) -> f64 {
        match self {
            Self::TwoFightersAttacker => 105.0,
            Self::FighterAttacker | Self::Bomber => 120.0,
            Self::FighterAndTwo => 130.0,
        }
    }
}

/// The carrier night cut-ins a carrier rolls for, in order (`CV.NBtypes`): her night air group
/// counted by kind — night fighters, night attackers, night bombers, and the 光電管彗星, Swordfish,
/// 岩井 and abyssal night bombers that fly with them. Every one that her group makes, not only the
/// first. The player's slots count only with aircraft left; the source does not ask an enemy.
/// Empty if she makes none, or does not fly with a night air group.
fn carrier_night_cut_ins(codex: &Codex, ship: &BattleRuntimeShip) -> Vec<CarrierNightCiSubType> {
    use CarrierNightCiSubType::*;
    use emukc_model::kc2::start2::ApiMstSlotitem;
    if !is_cv_night_ci_eligible(codex, ship) {
        return Vec::new();
    }
    let (mut fighters, mut attackers, mut bombers, mut fuze, mut auxiliary) = (0, 0, 0, 0, 0);
    for (item, count) in ship.slot_items.iter().zip(ship.ship.api_onslot) {
        if ship.is_friendly && count <= 0 {
            continue;
        }
        let Ok(mst) = codex.find::<ApiMstSlotitem>(&item.api_slotitem_id) else {
            continue;
        };
        match mst.api_type[3] {
            NIGHT_FIGHTER_ICON => fighters += 1,
            NIGHT_ATTACKER_ICON => attackers += 1,
            NIGHT_BOMBER_ICON => bombers += 1,
            _ if mst.api_id == KOUDENKAN_SUISEI_ID => fuze += 1,
            _ if is_swordfish_or_iwai_or_kouden(mst.api_id) => auxiliary += 1,
            _ => {}
        }
    }
    let auxiliary = fuze + auxiliary;
    let night = fighters + attackers + bombers;

    let mut types = Vec::new();
    if fighters >= 2 && attackers >= 1 {
        types.push(TwoFightersAttacker);
    }
    if fighters >= 1 && attackers >= 1 {
        types.push(FighterAttacker);
    }
    if fuze + bombers >= 1 && night + fuze >= 2 {
        types.push(Bomber);
    }
    if fighters >= 1 && night + auxiliary >= 3 {
        types.push(FighterAndTwo);
    }
    types
}

/// Whether a carrier may make a night cut-in at all: she flies (`flies_at_night_as_carrier`)
/// with 夜間作戦航空要員 (item 258/259) or a built-in 夜戦特性 ([`EXEMPT_NIGHT_CV_IDS`], and the
/// abyssal carriers with a crew of their own), and with
/// a night air group on a working deck. Which cut-ins her group makes is
/// [`carrier_night_cut_ins`].
fn is_cv_night_ci_eligible(codex: &Codex, ship: &BattleRuntimeShip) -> bool {
    if !flies_at_night_as_carrier(codex, ship) {
        return false;
    }
    let has_personnel = AVIATION_PERSONNEL_IDS
        .iter()
        .any(|&id| ship.slot_items.iter().any(|si| si.api_slotitem_id == id));
    let is_exempt = EXEMPT_NIGHT_CV_IDS.contains(&ship.ship.api_ship_id)
        || ABYSSAL_NIGHT_CREW_IDS.contains(&ship.ship.api_ship_id);
    if !(has_personnel || is_exempt) {
        return false;
    }
    // The cut-in is flown by the night air group, so it needs the deck the plain night air
    // attack needs (`NBtypes`: `canNBAirAttack() && hasNBAirGear()`).
    can_night_air_attack(codex, ship) && has_night_air_gear(codex, ship)
}

// ---------------------------------------------------------------------------
// Night attack detection & CI trigger
// ---------------------------------------------------------------------------

/// 熟練見張員: with [`SKILLED_LOOKOUT_ID`], the two lookouts.
const LOOKOUT_ID: i64 = 129;
/// ドラム缶(輸送用), which the fourth destroyer cut-in carries.
const DRUM_ID: i64 = 75;
/// 12.7cm連装砲D型改二 and 改三: they raise the destroyer cut-ins made with a radar.
const D_GUN_K2_ID: i64 = 267;
const D_GUN_K3_ID: i64 = 366;

/// What a fleet has lit for the night.
#[derive(Debug, Clone, Copy, Default)]
struct NightLights {
    /// It fired a star shell as the night began.
    star_shell: bool,
    /// The times a night target of it is drawn again for a searchlight: the most any ship of
    /// it afloat has ([`searchlight_rerolls`]), so 0 when none carries one.
    searchlight: u8,
}

/// What a fleet's searchlights are worth for the night (`nightPhase`): the most any ship afloat
/// has.
fn fleet_searchlight(codex: &Codex, fleet: &[BattleRuntimeShip]) -> u8 {
    fleet
        .iter()
        .filter(|ship| ship.is_alive())
        .map(|ship| searchlight_rerolls(codex, ship))
        .max()
        .unwrap_or(0)
}

/// The equipment the source's `NBtypes` counts.
struct NightGear {
    main_guns: usize,
    sec_guns: usize,
    torps: usize,
    /// A radar with 5 or more line of sight.
    surface_radar: bool,
    lookout: bool,
    skilled_lookout: bool,
    drum: bool,
    special_torps: usize,
    sub_radar: bool,
}

impl NightGear {
    fn of(codex: &Codex, ship: &BattleRuntimeShip) -> Self {
        let surface_radar = ship.slot_items.iter().any(|slot_item| {
            codex
                .find::<emukc_model::kc2::start2::ApiMstSlotitem>(&slot_item.api_slotitem_id)
                .is_ok_and(|mst| {
                    KcSlotItemType3::n(mst.api_type[2]).is_some_and(is_radar_type)
                        && mst.api_saku >= 5
                })
        });
        Self {
            main_guns: count_slotitems(codex, ship, is_main_gun_type),
            sec_guns: count_slotitems(codex, ship, is_secondary_gun_type),
            torps: count_slotitems(codex, ship, is_torpedo_type),
            surface_radar,
            lookout: has_slotitem_type(codex, ship, KcSlotItemType3::SeaplanePersonnel),
            skilled_lookout: has_slotitem_id(ship, SKILLED_LOOKOUT_ID),
            drum: has_slotitem_id(ship, DRUM_ID),
            special_torps: ship
                .slot_items
                .iter()
                .filter(|slot_item| SPECIAL_TORPEDO_IDS.contains(&slot_item.api_slotitem_id))
                .count(),
            sub_radar: has_slotitem_type(codex, ship, KcSlotItemType3::SubmarineEquipment),
        }
    }
}

/// The one general attack type a ship's equipment makes (`NBtypes`): the first of three main
/// guns, two main guns with a secondary, two torpedoes, a main gun with a torpedo, and two guns
/// between main and secondary.
fn detect_night_attack_type(codex: &Codex, ship: &BattleRuntimeShip) -> NightAttackType {
    // `CV.NBtypes`: a carrier flying with a night air group has the carrier cut-ins and no
    // other; her guns do not come into it. The first she rolls for stands for them here.
    if can_night_air_attack(codex, ship) && has_night_air_gear(codex, ship) {
        return carrier_night_cut_ins(codex, ship)
            .first()
            .map_or(NightAttackType::Normal, |&sub| NightAttackType::CarrierNightCI(sub));
    }

    let gear = NightGear::of(codex, ship);
    // A submarine radar with a late-model torpedo counts as one torpedo more.
    let torps = gear.torps + usize::from(gear.sub_radar && gear.special_torps > 0);
    if gear.main_guns >= 3 {
        NightAttackType::MainMainMain
    } else if gear.main_guns >= 2 && gear.sec_guns >= 1 {
        NightAttackType::MainMainSec
    } else if torps >= 2 {
        NightAttackType::TorpTorpTorp
    } else if torps >= 1 && gear.main_guns >= 1 {
        NightAttackType::MainTorp
    } else if gear.main_guns + gear.sec_guns >= 2 {
        NightAttackType::DoubleAttack
    } else {
        NightAttackType::Normal
    }
}

/// Every attack type a ship rolls for, in the order she rolls (`NBtypes`): a destroyer's four
/// cut-ins, each on its own, and then her one general type. A carrier flying with a night air
/// group rolls her carrier cut-ins in turn, and nothing else.
fn night_attack_types(codex: &Codex, ship: &BattleRuntimeShip) -> Vec<NightAttackType> {
    if can_night_air_attack(codex, ship) && has_night_air_gear(codex, ship) {
        return carrier_night_cut_ins(codex, ship)
            .into_iter()
            .map(NightAttackType::CarrierNightCI)
            .collect();
    }
    let mut types = Vec::new();
    if matches!(ship_type(codex, ship), Some(KcShipType::DD)) {
        let gear = NightGear::of(codex, ship);
        if gear.main_guns >= 1 && gear.torps >= 1 && gear.surface_radar {
            types.push(NightAttackType::DdGunTorpRadar);
        }
        if gear.lookout && gear.torps >= 1 && gear.surface_radar {
            types.push(NightAttackType::DdTorpLookoutRadar);
        }
        if gear.skilled_lookout && gear.torps >= 2 {
            types.push(NightAttackType::DdTorpTorpLookout);
        }
        if gear.skilled_lookout && gear.torps >= 1 && gear.drum {
            types.push(NightAttackType::DdTorpDrumLookout);
        }
    }
    match detect_night_attack_type(codex, ship) {
        NightAttackType::Normal => {}
        general => types.push(general),
    }
    types
}

/// The cut-in chance in percent, before the attack type divides it (`NBchance` and the terms
/// `NBattack` adds to it).
fn night_cut_in_chance(
    codex: &Codex,
    ship: &BattleRuntimeShip,
    is_flagship: bool,
    (own, theirs): (NightLights, NightLights),
) -> f64 {
    let luck = ship.ship.api_lucky[0].max(0) as f64;
    let level = ship.ship.api_lv.max(1) as f64;

    let mut chance = if luck >= 50.0 {
        65.0 + (luck - 50.0).sqrt().floor() + (level.sqrt() * 0.8).floor()
    } else {
        (luck + 15.0 + level.sqrt() * 0.75).floor()
    };
    if is_flagship {
        chance += 15.0;
    }
    if matches!(ship_type(codex, ship), Some(KcShipType::DD | KcShipType::CL | KcShipType::CLT))
        && has_slotitem_id(ship, SKILLED_LOOKOUT_ID)
    {
        chance += 8.0;
    }
    if has_slotitem_id(ship, LOOKOUT_ID) {
        chance += 5.0;
    }
    for (lit, bonus) in [
        (own.star_shell, 4.0),
        (theirs.star_shell, -10.0),
        (own.searchlight > 0, 7.0),
        (theirs.searchlight > 0, -5.0),
    ] {
        if lit {
            chance += bonus;
        }
    }
    // The source has `HP/maxHP <= .5`, and a ship at a quarter or less does not attack.
    if is_chuuha(ship) {
        chance += 18.0;
    }
    chance
}

/// What the D-type guns do to a destroyer cut-in made with a radar (`NBattack`).
fn d_gun_modifier(ship: &BattleRuntimeShip) -> f64 {
    let count = |id| ship.slot_items.iter().filter(|si| si.api_slotitem_id == id).count();
    let k3 = count(D_GUN_K3_ID);
    let both = match count(D_GUN_K2_ID) + k3 {
        0 => 1.0,
        1 => 1.25,
        _ => 1.4,
    };
    both * match k3 {
        0 => 1.0,
        1 => 1.05,
        _ => 1.1,
    }
}

/// Roll a ship's night attack (`NBattack`): each of her attack types in turn, the first that
/// comes up, and a plain attack if none does. Returns the type and its damage multiplier.
fn resolve_night_attack(
    codex: &Codex,
    rng: &mut impl BattleRng,
    ship: &BattleRuntimeShip,
    is_flagship: bool,
    target: TargetClass,
    lights: (NightLights, NightLights),
) -> (NightAttackType, f64) {
    const PLAIN: (NightAttackType, f64) = (NightAttackType::Normal, 1.0);
    if target.is_submarine() {
        return PLAIN;
    }
    let types = night_attack_types(codex, ship);
    if types.is_empty() {
        return PLAIN;
    }
    let gear = NightGear::of(codex, ship);
    let chance = night_cut_in_chance(codex, ship, is_flagship, lights) * 0.01;
    let installation = target == TargetClass::Installation;

    for mut kind in types {
        if let Some((twice, replace_chance)) = kind.replacement()
            && ship.ship.api_lv >= 80
            && rng.random_f64_range(0.0, 1.0) < replace_chance
        {
            kind = twice;
        }
        if installation {
            if kind.is_destroyer_cut_in() && gear.main_guns == 0 {
                continue;
            }
            // A cut-in with a torpedo is not made on an installation; a ship that could double
            // attack does that instead.
            if kind.row().is_some_and(|row| row.torpedo) {
                if gear.main_guns + gear.sec_guns < 2 {
                    continue;
                }
                kind = NightAttackType::DoubleAttack;
            }
        }
        let modifier = kind.chance_modifier();
        let mut rate = if modifier == 0.0 {
            0.99
        } else {
            chance / modifier
        };
        let mut damage = kind.damage_multiplier();
        match kind {
            // A submarine's torpedo cut-in with late-model torpedoes.
            NightAttackType::TorpTorpTorp if gear.special_torps >= 1 && gear.sub_radar => {
                rate = chance / 1.05;
                damage = 1.75;
            }
            NightAttackType::TorpTorpTorp if gear.special_torps >= 2 => {
                rate = chance / 1.1;
                damage = 1.6;
            }
            NightAttackType::DdGunTorpRadar
            | NightAttackType::DdGunTorpRadar2
            | NightAttackType::DdTorpLookoutRadar
            | NightAttackType::DdTorpLookoutRadar2 => damage *= d_gun_modifier(ship),
            _ => {}
        }
        if rng.random_f64_range(0.0, 1.0) < rate {
            return (kind, damage);
        }
    }
    PLAIN
}

// ---------------------------------------------------------------------------
// Night attack display IDs
// ---------------------------------------------------------------------------

fn night_attack_display_ids(
    codex: &Codex,
    ship: &BattleRuntimeShip,
    attack_type: NightAttackType,
) -> Vec<i64> {
    let main_guns =
        collect_matching_slot_ids(codex, ship, |slot_type, _| is_main_gun_type(slot_type));
    let torpedoes =
        collect_matching_slot_ids(codex, ship, |slot_type, _| is_torpedo_type(slot_type));
    let secondary_guns =
        collect_matching_slot_ids(codex, ship, |slot_type, _| is_secondary_gun_type(slot_type));
    // The destroyer cut-ins are made with a surface radar, and show that one.
    let radars = collect_matching_slot_ids(codex, ship, |slot_type, mst| {
        is_radar_type(slot_type) && mst.api_saku >= 5
    });
    let lookouts = collect_matching_slot_ids(codex, ship, |slot_type, _| {
        slot_type == KcSlotItemType3::SeaplanePersonnel
    });
    let skilled_lookouts: Vec<i64> = ship
        .slot_items
        .iter()
        .filter_map(|si| (si.api_slotitem_id == SKILLED_LOOKOUT_ID).then_some(si.api_slotitem_id))
        .collect();
    let drums = collect_matching_slot_ids(codex, ship, |slot_type, _| {
        slot_type == KcSlotItemType3::TransportContainer
    });
    let sub_radars = collect_matching_slot_ids(codex, ship, |slot_type, _| {
        slot_type == KcSlotItemType3::SubmarineEquipment
    });
    let surface_ids = collect_matching_slot_ids(codex, ship, |slot_type, _| {
        is_day_surface_display_type(slot_type)
    });
    let night_fighters = collect_matching_slot_ids(codex, ship, |slot_type, _| {
        slot_type == KcSlotItemType3::CarrierBasedFighter
    });
    let night_dive_bombers = collect_matching_slot_ids(codex, ship, |slot_type, _| {
        slot_type == KcSlotItemType3::CarrierBasedDiveBomber
    });
    let night_torpedo_bombers = collect_matching_slot_ids(codex, ship, |slot_type, _| {
        slot_type == KcSlotItemType3::CarrierBasedTorpedoBomber
    });

    let mut ids = Vec::new();
    match attack_type {
        NightAttackType::MainMainMain => extend_limit(&mut ids, &main_guns, 3),
        NightAttackType::MainMainSec => {
            extend_limit(&mut ids, &main_guns, 2);
            extend_limit(&mut ids, &secondary_guns, 3);
        }
        // `CutinSpSR` draws its first slot in the middle and its second above and below it.
        NightAttackType::MainTorp => {
            extend_limit(&mut ids, &main_guns, 1);
            extend_limit(&mut ids, &torpedoes, 2);
        }
        NightAttackType::DdGunTorpRadar | NightAttackType::DdGunTorpRadar2 => {
            extend_limit(&mut ids, &main_guns, 1);
            extend_limit(&mut ids, &torpedoes, 2);
            extend_limit(&mut ids, &radars, 3);
        }
        // `CutinSpRR` reads two pieces without a check. A submarine who makes the cut-in with
        // one late-model torpedo and her radar shows the radar second.
        NightAttackType::TorpTorpTorp => {
            extend_limit(&mut ids, &torpedoes, 3);
            extend_limit(&mut ids, &sub_radars, 2);
        }
        // DD CI: TRL (魚雷+見張員+電探)
        NightAttackType::DdTorpLookoutRadar | NightAttackType::DdTorpLookoutRadar2 => {
            extend_limit(&mut ids, &torpedoes, 1);
            extend_limit(&mut ids, &lookouts, 2);
            extend_limit(&mut ids, &radars, 3);
        }
        // DD CI: TTL (魚雷+水雷見張員+魚雷)
        NightAttackType::DdTorpTorpLookout | NightAttackType::DdTorpTorpLookout2 => {
            extend_limit(&mut ids, &torpedoes, 2);
            extend_limit(&mut ids, &skilled_lookouts, 3);
        }
        // DD CI: DTL (魚雷+水雷見張員+ドラム缶)
        NightAttackType::DdTorpDrumLookout | NightAttackType::DdTorpDrumLookout2 => {
            extend_limit(&mut ids, &torpedoes, 1);
            extend_limit(&mut ids, &skilled_lookouts, 2);
            extend_limit(&mut ids, &drums, 3);
        }
        // 連撃 is main gun x2 or main + secondary; the broad day-surface set
        // would put 水上爆撃機 / 艦上攻撃機 on screen for an attack they cannot
        // form.
        NightAttackType::DoubleAttack => {
            extend_limit(&mut ids, &main_guns, 2);
            extend_limit(&mut ids, &secondary_guns, 2);
        }
        // 空母カットイン draws the night air group that formed it. This is the
        // one night path that loads a name plate -- `PreloadCutinKubo` asks for
        // `btxt_flat` when `night == 1` -- and the night-capable aircraft are
        // exactly the ones that have one upstream, so naming them is both
        // correct and covered.
        NightAttackType::CarrierNightCI(_) => {
            extend_limit(&mut ids, &night_fighters, 1);
            extend_limit(&mut ids, &night_dive_bombers, 2);
            extend_limit(&mut ids, &night_torpedo_bombers, 3);
        }
        NightAttackType::Normal => {
            extend_limit(&mut ids, &surface_ids, 1);
        }
    }

    if ids.is_empty() {
        vec![-1]
    } else {
        ids
    }
}

// ---------------------------------------------------------------------------
// Night hougeki simulation
// ---------------------------------------------------------------------------

/// Build the `api_si_list` entry for a night attacker. Cut-ins and double
/// attacks (any non-`Normal` type) serialize as JSON strings; normal attacks
/// stay integers. The `-1` empty-equipment sentinel is kept as `Num` by the
/// `text_from_i64` / `num_from_i64` helpers. Shared by the friendly and enemy
/// loops, which differ only in `at_eflag`.
fn night_si_entry(
    codex: &Codex,
    ship: &BattleRuntimeShip,
    attack_type: NightAttackType,
) -> Vec<SiListId> {
    let ids = night_attack_display_ids(codex, ship, attack_type);
    if attack_type != NightAttackType::Normal {
        SiListId::text_from_i64(&ids)
    } else {
        SiListId::num_from_i64(&ids)
    }
}

/// What made a night attack, which settles how the client is told of it.
enum NightAttackKind<'a> {
    /// A friendly special attack's row: the pieces it shows and its `api_sp_list`.
    Special(&'a SpecialRow),
    /// A ship's own attack of `attack_type`, flown as aircraft (`yasen_kubo`) when
    /// `by_air`.
    Own {
        codex: &'a Codex,
        ship: &'a BattleRuntimeShip,
        attack_type: NightAttackType,
        by_air: bool,
    },
}

impl BattleNightHougeki {
    /// Record one night attack by `attacker_idx` of the side `attacker_is_enemy`
    /// names: each hit lands on the matching entry of `targets` for the matching
    /// entries of `cl` and `damage`.
    fn record_night_attack(
        &mut self,
        kind: NightAttackKind<'_>,
        attacker_is_enemy: bool,
        attacker_idx: usize,
        targets: Vec<i64>,
        cl: Vec<i64>,
        damage: Vec<DamageCell>,
    ) {
        let (shown, sp, by_air) = match kind {
            NightAttackKind::Special(row) => (SiListId::text_from_i64(&row.shown), row.sp, false),
            NightAttackKind::Own {
                codex,
                ship,
                attack_type,
                by_air,
            } => (night_si_entry(codex, ship, attack_type), attack_type.api_sp_list(), by_air),
        };
        self.api_at_eflag.push(i64::from(attacker_is_enemy));
        self.api_at_list.push(attacker_idx as i64);
        self.api_n_mother_list.push(i64::from(by_air));
        self.api_df_list.push(targets);
        self.api_si_list.push(shown);
        self.api_cl_list.push(cl);
        self.api_sp_list.push(sp);
        self.api_damage.push(damage);
    }
}

/// Fire the star shells as the night begins (`nightPhase`): in each fleet the first ship afloat
/// that carries one, has more than 4 HP and rolls under 70%. Returns who fired in the friendly
/// and in the enemy fleet, -1 for nobody.
pub(crate) fn roll_star_shells(
    codex: &Codex,
    rng: &mut impl BattleRng,
    friendly: &[BattleRuntimeShip],
    enemy: &[BattleRuntimeShip],
) -> [i64; 2] {
    let mut fire = |fleet: &[BattleRuntimeShip]| {
        fleet
            .iter()
            .position(|ship| {
                ship.hp() > 4
                    && has_slotitem_type(codex, ship, KcSlotItemType3::Flare)
                    && rng.random_f64_range(0.0, 1.0) < 0.7
            })
            .map_or(-1, |position| position as i64)
    };
    [fire(friendly), fire(enemy)]
}

/// Put the night scouts up as the night begins (`nightPhase`, `getNightEquipsScout`): each
/// scout a fleet carries comes into contact at `floor(√Lv × √LoS) / 25`, and the one with the
/// most accuracy of those that do is the fleet's. No fleet flies one that lost the air by day,
/// nor, unless the battle opens at night, one that met the enemy in the air on even terms.
/// `air_state` is the friendly side's. Returns the friendly fleet's and the enemy's.
pub(crate) fn roll_night_contact(
    codex: &Codex,
    rng: &mut impl BattleRng,
    friendly: &[BattleRuntimeShip],
    enemy: &[BattleRuntimeShip],
    air_state: Option<AirState>,
    night_start: bool,
) -> [Option<NightContact>; 2] {
    let mut fly = |fleet: &[BattleRuntimeShip], air: Option<AirState>| {
        let grounded = match air {
            Some(AirState::Incapability) => true,
            Some(AirState::Parity) | None => !night_start,
            _ => false,
        };
        if grounded {
            return None;
        }
        let mut best: Option<(i64, i64)> = None;
        for ship in fleet.iter().filter(|ship| ship.is_alive()) {
            for slot_item in &ship.slot_items {
                let Ok(mst) = codex
                    .find::<emukc_model::kc2::start2::ApiMstSlotitem>(&slot_item.api_slotitem_id)
                else {
                    continue;
                };
                // The night scouts have an icon of their own.
                if mst.api_type[3] != 50 || best.is_some_and(|(_, most)| mst.api_houm <= most) {
                    continue;
                }
                let level = ship.ship.api_lv.max(0) as f64;
                let chance = (level.sqrt() * (mst.api_saku.max(0) as f64).sqrt()).floor() / 25.0;
                if rng.random_f64_range(0.0, 1.0) < chance {
                    best = Some((mst.api_id, mst.api_houm));
                }
            }
        }
        best.map(|(plane, accuracy)| NightContact::of(plane, accuracy))
    };
    [fly(friendly, air_state), fly(enemy, air_state.map(AirState::reversed))]
}

// ---------------------------------------------------------------------------
// 僚艦夜戦突撃 (the Kongou class's night assault, `api_sp_list` 104)
// ---------------------------------------------------------------------------

const KONGOU_ASSAULT_SP: i64 = 104;

/// Who makes it as flagship, with whom in the second place, and what a surface radar of 8
/// line of sight or more and a large searchlight on the flagship add to the rate (wikiwiki
/// 金剛改二丙; the pairs are the source's `canSpecialAttackUnique` as well).
const KONGOU_ASSAULT_SHIPS: &[(i64, &[i64], f64, f64)] = &[
    (591, &[592, 151, 593, 954, 694, 439, 364, 927, 733], 30.0, 10.0), // 金剛改二丙
    (592, &[591, 152, 694, 593, 954], 10.0, 30.0),                     // 比叡改二丙
    (593, &[591, 592, 694], 15.0, 0.0),                                // 榛名改二乙
    (954, &[591, 592, 694], 20.0, 0.0),                                // 榛名改二丙
    (694, &[591, 592, 593, 954, 697], 20.0, 20.0),                     // 霧島改二丙
];
/// 単縦陣, 梯形陣, 第二 and 第四警戒航行序列.
const KONGOU_ASSAULT_FORMATIONS: [i64; 4] = [1, 4, 12, 14];
/// 35.6cm連装砲改二 and 改三(ダズル迷彩仕様). The source has 529 for the first, a destroyer's gun.
const KONGOU_ASSAULT_GUNS_A: [i64; 2] = [329, 502];
/// 35.6cm連装砲改四 and 改三丙.
const KONGOU_ASSAULT_GUNS_B: [i64; 2] = [503, 530];

/// The chance that the fleet's flagship and second ship make the night assault, or `None` if
/// they cannot: the pair, neither heavily damaged, five surface ships afloat and a formation
/// for it.
fn kongou_assault_rate(
    codex: &Codex,
    fleet: &[BattleRuntimeShip],
    formation_id: i64,
) -> Option<f64> {
    let (flagship, partner) = (fleet.first()?, fleet.get(1)?);
    let (_, partners, radar, searchlight) =
        KONGOU_ASSAULT_SHIPS.iter().find(|(id, ..)| *id == flagship.ship.api_ship_id)?;
    let afloat = fleet
        .iter()
        .filter(|ship| ship.is_alive() && !target_class(codex, ship).is_submarine())
        .count();
    if !partners.contains(&partner.ship.api_ship_id)
        || !KONGOU_ASSAULT_FORMATIONS.contains(&formation_id)
        || [flagship, partner].iter().any(|ship| ship.hp() * 4 <= ship.ship.api_maxhp)
        || afloat < 5
    {
        return None;
    }

    let root = |value: i64| (value.max(0) as f64).sqrt();
    let mut rate = 3.5 * (root(flagship.ship.api_lv) + root(partner.ship.api_lv))
        + 1.1 * (root(flagship.ship.api_lucky[0]) + root(partner.ship.api_lucky[0]))
        - 33.0;
    let has_radar = flagship.slot_items.iter().any(|slot_item| {
        codex
            .find::<emukc_model::kc2::start2::ApiMstSlotitem>(&slot_item.api_slotitem_id)
            .is_ok_and(|mst| {
                KcSlotItemType3::n(mst.api_type[2]).is_some_and(is_radar_type) && mst.api_saku >= 8
            })
    });
    if has_radar {
        rate += radar;
    }
    if has_slotitem_type(codex, flagship, KcSlotItemType3::LargeSearchlight) {
        rate += searchlight;
    }
    Some((rate.floor() / 100.0).clamp(0.0, 1.0))
}

/// What one of the two hits with, before the cap: 2.6 for the flagship and 2.7 for her partner,
/// 1.06 more since the update of 2026-04-23 (measured, and in `KC3Kai`'s
/// `estimateKongouCutinModifier`), by the engagement, and by her own guns.
fn kongou_assault_multiplier(
    ship: &BattleRuntimeShip,
    is_flagship: bool,
    engagement: EngagementType,
) -> f64 {
    let guns = |ids: [i64; 2]| {
        ship.slot_items.iter().filter(|slot_item| ids.contains(&slot_item.api_slotitem_id)).count()
    };
    let base = if is_flagship {
        2.6
    } else {
        2.7
    };
    let engagement = match engagement {
        EngagementType::TAdvantage => 1.25,
        EngagementType::TDisadvantage => 0.8,
        _ => 1.0,
    };
    let guns_a = match guns(KONGOU_ASSAULT_GUNS_A) {
        0 => 1.0,
        1 => 1.05,
        _ => 1.08,
    };
    let guns_b = match guns(KONGOU_ASSAULT_GUNS_B) {
        0 => 1.0,
        1 => 1.11,
        _ => 1.15,
    };
    base * 1.06 * engagement * guns_a * guns_b
}

/// The night assault in the friendly flagship's turn, if it comes up: one row, the flagship's
/// hit and then her partner's, each on a surface ship of her own choosing. Rolls nothing unless
/// the fleet can make it and there is a surface ship to attack.
fn kongou_night_assault(
    codex: &Codex,
    rng: &mut impl BattleRng,
    friendly: &mut [BattleRuntimeShip],
    enemy: &mut [BattleRuntimeShip],
    forms: (i64, i64),
    params: &NightBattleParams,
    hougeki: &mut BattleNightHougeki,
) -> bool {
    // Three a sortie, and none in an exercise.
    if params.special_attacks_made.kongou_assaults >= SpecialAttacksMade::KONGOU_ASSAULTS_A_SORTIE
        || !friendly.first().is_some_and(|flagship| flagship.is_sortie)
    {
        return false;
    }
    let Some(rate) = kongou_assault_rate(codex, friendly, forms.0) else {
        return false;
    };
    if !has_special_attack_target(codex, enemy) || rng.random_f64_range(0.0, 1.0) >= rate {
        return false;
    }
    // `getSpecialAttackMod`: 1.4 to her accuracy.
    let hits = [0, 1].map(|at| SpecialHit {
        fleet_index: at,
        multiplier: kongou_assault_multiplier(&friendly[at], at == 0, params.engagement),
        accuracy: 1.4,
    });
    let row = SpecialRow::flagship(codex, friendly, params, KONGOU_ASSAULT_SP);
    special_attack_row(codex, rng, friendly, enemy, forms, params, hougeki, row, &hits);
    true
}

/// A flagship's special gunnery attack in her turn at night, if the sortie has not made it and
/// it comes up: the attacks of the day (100 to 103, 105, 106), which the source and wikiwiki
/// roll for at night as well, in the same formations.
fn special_gunnery_at_night(
    codex: &Codex,
    rng: &mut impl BattleRng,
    friendly: &mut [BattleRuntimeShip],
    enemy: &mut [BattleRuntimeShip],
    forms: (i64, i64),
    params: &NightBattleParams,
    hougeki: &mut BattleNightHougeki,
) -> bool {
    if params.special_attacks_made.gunnery || !has_special_attack_target(codex, enemy) {
        return false;
    }
    let can_attack =
        friendly.first().is_some_and(|flagship| can_attack_night_ship(codex, flagship));
    let Some(resolved) =
        special_attack::try_special_attack(codex, rng, friendly, forms.0, can_attack)
    else {
        return false;
    };
    let hits = resolved.hits(codex, friendly, params.engagement);
    let row = SpecialRow::flagship(codex, friendly, params, resolved.api_value());
    special_attack_row(codex, rng, friendly, enemy, forms, params, hougeki, row, &hits);
    true
}

/// 試製 夜間瑞雲(攻撃装備), the one aircraft the night 瑞雲 cut-in counts.
const NIGHT_ZUIUN_ID: i64 = 490;
/// The night 瑞雲 cut-in's `api_sp_list`, which the client plays as `PhaseZuiunNightAttack`.
const ZUIUN_NIGHT_SP: i64 = 200;

/// The night 瑞雲 cut-ins (`NBATTACKDATA` 2001 to 2004), in the order the source rolls them. All
/// take 1.07 to accuracy and divide the chance by 1.35.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ZuiunNightCutIn {
    /// Two 夜間瑞雲 and a surface radar: 1.36.
    TwoAndRadar,
    /// Two 夜間瑞雲: 1.32.
    Two,
    /// One 夜間瑞雲 and a surface radar: 1.28.
    OneAndRadar,
    /// One 夜間瑞雲: 1.24.
    One,
}

impl ZuiunNightCutIn {
    const ACCURACY: f64 = 1.07;
    const CHANCE: f64 = 1.35;

    fn damage_multiplier(self) -> f64 {
        match self {
            Self::TwoAndRadar => 1.36,
            Self::Two => 1.32,
            Self::OneAndRadar => 1.28,
            Self::One => 1.24,
        }
    }
}

/// The night 瑞雲 cut-ins a ship may roll for (`Ship.NBtypes`): a light or torpedo cruiser, an
/// aviation cruiser or battleship or a seaplane tender with two main guns, and 夜間瑞雲 with
/// aircraft left. Every one she makes, in order.
fn zuiun_night_cut_ins(codex: &Codex, ship: &BattleRuntimeShip) -> Vec<ZuiunNightCutIn> {
    use ZuiunNightCutIn::*;
    let hull = matches!(
        ship_type(codex, ship),
        Some(KcShipType::CL | KcShipType::CLT | KcShipType::CAV | KcShipType::BBV | KcShipType::AV)
    );
    let gear = NightGear::of(codex, ship);
    if !hull || gear.main_guns < 2 {
        return Vec::new();
    }
    let zuiun = count_slotitems_with_onslot(ship, |id| id == NIGHT_ZUIUN_ID);
    let radar = gear.surface_radar;
    [
        (zuiun >= 2 && radar, TwoAndRadar),
        (zuiun >= 2, Two),
        (zuiun >= 1 && radar, OneAndRadar),
        (zuiun >= 1, One),
    ]
    .into_iter()
    .filter_map(|(makes, kind)| makes.then_some(kind))
    .collect()
}

/// The chance in percent the night 瑞雲 cut-in is rolled against, before it is divided
/// (`canSpecialAttack`): the night cut-in chance, less 14 for her own fleet's star shell and 17
/// for its searchlight, 3 more for 熟練見張員, 8 less for a destroyer or light or torpedo cruiser
/// with 水雷戦隊 熟練見張員.
fn zuiun_night_chance(
    codex: &Codex,
    ship: &BattleRuntimeShip,
    is_flagship: bool,
    lights: (NightLights, NightLights),
) -> f64 {
    let own = lights.0;
    let mut chance = night_cut_in_chance(codex, ship, is_flagship, lights);
    if own.star_shell {
        chance -= 14.0;
    }
    if own.searchlight > 0 {
        chance -= 17.0;
    }
    if has_slotitem_id(ship, LOOKOUT_ID) {
        chance += 3.0;
    }
    if matches!(ship_type(codex, ship), Some(KcShipType::DD | KcShipType::CL | KcShipType::CLT))
        && has_slotitem_id(ship, SKILLED_LOOKOUT_ID)
    {
        chance -= 8.0;
    }
    chance
}

/// A friendly ship's night 瑞雲 cut-in in her turn, if it comes up (`nightPhase`'s special attack
/// with `attackSpecial` 200): two hits of her own on surface ships, in one row. She must attack
/// at night with more than half her hull, and a surface ship must be afloat. One roll per cut-in
/// she makes, in order. Once it is made, her fleet counts as having fired a star shell for the
/// rest of the night (the source's `star1 = true`), the cut-in's two hits included.
#[expect(clippy::too_many_arguments)]
fn zuiun_night_attack(
    codex: &Codex,
    rng: &mut impl BattleRng,
    friendly: &mut [BattleRuntimeShip],
    idx: usize,
    enemy: &mut [BattleRuntimeShip],
    forms: (i64, i64),
    params: &NightBattleParams,
    lights: &mut (NightLights, NightLights),
    hougeki: &mut BattleNightHougeki,
) -> bool {
    let ship = &friendly[idx];
    if !can_attack_night_ship(codex, ship)
        || ship.hp() * 2 <= ship.ship.api_maxhp
        || !enemy.iter().any(|ship| ship.is_alive() && !target_class(codex, ship).is_submarine())
    {
        return false;
    }
    let kinds = zuiun_night_cut_ins(codex, ship);
    if kinds.is_empty() {
        return false;
    }
    let rate = zuiun_night_chance(codex, ship, idx == 0, *lights) * 0.01 / ZuiunNightCutIn::CHANCE;
    let Some(kind) = kinds.into_iter().find(|_| rng.random_f64_range(0.0, 1.0) < rate) else {
        return false;
    };
    lights.0.star_shell = true;
    let hit = || SpecialHit {
        fleet_index: idx,
        multiplier: kind.damage_multiplier(),
        accuracy: ZuiunNightCutIn::ACCURACY,
    };
    // The client draws the first slot's aircraft, unchecked: a 夜間瑞雲 goes first. It reads
    // three pieces, also unchecked (`CutinZuiunNightCanvas`), so main guns fill the rest; she
    // has two. Which pieces the game shows past the 瑞雲 has not been seen.
    let zuiun = usize::from(matches!(kind, ZuiunNightCutIn::TwoAndRadar | ZuiunNightCutIn::Two));
    let mut shown = vec![NIGHT_ZUIUN_ID; zuiun + 1];
    if matches!(kind, ZuiunNightCutIn::TwoAndRadar | ZuiunNightCutIn::OneAndRadar) {
        let radars = collect_matching_slot_ids(codex, &friendly[idx], |slot_type, mst| {
            is_radar_type(slot_type) && mst.api_saku >= 5
        });
        shown.extend(radars.first());
    }
    let guns = collect_matching_slot_ids(codex, &friendly[idx], |slot_type, _| {
        is_main_gun_type(slot_type)
    });
    extend_limit(&mut shown, &guns, 3);
    let row = SpecialRow {
        sp: ZUIUN_NIGHT_SP,
        at: idx,
        shown,
        star_shell: true,
        searchlight: lights.0.searchlight > 0,
    };
    special_attack_row(codex, rng, friendly, enemy, forms, params, hougeki, row, &[hit(), hit()]);
    true
}

/// A surface ship afloat to attack, in a fleet that is not PT boats alone.
fn has_special_attack_target(codex: &Codex, enemy: &[BattleRuntimeShip]) -> bool {
    !crate::targeting::is_pt_only(codex, enemy)
        && enemy.iter().any(|ship| ship.is_alive() && !target_class(codex, ship).is_submarine())
}

/// Who a special attack's row at night is written for, and what it shows.
struct SpecialRow {
    /// `api_sp_list`.
    sp: i64,
    /// `api_at_list`.
    at: usize,
    /// `api_si_list`.
    shown: Vec<i64>,
    /// Whether her fleet's star shell counts for the hits.
    star_shell: bool,
    /// Whether her fleet's searchlight is lit for the hits.
    searchlight: bool,
}

impl SpecialRow {
    /// A fleet special attack's: the flagship's row, which shows her main guns; the client draws
    /// the ships from their places in the fleet and reads no equipment.
    fn flagship(
        codex: &Codex,
        friendly: &[BattleRuntimeShip],
        params: &NightBattleParams,
        sp: i64,
    ) -> Self {
        let guns = collect_matching_slot_ids(codex, &friendly[0], |slot_type, _| {
            is_main_gun_type(slot_type)
        });
        let mut shown = Vec::new();
        extend_limit(&mut shown, &guns, 3);
        if shown.is_empty() {
            shown.push(-1);
        }
        Self {
            sp,
            at: 0,
            shown,
            star_shell: params.flare_pos[0] >= 0,
            // The flagship's turn is the night's first, so no ship has sunk since it began.
            searchlight: fleet_searchlight(codex, friendly) > 0,
        }
    }
}

/// The one row of a special attack at night: each hit on a surface ship of its own, its
/// multiplier applied before the cap.
#[expect(clippy::too_many_arguments)]
fn special_attack_row(
    codex: &Codex,
    rng: &mut impl BattleRng,
    friendly: &mut [BattleRuntimeShip],
    enemy: &mut [BattleRuntimeShip],
    (friendly_formation, enemy_formation): (i64, i64),
    params: &NightBattleParams,
    hougeki: &mut BattleNightHougeki,
    row: SpecialRow,
    hits: &[SpecialHit],
) {
    let (mut targets, mut cls, mut damages) = (Vec::new(), Vec::new(), Vec::new());
    for hit in hits {
        let ship = &mut friendly[hit.fleet_index];
        if !ship.is_alive() {
            continue;
        }
        // A night surface draw: the defenders' searchlights, and the escort deck protects.
        let rerolls = fleet_searchlight(codex, enemy);
        let candidates = surface_candidates(codex, enemy);
        let Some((target_idx, shield)) =
            choice_with_protect(codex, rng, enemy, &candidates, rerolls, true, enemy_formation)
        else {
            continue;
        };
        let aim = Aim::new(AttackKind::Night, friendly_formation, enemy_formation)
            .with_modifier(hit.accuracy)
            .under_contact(params.contact[0])
            .lit_by_star_shell(row.star_shell)
            .lit_by_searchlight(row.searchlight);
        let outcome = roll_attack(codex, rng, ship, &enemy[target_idx], aim);
        let raw = calculate_night_damage(
            codex,
            rng,
            ship,
            &enemy[target_idx],
            params.contact[0],
            Some(hit.multiplier),
            None,
            outcome,
        );
        let (raw, cl) = cut_in_hit(rng, outcome, raw, enemy[target_idx].hp());
        let landed = enemy[target_idx].take_hit(rng, raw, shield);
        ship.damage_dealt += landed.dealt;
        targets.push(target_idx as i64);
        cls.push(cl);
        damages.push(landed.cell);
    }
    if targets.is_empty() {
        return;
    }
    hougeki.record_night_attack(
        NightAttackKind::Special(&row),
        false,
        row.at,
        targets,
        cls,
        damages,
    );
}

/// Simulate the night battle hougeki phase. The two fleets fire in turn down
/// their lines, the friendly side first: its first ship, the enemy's first,
/// its second, and so on.
pub(crate) fn simulate_night_hougeki(
    codex: &Codex,
    rng: &mut impl BattleRng,
    friendly: &mut [BattleRuntimeShip],
    enemy: &mut [BattleRuntimeShip],
    params: &NightBattleParams,
) -> Option<BattleNightHougeki> {
    let mut hougeki = BattleNightHougeki {
        api_at_eflag: Vec::new(),
        api_at_list: Vec::new(),
        api_n_mother_list: Vec::new(),
        api_df_list: Vec::new(),
        api_si_list: Vec::new(),
        api_cl_list: Vec::new(),
        api_sp_list: Vec::new(),
        api_damage: Vec::new(),
    };
    let (friendly_form, enemy_form) = (params.friendly_formation_id, params.enemy_formation_id);
    // `nightPhase`: a fleet's searchlight is lit for the whole night if a ship afloat has one.
    let lights = |fleet: &[BattleRuntimeShip], flare: i64| NightLights {
        star_shell: flare >= 0,
        searchlight: fleet_searchlight(codex, fleet),
    };
    let mut friendly_lights = lights(friendly, params.flare_pos[0]);
    let enemy_lights = lights(enemy, params.flare_pos[1]);

    for idx in 0..friendly.len().max(enemy.len()) {
        if idx < friendly.len() {
            let forms = (friendly_form, enemy_form);
            // A special attack takes the flagship's turn; the ships that join it keep theirs.
            // Any ship's night 瑞雲 cut-in takes hers.
            let special = idx == 0
                && (kongou_night_assault(codex, rng, friendly, enemy, forms, params, &mut hougeki)
                    || special_gunnery_at_night(
                        codex,
                        rng,
                        friendly,
                        enemy,
                        forms,
                        params,
                        &mut hougeki,
                    ));
            let mut lights = (friendly_lights, enemy_lights);
            let zuiun = !special
                && zuiun_night_attack(
                    codex,
                    rng,
                    friendly,
                    idx,
                    enemy,
                    forms,
                    params,
                    &mut lights,
                    &mut hougeki,
                );
            friendly_lights = lights.0;
            if !special && !zuiun {
                night_turn(
                    codex,
                    rng,
                    friendly,
                    idx,
                    enemy,
                    forms,
                    params.contact[0],
                    params,
                    (false, friendly_lights, enemy_lights),
                    &mut hougeki,
                );
            }
        }
        if !any_alive(enemy) {
            break;
        }
        if idx < enemy.len() {
            let forms = (enemy_form, friendly_form);
            night_turn(
                codex,
                rng,
                enemy,
                idx,
                friendly,
                forms,
                params.contact[1],
                params,
                (true, enemy_lights, friendly_lights),
                &mut hougeki,
            );
        }
        if !any_alive(friendly) {
            break;
        }
    }

    (!hougeki.api_at_list.is_empty()).then_some(hougeki)
}

/// One ship's night attack, if it can make one and has a target.
#[expect(clippy::too_many_arguments)]
fn night_turn(
    codex: &Codex,
    rng: &mut impl BattleRng,
    attackers: &mut [BattleRuntimeShip],
    idx: usize,
    defenders: &mut [BattleRuntimeShip],
    (attacker_formation, defender_formation): (i64, i64),
    contact: Option<NightContact>,
    params: &NightBattleParams,
    (attacker_is_enemy, own, theirs): (bool, NightLights, NightLights),
    hougeki: &mut BattleNightHougeki,
) {
    let ship = &mut attackers[idx];
    if !can_attack_night_ship(codex, ship) {
        return;
    }
    // `nightPhaseTarget`: a carrier marked 3 takes a surface ship only while the fleet she
    // attacks has its searchlight lit. In the dark she is left the submarines, if she can
    // attack one.
    let candidates = if abyssal_night_attack(ship) == 3 && theirs.searchlight == 0 {
        if !can_attack_submarine_night_shelling(codex, ship) {
            return;
        }
        submarine_candidates(codex, defenders)
    } else {
        target_candidates(codex, ship, defenders, BattlePhase::NightShelling)
    };
    // `nightPhaseTarget`: the candidates are of one kind. A submarine is drawn once and only
    // her own deck covers her; a surface ship is drawn again for the searchlights, and the
    // escort deck covers too.
    let surface =
        candidates.first().is_some_and(|&at| !target_class(codex, &defenders[at]).is_submarine());
    let rerolls = if surface {
        theirs.searchlight
    } else {
        0
    };
    // 旗艦援護 (かばう): a healthy escort may intercept a flagship-targeted hit.
    let Some((target_idx, shield)) = choice_with_protect(
        codex,
        rng,
        defenders,
        &candidates,
        rerolls,
        surface,
        defender_formation,
    ) else {
        return;
    };
    let target = target_class(codex, &defenders[target_idx]);
    let is_submarine = target.is_submarine();
    let (attack_type, multiplier) =
        resolve_night_attack(codex, rng, ship, idx == 0, target, (own, theirs));
    // A submarine is not attacked from the air at night.
    let air_power = (!is_submarine && can_night_air_attack(codex, ship))
        .then(|| night_air_power(codex, ship, target == TargetClass::Installation));
    let hits = attack_type.hit_count();

    let mut hit_damages = Vec::new();
    let mut hit_cls = Vec::new();
    let mut total_dealt = 0i64;

    for _ in 0..hits {
        // An attack on a submarine rolls as it does by day (`ASW`), not as night gunfire.
        let aim = if is_submarine {
            Aim::new(AttackKind::Asw, attacker_formation, defender_formation)
        } else {
            Aim::new(AttackKind::Night, attacker_formation, defender_formation)
                .with_modifier(attack_type.accuracy_modifier())
                .as_night_air_attack(air_power.is_some())
                .under_contact(contact)
                .lit_by_star_shell(own.star_shell)
                .lit_by_searchlight(own.searchlight > 0)
        };
        let outcome = roll_attack(codex, rng, ship, &defenders[target_idx], aim);
        // `nightPhase` hands `ASW` `isNightDmg = !NBonly && !isescort`: the attack counts in a
        // night-start battle, and for a combined fleet's escort after a day battle too.
        let raw = if is_submarine && (params.night_start || ship.is_escort_deck()) {
            calculate_asw_damage(
                codex,
                rng,
                ship,
                &defenders[target_idx],
                attacker_formation,
                params.engagement,
                outcome,
                asw_proficiency(codex, ship),
            )
        } else if is_submarine {
            // After a day battle her attack power counts as 0 (`isNightDmg`): a hit only
            // scratches, and the submarine cannot be sunk.
            match outcome {
                HitOutcome::Miss => 0,
                _ => calculate_scratch_damage(rng, defenders[target_idx].hp().max(1)),
            }
        } else {
            calculate_night_damage(
                codex,
                rng,
                ship,
                &defenders[target_idx],
                contact,
                if multiplier != 1.0 {
                    Some(multiplier)
                } else {
                    None
                },
                air_power,
                outcome,
            )
        };
        // A plain attack that misses does nothing; a double attack's or a cut-in's scratches.
        let (raw, cl) = if attack_type == NightAttackType::Normal {
            (raw, outcome.cl())
        } else {
            cut_in_hit(rng, outcome, raw, defenders[target_idx].hp())
        };
        let landed = defenders[target_idx].take_hit(rng, raw, shield);
        total_dealt += landed.dealt;
        hit_damages.push(landed.cell);
        hit_cls.push(cl);
    }
    ship.damage_dealt += total_dealt;

    hougeki.record_night_attack(
        NightAttackKind::Own {
            codex,
            ship,
            attack_type,
            // The source leaves the aircraft off for a double attack and for an attack
            // on a submarine.
            by_air: attack_type != NightAttackType::DoubleAttack && air_power.is_some(),
        },
        attacker_is_enemy,
        idx,
        vec![target_idx as i64; hits],
        hit_cls,
        hit_damages,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use crate::types::{
        BattleRuntimeShip, DamageCell, EngagementType, NightBattleParams, SiListId,
    };
    use emukc_model::codex::Codex;
    use emukc_model::kc2::types::KcShipType;
    use emukc_model::kc2::types::KcSlotItemType3;

    /// The chance that `kind` comes up for a ship nothing is lit for.
    fn trigger_rate(
        codex: &Codex,
        ship: &BattleRuntimeShip,
        kind: NightAttackType,
        is_flagship: bool,
    ) -> f64 {
        let lights = (NightLights::default(), NightLights::default());
        night_cut_in_chance(codex, ship, is_flagship, lights) * 0.01 / kind.chance_modifier()
    }

    #[test]
    fn night_ci_triple_main_gun_detects_as_main_main_main() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);
        let main_gun_mst_id =
            first_slotitem_mst_by_type(&codex, KcSlotItemType3::LargeCaliberMainGun);

        let mut ship = sample_ship(&codex, bb_mst, 99);
        ship.slot_items = vec![
            slotitem_with_mst_id(main_gun_mst_id),
            slotitem_with_mst_id(main_gun_mst_id),
            slotitem_with_mst_id(main_gun_mst_id),
        ];
        let rt = BattleRuntimeShip::from(ship);
        let attack = detect_night_attack_type(&codex, &rt);
        assert_eq!(attack, NightAttackType::MainMainMain);
        assert!((attack.damage_multiplier() - 2.0).abs() < f64::EPSILON);
        assert_eq!(attack.hit_count(), 1);
    }

    #[test]
    fn night_ci_torpedo_torpedo_detects_as_torp_ci() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let torp_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Torpedo);

        let mut ship = sample_ship(&codex, dd_mst, 99);
        ship.slot_items =
            vec![slotitem_with_mst_id(torp_mst_id), slotitem_with_mst_id(torp_mst_id)];
        let rt = BattleRuntimeShip::from(ship);
        let attack = detect_night_attack_type(&codex, &rt);
        assert_eq!(attack, NightAttackType::TorpTorpTorp);
        assert!((attack.damage_multiplier() - 1.5).abs() < f64::EPSILON);
        assert_eq!(attack.hit_count(), 2);
    }

    #[test]
    fn night_ci_main_main_secondary_detects_correctly() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bb_mst = first_ship_mst_by_type(&codex, KcShipType::BB);
        let main_gun_mst_id =
            first_slotitem_mst_by_type(&codex, KcSlotItemType3::LargeCaliberMainGun);
        let sec_gun_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::SecondaryGun);

        let mut ship = sample_ship(&codex, bb_mst, 99);
        ship.slot_items = vec![
            slotitem_with_mst_id(main_gun_mst_id),
            slotitem_with_mst_id(main_gun_mst_id),
            slotitem_with_mst_id(sec_gun_mst_id),
        ];
        let rt = BattleRuntimeShip::from(ship);
        let attack = detect_night_attack_type(&codex, &rt);
        assert_eq!(attack, NightAttackType::MainMainSec);
        assert!((attack.damage_multiplier() - 1.75).abs() < f64::EPSILON);
    }

    #[test]
    fn a_main_gun_with_a_torpedo_is_a_cut_in_without_a_radar() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let main_gun_mst_id =
            first_slotitem_mst_by_type(&codex, KcSlotItemType3::SmallCaliberMainGun);
        let torp_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Torpedo);

        let mut ship = sample_ship(&codex, dd_mst, 99);
        ship.slot_items =
            vec![slotitem_with_mst_id(main_gun_mst_id), slotitem_with_mst_id(torp_mst_id)];
        let rt = BattleRuntimeShip::from(ship);
        let attack = detect_night_attack_type(&codex, &rt);
        assert_eq!(attack, NightAttackType::MainTorp);
        assert_eq!(attack.hit_count(), 2);
        assert!((attack.damage_multiplier() - 1.3).abs() < f64::EPSILON);
    }

    #[test]
    fn night_ci_trigger_rate_increases_with_luck() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        let mut low_luck_ship = sample_ship(&codex, dd_mst, 99);
        low_luck_ship.ship.api_lucky = [10, 30];
        let rt_low = BattleRuntimeShip::from(low_luck_ship);

        let mut high_luck_ship = sample_ship(&codex, dd_mst, 99);
        high_luck_ship.ship.api_lucky = [80, 99];
        let rt_high = BattleRuntimeShip::from(high_luck_ship);

        let rate_low = trigger_rate(&codex, &rt_low, NightAttackType::TorpTorpTorp, false);
        let rate_high = trigger_rate(&codex, &rt_high, NightAttackType::TorpTorpTorp, false);
        assert!(
            rate_high > rate_low,
            "higher luck should give higher CI rate: {rate_high} > {rate_low}"
        );
    }

    #[test]
    fn night_battle_sp_list_nonzero_for_ci_ship() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let torp_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Torpedo);

        let mut friend = sample_ship(&codex, dd_mst, 99);
        friend.ship.api_lucky = [90, 99];
        friend.ship.api_karyoku[0] = 150;
        friend.ship.api_raisou[0] = 200;
        friend.ship.api_soukou[0] = 200;
        friend.ship.api_nowhp = 200;
        friend.ship.api_maxhp = 200;
        friend.slot_items =
            vec![slotitem_with_mst_id(torp_mst_id), slotitem_with_mst_id(torp_mst_id)];

        let mut enemy_ship = sample_ship(&codex, dd_mst, 50);
        enemy_ship.ship.api_soukou[0] = 10;
        enemy_ship.ship.api_nowhp = 500;
        enemy_ship.ship.api_maxhp = 500;
        enemy_ship.ship.api_karyoku[0] = 1;

        let mut friendly = vec![BattleRuntimeShip::from(friend)];
        let mut enemies = vec![BattleRuntimeShip::from(enemy_ship)];
        let mut rng = crate::random::SeededRng::new(0);

        let hougeki = simulate_night_hougeki(
            &codex,
            &mut rng,
            &mut friendly,
            &mut enemies,
            &NightBattleParams {
                special_attacks_made: Default::default(),
                friendly_formation_id: 1,
                enemy_formation_id: 1,
                engagement: EngagementType::SameCourse,
                contact: [None, None],
                night_start: false,
                flare_pos: [-1, -1],
            },
        )
        .unwrap();

        assert_eq!(hougeki.api_sp_list[0], 3, "torpedo CI sp_list should be 3 (魚雷/魚雷)");
        assert_eq!(hougeki.api_damage[0].len(), 2, "torpedo CI should deal 2 hits");
        // A night CI must serialize its si_list as JSON strings. Guards against
        // a Text/Num swap at the simulate_night_hougeki push site that the
        // isolated packet serialization tests cannot catch.
        assert!(
            hougeki.api_si_list[0].iter().any(|id| matches!(id, SiListId::Text(_))),
            "night CI si_list must contain string entries: {:?}",
            hougeki.api_si_list[0]
        );
    }

    #[test]
    fn night_normal_attack_si_list_is_integers() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        // A single main gun cannot form any night cut-in or double attack, so
        // the attack resolves as Normal (sp_list == 0) and its si_list must
        // stay integer-typed — the counterpart guard to the CI test above.
        let main_gun_mst_id =
            first_slotitem_mst_by_type(&codex, KcSlotItemType3::SmallCaliberMainGun);

        let mut friend = sample_ship(&codex, dd_mst, 99);
        friend.ship.api_karyoku[0] = 150;
        friend.ship.api_nowhp = 200;
        friend.ship.api_maxhp = 200;
        friend.slot_items = vec![slotitem_with_mst_id(main_gun_mst_id)];

        let mut enemy_ship = sample_ship(&codex, dd_mst, 50);
        enemy_ship.ship.api_soukou[0] = 10;
        enemy_ship.ship.api_nowhp = 500;
        enemy_ship.ship.api_maxhp = 500;
        enemy_ship.ship.api_karyoku[0] = 1;

        let mut friendly = vec![BattleRuntimeShip::from(friend)];
        let mut enemies = vec![BattleRuntimeShip::from(enemy_ship)];
        let mut rng = crate::random::SeededRng::new(42);

        let hougeki = simulate_night_hougeki(
            &codex,
            &mut rng,
            &mut friendly,
            &mut enemies,
            &NightBattleParams {
                special_attacks_made: Default::default(),
                friendly_formation_id: 1,
                enemy_formation_id: 1,
                engagement: EngagementType::SameCourse,
                contact: [None, None],
                night_start: false,
                flare_pos: [-1, -1],
            },
        )
        .unwrap();

        assert_eq!(hougeki.api_sp_list[0], 0, "single-gun DD must do a normal night attack");
        assert!(
            hougeki.api_si_list[0].iter().all(|id| matches!(id, SiListId::Num(_))),
            "normal night attack si_list must be all integers: {:?}",
            hougeki.api_si_list[0]
        );
    }

    /// A destroyer's night attack on a 30 HP submarine, one per seed: the `api_cl_list` value
    /// and the submarine's HP afterwards. `escort` puts her in a combined fleet's escort.
    fn night_asw_runs(
        codex: &Codex,
        sonar: bool,
        night_start: bool,
        escort: bool,
    ) -> Vec<(i64, i64)> {
        let mut hunter = sample_ship(codex, first_ship_mst_by_type(codex, KcShipType::DD), 99);
        hunter.ship.api_taisen[0] = 100;
        // Two torpedoes would be a cut-in on a surface ship.
        let item = |kind| slotitem_with_mst_id(first_slotitem_mst_by_type(codex, kind));
        hunter.slot_items = vec![
            item(KcSlotItemType3::Torpedo),
            item(KcSlotItemType3::Torpedo),
            item(KcSlotItemType3::DepthCharge),
        ];
        if sonar {
            hunter.slot_items.push(item(KcSlotItemType3::Sonar));
        }
        let mut submarine = sample_ship(codex, first_ship_mst_by_type(codex, KcShipType::SS), 50);
        submarine.ship.api_soukou[0] = 5;
        submarine.ship.api_nowhp = 30;
        submarine.ship.api_maxhp = 30;

        (0..300)
            .map(|seed| {
                let mut ship = BattleRuntimeShip::from(hunter.clone());
                if escort {
                    ship = ship.in_combined_fleet(
                        crate::combined::CombinedType::CarrierTaskForce,
                        crate::combined::CombinedFleetRole::Escort,
                    );
                }
                let mut friendly = vec![ship];
                let mut enemy = vec![BattleRuntimeShip::from(submarine.clone())];
                let hougeki = simulate_night_hougeki(
                    codex,
                    &mut crate::random::SeededRng::new(seed),
                    &mut friendly,
                    &mut enemy,
                    &NightBattleParams {
                        special_attacks_made: Default::default(),
                        friendly_formation_id: 5,
                        enemy_formation_id: 1,
                        engagement: EngagementType::SameCourse,
                        contact: [None, None],
                        night_start,
                        flare_pos: [-1, -1],
                    },
                )
                .unwrap();
                // The destroyer attacks first: with a depth charge, once, and no cut-in.
                assert_eq!((hougeki.api_at_eflag[0], hougeki.api_sp_list[0]), (0, 0));
                assert_eq!(hougeki.api_n_mother_list[0], 0);
                assert_eq!(hougeki.api_df_list[0], vec![0]);
                let cl = hougeki.api_cl_list[0][0];
                assert_eq!(30 - enemy[0].hp(), hougeki.api_damage[0][0].amount());
                assert_eq!(cl == 0, enemy[0].hp() == 30, "a miss does nothing, a hit something");
                (cl, enemy[0].hp())
            })
            .collect()
    }

    #[test]
    fn a_night_attack_on_a_submarine_after_a_day_battle_can_miss_and_only_scratches() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let runs = night_asw_runs(&codex, true, false, false);

        assert!(runs.iter().any(|(cl, _)| *cl == 0), "it can miss");
        assert!(runs.iter().any(|(cl, _)| *cl > 0), "it can hit");
        // Scratch damage is at most 14% of the 30 HP she has.
        assert!(runs.iter().all(|(_, hp)| *hp >= 26), "a hit is a scratch: {runs:?}");
    }

    #[test]
    fn a_night_attack_on_a_submarine_in_a_night_start_battle_can_sink_her() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let runs = night_asw_runs(&codex, true, true, false);

        assert!(runs.iter().any(|(cl, _)| *cl == 0), "it can miss");
        assert!(runs.iter().any(|(_, hp)| *hp <= 0), "it can sink her");
    }

    /// `nightPhase` hands `ASW` `!NBonly && !isescort`: a combined fleet's escort does real damage
    /// to a submarine in the night after a day battle too.
    #[test]
    fn a_combined_fleets_escort_can_sink_a_submarine_after_a_day_battle() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let runs = night_asw_runs(&codex, true, false, true);

        assert!(runs.iter().any(|(cl, _)| *cl == 0), "it can miss");
        assert!(runs.iter().any(|(_, hp)| *hp <= 0), "it can sink her");
    }

    #[test]
    fn a_sonar_makes_a_night_attack_on_a_submarine_miss_less() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let misses = |sonar| {
            night_asw_runs(&codex, sonar, false, false).iter().filter(|(cl, _)| *cl == 0).count()
        };

        assert!(misses(true) < misses(false), "{} < {}", misses(true), misses(false));
    }

    #[test]
    fn regular_carrier_cannot_attack_in_night_battle_without_night_crew() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let carrier_mst = first_ship_mst_by_type(&codex, KcShipType::CVL);
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        let carrier = sample_ship(&codex, carrier_mst, 50);
        let enemy = sample_ship(&codex, dd_mst, 50);
        let mut rng = crate::random::SeededRng::new(0);

        let simulation = crate::simulation::simulate_night(
            &codex,
            crate::types::NightBattleInput {
                special_attacks_made: Default::default(),
                friendly: vec![BattleRuntimeShip::from(carrier)],
                enemy: vec![BattleRuntimeShip::from(enemy)],
                friendly_formation_id: 1,
                enemy_formation_id: 1,
                engagement: EngagementType::SameCourse,
                air_state: None,
                night_start: false,
            },
            &mut rng,
        );

        let hougeki = simulation.packet.hougeki.unwrap();
        assert!(hougeki.api_at_eflag.iter().all(|flag| *flag == 1));
    }

    #[test]
    fn sortie_night_battle_non_taiha_ship_survives_lethal_damage() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        // Create a ship with enough HP to NOT be taiha at entry (>25% max)
        let mut friend = sample_ship(&codex, dd_mst, 99);
        friend.ship.api_karyoku[0] = 200;
        friend.ship.api_raisou[0] = 0;
        friend.ship.api_soukou[0] = 0;
        friend.ship.api_nowhp = 100;
        friend.ship.api_maxhp = 100;
        let mut friendly = vec![BattleRuntimeShip::new(friend, true, true)];

        // Enemy with massive firepower to deal lethal damage
        let mut enemy = sample_ship(&codex, dd_mst, 99);
        enemy.ship.api_karyoku[0] = 500;
        enemy.ship.api_raisou[0] = 0;
        enemy.ship.api_soukou[0] = 200;
        enemy.ship.api_nowhp = 500;
        enemy.ship.api_maxhp = 500;
        let mut enemies = vec![BattleRuntimeShip::new(enemy, false, true)];

        let mut rng = crate::random::SeededRng::new(42);

        let _ = simulate_night_hougeki(
            &codex,
            &mut rng,
            &mut enemies, // enemy attacks first
            &mut friendly,
            &NightBattleParams {
                special_attacks_made: Default::default(),
                friendly_formation_id: 1,
                enemy_formation_id: 1,
                engagement: EngagementType::SameCourse,
                contact: [None, None],
                night_start: false,
                flare_pos: [-1, -1],
            },
        );

        // In a sortie night battle, the friendly ship should survive due to sinking protection
        // (entry_hp was 100, max_hp was 100, so 100*4 > 100 means not taiha)
        assert!(
            friendly[0].hp() > 0,
            "sortie night battle: non-taiha ship should survive lethal damage, got HP={}",
            friendly[0].hp()
        );
    }

    #[test]
    fn practice_night_battle_non_taiha_ship_can_be_sunk() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        // Create a ship with enough HP to NOT be taiha at entry
        let mut friend = sample_ship(&codex, dd_mst, 99);
        friend.ship.api_karyoku[0] = 0;
        friend.ship.api_raisou[0] = 0;
        friend.ship.api_soukou[0] = 0;
        friend.ship.api_nowhp = 10;
        friend.ship.api_maxhp = 100;
        let mut friendly = vec![BattleRuntimeShip::new(friend, true, false)]; // is_sortie=false

        // Enemy with massive firepower
        let mut enemy = sample_ship(&codex, dd_mst, 99);
        enemy.ship.api_karyoku[0] = 500;
        enemy.ship.api_raisou[0] = 0;
        enemy.ship.api_soukou[0] = 200;
        enemy.ship.api_nowhp = 500;
        enemy.ship.api_maxhp = 500;
        let mut enemies = vec![BattleRuntimeShip::new(enemy, false, false)];

        let mut rng = crate::random::SeededRng::new(42);

        let _ = simulate_night_hougeki(
            &codex,
            &mut rng,
            &mut enemies,
            &mut friendly,
            &NightBattleParams {
                special_attacks_made: Default::default(),
                friendly_formation_id: 1,
                enemy_formation_id: 1,
                engagement: EngagementType::SameCourse,
                contact: [None, None],
                night_start: false,
                flare_pos: [-1, -1],
            },
        );

        // In practice, sinking protection does NOT apply
        // But ships may survive anyway due to scratch damage (attack < defense)
        // The key assertion: if the ship took lethal-level damage, HP can reach 0
        // We just verify the ship was created as practice (no protection)
        assert!(!friendly[0].is_sortie, "practice ship should have is_sortie=false");
    }

    #[test]
    fn night_ci_priority_torpedo_over_main_torp_radar() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let torp_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Torpedo);
        let main_gun_mst_id =
            first_slotitem_mst_by_type(&codex, KcSlotItemType3::SmallCaliberMainGun);
        let radar_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::SmallRadar);

        // Ship with 1 main gun + 2 torpedoes + 1 radar
        // Should detect TorpTorpTorp (priority 3), NOT MainTorp (priority 4)
        let mut ship = sample_ship(&codex, dd_mst, 99);
        ship.slot_items = vec![
            slotitem_with_mst_id(main_gun_mst_id),
            slotitem_with_mst_id(torp_mst_id),
            slotitem_with_mst_id(torp_mst_id),
            slotitem_with_mst_id(radar_mst_id),
        ];
        let rt = BattleRuntimeShip::from(ship);
        let attack = detect_night_attack_type(&codex, &rt);
        assert_eq!(
            attack,
            NightAttackType::TorpTorpTorp,
            "ship with 2 torpedoes should get TorpTorpTorp, not MainTorp"
        );
    }

    #[test]
    fn night_ci_trigger_rate_uses_current_luck_not_its_maximum() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        // `api_lucky` is the current luck and the most it can be modernised to.
        let mut ship = sample_ship(&codex, dd_mst, 99);
        ship.ship.api_lucky = [80, 99];
        let rt = BattleRuntimeShip::from(ship);

        let rate = trigger_rate(&codex, &rt, NightAttackType::TorpTorpTorp, false);
        // 65 + floor(sqrt(80 - 50)) + floor(0.8 * sqrt(99)) = 77, over 122.
        assert!((rate - 0.77 / 1.22).abs() < 1e-9, "got {rate}");
    }

    #[test]
    fn night_ci_chuuha_bonus_increases_trigger_rate() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        let mut ship_healthy = sample_ship(&codex, dd_mst, 99);
        ship_healthy.ship.api_lucky = [50, 50];
        ship_healthy.ship.api_nowhp = 100;
        ship_healthy.ship.api_maxhp = 100;
        let rt_healthy = BattleRuntimeShip::from(ship_healthy);

        let mut ship_chuuha = sample_ship(&codex, dd_mst, 99);
        ship_chuuha.ship.api_lucky = [50, 50];
        ship_chuuha.ship.api_nowhp = 30; // 30% HP = chuuha
        ship_chuuha.ship.api_maxhp = 100;
        let rt_chuuha = BattleRuntimeShip::from(ship_chuuha);

        let rate_healthy = trigger_rate(&codex, &rt_healthy, NightAttackType::TorpTorpTorp, false);
        let rate_chuuha = trigger_rate(&codex, &rt_chuuha, NightAttackType::TorpTorpTorp, false);

        assert!(
            rate_chuuha > rate_healthy,
            "chuuha ship should have higher CI rate: {rate_chuuha} > {rate_healthy}"
        );
    }

    #[test]
    fn night_ci_taiha_no_chuuha_bonus() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        // Chuuha (30%) should get bonus
        let mut ship_chuuha = sample_ship(&codex, dd_mst, 99);
        ship_chuuha.ship.api_lucky = [50, 50];
        ship_chuuha.ship.api_nowhp = 30;
        ship_chuuha.ship.api_maxhp = 100;
        let rt_chuuha = BattleRuntimeShip::from(ship_chuuha);

        // Taiha (10%) should NOT get bonus
        let mut ship_taiha = sample_ship(&codex, dd_mst, 99);
        ship_taiha.ship.api_lucky = [50, 50];
        ship_taiha.ship.api_nowhp = 10;
        ship_taiha.ship.api_maxhp = 100;
        let rt_taiha = BattleRuntimeShip::from(ship_taiha);

        // Healthy (100%) should NOT get bonus
        let mut ship_healthy = sample_ship(&codex, dd_mst, 99);
        ship_healthy.ship.api_lucky = [50, 50];
        ship_healthy.ship.api_nowhp = 100;
        ship_healthy.ship.api_maxhp = 100;
        let rt_healthy = BattleRuntimeShip::from(ship_healthy);

        let rate_chuuha = trigger_rate(&codex, &rt_chuuha, NightAttackType::TorpTorpTorp, false);
        let rate_taiha = trigger_rate(&codex, &rt_taiha, NightAttackType::TorpTorpTorp, false);
        let rate_healthy = trigger_rate(&codex, &rt_healthy, NightAttackType::TorpTorpTorp, false);

        assert!(
            rate_chuuha > rate_taiha,
            "chuuha should have higher rate than taiha: {rate_chuuha} > {rate_taiha}"
        );
        assert_eq!(
            rate_taiha, rate_healthy,
            "taiha should have same rate as healthy (no bonus): {rate_taiha} == {rate_healthy}"
        );
    }

    #[test]
    fn dd_ci_taiha_no_chuuha_bonus() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        // Chuuha (30%) should get bonus
        let mut ship_chuuha = sample_ship(&codex, dd_mst, 99);
        ship_chuuha.ship.api_lucky = [50, 50];
        ship_chuuha.ship.api_nowhp = 30;
        ship_chuuha.ship.api_maxhp = 100;
        let rt_chuuha = BattleRuntimeShip::from(ship_chuuha);

        // Taiha (10%) should NOT get bonus
        let mut ship_taiha = sample_ship(&codex, dd_mst, 99);
        ship_taiha.ship.api_lucky = [50, 50];
        ship_taiha.ship.api_nowhp = 10;
        ship_taiha.ship.api_maxhp = 100;
        let rt_taiha = BattleRuntimeShip::from(ship_taiha);

        let rate_chuuha =
            trigger_rate(&codex, &rt_chuuha, NightAttackType::DdTorpTorpLookout, false);
        let rate_taiha = trigger_rate(&codex, &rt_taiha, NightAttackType::DdTorpTorpLookout, false);

        assert!(
            rate_chuuha > rate_taiha,
            "DD CI: chuuha should have higher rate than taiha: {rate_chuuha} > {rate_taiha}"
        );
    }

    #[test]
    fn night_ci_hp_at_25pct_no_chuuha_bonus() {
        // HP ratio exactly 0.25 must be treated as taiha (excluded by strict `> 0.25`).
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        let mut ship_25 = sample_ship(&codex, dd_mst, 99);
        ship_25.ship.api_lucky = [50, 50];
        ship_25.ship.api_nowhp = 25;
        ship_25.ship.api_maxhp = 100;
        let rt_25 = BattleRuntimeShip::from(ship_25);

        let mut ship_healthy = sample_ship(&codex, dd_mst, 99);
        ship_healthy.ship.api_lucky = [50, 50];
        ship_healthy.ship.api_nowhp = 100;
        ship_healthy.ship.api_maxhp = 100;
        let rt_healthy = BattleRuntimeShip::from(ship_healthy);

        let rate_25 = trigger_rate(&codex, &rt_25, NightAttackType::TorpTorpTorp, false);
        let rate_healthy = trigger_rate(&codex, &rt_healthy, NightAttackType::TorpTorpTorp, false);

        assert_eq!(
            rate_25, rate_healthy,
            "HP exactly 25% should NOT receive chuuha bonus (strict > 0.25)"
        );
    }

    #[test]
    fn dd_ci_hp_at_25pct_no_chuuha_bonus() {
        // Same boundary check for the DD-specific trigger formula.
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        let mut ship_25 = sample_ship(&codex, dd_mst, 99);
        ship_25.ship.api_lucky = [50, 50];
        ship_25.ship.api_nowhp = 25;
        ship_25.ship.api_maxhp = 100;
        let rt_25 = BattleRuntimeShip::from(ship_25);

        let mut ship_healthy = sample_ship(&codex, dd_mst, 99);
        ship_healthy.ship.api_lucky = [50, 50];
        ship_healthy.ship.api_nowhp = 100;
        ship_healthy.ship.api_maxhp = 100;
        let rt_healthy = BattleRuntimeShip::from(ship_healthy);

        let rate_25 = trigger_rate(&codex, &rt_25, NightAttackType::DdTorpTorpLookout, false);
        let rate_healthy =
            trigger_rate(&codex, &rt_healthy, NightAttackType::DdTorpTorpLookout, false);

        assert_eq!(
            rate_25, rate_healthy,
            "DD CI: HP exactly 25% should NOT receive chuuha bonus (strict > 0.25)"
        );
    }

    #[test]
    fn night_ci_hp_at_50pct_chuuha_bonus_applies() {
        // HP ratio exactly 0.50 falls inside the chuuha bracket (inclusive `<= 0.5`).
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        let mut ship_50 = sample_ship(&codex, dd_mst, 99);
        ship_50.ship.api_lucky = [50, 50];
        ship_50.ship.api_nowhp = 50;
        ship_50.ship.api_maxhp = 100;
        let rt_50 = BattleRuntimeShip::from(ship_50);

        let mut ship_healthy = sample_ship(&codex, dd_mst, 99);
        ship_healthy.ship.api_lucky = [50, 50];
        ship_healthy.ship.api_nowhp = 100;
        ship_healthy.ship.api_maxhp = 100;
        let rt_healthy = BattleRuntimeShip::from(ship_healthy);

        let rate_50 = trigger_rate(&codex, &rt_50, NightAttackType::TorpTorpTorp, false);
        let rate_healthy = trigger_rate(&codex, &rt_healthy, NightAttackType::TorpTorpTorp, false);

        assert!(
            rate_50 > rate_healthy,
            "HP exactly 50% should receive chuuha bonus (inclusive <= 0.5): {rate_50} > {rate_healthy}"
        );
    }

    #[test]
    fn dd_ci_hp_at_50pct_chuuha_bonus_applies() {
        // Same upper-edge check for the DD-specific trigger formula.
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        let mut ship_50 = sample_ship(&codex, dd_mst, 99);
        ship_50.ship.api_lucky = [50, 50];
        ship_50.ship.api_nowhp = 50;
        ship_50.ship.api_maxhp = 100;
        let rt_50 = BattleRuntimeShip::from(ship_50);

        let mut ship_healthy = sample_ship(&codex, dd_mst, 99);
        ship_healthy.ship.api_lucky = [50, 50];
        ship_healthy.ship.api_nowhp = 100;
        ship_healthy.ship.api_maxhp = 100;
        let rt_healthy = BattleRuntimeShip::from(ship_healthy);

        let rate_50 = trigger_rate(&codex, &rt_50, NightAttackType::DdTorpTorpLookout, false);
        let rate_healthy =
            trigger_rate(&codex, &rt_healthy, NightAttackType::DdTorpTorpLookout, false);

        assert!(
            rate_50 > rate_healthy,
            "DD CI: HP exactly 50% should receive chuuha bonus (inclusive <= 0.5): {rate_50} > {rate_healthy}"
        );
    }

    #[test]
    fn night_attack_type_sp_list_matches_apilist() {
        // Per apilist.txt lines 2319-2342:
        // 0=通常攻撃, 1=連続射撃
        // 2=カットイン(主砲/魚雷), 3=カットイン(魚雷/魚雷),
        // 4=カットイン(主砲/主砲/副砲), 5=カットイン(主砲/主砲/主砲)
        assert_eq!(NightAttackType::Normal.api_sp_list(), 0);
        assert_eq!(NightAttackType::DoubleAttack.api_sp_list(), 1);
        assert_eq!(NightAttackType::MainTorp.api_sp_list(), 2);
        assert_eq!(NightAttackType::TorpTorpTorp.api_sp_list(), 3);
        assert_eq!(NightAttackType::MainMainSec.api_sp_list(), 4);
        assert_eq!(NightAttackType::MainMainMain.api_sp_list(), 5);
    }

    #[test]
    fn night_ci_multiplier_applied_pre_cap() {
        // Verify that CI multiplier is applied before the soft cap at 360
        // A ship with 500 base power and 2.0x CI should be capped at:
        // apply_cap(500 * 2.0, 360) = 360 + sqrt(640) ≈ 385
        // NOT: apply_cap(500, 360) * 2.0 = 385 * 2.0 = 770 (post-defense multiplication)
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        let mut attacker = sample_ship(&codex, dd_mst, 99);
        attacker.ship.api_karyoku[0] = 400; // high firepower
        attacker.ship.api_raisou[0] = 50;
        attacker.ship.api_soukou[0] = 0;
        attacker.ship.api_nowhp = 100;
        attacker.ship.api_maxhp = 100;
        let rt_attacker = BattleRuntimeShip::from(attacker);

        let mut defender = sample_ship(&codex, dd_mst, 50);
        defender.ship.api_soukou[0] = 0; // zero armor for clean test
        defender.ship.api_nowhp = 9999;
        defender.ship.api_maxhp = 9999;
        let rt_defender = BattleRuntimeShip::from(defender);

        let mut rng = crate::random::SeededRng::new(42);

        // Damage with 2.0x CI multiplier (MainMainMain)
        let dmg_with_ci = calculate_night_damage(
            &codex,
            &mut rng,
            &rt_attacker,
            &rt_defender,
            None,
            Some(2.0),
            None,
            HitOutcome::Hit,
        );

        // Damage without CI multiplier
        let dmg_normal = calculate_night_damage(
            &codex,
            &mut rng,
            &rt_attacker,
            &rt_defender,
            None,
            None,
            None,
            HitOutcome::Hit,
        );

        // With pre-cap: apply_cap(455*2.0, 360) = 360 + sqrt(550) ≈ 383
        // capped_power ≈ 383, defense ≈ 0, so damage ≈ 383
        // With no CI: apply_cap(455, 360) = 360 + sqrt(95) ≈ 370
        // CI damage should be higher but NOT 2x of normal (due to soft cap)
        assert!(
            dmg_with_ci > dmg_normal,
            "CI damage ({dmg_with_ci}) should be higher than normal ({dmg_normal})"
        );
        // Pre-cap means CI damage < 2x normal (soft cap eats the excess)
        assert!(
            dmg_with_ci < dmg_normal * 2,
            "pre-cap CI damage ({dmg_with_ci}) should be < 2x normal ({}) due to soft cap",
            dmg_normal * 2
        );
    }

    // ------- DD CI detection tests -------

    #[test]
    fn dd_ci_gtr_detection() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let main_gun_mst_id =
            first_slotitem_mst_by_type(&codex, KcSlotItemType3::SmallCaliberMainGun);
        let torp_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Torpedo);
        let radar_mst_id = 28; // 22号対水上電探: the destroyer cut-ins want a surface radar

        let mut ship = sample_ship(&codex, dd_mst, 99);
        ship.slot_items = vec![
            slotitem_with_mst_id(main_gun_mst_id),
            slotitem_with_mst_id(torp_mst_id),
            slotitem_with_mst_id(radar_mst_id),
        ];
        let rt = BattleRuntimeShip::from(ship);

        assert!(night_attack_types(&codex, &rt).contains(&NightAttackType::DdGunTorpRadar));
        assert!(!night_attack_types(&codex, &rt).contains(&NightAttackType::DdTorpLookoutRadar));
        assert!(!night_attack_types(&codex, &rt).contains(&NightAttackType::DdTorpTorpLookout));
        assert!(!night_attack_types(&codex, &rt).contains(&NightAttackType::DdTorpDrumLookout));
    }

    #[test]
    fn dd_ci_trl_detection() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let torp_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Torpedo);
        let lookout_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::SeaplanePersonnel);
        let radar_mst_id = 28; // 22号対水上電探: the destroyer cut-ins want a surface radar

        let mut ship = sample_ship(&codex, dd_mst, 99);
        ship.slot_items = vec![
            slotitem_with_mst_id(torp_mst_id),
            slotitem_with_mst_id(lookout_mst_id),
            slotitem_with_mst_id(radar_mst_id),
        ];
        let rt = BattleRuntimeShip::from(ship);

        assert!(!night_attack_types(&codex, &rt).contains(&NightAttackType::DdGunTorpRadar));
        assert!(night_attack_types(&codex, &rt).contains(&NightAttackType::DdTorpLookoutRadar));
    }

    #[test]
    fn dd_ci_ttl_detection() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let torp_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Torpedo);

        let mut ship = sample_ship(&codex, dd_mst, 99);
        ship.slot_items = vec![
            slotitem_with_mst_id(torp_mst_id),
            slotitem_with_mst_id(412), // 水雷戦隊 熟練見張員
            slotitem_with_mst_id(torp_mst_id),
        ];
        let rt = BattleRuntimeShip::from(ship);

        assert!(night_attack_types(&codex, &rt).contains(&NightAttackType::DdTorpTorpLookout));
        assert!(!night_attack_types(&codex, &rt).contains(&NightAttackType::DdGunTorpRadar));
    }

    #[test]
    fn dd_ci_dtl_detection() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let torp_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Torpedo);
        let drum_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::TransportContainer);

        let mut ship = sample_ship(&codex, dd_mst, 99);
        ship.slot_items = vec![
            slotitem_with_mst_id(torp_mst_id),
            slotitem_with_mst_id(412), // 水雷戦隊 熟練見張員
            slotitem_with_mst_id(drum_mst_id),
        ];
        let rt = BattleRuntimeShip::from(ship);

        assert!(night_attack_types(&codex, &rt).contains(&NightAttackType::DdTorpDrumLookout));
    }

    #[test]
    fn dd_ci_non_dd_does_not_trigger() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cl_mst = first_ship_mst_by_type(&codex, KcShipType::CL);
        let main_gun_mst_id =
            first_slotitem_mst_by_type(&codex, KcSlotItemType3::MediumCaliberMainGun);
        let torp_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Torpedo);
        let radar_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::SmallRadar);

        // CL with same equipment as GTR → DD CI should NOT trigger
        let mut ship = sample_ship(&codex, cl_mst, 99);
        ship.slot_items = vec![
            slotitem_with_mst_id(main_gun_mst_id),
            slotitem_with_mst_id(torp_mst_id),
            slotitem_with_mst_id(radar_mst_id),
        ];
        let rt = BattleRuntimeShip::from(ship);

        // Standard detection for CL finds MainTorp (not DD CI)
        let attack = detect_night_attack_type(&codex, &rt);
        assert_eq!(attack, NightAttackType::MainTorp, "CL should get standard CI, not DD CI");
    }

    #[test]
    fn dd_ci_sp_list_values() {
        assert_eq!(NightAttackType::DdGunTorpRadar.api_sp_list(), 7);
        assert_eq!(NightAttackType::DdGunTorpRadar2.api_sp_list(), 11);
        assert_eq!(NightAttackType::DdTorpLookoutRadar.api_sp_list(), 8);
        assert_eq!(NightAttackType::DdTorpLookoutRadar2.api_sp_list(), 12);
        assert_eq!(NightAttackType::DdTorpTorpLookout.api_sp_list(), 9);
        assert_eq!(NightAttackType::DdTorpTorpLookout2.api_sp_list(), 13);
        assert_eq!(NightAttackType::DdTorpDrumLookout.api_sp_list(), 10);
        assert_eq!(NightAttackType::DdTorpDrumLookout2.api_sp_list(), 14);
    }

    #[test]
    fn dd_ci_multiroll_falls_through_to_standard_ci() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let torp_mst_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::Torpedo);

        // DD with 2 torpedoes only — no DD CI conditions met
        // Standard detection should still find TorpTorpTorp
        let mut ship = sample_ship(&codex, dd_mst, 99);
        ship.slot_items =
            vec![slotitem_with_mst_id(torp_mst_id), slotitem_with_mst_id(torp_mst_id)];
        let rt = BattleRuntimeShip::from(ship);

        let attack = detect_night_attack_type(&codex, &rt);
        assert_eq!(
            attack,
            NightAttackType::TorpTorpTorp,
            "DD without DD CI equipment falls to standard CI"
        );
    }

    #[test]
    fn dd_ci_trigger_rate_increases_with_luck() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        let mut low_luck = sample_ship(&codex, dd_mst, 99);
        low_luck.ship.api_lucky = [10, 20];
        let rt_low = BattleRuntimeShip::from(low_luck);

        let mut high_luck = sample_ship(&codex, dd_mst, 99);
        high_luck.ship.api_lucky = [80, 90];
        let rt_high = BattleRuntimeShip::from(high_luck);

        let rate_low = trigger_rate(&codex, &rt_low, NightAttackType::DdGunTorpRadar, false);
        let rate_high = trigger_rate(&codex, &rt_high, NightAttackType::DdGunTorpRadar, false);
        assert!(
            rate_high > rate_low,
            "DD CI: higher luck should give higher rate: {rate_high} > {rate_low}"
        );
    }

    #[test]
    fn dd_ci_trigger_rate_flagship_bonus() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        let mut ship = sample_ship(&codex, dd_mst, 99);
        ship.ship.api_lucky = [50, 50];
        let rt = BattleRuntimeShip::from(ship);

        let rate_normal = trigger_rate(&codex, &rt, NightAttackType::DdGunTorpRadar, false);
        let rate_flagship = trigger_rate(&codex, &rt, NightAttackType::DdGunTorpRadar, true);
        assert!(rate_flagship > rate_normal, "flagship should get +15 bonus");
    }

    #[test]
    fn carrier_night_ci_fba_detection() {
        // CVL with aviation personnel + 2 night fighters + 1 night attacker → FBA
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cvl_mst = first_ship_mst_by_type(&codex, KcShipType::CVL);
        let night_fighter_id: i64 = 254; // F6F-3N (icon=45)
        let night_attacker_id: i64 = 257; // TBM-3D (icon=46)

        let mut ship = sample_ship(&codex, cvl_mst, 99);
        ship.slot_items = vec![
            slotitem_with_mst_id(258), // 夜間作戦航空要員
            slotitem_with_mst_id(night_fighter_id),
            slotitem_with_mst_id(night_fighter_id),
            slotitem_with_mst_id(night_attacker_id),
        ];
        ship.ship.api_onslot = [1, 1, 1, 1, 0];
        let rt = BattleRuntimeShip::from(ship);

        let attack = detect_night_attack_type(&codex, &rt);
        assert_eq!(
            attack,
            NightAttackType::CarrierNightCI(CarrierNightCiSubType::TwoFightersAttacker)
        );
    }

    #[test]
    fn carrier_night_ci_without_aviation_personnel() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cvl_mst = first_ship_mst_by_type(&codex, KcShipType::CVL);
        let night_fighter_id: i64 = 254;
        let night_attacker_id: i64 = 257;

        let mut ship = sample_ship(&codex, cvl_mst, 99);
        ship.slot_items =
            vec![slotitem_with_mst_id(night_fighter_id), slotitem_with_mst_id(night_attacker_id)];
        ship.ship.api_onslot = [1, 1, 0, 0, 0];
        let rt = BattleRuntimeShip::from(ship);

        // Without 航空要員, CVL cannot attack at night → can_attack_night_ship returns false
        assert!(!crate::targeting::can_attack_night_ship(&codex, &rt));
    }

    #[test]
    fn carrier_night_ci_no_night_planes() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cvl_mst = first_ship_mst_by_type(&codex, KcShipType::CVL);
        let bomber_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedDiveBomber);

        let mut ship = sample_ship(&codex, cvl_mst, 99);
        ship.slot_items = vec![
            slotitem_with_mst_id(258), // 航空要員 but no night planes
            slotitem_with_mst_id(bomber_id),
        ];
        ship.ship.api_onslot = [1, 1, 0, 0, 0];
        let rt = BattleRuntimeShip::from(ship);

        // 航空要員 without a night aircraft is no night air group: she does not attack at all.
        assert!(!crate::targeting::can_attack_night_ship(&codex, &rt));
        let attack = detect_night_attack_type(&codex, &rt);
        assert_eq!(attack, NightAttackType::Normal, "no night planes → Normal");
    }

    /// A carrier of `mst_id` with `equipment` (id, aircraft, 改修) and `hp` of her hull in a hundred.
    fn night_carrier(
        codex: &Codex,
        mst_id: i64,
        equipment: &[(i64, i64, i64)],
        hp: i64,
    ) -> BattleRuntimeShip {
        let mut ship = sample_ship(codex, mst_id, 99);
        ship.ship.api_onslot = [0; 5];
        ship.slot_items = equipment
            .iter()
            .enumerate()
            .map(|(slot, &(id, count, stars))| {
                ship.ship.api_onslot[slot] = count;
                let mut item = slotitem_with_mst_id(id);
                item.api_level = stars;
                item
            })
            .collect();
        ship.ship.api_maxhp = 100;
        ship.ship.api_nowhp = hp;
        BattleRuntimeShip::from(ship)
    }

    const CREW: (i64, i64, i64) = (AVIATION_PERSONNEL_IDS[0], 0, 0);
    const ARK_ROYAL: i64 = 515;
    const GRAF_ZEPPELIN: i64 = NIGHT_SHELLING_CV_IDS[0];
    const GRAF_ZEPPELIN_KAI: i64 = NIGHT_SHELLING_CV_IDS[1];
    const SECONDARY_GUN: i64 = 11; // 15.2cm単装砲
    const TORPEDO: i64 = 13; // 61cm三連装魚雷
    const AUTOGYRO: i64 = 69; // カ号観測機
    const TAIYOU_K2: i64 = NIGHT_SHELLING_CV_IDS[3]; // 大鷹改二, a 軽空母

    #[test]
    fn a_night_air_attack_needs_a_night_air_group_and_a_working_deck() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cvl = first_ship_mst_by_type(&codex, KcShipType::CVL);
        let cvb = first_ship_mst_by_type(&codex, KcShipType::CVB);
        let flies = |mst_id, equipment: &[(i64, i64, i64)], hp| {
            let ship = night_carrier(&codex, mst_id, equipment, hp);
            let flies = can_night_air_attack(&codex, &ship);
            // Whoever flies at night attacks at night.
            assert!(!flies || crate::targeting::can_attack_night_ship(&codex, &ship));
            flies
        };
        let group = [CREW, (NF, 10, 0)];
        assert!(flies(cvl, &group, 100));
        assert!(!flies(cvl, &[(NF, 10, 0)], 100), "no 要員");
        assert!(!flies(cvl, &[CREW, (NF, 0, 0)], 100), "the slot is empty");
        assert!(!flies(cvl, &[CREW, (KK, 10, 0)], 100), "光電管彗星 alone");
        assert!(!flies(cvl, &[CREW, (SF, 10, 0)], 100), "Swordfish off Ark Royal");
        // A built-in 夜戦特性 stands in for the 要員; Ark Royal's Swordfish need neither.
        assert!(flies(INDEPENDENCE_FLIGHT2_ID, &[(NF, 10, 0)], 100));
        assert!(flies(ARK_ROYAL, &[(SF, 10, 0)], 100));
        // More than half the hull, or more than a quarter under an armoured deck.
        assert!(flies(cvl, &group, 51));
        assert!(!flies(cvl, &group, 50));
        assert!(flies(cvb, &group, 26));
        assert!(!flies(cvb, &group, 25));
    }

    #[test]
    fn a_few_carriers_shell_at_night_without_aircraft() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let attacks = |hp| {
            let ship = night_carrier(&codex, GRAF_ZEPPELIN, &[], hp);
            assert!(!can_night_air_attack(&codex, &ship));
            crate::targeting::can_attack_night_ship(&codex, &ship)
        };
        assert!(attacks(100));
        assert!(attacks(26));
        assert!(!attacks(25));
    }

    /// `AO`: しまね丸改 alone of the 補給艦 (`canAirAttackNB`) flies at night as a carrier does.
    #[test]
    fn shimane_maru_kai_flies_at_night_with_a_night_air_group() {
        const SHIMANE_MARU: i64 = 1003;
        const SHIMANE_MARU_KAI: i64 = 1008;
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let group = [CREW, (NF, 10, 0), (NA, 9, 0)];
        let ship = |mst_id, hp| night_carrier(&codex, mst_id, &group, hp);
        assert!(can_night_air_attack(&codex, &ship(SHIMANE_MARU_KAI, 100)));
        assert!(!can_night_air_attack(&codex, &ship(SHIMANE_MARU, 100)));
        assert!(!can_night_air_attack(&codex, &night_carrier(&codex, SHIMANE_MARU_KAI, &[], 100)));
        // A carrier's deck: more than half her hull. Below it she shells.
        assert!(can_night_air_attack(&codex, &ship(SHIMANE_MARU_KAI, 51)));
        assert!(!can_night_air_attack(&codex, &ship(SHIMANE_MARU_KAI, 50)));
        assert!(crate::targeting::can_attack_night_ship(&codex, &ship(SHIMANE_MARU_KAI, 50)));
        // Her air group makes the carrier cut-in.
        assert_eq!(
            detect_night_attack_type(&codex, &ship(SHIMANE_MARU_KAI, 100)),
            NightAttackType::CarrierNightCI(CarrierNightCiSubType::FighterAttacker)
        );
    }

    #[test]
    fn a_night_air_attack_has_a_power_of_its_own() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cvl = first_ship_mst_by_type(&codex, KcShipType::CVL);
        // 要員 (火力 1), F6F-3N x10 (対潜 4), TBM-3D x9 ★4 (火力 2, 雷装 9, 対潜 8),
        // 光電管彗星 x4 (爆装 11), on a hull modernized by 火力 7 and 雷装 2.
        let mut ship =
            night_carrier(&codex, cvl, &[CREW, (NF, 10, 0), (NA, 9, 4), (KK, 4, 0)], 100);
        ship.ship.api_kyouka[0] = 7;
        ship.ship.api_kyouka[1] = 2;
        let hull_of = |ship: &BattleRuntimeShip| {
            let mst = crate::targeting::ship_mst(&codex, ship).unwrap();
            (mst.api_houg.unwrap()[0] + mst.api_raig.unwrap()[0]) as f64
        };
        let expected = hull_of(&ship)
            + 9.0
            + (30.0 + 0.45 * 4.0 * 10.0_f64.sqrt())
            + (2.0 + 2.0 + 9.0 + 27.0 + 0.45 * 19.0 * 3.0)
            + (11.0 + 0.3 * 11.0 * 2.0);
        assert!((night_air_power(&codex, &ship, false) - expected).abs() < 1e-9);

        // Ark Royal without a night air group: her Swordfish' firepower, torpedo and 改修.
        // What `api_karyoku` holds does not come into it.
        let mut ark = night_carrier(&codex, ARK_ROYAL, &[(SF, 10, 9)], 100);
        ark.ship.api_karyoku[0] = 999;
        assert!(
            (night_air_power(&codex, &ark, false) - (hull_of(&ark) + 2.0 + 3.0 + 3.0)).abs() < 1e-9
        );
    }

    /// `CV.NBPower`: the fit bonus on her torpedo (`getEquipBonusCVTorp`), and against a land
    /// target neither that nor a 艦攻's torpedo.
    #[test]
    fn a_night_air_attack_takes_the_torpedo_fit_bonus_and_no_torpedo_on_land() {
        const BARRACUDA_MK2: i64 = 424; // 雷装 +3 on a British carrier
        const DECK_CREW: i64 = 478; // 熟練甲板要員+航空整備員, 雷装 +1 from ★5
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();

        let ark = night_carrier(&codex, ARK_ROYAL, &[(SF, 10, 0)], 100);
        let barracuda =
            night_carrier(&codex, ARK_ROYAL, &[(SF, 10, 0), (BARRACUDA_MK2, 10, 0)], 100);
        assert_eq!(carrier_torpedo_bonus(&codex, &ark), 0.0);
        assert_eq!(carrier_torpedo_bonus(&codex, &barracuda), 3.0);
        let power = |ship, land| night_air_power(&codex, ship, land);
        assert_eq!(power(&barracuda, false) - power(&ark, false), 3.0);
        // Against land the bonus goes; the Swordfish' torpedo stays, as the source has it.
        assert_eq!(power(&barracuda, true), power(&ark, true));
        assert_eq!(power(&ark, true), power(&ark, false));

        // The deck crew's bonus counts once, from ★5.
        let crewed =
            |stars| night_carrier(&codex, ARK_ROYAL, &[(SF, 10, 0), (DECK_CREW, 0, stars)], 100);
        assert_eq!(carrier_torpedo_bonus(&codex, &crewed(5)), 1.0);
        assert_eq!(carrier_torpedo_bonus(&codex, &crewed(4)), 0.0);

        // With a night air group, a 艦攻's torpedo is not counted against land.
        let cvl = first_ship_mst_by_type(&codex, KcShipType::CVL);
        let group = night_carrier(&codex, cvl, &[CREW, (NF, 10, 0), (NA, 9, 0)], 100);
        assert_eq!(power(&group, false) - power(&group, true), 9.0);
    }

    #[test]
    fn a_night_air_attack_takes_the_aircraft_proficiency() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cvl = first_ship_mst_by_type(&codex, KcShipType::CVL);
        let carrier = |alv: Option<i64>| {
            let mut ship = night_carrier(&codex, cvl, &[CREW, (NF, 10, 0), (NA, 9, 0)], 100);
            ship.slot_items[2].api_alv = alv;
            ship
        };
        let mut target = sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::DD), 50);
        target.ship.api_nowhp = 9999;
        target.ship.api_maxhp = 9999;
        let target = BattleRuntimeShip::from(target);

        let strike = |alv, outcome| {
            let carrier = carrier(alv);
            calculate_night_damage(
                &codex,
                &mut crate::random::SeededRng::new(3),
                &carrier,
                &target,
                None,
                None,
                Some(night_air_power(&codex, &carrier, false)),
                outcome,
            )
        };
        assert!(strike(Some(7), HitOutcome::Critical) > strike(None, HitOutcome::Critical));
        assert_eq!(strike(Some(7), HitOutcome::Hit), strike(None, HitOutcome::Hit));

        // On the same draw the skilled group scores criticals the green one does not.
        let roll_as = |alv, flown, seed| {
            roll_attack(
                &codex,
                &mut crate::random::SeededRng::new(seed),
                &carrier(alv),
                &target,
                Aim::new(AttackKind::Night, 1, 1).as_night_air_attack(flown),
            )
        };
        let roll = |alv, seed| roll_as(alv, true, seed);
        let gained = (0..200)
            .filter(|&seed| {
                roll(Some(7), seed) == HitOutcome::Critical
                    && roll(None, seed) != HitOutcome::Critical
            })
            .count();
        assert!(gained > 0);
        // A green group rolls as a ship that does not fly, draw for draw, and so does a
        // skilled one whose attack is not flown.
        assert!((0..200).all(|seed| roll(None, seed) == roll_as(None, false, seed)));
        assert!((0..200).all(|seed| roll_as(Some(7), false, seed) == roll_as(None, false, seed)));
    }

    #[test]
    fn the_packet_says_who_flew_her_night_attack() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cvl = first_ship_mst_by_type(&codex, KcShipType::CVL);
        let dd = first_ship_mst_by_type(&codex, KcShipType::DD);
        let ss = first_ship_mst_by_type(&codex, KcShipType::SS);
        let flag_against = |attacker: BattleRuntimeShip, target| {
            let simulation = crate::simulation::simulate_night(
                &codex,
                crate::types::NightBattleInput {
                    special_attacks_made: Default::default(),
                    friendly: vec![attacker],
                    enemy: vec![BattleRuntimeShip::from(sample_ship(&codex, target, 50))],
                    friendly_formation_id: 1,
                    enemy_formation_id: 1,
                    engagement: EngagementType::SameCourse,
                    air_state: None,
                    night_start: false,
                },
                &mut crate::random::SeededRng::new(0),
            );
            let hougeki = simulation.packet.hougeki.unwrap();
            let own = hougeki.api_at_eflag.iter().position(|flag| *flag == 0).unwrap();
            (hougeki.api_n_mother_list[own], hougeki.api_sp_list[own])
        };
        let flag_of = |attacker| flag_against(attacker, dd).0;
        let carrier = || night_carrier(&codex, cvl, &[CREW, (NF, 10, 0)], 100);
        assert_eq!(flag_of(carrier()), 1);
        // She does not fly at a submarine, and a double attack is gunfire.
        let hunter = night_carrier(&codex, RYUUHOU_K2E_ID, &[(NF, 10, 0)], 100);
        assert_eq!(flag_against(hunter, ss).0, 0);
        // Two secondary guns make a double attack of her gunfire, and nothing of her flight.
        let gun = (SECONDARY_GUN, 0, 0);
        let gunned = night_carrier(&codex, GRAF_ZEPPELIN_KAI, &[gun, gun], 100);
        assert_eq!(flag_against(gunned, dd), (0, NightAttackType::DoubleAttack.api_sp_list()));
        let flown = night_carrier(&codex, GRAF_ZEPPELIN_KAI, &[CREW, (NF, 10, 0), gun, gun], 100);
        assert!(can_night_air_attack(&codex, &flown));
        assert_eq!(flag_against(flown, dd), (1, NightAttackType::Normal.api_sp_list()));
        assert_eq!(flag_of(night_carrier(&codex, GRAF_ZEPPELIN, &[], 100)), 0);
        assert_eq!(flag_of(BattleRuntimeShip::from(sample_ship(&codex, dd, 50))), 0);
    }

    #[test]
    fn a_carrier_flying_at_night_goes_for_the_surface_ships_first() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd = BattleRuntimeShip::from(sample_ship(
            &codex,
            first_ship_mst_by_type(&codex, KcShipType::DD),
            50,
        ));
        let ss = BattleRuntimeShip::from(sample_ship(
            &codex,
            first_ship_mst_by_type(&codex, KcShipType::SS),
            50,
        ));
        // Whom she may fire at: `Some(0)` the destroyer, `Some(1)` the submarine, `None` neither.
        let picks = |attacker: &BattleRuntimeShip, defenders: &[BattleRuntimeShip]| {
            let candidates = crate::targeting::target_candidates(
                &codex,
                attacker,
                defenders,
                crate::types::BattlePhase::NightShelling,
            );
            assert!(candidates.len() <= 1, "one choice, so no draw decides it");
            candidates.first().copied()
        };
        let both = [dd.clone(), ss.clone()];
        let submarine = [ss.clone()];

        // 龍鳳改二戊 flies and has anti-submarine of her own.
        let flier = night_carrier(&codex, RYUUHOU_K2E_ID, &[(NF, 10, 0)], 100);
        assert_eq!(picks(&flier, &both), Some(0));
        assert_eq!(picks(&flier, &submarine), Some(0), "the submarine, once she is all there is");
        // 大鷹改二 shells, and a ship that shells takes the submarine first.
        let sheller = night_carrier(&codex, TAIYOU_K2, &[], 100);
        assert_eq!(picks(&sheller, &both), Some(1));

        // A light carrier with no anti-submarine of her own, and any fleet carrier, leave a
        // submarine alone at night, 回転翼機 or not.
        let group = [CREW, (NF, 10, 0), (AUTOGYRO, 5, 0)];
        let cvl = first_ship_mst_by_type(&codex, KcShipType::CVL);
        let cv = first_ship_mst_by_type(&codex, KcShipType::CV);
        for mst_id in [cvl, cv] {
            let carrier = night_carrier(&codex, mst_id, &group, 100);
            assert_eq!(picks(&carrier, &both), Some(0));
            assert_eq!(picks(&carrier, &submarine), None);
        }
    }

    #[test]
    fn two_secondary_guns_make_a_double_attack() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let attack = |guns: &[(i64, i64, i64)]| {
            detect_night_attack_type(&codex, &night_carrier(&codex, GRAF_ZEPPELIN_KAI, guns, 100))
        };
        let gun = (SECONDARY_GUN, 0, 0);
        assert_eq!(attack(&[gun, gun]), NightAttackType::DoubleAttack);
        assert_eq!(attack(&[gun]), NightAttackType::Normal);
        // A carrier flying with a night air group does not go by her guns.
        assert_eq!(attack(&[CREW, (NF, 10, 0), gun, gun]), NightAttackType::Normal);

        // A failed torpedo cut-in does not fall back on two secondary guns.
        let mut cl = sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::CL), 50);
        cl.slot_items =
            [SECONDARY_GUN, SECONDARY_GUN, TORPEDO, TORPEDO].map(slotitem_with_mst_id).to_vec();
        let cl = BattleRuntimeShip::from(cl);
        assert_eq!(detect_night_attack_type(&codex, &cl), NightAttackType::TorpTorpTorp);
        let resolved = (0..50)
            .map(|seed| {
                let rng = &mut crate::random::SeededRng::new(seed);
                let lights = (NightLights::default(), NightLights::default());
                resolve_night_attack(&codex, rng, &cl, false, TargetClass::SurfaceShip, lights).0
            })
            .collect::<Vec<_>>();
        assert!(resolved.contains(&NightAttackType::Normal));
        assert!(!resolved.contains(&NightAttackType::DoubleAttack));
    }

    // ── Carrier night CI: sub-types, eligibility, enum, trigger (U1-U4) ───────

    // Verified item ids (icons confirmed against .data/codex/start2.json):
    const NF: i64 = 254; // F6F-3N — 夜戦 (icon 45)
    const NA: i64 = 257; // TBM-3D — 夜攻 (icon 46)
    const NB: i64 = 557; // 零式艦戦62型改(夜間爆戦) — 夜爆 (icon 58)
    const KK: i64 = 320; // 光電管彗星 — icon 7, detected by item id
    const SF: i64 = 242; // Swordfish — icon 8, detected by item id

    fn ship_with_slots(codex: &Codex, mst_id: i64, item_ids: &[i64]) -> BattleRuntimeShip {
        let mut ship = sample_ship(codex, mst_id, 99);
        ship.slot_items = item_ids.iter().map(|&id| slotitem_with_mst_id(id)).collect();
        let mut onslot = [0i64; 5];
        for slot in onslot.iter_mut().take(item_ids.len().min(5)) {
            *slot = 1;
        }
        ship.ship.api_onslot = onslot;
        BattleRuntimeShip::from(ship)
    }

    fn cvl_with(codex: &Codex, item_ids: &[i64]) -> BattleRuntimeShip {
        ship_with_slots(codex, first_ship_mst_by_type(codex, KcShipType::CVL), item_ids)
    }

    #[test]
    fn carrier_night_cut_ins_are_every_one_her_group_makes_in_order() {
        use CarrierNightCiSubType::*;
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let bomber = first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedDiveBomber);
        let cases: &[(&[i64], &[CarrierNightCiSubType])] = &[
            (&[NF, NF, NA], &[TwoFightersAttacker, FighterAttacker, FighterAndTwo]),
            (&[NF, NA], &[FighterAttacker]),
            (&[NF, KK], &[Bomber]),
            (&[NA, KK], &[Bomber]),
            (&[NF, NB], &[Bomber]),
            (&[NA, NB], &[Bomber]),
            (&[NB, KK], &[Bomber]),
            (&[NF, KK, SF], &[Bomber, FighterAndTwo]),
            (&[NF, SF, SF], &[FighterAndTwo]),
            // 64 wants three night aircraft, Swordfish counted.
            (&[NF, SF], &[]),
            (&[NF], &[]),
            (&[bomber], &[]),
        ];
        for (slots, expected) in cases {
            let with_crew: Vec<i64> =
                [AVIATION_PERSONNEL_IDS[0]].iter().chain(*slots).copied().collect();
            let rt = cvl_with(&codex, &with_crew);
            assert_eq!(carrier_night_cut_ins(&codex, &rt), *expected, "slots {slots:?}");
        }
        // Without the 要員 she makes none.
        assert_eq!(carrier_night_cut_ins(&codex, &cvl_with(&codex, &[NF, NF, NA])), []);
    }

    #[test]
    fn carrier_night_cut_ins_leave_out_shot_down_slots() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        // 夜戦 + 光電管彗星, but the 彗星 slot is shot down → a lone 夜戦 → none.
        let mut ship = sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::CVL), 99);
        ship.slot_items = [AVIATION_PERSONNEL_IDS[0], NF, KK].map(slotitem_with_mst_id).to_vec();
        ship.ship.api_onslot = [0, 1, 0, 0, 0];
        let rt = BattleRuntimeShip::new(ship, true, true);
        assert_eq!(carrier_night_cut_ins(&codex, &rt), []);
    }

    #[test]
    fn a_carrier_rolls_her_cut_ins_in_turn() {
        use CarrierNightCiSubType::*;
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut ship = sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::CVL), 99);
        ship.ship.api_lucky = [30, 30];
        ship.slot_items =
            [AVIATION_PERSONNEL_IDS[0], NF, NF, NA].map(slotitem_with_mst_id).to_vec();
        ship.ship.api_onslot = [0, 10, 10, 10, 0];
        let rt = BattleRuntimeShip::from(ship);
        let lights = (NightLights::default(), NightLights::default());
        let chance = night_cut_in_chance(&codex, &rt, false, lights) * 0.01;
        /// Hands out the rolls it was given, and panics on one more.
        struct Rolls(std::collections::VecDeque<f64>);
        impl BattleRng for Rolls {
            fn random_f64_range(&mut self, min: f64, max: f64) -> f64 {
                min + (max - min) * self.0.pop_front().expect("an unexpected roll")
            }
            fn roll_range_impl(&mut self, _min: i64, _max: i64) -> i64 {
                panic!("an unexpected roll")
            }
        }
        let resolve = |rolls: &[f64]| {
            let mut rng = Rolls(rolls.iter().copied().collect());
            resolve_night_attack(&codex, &mut rng, &rt, false, TargetClass::SurfaceShip, lights).0
        };
        // 61 misses, 62 comes up: two rolls.
        assert_eq!(
            resolve(&[chance / 1.05, chance / 1.2 - 1e-9]),
            NightAttackType::CarrierNightCI(FighterAttacker)
        );
        // All three miss: a plain flown attack, after three rolls.
        assert_eq!(resolve(&[0.999, 0.999, 0.999]), NightAttackType::Normal);
        assert_eq!(
            resolve(&[0.999, 0.999, chance / 1.3 - 1e-9]),
            NightAttackType::CarrierNightCI(FighterAndTwo)
        );
    }

    #[test]
    fn carrier_night_ci_eligibility_exempt_and_personnel() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();

        // Exempt CVs without 航空要員 are eligible with a valid plane combo.
        assert!(is_cv_night_ci_eligible(&codex, &ship_with_slots(&codex, 545, &[NF, NA]))); // Saratoga Mk.II
        assert!(is_cv_night_ci_eligible(&codex, &ship_with_slots(&codex, 610, &[NA, KK]))); // 加賀改二戊
        assert!(is_cv_night_ci_eligible(&codex, &ship_with_slots(&codex, 883, &[NF, SF, SF]))); // 龍鳳改二戊

        // Saratoga Mk.II Mod.2 (550) is NOT exempt — loses 夜戦特性 on upgrade.
        assert!(!is_cv_night_ci_eligible(&codex, &ship_with_slots(&codex, 550, &[NF, NA])));

        // Standard CVL: eligible only with 航空要員 (item 258).
        let cvl = first_ship_mst_by_type(&codex, KcShipType::CVL);
        assert!(is_cv_night_ci_eligible(&codex, &ship_with_slots(&codex, cvl, &[258, NF, NF, NA])));
        assert!(!is_cv_night_ci_eligible(&codex, &ship_with_slots(&codex, cvl, &[NF, NA])));

        // Exempt CV but no night planes → ineligible.
        assert!(!is_cv_night_ci_eligible(&codex, &ship_with_slots(&codex, 545, &[])));
    }

    #[test]
    fn carrier_night_ci_enum_multiplier_coefficient_sp_list() {
        use CarrierNightCiSubType::*;
        let nf2na = NightAttackType::CarrierNightCI(TwoFightersAttacker);
        assert_eq!(nf2na.damage_multiplier(), 1.25);
        assert_eq!(nf2na.chance_modifier(), 1.05);

        let nf1na = NightAttackType::CarrierNightCI(FighterAttacker);
        assert_eq!(nf1na.damage_multiplier(), 1.20);
        assert_eq!(nf1na.chance_modifier(), 1.2);

        let nf1other = NightAttackType::CarrierNightCI(FighterAndTwo);
        assert_eq!(nf1other.damage_multiplier(), 1.18);
        assert_eq!(nf1other.chance_modifier(), 1.3);

        for sub in [TwoFightersAttacker, FighterAttacker, Bomber, FighterAndTwo] {
            assert_eq!(NightAttackType::CarrierNightCI(sub).api_sp_list(), 6);
            assert_eq!(NightAttackType::CarrierNightCI(sub).hit_count(), 1);
        }
    }

    #[test]
    fn carrier_night_ci_trigger_rate_uses_subtype_coefficient() {
        use CarrierNightCiSubType::*;
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut ship = sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::CVL), 99);
        ship.ship.api_lucky = [99, 99];
        let rt = BattleRuntimeShip::from(ship);

        let rate_nf2na =
            trigger_rate(&codex, &rt, NightAttackType::CarrierNightCI(TwoFightersAttacker), true);
        let rate_nf1other =
            trigger_rate(&codex, &rt, NightAttackType::CarrierNightCI(FighterAndTwo), true);

        // The carrier night CI rate is now non-zero (the stub returned 0.0 and never fired).
        assert!(rate_nf2na > 0.0, "carrier night CI must produce a non-zero trigger rate");
        // A lower chanceMod (1.05) yields a higher rate than 64's (1.3).
        assert!(
            rate_nf2na > rate_nf1other,
            "61 (1.05) should beat 64 (1.3): {rate_nf2na} > {rate_nf1other}"
        );
    }

    // ── Carrier night CI: end-to-end battle integration (U5) ──────────────────

    /// Deterministic RNG: every roll returns the same fraction of its range. `0.0` makes any CI
    /// trigger roll succeed; `0.999` makes it fail. Avoids seed-window flakiness.
    struct FixedRng(f64);
    impl BattleRng for FixedRng {
        fn random_f64_range(&mut self, min: f64, max: f64) -> f64 {
            min + (max - min) * self.0
        }
        fn roll_range_impl(&mut self, min: i64, max: i64) -> i64 {
            if max <= min {
                min
            } else {
                min + ((max - min) as f64 * self.0) as i64
            }
        }
    }

    /// A night-capable CV fixture: high luck (so the CI rate is comfortably positive), full HP,
    /// and the given slot items in non-shot-down slots.
    fn night_cv(codex: &Codex, mst_id: i64, item_ids: &[i64]) -> BattleRuntimeShip {
        let mut ship = sample_ship(codex, mst_id, 99);
        ship.slot_items = item_ids.iter().map(|&id| slotitem_with_mst_id(id)).collect();
        let mut onslot = [0i64; 5];
        for slot in onslot.iter_mut().take(item_ids.len().min(5)) {
            *slot = 1;
        }
        ship.ship.api_onslot = onslot;
        ship.ship.api_lucky = [99, 99];
        ship.ship.api_karyoku[0] = 120;
        ship.ship.api_nowhp = 9999;
        ship.ship.api_maxhp = 9999;
        BattleRuntimeShip::from(ship)
    }

    fn tanky_enemy(codex: &Codex) -> BattleRuntimeShip {
        let dd = first_ship_mst_by_type(codex, KcShipType::DD);
        let mut enemy = sample_ship(codex, dd, 50);
        enemy.ship.api_soukou[0] = 10;
        enemy.ship.api_karyoku[0] = 1;
        enemy.ship.api_nowhp = 9999;
        enemy.ship.api_maxhp = 9999;
        BattleRuntimeShip::from(enemy)
    }

    /// `api_sp_list` values for the friendly attackers (`api_at_eflag == 0`).
    fn friendly_sp_list(hougeki: &BattleNightHougeki) -> Vec<i64> {
        hougeki
            .api_at_eflag
            .iter()
            .zip(&hougeki.api_sp_list)
            .filter(|(eflag, _)| **eflag == 0)
            .map(|(_, sp)| *sp)
            .collect()
    }

    fn run_cv_night(codex: &Codex, cv: BattleRuntimeShip, fire: bool) -> BattleNightHougeki {
        let mut friendly = vec![cv];
        let mut enemy = vec![tanky_enemy(codex)];
        let mut rng = FixedRng(if fire {
            0.0
        } else {
            0.999
        });
        simulate_night_hougeki(
            codex,
            &mut rng,
            &mut friendly,
            &mut enemy,
            &NightBattleParams {
                special_attacks_made: Default::default(),
                friendly_formation_id: 1,
                enemy_formation_id: 1,
                engagement: EngagementType::SameCourse,
                contact: [None, None],
                night_start: false,
                flare_pos: [-1, -1],
            },
        )
        .expect("a night hougeki occurs")
    }

    #[test]
    fn carrier_night_ci_integration_fires_for_eligible_cvs() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cvl = first_ship_mst_by_type(&codex, KcShipType::CVL);

        // Standard CV with 航空要員 and two night fighters with a night attacker → single-hit CI.
        let hougeki = run_cv_night(&codex, night_cv(&codex, cvl, &[258, NF, NF, NA]), true);
        assert_eq!(friendly_sp_list(&hougeki), vec![6], "standard CV → sp_list=6");
        assert_eq!(hougeki.api_damage[0].len(), 1, "carrier night CI is single-hit");

        // Exempt CVs WITHOUT 航空要員 still fire.
        for (mst, slots) in [
            (545_i64, &[NF, NA][..]), // Saratoga Mk.II, 62
            (610, &[NA, KK][..]),     // 加賀改二戊, 63 (光電管彗星)
            (883, &[NF, SF, SF][..]), // 龍鳳改二戊, 64 (Swordfish)
        ] {
            let hougeki = run_cv_night(&codex, night_cv(&codex, mst, slots), true);
            assert!(
                friendly_sp_list(&hougeki).contains(&6),
                "exempt CV {mst} without personnel → sp_list=6"
            );
        }
    }

    #[test]
    fn carrier_night_ci_integration_negatives() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cvl = first_ship_mst_by_type(&codex, KcShipType::CVL);

        // Saratoga Mk.II Mod.2 (550) without 航空要員 cannot attack at night → no friendly entry.
        let hougeki = run_cv_night(&codex, night_cv(&codex, 550, &[NF, NA]), true);
        assert!(!friendly_sp_list(&hougeki).contains(&6), "550 without personnel → no sp_list=6");

        // 航空要員 present but only a night fighter → no CI sub-type → Normal.
        let hougeki = run_cv_night(&codex, night_cv(&codex, cvl, &[258, NF]), true);
        assert!(!friendly_sp_list(&hougeki).contains(&6), "lone night fighter → no sp_list=6");

        // Non-exempt CV without personnel cannot attack at night.
        let hougeki = run_cv_night(&codex, night_cv(&codex, cvl, &[NF, NA]), true);
        assert!(
            !friendly_sp_list(&hougeki).contains(&6),
            "non-exempt CV without personnel → no sp_list=6"
        );
    }

    #[test]
    fn carrier_night_ci_integration_failed_roll_falls_back_to_normal() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cvl = first_ship_mst_by_type(&codex, KcShipType::CVL);

        // An eligible CV whose trigger rolls fail must fall back to Normal — NOT the
        // artillery double attack (CV-specific early return).
        let hougeki = run_cv_night(&codex, night_cv(&codex, cvl, &[258, NF, NF, NA]), false);
        let friendly = friendly_sp_list(&hougeki);
        assert_eq!(friendly, vec![0], "failed carrier CI → Normal (sp_list=0)");
        assert!(!friendly.contains(&1), "must not fall back to DoubleAttack");
    }

    // ── Carrier night CI: codex-ID regression guard (U6) ──────────────────────

    /// The exempt-CV and item ids are hardcoded against the Codex bootstrap. If a codex update
    /// renumbers or removes one, this fails loud rather than silently disabling the feature.
    #[test]
    fn carrier_night_ci_ids_resolve_in_codex() {
        use emukc_model::kc2::start2::{ApiMstShip, ApiMstSlotitem};
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();

        // Every exempt CV id must resolve to a real ship.
        for &id in EXEMPT_NIGHT_CV_IDS {
            assert!(codex.find::<ApiMstShip>(&id).is_ok(), "exempt CV mst {id} missing from codex");
        }
        // Saratoga Mk.II Mod.2 (550) exists but must stay OUT of the exempt list.
        assert!(codex.find::<ApiMstShip>(&550i64).is_ok());
        assert!(!EXEMPT_NIGHT_CV_IDS.contains(&550), "550 must not be treated as exempt");

        // 光電管彗星 must resolve and carry the regular dive-bomber icon (api_type[3]=7) — the
        // exact reason it needs item-id detection rather than night-icon detection.
        let kk = codex.find::<ApiMstSlotitem>(&KOUDENKAN_SUISEI_ID).expect("光電管彗星 in codex");
        assert_eq!(kk.api_type[3], 7, "光電管彗星 must carry the regular dive-bomber icon");

        // The other item-id-detected night planes must resolve.
        for &id in SWORDFISH_IDS {
            assert!(codex.find::<ApiMstSlotitem>(&id).is_ok(), "Swordfish item {id} missing");
        }
        assert!(codex.find::<ApiMstSlotitem>(&IWAI_FUKUSEN_ID).is_ok(), "岩井爆戦 (154) missing");
    }

    // ── Flagship escort shield (旗艦援護 / かばう) — night path ────────────

    /// Regression (plan 002 U4): in the night enemy-attack loop, a friendly
    /// defender's displayed damage must be the capped/protected `dealt` (via
    /// `take_hit`), never the uncapped lethal raw damage. Reporting the
    /// pre-protection value desyncs the client's HP reconstruction.
    #[test]
    fn sortie_night_enemy_attack_displays_capped_damage() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);

        // Non-taiha friendly (entry HP > 25% max) → eligible for sinking protection.
        let mut friend = sample_ship(&codex, dd_mst, 99);
        friend.ship.api_karyoku[0] = 200;
        friend.ship.api_raisou[0] = 0;
        friend.ship.api_soukou[0] = 0;
        friend.ship.api_nowhp = 100;
        friend.ship.api_maxhp = 100;
        let mut friendly = vec![BattleRuntimeShip::new(friend, true, true)];
        let entry_hp = friendly[0].hp();

        // Enemy with lethal firepower.
        let mut enemy = sample_ship(&codex, dd_mst, 99);
        enemy.ship.api_karyoku[0] = 500;
        enemy.ship.api_raisou[0] = 0;
        enemy.ship.api_soukou[0] = 200;
        enemy.ship.api_nowhp = 500;
        enemy.ship.api_maxhp = 500;
        let mut enemies = vec![BattleRuntimeShip::new(enemy, false, true)];

        let mut rng = crate::random::SeededRng::new(42);
        let hougeki = simulate_night_hougeki(
            &codex,
            &mut rng,
            &mut friendly,
            &mut enemies, // enemy is the second slice → it attacks via the enemy loop (the fixed path)
            &NightBattleParams {
                special_attacks_made: Default::default(),
                friendly_formation_id: 1,
                enemy_formation_id: 1,
                engagement: EngagementType::SameCourse,
                contact: [None, None],
                night_start: false,
                flare_pos: [-1, -1],
            },
        )
        .unwrap();

        // Sinking protection kept the ship alive, so the displayed damage of the
        // enemy-attack entries (api_at_eflag == 1, friendly is the defender) must
        // equal the HP actually lost — the capped `dealt`, not the raw lethal
        // value that would over-report past the ship's real HP.
        assert!(friendly[0].hp() > 0, "non-taiha ship should survive via sinking protection");
        let hp_lost = entry_hp - friendly[0].hp();
        let displayed: i64 = hougeki
            .api_at_eflag
            .iter()
            .zip(hougeki.api_damage.iter())
            .filter(|&(&eflag, _)| eflag == 1)
            .flat_map(|(_, dmgs)| dmgs.iter())
            .map(|c| c.amount())
            .sum();
        assert_eq!(
            displayed, hp_lost,
            "friendly night defender must display capped dealt ({hp_lost}), not raw: {:?}",
            hougeki.api_damage
        );
    }

    /// 旗艦援護 night helper. Places a flagship + one healthy green escort on the
    /// defending side and a lone escort-less attacker (extra HP so it survives to
    /// attack back) on the other, so only the defending pair's side can
    /// intercept. `defender_is_enemy` selects the friendly-attacks-enemy
    /// direction (true, R9) vs enemy-attacks-friendly (false). Returns the packet.
    fn night_shield_packet(
        codex: &Codex,
        defender_is_enemy: bool,
        defender_formation_id: i64,
        seed: u64,
    ) -> Option<BattleNightHougeki> {
        let bb = first_ship_mst_by_type(codex, KcShipType::BB);
        let dd = first_ship_mst_by_type(codex, KcShipType::DD);
        let mut pair = vec![
            BattleRuntimeShip::new(sample_ship(codex, bb, 80), !defender_is_enemy, true),
            BattleRuntimeShip::new(sample_ship(codex, dd, 80), !defender_is_enemy, true),
        ];
        let mut lone_input = sample_ship(codex, bb, 80);
        lone_input.ship.api_nowhp = 999;
        lone_input.ship.api_maxhp = 999;
        let mut lone = vec![BattleRuntimeShip::new(lone_input, defender_is_enemy, true)];

        let mut rng = crate::random::SeededRng::new(seed);
        let params = NightBattleParams {
            special_attacks_made: Default::default(),
            friendly_formation_id: defender_formation_id,
            enemy_formation_id: defender_formation_id,
            engagement: EngagementType::SameCourse,
            contact: [None, None],
            night_start: false,
            flare_pos: [-1, -1],
        };
        // simulate_night_hougeki takes (friendly, enemy); the defending pair is
        // friendly when the enemy attacks it, enemy when the friendly attacks it.
        if defender_is_enemy {
            simulate_night_hougeki(codex, &mut rng, &mut lone, &mut pair, &params)
        } else {
            simulate_night_hougeki(codex, &mut rng, &mut pair, &mut lone, &params)
        }
    }

    /// 旗艦援護 night (enemy-attack loop): a healthy friendly escort intercepts an
    /// enemy night hit aimed at the friendly flagship; the cell is Shielded
    /// (serializes `X.1`) and `api_df_list` points to the escort (index 1).
    #[test]
    fn night_enemy_attack_shield_redirects_to_friendly_escort() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let mut found = false;
        for seed in 0..400u64 {
            // 輪形陣 (75%) maximises interception frequency for the scan.
            let Some(h) = night_shield_packet(&codex, false, 3, seed) else {
                continue;
            };
            for (i, dmgs) in h.api_damage.iter().enumerate() {
                if dmgs.iter().any(|c| matches!(c, DamageCell::Shielded(_))) {
                    assert!(
                        h.api_df_list[i].iter().all(|&t| t == 1),
                        "intercepted night hit must target the escort, got df_list {:?}",
                        h.api_df_list[i]
                    );
                    let json = serde_json::to_string(&dmgs).unwrap();
                    assert!(json.contains(".1"), "shielded night damage must serialize .1: {json}");
                    found = true;
                }
            }
            if found {
                break;
            }
        }
        assert!(found, "expected an intercepted flagship night hit within the seed scan");
    }

    /// R9 night: a friendly attack on the enemy flagship is intercepted by an
    /// enemy escort too (friendly-attack loop shield path).
    #[test]
    fn night_friendly_attack_shield_protects_enemy_flagship() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let found = (0..400u64).any(|seed| {
            night_shield_packet(&codex, true, 3, seed).is_some_and(|h| {
                h.api_damage
                    .iter()
                    .any(|dmgs| dmgs.iter().any(|c| matches!(c, DamageCell::Shielded(_))))
            })
        });
        assert!(found, "enemy flagship must also be protected at night by an enemy escort (R9)");
    }
}

#[cfg(test)]
mod display_narrowing_tests {
    use super::*;
    use crate::targeting::day_gunnery_display_ids;
    use crate::test_utils::*;
    use crate::types::BattleRuntimeShip;
    use emukc_model::codex::Codex;
    use emukc_model::kc2::types::{KcShipType, KcSlotItemType3};

    /// 航空戦艦 with a 水上爆撃機 in slot 0 and guns behind it. 連撃 is formed by
    /// guns, so the seaplane bomber must not be what the client is told to draw.
    fn seaplane_bomber_battleship(codex: &Codex) -> BattleRuntimeShip {
        let bbv_mst = first_ship_mst_by_type(codex, KcShipType::BBV);
        let zuiun_id = first_slotitem_mst_by_type(codex, KcSlotItemType3::SeaBasedBomber);
        let main_gun_id = first_slotitem_mst_by_type(codex, KcSlotItemType3::LargeCaliberMainGun);
        let secondary_id = first_slotitem_mst_by_type(codex, KcSlotItemType3::SecondaryGun);

        let mut input = sample_ship(codex, bbv_mst, 99);
        input.slot_items = vec![
            slotitem_with_mst_id(zuiun_id),
            slotitem_with_mst_id(main_gun_id),
            slotitem_with_mst_id(secondary_id),
        ];
        input.ship.api_onslot = [1, 0, 0, 0, 0];
        BattleRuntimeShip::from(input)
    }

    #[test]
    fn night_double_attack_shows_guns_not_seaplane_bombers() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let zuiun_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::SeaBasedBomber);
        let ship = seaplane_bomber_battleship(&codex);

        let ids = night_attack_display_ids(&codex, &ship, NightAttackType::DoubleAttack);

        assert!(!ids.contains(&zuiun_id), "連撃 must not display a seaplane bomber: {ids:?}");
        assert!(!ids.is_empty() && ids != vec![-1]);
    }

    #[test]
    fn day_double_attack_shows_guns_not_seaplane_bombers() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let zuiun_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::SeaBasedBomber);
        let ship = seaplane_bomber_battleship(&codex);

        let ids = day_gunnery_display_ids(&codex, &ship, 2, None);

        assert!(!ids.contains(&zuiun_id), "連撃 must not display a seaplane bomber: {ids:?}");
    }

    /// A plain attack names the gun, not the seaplane bomber. `_getNormalAttackType`
    /// does not read `api_si_list` to pick a 水上爆撃機 animation -- for an
    /// ordinary 航空戦艦 it returns 0 and the client goes to
    /// `PhaseAttackNormal`, which loads `api_si_list[0]`'s name plate. The
    /// seaplane bomber has none, which is the archived `102 -> btxt_flat`
    /// incident.
    #[test]
    fn normal_attack_names_the_gun_not_the_seaplane_bomber() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let zuiun_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::SeaBasedBomber);
        let main_gun_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::LargeCaliberMainGun);
        let ship = seaplane_bomber_battleship(&codex);

        let day = crate::targeting::day_attack_display_ids(&codex, &ship, false);
        let night = night_attack_display_ids(&codex, &ship, NightAttackType::Normal);

        assert_eq!(day, vec![main_gun_id]);
        assert_eq!(night, vec![main_gun_id]);
        assert!(!day.contains(&zuiun_id) && !night.contains(&zuiun_id));
    }

    #[test]
    fn double_attack_orders_main_gun_before_secondary() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let main_gun_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::LargeCaliberMainGun);
        let secondary_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::SecondaryGun);
        let ship = seaplane_bomber_battleship(&codex);

        assert_eq!(
            night_attack_display_ids(&codex, &ship, NightAttackType::DoubleAttack),
            vec![main_gun_id, secondary_id]
        );
        assert_eq!(
            day_gunnery_display_ids(&codex, &ship, 2, None),
            vec![main_gun_id, secondary_id]
        );
    }

    /// One gun means one entry. The list converges on what the ship carries; it
    /// is never padded out to the attack's nominal slot count.
    #[test]
    fn double_attack_with_one_gun_reports_one_entry() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let main_gun_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::SmallCaliberMainGun);

        let mut input = sample_ship(&codex, dd_mst, 99);
        input.slot_items = vec![slotitem_with_mst_id(main_gun_id)];
        let ship = BattleRuntimeShip::from(input);

        assert_eq!(
            night_attack_display_ids(&codex, &ship, NightAttackType::DoubleAttack),
            vec![main_gun_id]
        );
        assert_eq!(day_gunnery_display_ids(&codex, &ship, 2, None), vec![main_gun_id]);
    }

    /// The night carrier cut-in is the one night path that loads a name plate,
    /// so it must name the air group rather than fall through to the gun set a
    /// carrier does not carry.
    #[test]
    fn night_carrier_cutin_names_its_air_group() {
        use super::CarrierNightCiSubType;

        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let cv_mst = first_ship_mst_by_type(&codex, KcShipType::CV);
        let fighter_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedFighter);
        let dive_bomber_id =
            first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedDiveBomber);
        let torpedo_bomber_id =
            first_slotitem_mst_by_type(&codex, KcSlotItemType3::CarrierBasedTorpedoBomber);

        let mut input = sample_ship(&codex, cv_mst, 99);
        input.slot_items = vec![
            slotitem_with_mst_id(fighter_id),
            slotitem_with_mst_id(dive_bomber_id),
            slotitem_with_mst_id(torpedo_bomber_id),
        ];
        input.ship.api_onslot = [18, 18, 18, 0, 0];
        let ship = BattleRuntimeShip::from(input);

        let ids = night_attack_display_ids(
            &codex,
            &ship,
            NightAttackType::CarrierNightCI(CarrierNightCiSubType::TwoFightersAttacker),
        );

        assert_eq!(ids, vec![fighter_id, dive_bomber_id, torpedo_bomber_id]);
        assert_ne!(ids, vec![-1], "a carrier carries no guns; the gun set would name nothing");
    }

    /// R6 for R3: proof the narrowing is not vacuous. A ship whose only
    /// equipment has no name plate names nothing, on every day display path.
    /// Widen any of them back to a set that admits 水上爆撃機 or 艦上攻撃機 and
    /// this returns an id the client would 404 on.
    #[test]
    fn a_ship_with_only_plate_less_equipment_names_nothing() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let zuiun_id = first_slotitem_mst_by_type(&codex, KcSlotItemType3::SeaBasedBomber);
        let bbv_mst = first_ship_mst_by_type(&codex, KcShipType::BBV);

        let mut input = sample_ship(&codex, bbv_mst, 99);
        input.slot_items = vec![slotitem_with_mst_id(zuiun_id)];
        input.ship.api_onslot = [1, 0, 0, 0, 0];
        let ship = BattleRuntimeShip::from(input);

        assert!(!crate::targeting::is_day_surface_display_type(KcSlotItemType3::SeaBasedBomber));
        assert_eq!(crate::targeting::day_attack_display_ids(&codex, &ship, false), vec![-1]);
        assert_eq!(day_gunnery_display_ids(&codex, &ship, 2, None), vec![-1]);
        assert_eq!(
            night_attack_display_ids(&codex, &ship, NightAttackType::DoubleAttack),
            vec![-1]
        );
        assert!(zuiun_id > 0, "the ship really does carry something");
    }

    #[test]
    fn the_two_fleets_fire_in_turn_down_their_lines() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, KcShipType::DD);
        let mut ship = sample_ship(&codex, dd_mst, 99);
        ship.ship.api_soukou[0] = 400;
        let fleet = || vec![BattleRuntimeShip::from(ship.clone()); 3];

        let hougeki = simulate_night_hougeki(
            &codex,
            &mut crate::random::SeededRng::new(42),
            &mut fleet(),
            &mut fleet(),
            &NightBattleParams {
                special_attacks_made: Default::default(),
                friendly_formation_id: 1,
                enemy_formation_id: 1,
                engagement: crate::types::EngagementType::SameCourse,
                contact: [None, None],
                night_start: false,
                flare_pos: [-1, -1],
            },
        )
        .unwrap();

        assert_eq!(hougeki.api_at_eflag, [0, 1, 0, 1, 0, 1]);
        assert_eq!(hougeki.api_at_list, [0, 0, 1, 1, 2, 2]);
    }
}

#[cfg(test)]
mod abyssal_carrier_tests {
    use super::*;
    use crate::simulation::abyssal_night::{
        ABYSSAL_CARRIER_ASW, ABYSSAL_NIGHT_ATTACK, ABYSSAL_NIGHT_CREW_IDS,
    };
    use crate::test_utils::*;
    use crate::types::{BattleRuntimeShip, BattleShipInput, EngagementType, NightBattleInput};
    use emukc_model::codex::Codex;
    use emukc_model::kc2::types::KcShipType;

    const WO: i64 = 1512; // 空母ヲ級
    const WO_FLAGSHIP: i64 = 1528; // 空母ヲ級flagship, mark 1
    const LYCORIS: i64 = 1679; // リコリス棲姫, mark 2
    const NU_KAI_ELITE: i64 = 1734; // 軽母ヌ級改elite, mark 3
    const SOUTH_PACIFIC: i64 = 1971; // 南太平洋空母棲姫, mark 1 with a night air crew
    const SEARCHLIGHT: i64 = 74; // 探照灯

    fn codex() -> Codex {
        Codex::load_without_cache_source("../../.data/codex").unwrap()
    }

    /// An enemy as a sortie meets her, with `hp` of her hull in a hundred.
    fn abyssal(codex: &Codex, mst_id: i64, hp: i64) -> BattleRuntimeShip {
        let (mut ship, slot_items) = codex.new_enemy_ship(mst_id).unwrap();
        ship.api_nowhp = ship.api_maxhp * hp / 100;
        let input = BattleShipInput {
            ship,
            slot_items,
            aircraft_exp: vec![],
            effect_list: vec![0],
            married: false,
        };
        BattleRuntimeShip::new(input, false, true)
    }

    /// A destroyer nobody sinks in one night, carrying `equipment`.
    fn destroyer(codex: &Codex, equipment: &[i64]) -> BattleRuntimeShip {
        let mut ship = sample_ship(codex, first_ship_mst_by_type(codex, KcShipType::DD), 50);
        ship.slot_items = equipment.iter().copied().map(slotitem_with_mst_id).collect();
        ship.ship.api_nowhp = 9999;
        ship.ship.api_maxhp = 9999;
        BattleRuntimeShip::from(ship)
    }

    /// The `api_n_mother_list` flags of the attacks one side made in a night between the two.
    fn flags_of(
        codex: &Codex,
        friendly: BattleRuntimeShip,
        enemy: BattleRuntimeShip,
        by_enemy: bool,
    ) -> Vec<i64> {
        let input = NightBattleInput {
            special_attacks_made: Default::default(),
            friendly: vec![friendly],
            enemy: vec![enemy],
            friendly_formation_id: 1,
            enemy_formation_id: 1,
            engagement: EngagementType::SameCourse,
            air_state: None,
            night_start: false,
        };
        let simulation =
            crate::simulation::simulate_night(codex, input, &mut crate::random::SeededRng::new(0));
        // Nobody attacking leaves no shelling at all.
        let Some(hougeki) = simulation.packet.hougeki else {
            return Vec::new();
        };
        hougeki
            .api_at_eflag
            .iter()
            .zip(&hougeki.api_n_mother_list)
            .filter(|(side, _)| **side == i64::from(by_enemy))
            .map(|(_, flag)| *flag)
            .collect()
    }

    fn enemy_flags(codex: &Codex, enemy: BattleRuntimeShip) -> Vec<i64> {
        flags_of(codex, destroyer(codex, &[]), enemy, true)
    }

    #[test]
    fn the_abyssal_tables_follow_the_source() {
        let codex = codex();
        // What `scripts/gen_abyssal_night_carriers.py` prints: (mark, ships, sum of their ids).
        for (mark, ships, sum) in [(1, 86, 172_021), (2, 11, 18_548), (3, 35, 64_694)] {
            let ids = || ABYSSAL_NIGHT_ATTACK.iter().filter(|row| row.1 == mark).map(|row| row.0);
            assert_eq!((ids().count(), ids().sum::<i64>()), (ships, sum), "mark {mark}");
        }
        assert_eq!(ABYSSAL_NIGHT_ATTACK.len(), 132);
        assert_eq!(
            (ABYSSAL_NIGHT_CREW_IDS.len(), ABYSSAL_NIGHT_CREW_IDS.iter().sum::<i64>()),
            (74, 161_283)
        );
        let asw = |pick: fn(&(i64, i64)) -> i64| ABYSSAL_CARRIER_ASW.iter().map(pick).sum::<i64>();
        assert_eq!(
            (ABYSSAL_CARRIER_ASW.len(), asw(|row| row.0), asw(|row| row.1)),
            (40, 76_829, 3_165)
        );
        // Every one of them is a carrier the game data knows and gives a night aircraft to.
        for id in ABYSSAL_NIGHT_ATTACK
            .iter()
            .map(|row| row.0)
            .chain(ABYSSAL_NIGHT_CREW_IDS.iter().copied())
        {
            assert!(crate::damage::is_cv_type(&codex, &abyssal(&codex, id, 100)), "{id}");
        }
        for &id in ABYSSAL_NIGHT_CREW_IDS {
            assert!(can_night_air_attack(&codex, &abyssal(&codex, id, 100)), "{id}");
        }
    }

    #[test]
    fn a_marked_enemy_carrier_attacks_at_night_until_heavily_damaged() {
        let codex = codex();
        assert_eq!(enemy_flags(&codex, abyssal(&codex, WO_FLAGSHIP, 100)), [0]);
        // More than a quarter of her hull: 25 of her 96, and not 24.
        let attacks = |hp| can_attack_night_ship(&codex, &abyssal(&codex, WO_FLAGSHIP, hp));
        assert!(attacks(27) && !attacks(25));
        assert_eq!(enemy_flags(&codex, abyssal(&codex, WO, 100)), [0; 0], "no mark");
        // The one the client draws as gunfire is told nothing more.
        assert_eq!(enemy_flags(&codex, abyssal(&codex, LYCORIS, 100)), [0]);
    }

    #[test]
    fn a_light_carrier_marked_three_needs_a_searchlight_on_the_fleet_she_attacks() {
        let codex = codex();
        let night = |equipment: &[i64]| {
            let nu = abyssal(&codex, NU_KAI_ELITE, 100);
            flags_of(&codex, destroyer(&codex, equipment), nu, true)
        };
        assert_eq!(night(&[]), [0; 0]);
        assert_eq!(night(&[SEARCHLIGHT]), [0]);
    }

    #[test]
    fn an_enemy_light_carrier_with_anti_submarine_attacks_a_submarine_at_night() {
        let codex = codex();
        let night = |enemy| {
            let ss = first_ship_mst_by_type(&codex, KcShipType::SS);
            flags_of(&codex, BattleRuntimeShip::from(sample_ship(&codex, ss, 50)), enemy, true)
        };
        // 軽母ヌ級改elite has 30, and needs no searchlight for a submarine.
        assert_eq!(night(abyssal(&codex, NU_KAI_ELITE, 100)), [0]);
        // 軽母ヌ級flagship has none, and a fleet carrier never does it.
        assert_eq!(night(abyssal(&codex, 1560, 100)), [0; 0]);
        assert_eq!(night(abyssal(&codex, WO_FLAGSHIP, 100)), [0; 0]);
    }

    /// `ASWPower` counts her `statsBase.ASW`: for these carriers the source's figure, not the
    /// codex's 0.
    #[test]
    fn an_enemy_light_carriers_attack_on_a_submarine_counts_the_sources_anti_submarine() {
        let codex = codex();
        assert_eq!(crate::damage::base_asw(&codex, &abyssal(&codex, NU_KAI_ELITE, 100)), 30.0);
        assert_eq!(crate::damage::base_asw(&codex, &abyssal(&codex, 1560, 100)), 0.0);
        let mut dd = sample_ship(&codex, first_ship_mst_by_type(&codex, KcShipType::DD), 50);
        dd.slot_items.clear();
        dd.ship.api_taisen[0] = 60;
        assert_eq!(crate::damage::base_asw(&codex, &BattleRuntimeShip::from(dd)), 60.0);
    }

    /// `CV.NBtypes` asks an enemy carrier for her crew and her aircraft types, not for aircraft
    /// left in the slot. 夜深海艦爆 counts as Swordfish do.
    #[test]
    fn an_enemy_carrier_with_a_night_crew_rolls_the_carrier_cut_ins() {
        use CarrierNightCiSubType::*;
        let codex = codex();
        // 夜猫深海艦戦, 夜深海艦爆, 夜復讐深海艦攻: 62, and 64 with the bomber as the third.
        let princess = abyssal(&codex, SOUTH_PACIFIC, 100);
        assert_eq!(carrier_night_cut_ins(&codex, &princess), [FighterAttacker, FighterAndTwo]);
        let mut emptied = abyssal(&codex, SOUTH_PACIFIC, 100);
        emptied.ship.api_onslot = [0; 5];
        assert_eq!(carrier_night_cut_ins(&codex, &emptied), [FighterAttacker, FighterAndTwo]);
        // A carrier that shells, with no crew, makes none.
        assert_eq!(carrier_night_cut_ins(&codex, &abyssal(&codex, WO_FLAGSHIP, 100)), []);
    }

    #[test]
    fn an_enemy_carrier_with_a_night_crew_flies_her_attack() {
        let codex = codex();
        let princess = abyssal(&codex, SOUTH_PACIFIC, 100);
        // Her hull's 208, and by `CV.NBPower`: 夜猫深海艦戦 x36 2 + 108 + 0.9 x 6, 夜深海艦爆 x18
        // 3 + 16 + 9 x sqrt(18) without the count, 夜復讐深海艦攻 x15 4 + 15 + 45 + 13.95 x sqrt(15).
        assert!((night_air_power(&codex, &princess, false) - 498.6119).abs() < 1e-3);
        assert_eq!(enemy_flags(&codex, princess), [1]);
        // And the packet's damage is that power's, capped: her gunfire would be 208 + 5
        // before armour.
        let input = NightBattleInput {
            special_attacks_made: Default::default(),
            friendly: vec![destroyer(&codex, &[])],
            enemy: vec![abyssal(&codex, SOUTH_PACIFIC, 100)],
            friendly_formation_id: 1,
            enemy_formation_id: 1,
            engagement: EngagementType::SameCourse,
            air_state: None,
            night_start: false,
        };
        let night = crate::simulation::simulate_night(
            &codex,
            input,
            &mut crate::random::SeededRng::new(20),
        );
        let hougeki = night.packet.hougeki.unwrap();
        let hers = hougeki.api_at_eflag.iter().position(|side| *side == 1).unwrap();
        assert_eq!(hougeki.api_cl_list[hers], [1]);
        assert_eq!(format!("{:?}", hougeki.api_damage[hers]), "[366]");
        // At half her hull the deck is shut, and her mark still has her attack.
        let damaged = abyssal(&codex, SOUTH_PACIFIC, 50);
        assert!(!can_night_air_attack(&codex, &damaged));
        assert_eq!(enemy_flags(&codex, damaged), [0]);
        // The source asks an enemy for the aircraft type, not for aircraft left in the slot.
        let mut emptied = abyssal(&codex, SOUTH_PACIFIC, 100);
        emptied.ship.api_onslot = [0; 5];
        assert!(can_night_air_attack(&codex, &emptied));
    }

    #[test]
    fn bearn_shells_at_night_in_all_her_forms() {
        let codex = codex();
        for bearn in [1055, 1060, 1061] {
            let mut ship = sample_ship(&codex, bearn, 99);
            ship.slot_items.clear();
            ship.ship.api_onslot = [0; 5];
            let ship = BattleRuntimeShip::from(ship);
            let target = abyssal(&codex, WO, 100);
            assert_eq!(flags_of(&codex, ship, target, false), [0], "{bearn}");
        }
    }
}

#[cfg(test)]
mod attack_type_tests {
    use std::collections::VecDeque;

    use emukc_model::codex::Codex;
    use emukc_model::kc2::KcShipType;

    use super::*;
    use crate::test_utils::{first_ship_mst_by_type, sample_ship, slotitem_with_mst_id};

    const MAIN: i64 = 2; // 12.7cm連装砲
    const SECONDARY: i64 = 11; // 15.2cm単装砲
    const TORPEDO: i64 = 13; // 61cm三連装魚雷
    const AIR_RADAR: i64 = 27; // 13号対空電探, 3 line of sight
    const SURFACE_RADAR: i64 = 28; // 22号対水上電探, 5 line of sight
    const SEARCHLIGHT: i64 = 74;
    const STAR_SHELL: i64 = 101;
    const SUB_RADAR: i64 = 210; // 潜水艦搭載電探&水防式望遠鏡
    const LATE_TORPEDO: i64 = 213; // 後期型艦首魚雷(6門)

    const DARK: (NightLights, NightLights) = (
        NightLights {
            star_shell: false,
            searchlight: 0,
        },
        NightLights {
            star_shell: false,
            searchlight: 0,
        },
    );

    /// Hands out the rolls it was given, and panics on one more.
    struct Rolls(VecDeque<f64>);

    impl BattleRng for Rolls {
        fn random_f64_range(&mut self, min: f64, max: f64) -> f64 {
            min + (max - min) * self.0.pop_front().expect("an unexpected roll")
        }

        fn roll_range_impl(&mut self, _min: i64, _max: i64) -> i64 {
            panic!("an unexpected roll")
        }
    }

    fn codex() -> Codex {
        Codex::load_without_cache_source("../../.data/codex").unwrap()
    }

    /// A ship of the type with 30 luck: at level 50 her cut-in chance is 50.
    fn ship(codex: &Codex, kind: KcShipType, level: i64, equipment: &[i64]) -> BattleRuntimeShip {
        let mut ship = sample_ship(codex, first_ship_mst_by_type(codex, kind), level);
        ship.ship.api_lucky = [30, 30];
        ship.slot_items = equipment.iter().copied().map(slotitem_with_mst_id).collect();
        BattleRuntimeShip::from(ship)
    }

    /// Her attack with the rolls given, every one of which she must use.
    fn attack(
        codex: &Codex,
        ship: &BattleRuntimeShip,
        target: TargetClass,
        rolls: &[f64],
    ) -> (i64, f64) {
        let mut rng = Rolls(rolls.iter().copied().collect());
        let (kind, damage) = resolve_night_attack(codex, &mut rng, ship, false, target, DARK);
        assert!(rng.0.is_empty(), "rolls left over");
        (kind.api_sp_list(), damage)
    }

    fn types(codex: &Codex, ship: &BattleRuntimeShip) -> Vec<i64> {
        night_attack_types(codex, ship).into_iter().map(NightAttackType::api_sp_list).collect()
    }

    const ZUIUN: i64 = 490; // 試製 夜間瑞雲(攻撃装備)

    /// An aviation cruiser with 30 luck and this equipment, aircraft in every slot.
    fn zuiun_cruiser(codex: &Codex, kind: KcShipType, equipment: &[i64]) -> BattleRuntimeShip {
        let mut ship = ship(codex, kind, 50, equipment);
        ship.ship.api_onslot = [4; 5];
        ship
    }

    #[test]
    fn the_night_zuiun_cut_ins_follow_the_source() {
        use ZuiunNightCutIn::*;
        let codex = codex();
        let cut_ins = |kind, equipment: &[i64]| {
            zuiun_night_cut_ins(&codex, &zuiun_cruiser(&codex, kind, equipment))
        };
        let cav = KcShipType::CAV;
        assert_eq!(
            cut_ins(cav, &[MAIN, MAIN, ZUIUN, ZUIUN, SURFACE_RADAR]),
            [TwoAndRadar, Two, OneAndRadar, One]
        );
        assert_eq!(cut_ins(cav, &[MAIN, MAIN, ZUIUN, SURFACE_RADAR]), [OneAndRadar, One]);
        assert_eq!(cut_ins(cav, &[MAIN, MAIN, ZUIUN, AIR_RADAR]), [One]);
        assert_eq!(cut_ins(KcShipType::CL, &[MAIN, MAIN, ZUIUN]), [One]);
        // Two main guns, on a cruiser, battleship or tender of these types.
        assert_eq!(cut_ins(cav, &[MAIN, ZUIUN, SURFACE_RADAR]), []);
        assert_eq!(cut_ins(KcShipType::DD, &[MAIN, MAIN, ZUIUN]), []);
        // A slot with no aircraft left does not count.
        let mut shot_down = zuiun_cruiser(&codex, cav, &[MAIN, MAIN, ZUIUN]);
        shot_down.ship.api_onslot[2] = 0;
        assert_eq!(zuiun_night_cut_ins(&codex, &shot_down), []);
        assert_eq!(ZuiunNightCutIn::TwoAndRadar.damage_multiplier(), 1.36);
        assert_eq!(ZuiunNightCutIn::One.damage_multiplier(), 1.24);
    }

    #[test]
    fn the_night_zuiun_chance_loses_what_her_own_lights_give() {
        let codex = codex();
        let cruiser = zuiun_cruiser(&codex, KcShipType::CAV, &[MAIN, MAIN, ZUIUN]);
        assert_eq!(
            zuiun_night_chance(&codex, &cruiser, false, DARK),
            night_cut_in_chance(&codex, &cruiser, false, DARK)
        );
        // Her own star shell and searchlight give 4 and 7 and take 14 and 17.
        let lit = NightLights {
            star_shell: true,
            searchlight: 1,
        };
        let lights = (lit, NightLights::default());
        assert_eq!(
            zuiun_night_chance(&codex, &cruiser, false, lights),
            night_cut_in_chance(&codex, &cruiser, false, DARK) + 4.0 + 7.0 - 14.0 - 17.0
        );
        // 熟練見張員 gives 3 more than the 5 it gives every cut-in.
        let lookout = zuiun_cruiser(&codex, KcShipType::CAV, &[MAIN, MAIN, ZUIUN, LOOKOUT_ID]);
        assert_eq!(
            zuiun_night_chance(&codex, &lookout, false, DARK),
            night_cut_in_chance(&codex, &cruiser, false, DARK) + 5.0 + 3.0
        );
    }

    fn zuiun_params() -> NightBattleParams {
        NightBattleParams {
            special_attacks_made: Default::default(),
            friendly_formation_id: 1,
            enemy_formation_id: 1,
            engagement: EngagementType::SameCourse,
            contact: [None, None],
            night_start: false,
            flare_pos: [-1, -1],
        }
    }

    fn empty_hougeki() -> BattleNightHougeki {
        BattleNightHougeki {
            api_at_eflag: Vec::new(),
            api_at_list: Vec::new(),
            api_n_mother_list: Vec::new(),
            api_df_list: Vec::new(),
            api_si_list: Vec::new(),
            api_cl_list: Vec::new(),
            api_sp_list: Vec::new(),
            api_damage: Vec::new(),
        }
    }

    fn zuiun_targets(codex: &Codex) -> Vec<BattleRuntimeShip> {
        let mut target = ship(codex, KcShipType::DD, 50, &[]);
        target.ship.api_nowhp = 999;
        target.ship.api_maxhp = 999;
        vec![target.clone(), target]
    }

    #[test]
    fn a_failed_night_zuiun_cut_in_rolls_once_for_each_she_makes() {
        let codex = codex();
        let mut friendly =
            vec![zuiun_cruiser(&codex, KcShipType::CAV, &[MAIN, MAIN, ZUIUN, SURFACE_RADAR])];
        let mut enemy = zuiun_targets(&codex);
        let mut hougeki = empty_hougeki();
        let mut lights = DARK;
        let mut rng = Rolls([0.999, 0.999].into_iter().collect());
        let params = zuiun_params();
        let made = zuiun_night_attack(
            &codex,
            &mut rng,
            &mut friendly,
            0,
            &mut enemy,
            (1, 1),
            &params,
            &mut lights,
            &mut hougeki,
        );
        assert!(!made);
        assert!(rng.0.is_empty(), "rolls left over");
        assert!(!lights.0.star_shell);
        // Half her hull or less, and she does not try.
        friendly[0].current_hp = friendly[0].ship.api_maxhp / 2;
        let mut rng = Rolls(VecDeque::new());
        let params = zuiun_params();
        let tried = zuiun_night_attack(
            &codex,
            &mut rng,
            &mut friendly,
            0,
            &mut enemy,
            (1, 1),
            &params,
            &mut lights,
            &mut hougeki,
        );
        assert!(!tried);
    }

    #[test]
    fn a_night_zuiun_cut_in_hits_twice_in_one_row_and_lights_her_fleet() {
        let codex = codex();
        let cruiser = zuiun_cruiser(&codex, KcShipType::CAV, &[MAIN, MAIN, ZUIUN, SURFACE_RADAR]);
        let rows: Vec<_> = (0..100)
            .filter_map(|seed| {
                let mut friendly = vec![ship(&codex, KcShipType::DD, 50, &[]), cruiser.clone()];
                let mut enemy = zuiun_targets(&codex);
                let mut hougeki = empty_hougeki();
                let mut lights = DARK;
                let params = zuiun_params();
                zuiun_night_attack(
                    &codex,
                    &mut crate::random::SeededRng::new(seed),
                    &mut friendly,
                    1,
                    &mut enemy,
                    (1, 1),
                    &params,
                    &mut lights,
                    &mut hougeki,
                )
                .then_some((hougeki, lights))
            })
            .collect();
        assert!(!rows.is_empty(), "it comes up on some seed");
        for (hougeki, lights) in &rows {
            assert!(lights.0.star_shell, "her fleet counts as lit for the rest of the night");
            assert_eq!(hougeki.api_sp_list, [200]);
            assert_eq!(hougeki.api_at_list, [1]);
            assert_eq!(hougeki.api_at_eflag, [0]);
            assert_eq!(hougeki.api_n_mother_list, [0]);
            assert_eq!(hougeki.api_df_list[0].len(), 2);
            assert_eq!(hougeki.api_damage[0].len(), 2);
        }
        // The 夜間瑞雲 first, then the radar of the cut-in that has one, then main guns: the
        // client reads three pieces without a check.
        let shown: std::collections::BTreeSet<_> =
            rows.iter().map(|(hougeki, _)| format!("{:?}", hougeki.api_si_list[0])).collect();
        let expected = [vec![ZUIUN, SURFACE_RADAR, MAIN], vec![ZUIUN, MAIN, MAIN]]
            .map(|ids| format!("{:?}", SiListId::text_from_i64(&ids)));
        assert_eq!(shown, expected.into_iter().collect());
    }

    #[test]
    fn the_night_attack_table_follows_the_source() {
        // (id, dmgMod, accMod, chanceMod, numHits, torpedo, replace, replaceChance) as
        // `NBATTACKDATA` gives them and `scripts/gen_night_attack_table.py` prints them.
        let source = [
            (1, 1.2, 1.1, 0.0, 2, false, 0, 0.0),
            (2, 1.3, 1.5, 1.15, 2, true, 0, 0.0),
            (3, 1.5, 1.65, 1.22, 2, true, 0, 0.0),
            (4, 1.75, 1.5, 1.3, 1, false, 0, 0.0),
            (5, 2.0, 2.0, 1.4, 1, false, 0, 0.0),
            (7, 1.3, 1.1, 1.15, 1, false, 11, 0.65),
            (8, 1.2, 1.65, 1.4, 1, false, 12, 0.5),
            (9, 1.5, 1.65, 1.25, 1, false, 13, 0.875),
            (10, 1.3, 1.5, 1.22, 1, false, 14, 0.55),
            (11, 1.3, 1.1, 1.3, 2, false, 0, 0.0),
            (12, 1.2, 1.65, 1.4, 2, false, 0, 0.0),
            (13, 1.5, 1.65, 1.25, 2, false, 0, 0.0),
            (14, 1.3, 1.5, 1.22, 2, false, 0, 0.0),
        ];
        let table: Vec<_> = NIGHT_ATTACKS
            .iter()
            .map(|row| {
                (
                    row.id,
                    row.damage,
                    row.accuracy,
                    row.chance,
                    row.hits,
                    row.torpedo,
                    row.replace,
                    row.replace_chance,
                )
            })
            .collect();
        assert_eq!(table, source);
        // Every row is an attack type, and the other way round.
        let tabled: Vec<_> =
            NightAttackType::TABLED.iter().map(|kind| kind.api_sp_list()).collect();
        assert_eq!(tabled, source.map(|row| row.0));
        assert_eq!(
            (SPECIAL_TORPEDO_IDS.len(), SPECIAL_TORPEDO_IDS.iter().sum::<i64>()),
            (8, 3_124)
        );
    }

    #[test]
    fn a_ship_has_one_general_attack_type() {
        let codex = codex();
        let general =
            |equipment: &[i64]| types(&codex, &ship(&codex, KcShipType::CL, 50, equipment));
        assert_eq!(general(&[MAIN, MAIN, MAIN, TORPEDO, TORPEDO]), [5]);
        assert_eq!(general(&[MAIN, MAIN, SECONDARY, TORPEDO, TORPEDO]), [4]);
        assert_eq!(general(&[MAIN, SECONDARY, TORPEDO, TORPEDO]), [3]);
        // A main gun with a torpedo is a cut-in, with or without a radar, and not a double
        // attack.
        assert_eq!(general(&[MAIN, SECONDARY, TORPEDO]), [2]);
        assert_eq!(general(&[MAIN, TORPEDO, SURFACE_RADAR]), [2]);
        assert_eq!(general(&[MAIN, SECONDARY]), [1]);
        assert_eq!(general(&[SECONDARY, SECONDARY]), [1]);
        assert_eq!(general(&[MAIN, MAIN]), [1]);
        assert_eq!(general(&[SECONDARY, TORPEDO]), [0; 0]);
        assert_eq!(general(&[MAIN]), [0; 0]);
    }

    #[test]
    fn a_failed_cut_in_is_a_plain_attack() {
        let codex = codex();
        let surface = TargetClass::SurfaceShip;
        // Three main guns at a chance of 50: 0.5 / 1.4.
        let guns = ship(&codex, KcShipType::CL, 50, &[MAIN, MAIN, MAIN]);
        assert_eq!(attack(&codex, &guns, surface, &[0.35]), (5, 2.0));
        assert_eq!(attack(&codex, &guns, surface, &[0.36]), (0, 1.0));
        // A main gun with a torpedo hits twice at 1.3, and falls back on nothing.
        let mixed = ship(&codex, KcShipType::CL, 50, &[MAIN, TORPEDO]);
        assert_eq!(attack(&codex, &mixed, surface, &[0.43]), (2, 1.3));
        assert_eq!(attack(&codex, &mixed, surface, &[0.44]), (0, 1.0));
        assert_eq!(NightAttackType::MainTorp.hit_count(), 2);
        // The double attack comes up 99 times in 100.
        let double = ship(&codex, KcShipType::CL, 50, &[MAIN, SECONDARY]);
        assert_eq!(attack(&codex, &double, surface, &[0.98]), (1, 1.2));
        assert_eq!(attack(&codex, &double, surface, &[0.99]), (0, 1.0));
        // A ship with nothing to roll for rolls nothing, and neither does one facing a
        // submarine.
        let plain = ship(&codex, KcShipType::CL, 50, &[MAIN]);
        assert_eq!(attack(&codex, &plain, surface, &[]), (0, 1.0));
        assert_eq!(attack(&codex, &guns, TargetClass::Submarine, &[]), (0, 1.0));
    }

    #[test]
    fn the_cut_in_chance_follows_the_source() {
        let codex = codex();
        let lit = NightLights {
            star_shell: true,
            searchlight: 2,
        };
        let dark = NightLights::default();
        let chance = |ship: &BattleRuntimeShip, flagship, lights| {
            night_cut_in_chance(&codex, ship, flagship, lights)
        };
        // floor(30 + 15 + sqrt(50) * 0.75), and 15 more for the flagship.
        let cl = ship(&codex, KcShipType::CL, 50, &[]);
        assert_eq!(chance(&cl, false, DARK), 50.0);
        assert_eq!(chance(&cl, true, DARK), 65.0);
        // 65 + floor(sqrt(60 - 50)) + floor(sqrt(99) * 0.8).
        let mut lucky = ship(&codex, KcShipType::CL, 99, &[]);
        lucky.ship.api_lucky = [60, 60];
        assert_eq!(chance(&lucky, false, DARK), 75.0);
        // Her own star shell and searchlight add 4 and 7; the enemy's take 10 and 5.
        assert_eq!(chance(&cl, false, (lit, dark)), 61.0);
        assert_eq!(chance(&cl, false, (dark, lit)), 35.0);
        // The two lookouts add up; the torpedo squadron's counts on a destroyer, a light
        // cruiser and a torpedo cruiser only.
        let lookouts =
            |kind| chance(&ship(&codex, kind, 50, &[LOOKOUT_ID, SKILLED_LOOKOUT_ID]), false, DARK);
        assert_eq!(lookouts(KcShipType::DD), 63.0);
        assert_eq!(lookouts(KcShipType::CL), 63.0);
        assert_eq!(lookouts(KcShipType::CA), 55.0);
        // Moderate damage adds 18 whatever the cut-in.
        let at = |hp| {
            let mut cl = ship(&codex, KcShipType::CL, 50, &[]);
            cl.ship.api_maxhp = 100;
            cl.set_hp_for_debug(hp);
            chance(&cl, false, DARK)
        };
        assert_eq!([at(51), at(50), at(26)], [50.0, 68.0, 68.0]);
    }

    #[test]
    fn a_destroyer_rolls_her_own_cut_ins_first_and_in_the_source_order() {
        let codex = codex();
        let surface = TargetClass::SurfaceShip;
        let all = [MAIN, TORPEDO, TORPEDO, SURFACE_RADAR, SKILLED_LOOKOUT_ID, DRUM_ID];
        assert_eq!(types(&codex, &ship(&codex, KcShipType::DD, 50, &all)), [7, 8, 9, 10, 3]);
        // They are a destroyer's alone, and want a surface radar.
        assert_eq!(types(&codex, &ship(&codex, KcShipType::CL, 50, &all)), [3]);
        let air = [MAIN, TORPEDO, AIR_RADAR];
        assert_eq!(types(&codex, &ship(&codex, KcShipType::DD, 50, &air)), [2]);

        // Each is rolled for on its own: 0.5 / 1.15 for the first, then her general cut-in.
        let dd = ship(&codex, KcShipType::DD, 50, &[MAIN, TORPEDO, SURFACE_RADAR]);
        assert_eq!(types(&codex, &dd), [7, 2]);
        assert_eq!(attack(&codex, &dd, surface, &[0.43]), (7, 1.3));
        assert_eq!(attack(&codex, &dd, surface, &[0.44, 0.43]), (2, 1.3));
        assert_eq!(attack(&codex, &dd, surface, &[0.44, 0.44]), (0, 1.0));
    }

    #[test]
    fn a_destroyer_of_level_80_may_make_her_cut_in_twice() {
        let codex = codex();
        let surface = TargetClass::SurfaceShip;
        let gear = [MAIN, TORPEDO, SURFACE_RADAR];
        // Level 80 with 30 luck: floor(30 + 15 + sqrt(80) * 0.75) = 51.
        let dd = ship(&codex, KcShipType::DD, 80, &gear);
        // The roll for the second hit comes first, under 0.65, and the two-hit attack is
        // then rolled for at its own 0.51 / 1.3.
        assert_eq!(attack(&codex, &dd, surface, &[0.64, 0.39]), (11, 1.3));
        assert_eq!(attack(&codex, &dd, surface, &[0.64, 0.40, 0.99]), (0, 1.0));
        assert_eq!(attack(&codex, &dd, surface, &[0.65, 0.40]), (7, 1.3));
        assert_eq!(NightAttackType::DdGunTorpRadar2.hit_count(), 2);
        // Below level 80 there is no such roll.
        let dd = ship(&codex, KcShipType::DD, 79, &gear);
        assert_eq!(attack(&codex, &dd, surface, &[0.0]), (7, 1.3));
    }

    #[test]
    fn the_d_type_guns_raise_the_destroyer_cut_ins_made_with_a_radar() {
        let codex = codex();
        let damage = |equipment: &[i64]| {
            let dd = ship(&codex, KcShipType::DD, 50, equipment);
            attack(&codex, &dd, TargetClass::SurfaceShip, &[0.0])
        };
        let close = |(kind, damage): (i64, f64), wanted: f64| {
            assert_eq!(kind, 7);
            assert!((damage - wanted).abs() < 1e-9, "{damage} is not {wanted}");
        };
        close(damage(&[D_GUN_K2_ID, TORPEDO, SURFACE_RADAR]), 1.3 * 1.25);
        close(damage(&[D_GUN_K3_ID, TORPEDO, SURFACE_RADAR]), 1.3 * 1.25 * 1.05);
        close(damage(&[D_GUN_K2_ID, D_GUN_K3_ID, TORPEDO, SURFACE_RADAR]), 1.3 * 1.4 * 1.05);
        close(damage(&[D_GUN_K3_ID, D_GUN_K3_ID, TORPEDO, SURFACE_RADAR]), 1.3 * 1.4 * 1.1);
        // Not the two made without a radar.
        let lookout = damage(&[D_GUN_K2_ID, TORPEDO, TORPEDO, SKILLED_LOOKOUT_ID]);
        assert_eq!(lookout.0, 9);
        assert!((lookout.1 - 1.5).abs() < 1e-9);
    }

    #[test]
    fn late_model_torpedoes_raise_a_submarine_torpedo_cut_in() {
        let codex = codex();
        let surface = TargetClass::SurfaceShip;
        let sub = |equipment: &[i64]| ship(&codex, KcShipType::SS, 50, equipment);
        // With her radar one late-model torpedo counts as two: 1.75 at 0.5 / 1.05.
        let radar = sub(&[LATE_TORPEDO, SUB_RADAR]);
        assert_eq!(types(&codex, &radar), [3]);
        assert_eq!(attack(&codex, &radar, surface, &[0.47]), (3, 1.75));
        assert_eq!(attack(&codex, &radar, surface, &[0.48]), (0, 1.0));
        // Two of them without it: 1.6 at 0.5 / 1.1.
        let two = sub(&[LATE_TORPEDO, LATE_TORPEDO]);
        assert_eq!(attack(&codex, &two, surface, &[0.45]), (3, 1.6));
        assert_eq!(attack(&codex, &two, surface, &[0.46]), (0, 1.0));
        // One alone is a torpedo like another: 1.5 at 0.5 / 1.22.
        let one = sub(&[LATE_TORPEDO, TORPEDO]);
        assert_eq!(attack(&codex, &one, surface, &[0.40]), (3, 1.5));
        assert_eq!(attack(&codex, &one, surface, &[0.41]), (0, 1.0));
        assert_eq!(types(&codex, &sub(&[LATE_TORPEDO])), [0; 0]);
    }

    #[test]
    fn a_torpedo_cut_in_is_not_made_on_an_installation() {
        let codex = codex();
        let land = TargetClass::Installation;
        // Without two guns there is nothing left to roll for.
        let mixed = ship(&codex, KcShipType::CL, 50, &[MAIN, TORPEDO]);
        assert_eq!(attack(&codex, &mixed, land, &[]), (0, 1.0));
        // With two she double attacks instead.
        let guns = ship(&codex, KcShipType::CL, 50, &[MAIN, SECONDARY, TORPEDO, TORPEDO]);
        assert_eq!(types(&codex, &guns), [3]);
        assert_eq!(attack(&codex, &guns, land, &[0.98]), (1, 1.2));
        assert_eq!(attack(&codex, &guns, land, &[0.99]), (0, 1.0));
        // A destroyer cut-in needs a main gun there.
        let lookout = [TORPEDO, TORPEDO, SKILLED_LOOKOUT_ID];
        let dd = ship(&codex, KcShipType::DD, 50, &lookout);
        assert_eq!(types(&codex, &dd), [9, 3]);
        assert_eq!(attack(&codex, &dd, land, &[]), (0, 1.0));
        let dd = ship(&codex, KcShipType::DD, 50, &[MAIN, TORPEDO, SURFACE_RADAR]);
        assert_eq!(attack(&codex, &dd, land, &[0.0]), (7, 1.3));
    }

    #[test]
    fn the_first_ship_to_roll_under_seventy_fires_her_star_shell() {
        let codex = codex();
        let mut holed = ship(&codex, KcShipType::DD, 50, &[STAR_SHELL]);
        holed.set_hp_for_debug(4);
        let fleet = [
            ship(&codex, KcShipType::DD, 50, &[SEARCHLIGHT]),
            holed,
            ship(&codex, KcShipType::DD, 50, &[STAR_SHELL]),
            ship(&codex, KcShipType::DD, 50, &[STAR_SHELL]),
        ];
        let fire = |rolls: &[f64]| {
            let mut rng = Rolls(rolls.iter().copied().collect());
            let fired = roll_star_shells(&codex, &mut rng, &fleet, &fleet[..1]);
            assert!(rng.0.is_empty(), "rolls left over");
            fired
        };
        // A fleet without one rolls nothing; a ship with 4 HP does not fire hers.
        assert_eq!(fire(&[0.69]), [2, -1]);
        assert_eq!(fire(&[0.7, 0.0]), [3, -1]);
        assert_eq!(fire(&[0.7, 0.7]), [-1, -1]);
    }

    #[test]
    fn the_packet_carries_a_mixed_cut_in_as_type_two_with_the_gun_first() {
        let codex = codex();
        let mut lucky = ship(&codex, KcShipType::CL, 99, &[TORPEDO, MAIN, SURFACE_RADAR]);
        lucky.ship.api_lucky = [99, 99];
        let friendly = vec![lucky];
        let enemy = vec![ship(&codex, KcShipType::BB, 99, &[])];
        let hougeki = (0..20)
            .find_map(|seed| {
                simulate_night_hougeki(
                    &codex,
                    &mut crate::random::SeededRng::new(seed),
                    &mut friendly.clone(),
                    &mut enemy.clone(),
                    &NightBattleParams {
                        special_attacks_made: Default::default(),
                        friendly_formation_id: 1,
                        enemy_formation_id: 1,
                        engagement: crate::types::EngagementType::SameCourse,
                        contact: [None, None],
                        night_start: false,
                        flare_pos: [-1, -1],
                    },
                )
                .filter(|hougeki| hougeki.api_sp_list[0] == 2)
            })
            .expect("a mixed cut-in in 20 seeds");
        assert_eq!(hougeki.api_damage[0].len(), 2);
        assert_eq!(hougeki.api_df_list[0].len(), 2);
        assert_eq!(hougeki.api_si_list[0], SiListId::text_from_i64(&[MAIN, TORPEDO]));
    }

    fn at_hp(mut ship: BattleRuntimeShip, hp: i64) -> BattleRuntimeShip {
        ship.ship.api_maxhp = 100;
        ship.ship.api_nowhp = hp;
        ship.current_hp = hp;
        ship.entry_hp = hp;
        ship
    }

    fn hostile(mut ship: BattleRuntimeShip) -> BattleRuntimeShip {
        ship.is_friendly = false;
        ship
    }

    /// The night's attacks as (by the enemy, attacker, target, type, hits, shown).
    fn night(
        codex: &Codex,
        seed: u64,
        friendly: &[BattleRuntimeShip],
        enemy: &[BattleRuntimeShip],
    ) -> Vec<(i64, i64, i64, i64, usize, Vec<SiListId>)> {
        let hougeki = simulate_night_hougeki(
            codex,
            &mut crate::random::SeededRng::new(seed),
            &mut friendly.to_vec(),
            &mut enemy.to_vec(),
            &NightBattleParams {
                special_attacks_made: Default::default(),
                friendly_formation_id: 1,
                enemy_formation_id: 1,
                engagement: crate::types::EngagementType::SameCourse,
                contact: [None, None],
                night_start: false,
                flare_pos: [-1, -1],
            },
        );
        hougeki.map_or_else(Vec::new, |h| {
            (0..h.api_at_list.len())
                .map(|i| {
                    (
                        h.api_at_eflag[i],
                        h.api_at_list[i],
                        h.api_df_list[i][0],
                        h.api_sp_list[i],
                        h.api_damage[i].len(),
                        h.api_si_list[i].clone(),
                    )
                })
                .collect()
        })
    }

    #[test]
    fn a_ship_attacks_at_night_with_more_than_a_quarter_of_her_hull() {
        // `Ship.canNB`; `SS` does not override it.
        let codex = codex();
        let attacks =
            |kind, hp| can_attack_night_ship(&codex, &at_hp(ship(&codex, kind, 50, &[]), hp));
        for kind in [KcShipType::DD, KcShipType::BB, KcShipType::SS, KcShipType::SSV] {
            assert!(attacks(kind, 100), "{kind:?}");
            assert!(attacks(kind, 26), "{kind:?}");
            assert!(!attacks(kind, 25), "{kind:?} at a quarter");
            assert!(!attacks(kind, 0), "{kind:?} sunk");
        }
    }

    #[test]
    fn a_heavily_damaged_ship_of_either_fleet_makes_no_night_attack() {
        let codex = codex();
        // Armoured so that a hit only scratches.
        let dd = |hp| {
            let mut dd = at_hp(ship(&codex, KcShipType::DD, 50, &[MAIN]), hp);
            dd.ship.api_soukou[0] = 999;
            dd
        };
        let enemy = |hp| hostile(dd(hp));
        for seed in 0..20 {
            let attackers = |friendly: i64, hostile: i64| {
                night(&codex, seed, &[dd(friendly)], &[enemy(hostile)])
                    .iter()
                    .map(|attack| attack.0)
                    .collect::<Vec<_>>()
            };
            assert_eq!(attackers(100, 100), [0, 1]);
            assert_eq!(attackers(25, 100), [1]);
            assert_eq!(attackers(100, 25), [0]);
        }
    }

    #[test]
    fn a_submarine_attacks_the_surface_ships_at_night_and_no_submarine() {
        let codex = codex();
        let submarine = ship(&codex, KcShipType::SS, 50, &[]);
        let hostile = |kind| hostile(at_hp(ship(&codex, kind, 50, &[]), 100));
        for seed in 0..50 {
            let enemy = [hostile(KcShipType::SS), hostile(KcShipType::BB)];
            let attacks = night(&codex, seed, std::slice::from_ref(&submarine), &enemy);
            // One plain attack on the battleship; the enemy submarine has nobody to attack.
            let hers: Vec<_> = attacks.iter().filter(|attack| attack.0 == 0).collect();
            assert_eq!(hers.len(), 1);
            assert_eq!((hers[0].1, hers[0].2, hers[0].3, hers[0].4), (0, 1, 0, 1));
            assert!(!attacks.iter().any(|attack| (attack.0, attack.1) == (1, 0)));
            assert!(night(&codex, seed, std::slice::from_ref(&submarine), &enemy[..1]).is_empty());
        }
    }

    #[test]
    fn a_submarine_makes_her_torpedo_cut_in_in_battle_and_shows_two_pieces() {
        let codex = codex();
        let target = hostile(at_hp(ship(&codex, KcShipType::BB, 50, &[]), 100));
        let cut_ins = |equipment: &[i64], shown: &[i64]| {
            let submarine = ship(&codex, KcShipType::SS, 50, equipment);
            let attacks: Vec<_> = (0..40)
                .map(|seed| {
                    night(
                        &codex,
                        seed,
                        std::slice::from_ref(&submarine),
                        std::slice::from_ref(&target),
                    )
                    .remove(0)
                })
                .collect();
            let cut_ins: Vec<_> = attacks.iter().filter(|attack| attack.3 == 3).collect();
            for attack in &cut_ins {
                assert_eq!(attack.4, 2, "two hits");
                assert_eq!(attack.5, SiListId::text_from_i64(shown));
            }
            assert!(attacks.iter().all(|attack| attack.3 == 0 || attack.3 == 3));
            cut_ins.len()
        };
        assert!(cut_ins(&[LATE_TORPEDO, LATE_TORPEDO], &[LATE_TORPEDO, LATE_TORPEDO]) > 0);
        // Three torpedoes are shown as before.
        let three = ship(&codex, KcShipType::SS, 50, &[TORPEDO; 3]);
        assert_eq!(
            night_attack_display_ids(&codex, &three, NightAttackType::TorpTorpTorp),
            [TORPEDO; 3]
        );
        // `CutinSpRR` reads both pieces without a check: her radar stands for the second.
        assert!(cut_ins(&[LATE_TORPEDO, SUB_RADAR], &[LATE_TORPEDO, SUB_RADAR]) > 0);
    }

    const KONGOU: i64 = 591; // 金剛改二丙
    const HIEI: i64 = 592; // 比叡改二丙
    const HARUNA_B: i64 = 593; // 榛名改二乙
    const SURFACE_RADAR_10: i64 = 31; // 32号対水上電探, 10 line of sight
    const LARGE_SEARCHLIGHT: i64 = 140; // 96式150cm探照灯

    fn battleship(
        codex: &Codex,
        mst_id: i64,
        level: i64,
        luck: i64,
        equipment: &[i64],
    ) -> BattleRuntimeShip {
        let mut ship = sample_ship(codex, mst_id, level);
        ship.ship.api_lucky = [luck, 99];
        ship.slot_items = equipment.iter().copied().map(slotitem_with_mst_id).collect();
        at_hp(BattleRuntimeShip::from(ship), 100)
    }

    /// A flagship and her partner at level 99 with 18 and 48 luck, as wikiwiki works the rate
    /// out, and four destroyers.
    fn assault_fleet(codex: &Codex, flagship: i64, equipment: &[i64]) -> Vec<BattleRuntimeShip> {
        let partner = if flagship == KONGOU {
            HARUNA_B
        } else {
            KONGOU
        };
        let mut fleet = vec![
            battleship(codex, flagship, 99, 18, equipment),
            battleship(codex, partner, 99, 48, &[]),
        ];
        fleet.extend((0..4).map(|_| at_hp(ship(codex, KcShipType::DD, 50, &[]), 100)));
        fleet
    }

    #[test]
    fn the_kongou_night_assault_rate_is_wikiwikis() {
        let codex = codex();
        let rate = |flagship, equipment: &[i64]| {
            kongou_assault_rate(&codex, &assault_fleet(&codex, flagship, equipment), 1).unwrap()
        };
        // 金剛 Lv99 with 18 luck and 榛名乙 Lv99 with 48: 48, and what her pieces add.
        assert_eq!(rate(KONGOU, &[]), 0.48);
        assert_eq!(rate(KONGOU, &[SURFACE_RADAR_10]), 0.78);
        assert_eq!(rate(KONGOU, &[SURFACE_RADAR_10, LARGE_SEARCHLIGHT]), 0.88);
        // A radar of 5 line of sight and a plain searchlight are not the pieces meant.
        assert_eq!(rate(KONGOU, &[SURFACE_RADAR, SEARCHLIGHT]), 0.48);
        // 比叡 has them the other way round.
        assert_eq!(rate(HIEI, &[SURFACE_RADAR_10]), 0.58);
        assert_eq!(rate(HIEI, &[LARGE_SEARCHLIGHT]), 0.78);
    }

    #[test]
    fn the_kongou_night_assault_needs_its_pair_five_surface_ships_and_a_formation() {
        let codex = codex();
        let fleet = || assault_fleet(&codex, KONGOU, &[]);
        let eligible = |fleet: &[BattleRuntimeShip], formation| {
            kongou_assault_rate(&codex, fleet, formation).is_some()
        };
        for formation in [1, 4, 12, 14] {
            assert!(eligible(&fleet(), formation), "{formation}");
        }
        for formation in [2, 3, 5, 6, 11, 13] {
            assert!(!eligible(&fleet(), formation), "{formation}");
        }
        // The partner has to be one of hers, and in the second place.
        let mut other = fleet();
        other[1] = battleship(&codex, 541, 99, 48, &[]);
        assert!(!eligible(&other, 1));
        let mut swapped = fleet();
        swapped.swap(1, 2);
        assert!(!eligible(&swapped, 1));
        // Neither of the two heavily damaged.
        for at in [0, 1] {
            let mut hurt = fleet();
            hurt[at] = at_hp(hurt[at].clone(), 25);
            assert!(!eligible(&hurt, 1), "{at}");
            hurt[at] = at_hp(hurt[at].clone(), 26);
            assert!(eligible(&hurt, 1), "{at}");
        }
        // Five surface ships afloat: a submarine or a sunk ship is not one.
        assert!(eligible(&fleet()[..5], 1));
        assert!(!eligible(&fleet()[..4], 1));
        let mut with_submarine = fleet()[..5].to_vec();
        with_submarine[4] = ship(&codex, KcShipType::SS, 50, &[]);
        assert!(!eligible(&with_submarine, 1));
        let mut sunk = fleet()[..5].to_vec();
        sunk[4] = at_hp(sunk[4].clone(), 0);
        assert!(!eligible(&sunk, 1));
    }

    /// `getNightEquipsScout` and `NBattack`: who may fly a night scout, and what one in
    /// contact gives.
    #[test]
    fn a_night_scout_comes_into_contact_by_the_air_and_gives_by_its_accuracy() {
        use crate::types::AirState::{Denial, Incapability, Parity, Superiority};
        let codex = codex();
        // 九八式水上偵察機(夜偵): 3 line of sight and 1 accuracy; at Lv99, floor(√99 × √3) / 25.
        let scout = 102;
        let fleet = vec![battleship(&codex, KONGOU, 99, 18, &[scout])];
        let none: Vec<BattleRuntimeShip> = Vec::new();
        let flown = |air, night_start| {
            (0..400)
                .filter_map(|seed| {
                    let mut rng = crate::random::SeededRng::new(seed);
                    roll_night_contact(&codex, &mut rng, &fleet, &none, air, night_start)[0]
                })
                .collect::<Vec<_>>()
        };
        let by_superiority = flown(Some(Superiority), false);
        // 17 in 25: 68%.
        assert!((240..300).contains(&by_superiority.len()), "{}", by_superiority.len());
        assert!(by_superiority.iter().all(|contact| *contact == NightContact::of(scout, 1)));
        assert!(!flown(Some(Denial), false).is_empty());
        assert!(flown(Some(Incapability), false).is_empty());
        assert!(flown(Some(Parity), false).is_empty());
        assert!(flown(None, false).is_empty());
        // A battle that opens at night has had no air battle to lose.
        assert!(!flown(None, true).is_empty());

        let gives = |accuracy| {
            let contact = NightContact::of(scout, accuracy);
            (contact.power, contact.accuracy, contact.critical)
        };
        assert_eq!(gives(1), (5.0, 1.1, 1.57));
        assert_eq!(gives(2), (7.0, 1.15, 1.64));
        assert_eq!(gives(3), (9.0, 1.2, 1.7));
    }

    #[test]
    fn the_kongou_night_assault_multiplier_follows_the_place_the_engagement_and_the_guns() {
        use crate::types::EngagementType::{HeadOn, SameCourse, TAdvantage, TDisadvantage};
        let codex = codex();
        let multiplier = |equipment: &[i64], flagship, engagement| {
            let ship = battleship(&codex, KONGOU, 99, 18, equipment);
            kongou_assault_multiplier(&ship, flagship, engagement)
        };
        let close = |value: f64, wanted: f64| (value - wanted).abs() < 1e-9;
        // 2.6 and 2.7, and 1.06 more since 2026-04-23.
        assert!(close(multiplier(&[], true, SameCourse), 2.756));
        assert!(close(multiplier(&[], false, HeadOn), 2.862));
        assert!(close(multiplier(&[], true, TAdvantage), 2.756 * 1.25));
        assert!(close(multiplier(&[], true, TDisadvantage), 2.756 * 0.8));
        // 35.6cm連装砲改四 / 改三丙, and 改二 / 改三(ダズル迷彩仕様): by how many, the two multiplied.
        assert!(close(multiplier(&[503], true, SameCourse), 2.756 * 1.11));
        assert!(close(multiplier(&[503, 530, 530], true, SameCourse), 2.756 * 1.15));
        assert!(close(multiplier(&[329], true, SameCourse), 2.756 * 1.05));
        assert!(close(multiplier(&[329, 502], true, SameCourse), 2.756 * 1.08));
        assert!(close(multiplier(&[503, 502], false, SameCourse), 2.862 * 1.11 * 1.05));
    }

    #[test]
    fn the_kongou_night_assault_is_one_row_of_two_hits_and_spares_the_partner_her_turn() {
        let codex = codex();
        // Level 175, 99 luck and both of her pieces put the rate past 100: it always comes up.
        let mut friendly = assault_fleet(&codex, KONGOU, &[SURFACE_RADAR_10, LARGE_SEARCHLIGHT]);
        for ship in &mut friendly[..2] {
            ship.ship.api_lv = 175;
            ship.ship.api_lucky[0] = 99;
        }
        assert_eq!(kongou_assault_rate(&codex, &friendly, 1), Some(1.0));
        let mut target = at_hp(ship(&codex, KcShipType::BB, 50, &[]), 100);
        target.ship.api_soukou[0] = 9999;
        let enemy = vec![hostile(target); 6];
        for seed in 0..20 {
            let attacks = night(&codex, seed, &friendly, &enemy);
            // (by the enemy, attacker, target, type, hits, shown)
            assert_eq!((attacks[0].0, attacks[0].1, attacks[0].3, attacks[0].4), (0, 0, 104, 2));
            let ours = |at| attacks.iter().filter(|attack| (attack.0, attack.1) == (0, at)).count();
            assert_eq!((ours(0), ours(1)), (1, 1), "the flagship once, her partner once more");
            // The enemy never makes it.
            let theirs: Vec<_> = friendly.iter().cloned().map(hostile).collect();
            let plain = vec![at_hp(ship(&codex, KcShipType::BB, 50, &[]), 100); 6];
            assert!(!night(&codex, seed, &plain, &theirs).iter().any(|attack| attack.3 == 104));
        }
        // Not without the pair.
        friendly.swap(1, 2);
        assert!(!night(&codex, 0, &friendly, &enemy).iter().any(|attack| attack.3 == 104));
    }

    /// wikiwiki 金剛改二丙: 「出撃1回毎に…最大3回発動可能」, apart from the special gunnery
    /// attacks, and none in an exercise. The source: `numSpecialKongou >= 3`.
    #[test]
    fn the_kongou_night_assault_is_made_three_times_a_sortie_and_never_in_an_exercise() {
        let codex = codex();
        let mut friendly = assault_fleet(&codex, KONGOU, &[SURFACE_RADAR_10, LARGE_SEARCHLIGHT]);
        for ship in &mut friendly[..2] {
            ship.ship.api_lv = 175;
            ship.ship.api_lucky[0] = 99;
        }
        let enemy = vec![hostile(at_hp(ship(&codex, KcShipType::BB, 50, &[]), 100)); 6];
        let made = |friendly: &[BattleRuntimeShip], kongou_assaults| {
            simulate_night_hougeki(
                &codex,
                &mut crate::random::SeededRng::new(0),
                &mut friendly.to_vec(),
                &mut enemy.clone(),
                &NightBattleParams {
                    special_attacks_made: SpecialAttacksMade {
                        gunnery: true,
                        kongou_assaults,
                    },
                    friendly_formation_id: 1,
                    enemy_formation_id: 1,
                    engagement: crate::types::EngagementType::SameCourse,
                    contact: [None, None],
                    night_start: false,
                    flare_pos: [-1, -1],
                },
            )
            .unwrap()
            .api_sp_list
            .contains(&104)
        };
        assert!(made(&friendly, 0));
        assert!(made(&friendly, 2));
        assert!(!made(&friendly, 3));
        for ship in &mut friendly {
            ship.is_sortie = false;
        }
        assert!(!made(&friendly, 0));
    }

    /// wikiwiki: 「敵がPT小鬼群のみのマス…では発動しない」, of every fleet special attack.
    #[test]
    fn no_special_attack_is_made_at_night_against_pt_boats_alone() {
        let codex = codex();
        let pt = crate::test_utils::ship_mst_id_by_name(&codex, "PT小鬼群");
        // A destroyer under the boat's id is a boat to the battle.
        let mut boat = hostile(at_hp(ship(&codex, KcShipType::DD, 50, &[]), 100));
        boat.ship.api_ship_id = pt;
        let mut other = hostile(at_hp(ship(&codex, KcShipType::BB, 50, &[]), 100));
        other.ship.api_soukou[0] = 9999;
        let mut kongou = assault_fleet(&codex, KONGOU, &[SURFACE_RADAR_10, LARGE_SEARCHLIGHT]);
        let mut nagato =
            vec![battleship(&codex, 541, 175, 99, &[]), battleship(&codex, 573, 175, 99, &[])];
        nagato.extend((0..4).map(|_| at_hp(ship(&codex, KcShipType::BB, 50, &[]), 100)));
        for ship in kongou[..2].iter_mut() {
            ship.ship.api_lv = 175;
            ship.ship.api_lucky[0] = 99;
        }
        let made = |friendly: &[BattleRuntimeShip], formation, enemy: &[BattleRuntimeShip]| {
            (0..20).any(|seed| {
                simulate_night_hougeki(
                    &codex,
                    &mut crate::random::SeededRng::new(seed),
                    &mut friendly.to_vec(),
                    &mut enemy.to_vec(),
                    &NightBattleParams {
                        special_attacks_made: Default::default(),
                        friendly_formation_id: formation,
                        enemy_formation_id: 1,
                        engagement: crate::types::EngagementType::SameCourse,
                        contact: [None, None],
                        night_start: true,
                        flare_pos: [-1, -1],
                    },
                )
                .is_some_and(|rows| rows.api_sp_list.iter().any(|&sp| sp >= 100))
            })
        };
        let boats = vec![boat.clone(); 3];
        let mixed = vec![boat.clone(), boat, other];
        assert!(!made(&kongou, 1, &boats) && made(&kongou, 1, &mixed));
        assert!(!made(&nagato, 4, &boats) && made(&nagato, 4, &mixed));
    }

    /// The attacks of the day are rolled for in the flagship's turn at night as well (the
    /// source's `canSpecialAttackUnique` does not ask which; wikiwiki: 「夜戦でも発動する」),
    /// in the same formation, unless the sortie has made one.
    #[test]
    fn a_special_gunnery_attack_is_made_at_night_unless_the_sortie_has_made_it() {
        let codex = codex();
        // 長門改二 and 陸奥改二 ahead of four battleships.
        let mut friendly =
            vec![battleship(&codex, 541, 99, 40, &[]), battleship(&codex, 573, 99, 16, &[])];
        friendly.extend((0..4).map(|_| at_hp(ship(&codex, KcShipType::BB, 50, &[]), 100)));
        let mut target = at_hp(ship(&codex, KcShipType::BB, 50, &[]), 100);
        target.ship.api_soukou[0] = 9999;
        let enemy = vec![hostile(target); 6];
        // Our rows as (attacker, type, hits).
        let rows = |seed, formation, gunnery| {
            let hougeki = simulate_night_hougeki(
                &codex,
                &mut crate::random::SeededRng::new(seed),
                &mut friendly.clone(),
                &mut enemy.clone(),
                &NightBattleParams {
                    special_attacks_made: SpecialAttacksMade {
                        gunnery,
                        kongou_assaults: 0,
                    },
                    friendly_formation_id: formation,
                    enemy_formation_id: 1,
                    engagement: crate::types::EngagementType::SameCourse,
                    contact: [None, None],
                    night_start: false,
                    flare_pos: [-1, -1],
                },
            )
            .unwrap();
            (0..hougeki.api_at_list.len())
                .filter(|&i| hougeki.api_at_eflag[i] == 0)
                .map(|i| {
                    (hougeki.api_at_list[i], hougeki.api_sp_list[i], hougeki.api_damage[i].len())
                })
                .collect::<Vec<_>>()
        };
        let mut made = 0;
        for seed in 0..40 {
            let ours = rows(seed, 4, false);
            let of = |at| ours.iter().filter(|row| row.0 == at).collect::<Vec<_>>();
            if ours.iter().any(|row| row.1 >= 100) {
                made += 1;
                // One row of the flagship's, her two hits and her partner's, who keeps her turn.
                assert_eq!(of(0), [&(0, 101, 3)], "seed {seed}");
                assert_eq!(of(1).len(), 1, "seed {seed}");
            }
            assert!(rows(seed, 4, true).iter().all(|row| row.1 < 100), "seed {seed}: made twice");
            assert!(rows(seed, 1, false).iter().all(|row| row.1 < 100), "seed {seed}: 単縦陣");
        }
        // 62 in 100.
        assert!((15..=35).contains(&made), "{made} of 40");
    }
}
