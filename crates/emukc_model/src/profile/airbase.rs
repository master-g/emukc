use serde::{Deserialize, Serialize};

use crate::kc2::{KcApiAirBase, KcApiAirBaseExpandedInfo, KcApiDistance, KcApiPlaneInfo};

/// Airbase action assigned
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug, Default)]
pub enum AirbaseAction {
    /// Idle
    #[default]
    Idle = 0,
    /// Attack
    Attack = 1,
    /// Defense
    Defense = 2,
    /// Evasion
    Evasion = 3,
    /// Resort
    Resort = 4,
}

/// Air corps a single area can hold, reached by expanding with 設営隊.
///
/// The client's own limit (`AIRUNIT_MAX` in `main.decoded.js`).
pub const AIRUNIT_MAX: i64 = 3;

/// Highest 整備Lv an area can reach (the client's `AIRBASE_MAX_LEVEL`).
pub const MAINTENANCE_LEVEL_MAX: i64 = 3;

/// The 整備Lv entries `mapinfo` and `record` report: one per expanded area.
///
/// A regular area nobody expanded has no entry at all — a live account with
/// three air corps in area 6 reports none for it — and the client treats a
/// missing entry as level 0. An event area is different: upstream always
/// lists it, level 0 included, which this does not do yet.
pub fn expanded_info(airbases: &[Airbase]) -> Vec<KcApiAirBaseExpandedInfo> {
    let mut info: Vec<KcApiAirBaseExpandedInfo> = Vec::new();
    for airbase in airbases {
        if airbase.maintenance_level > 0 && info.iter().all(|i| i.api_area_id != airbase.area_id) {
            info.push(KcApiAirBaseExpandedInfo {
                api_area_id: airbase.area_id,
                api_maintenance_level: airbase.maintenance_level,
            });
        }
    }
    info
}

/// A squadron's condition on deployment, and where the orders stop restoring it.
///
/// Every figure of the condition model below is wikiwiki's 基地航空隊「疲労」and
/// 「整備Lv強化による効果」, which marks its own values as 推測される値: upstream
/// publishes none of them and the client only ever sees the three tiers.
pub const COND_DEPLOYED: i64 = 40;

/// The most a squadron's condition reaches, one point a tick past [`COND_DEPLOYED`].
pub const COND_MAX: i64 = 46;

/// Seconds between two recoveries of a squadron's condition.
pub const COND_TICK_SECS: i64 = 180;

/// Condition one sortie costs: both strikes on one cell, or spread over two.
pub const COND_COST_CONCENTRATED: i64 = 6;
/// See [`COND_COST_CONCENTRATED`].
pub const COND_COST_SPREAD: i64 = 8;

/// `api_cond` for an inner condition: 1 untired, 2 orange, 3 red — the only
/// three values the client's fatigue icon tells apart.
pub fn cond_tier(condition: i64) -> i64 {
    match condition {
        30.. => 1,
        20..=29 => 2,
        _ => 3,
    }
}

/// Condition a squadron regains each tick under an order, by the area's 整備Lv.
pub fn recovery_per_tick(action: AirbaseAction, maintenance_level: i64) -> i64 {
    let by_level: [i64; 4] = match action {
        AirbaseAction::Attack => [1, 1, 1, 2],
        AirbaseAction::Defense => [2, 2, 3, 3],
        AirbaseAction::Evasion => [3, 3, 4, 4],
        AirbaseAction::Idle => [4, 4, 5, 5],
        AirbaseAction::Resort => [8, 10, 12, 12],
    };
    by_level[maintenance_level.clamp(0, MAINTENANCE_LEVEL_MAX) as usize]
}

/// A condition after `ticks` recoveries of `rate`: the order restores it up to
/// [`COND_DEPLOYED`], and from there every tick adds one up to [`COND_MAX`].
pub fn recovered_condition(condition: i64, rate: i64, ticks: i64) -> i64 {
    if ticks <= 0 {
        return condition;
    }
    if condition >= COND_DEPLOYED {
        return (condition + ticks).min(COND_MAX);
    }
    let to_deployed = (COND_DEPLOYED - condition + rate - 1) / rate;
    if ticks < to_deployed {
        condition + rate * ticks
    } else {
        (COND_DEPLOYED + ticks - to_deployed).min(COND_MAX)
    }
}

/// Minutes a removed squadron stays in 配置転換, by the area's 整備Lv.
pub fn relocation_minutes(maintenance_level: i64) -> i64 {
    [12, 10, 8, 6][maintenance_level.clamp(0, MAINTENANCE_LEVEL_MAX) as usize]
}

/// Squadron slots every air corps has.
///
/// The client's own limit (`SQUADRON_MAX`, same line), and it draws exactly
/// this many rows — it never derives the count from the response, so the
/// server has to send one `api_plane_info` entry per slot, occupied or not.
pub const SQUADRON_MAX: i64 = 4;

/// Squadron strength and deployability, keyed by equipment type
/// (`api_mst_slotitem`'s `api_type[2]`).
///
/// `None` means the equipment cannot be assigned to a land base at all.
///
/// Both lists are the client's own. Which types may be deployed is what the
/// five tabs of its deployment list offer (`getEquipTypes` in
/// `main.decoded.js`), and the strength is `getKadouCount`, which the client
/// uses to tell whether the bauxite covers a deployment.
pub const fn squadron_capacity(equip_type: i64) -> Option<i64> {
    match equip_type {
        // Reconnaissance and the flying boat fly in fours.
        9 | 10 | 41 | 49 | 59 | 94 => Some(4),
        // 大型陸上機 in nines.
        53 => Some(9),
        // Everything else the list offers flies a full squadron.
        6 | 7 | 8 | 11 | 25 | 26 | 45 | 47 | 48 | 56 | 57 | 58 | 91 => Some(18),
        _ => None,
    }
}

/// User airbase
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Airbase {
    /// Instance id, the `airbase` table primary key
    pub id: i64,

    /// Airbase area id
    pub area_id: i64,

    /// Air base id
    pub rid: i64,

    /// Airbase action
    pub action: AirbaseAction,

    /// Airbase base range
    pub base_range: i64,

    /// Airbase range bonus
    pub bonus_range: i64,

    /// Airbase name
    pub name: String,

    /// maintenance level
    pub maintenance_level: i64,

    /// Squadrons, one entry per slot, `SQUADRON_MAX` of them
    pub planes: Vec<PlaneInfo>,
}

/// Plane status
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub enum PlaneState {
    /// Unassigned
    #[default]
    Unassigned = 0,
    /// Assigned
    Assigned = 1,
    /// Reassigning
    Reassigning = 2,
}

/// User plane(air base) info
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct PlaneInfo {
    /// Profile id
    pub id: i64,

    /// Airbase area id
    pub area_id: i64,

    /// Airbase id
    pub rid: i64,

    /// Slot item instance id
    pub slot_id: i64,

    /// Squadron id, index, starts from 1, up to 4
    pub squadron_id: i64,

    /// plane status
    pub state: PlaneState,

    /// plane condition, the inner value (see [`cond_tier`])
    pub condition: i64,

    /// plane count
    pub count: i64,

    /// plane max count
    pub max_count: i64,
}

impl From<Airbase> for KcApiAirBase {
    fn from(value: Airbase) -> Self {
        Self {
            api_action_kind: value.action as i64,
            api_area_id: value.area_id,
            api_distance: KcApiDistance {
                api_base: value.base_range,
                api_bonus: value.bonus_range,
            },
            api_name: value.name.clone(),
            api_plane_info: value.planes.into_iter().map(std::convert::Into::into).collect(),
            api_rid: value.rid,
        }
    }
}

impl From<PlaneInfo> for KcApiPlaneInfo {
    fn from(value: PlaneInfo) -> Self {
        // Only a squadron actually flying carries a count and a condition.
        // `docs/apilist.txt` marks all three as 未配属なら存在しない, and a live
        // removal answers `{squadron_id, state: 2, slotid}` with nothing else.
        let assigned = matches!(value.state, PlaneState::Assigned);

        Self {
            api_cond: assigned.then_some(cond_tier(value.condition)),
            api_count: assigned.then_some(value.count),
            api_max_count: assigned.then_some(value.max_count),
            api_slotid: value.slot_id,
            api_squadron_id: value.squadron_id,
            api_state: value.state as i64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The condition model's figures, pinned (wikiwiki, 推測される値).
    #[test]
    fn condition_model_is_pinned() {
        assert_eq!(
            [cond_tier(46), cond_tier(30), cond_tier(29), cond_tier(20), cond_tier(19)],
            [1, 1, 2, 2, 3]
        );
        assert_eq!(cond_tier(0), 3);

        assert_eq!(recovery_per_tick(AirbaseAction::Attack, 0), 1);
        assert_eq!(recovery_per_tick(AirbaseAction::Attack, 3), 2);
        assert_eq!(recovery_per_tick(AirbaseAction::Idle, 2), 5);
        assert_eq!(recovery_per_tick(AirbaseAction::Resort, 1), 10);

        // 休息 from red: 0 → 40 in five ticks, then one a tick up to 46.
        assert_eq!(recovered_condition(0, 8, 4), 32);
        assert_eq!(recovered_condition(0, 8, 5), 40);
        assert_eq!(recovered_condition(34, 8, 1), 40, "the order stops at 40");
        assert_eq!(recovered_condition(34, 8, 3), 42);
        assert_eq!(recovered_condition(40, 1, 100_000), 46);
        assert_eq!(recovered_condition(12, 4, 0), 12);

        assert_eq!([0, 1, 2, 3].map(relocation_minutes), [12, 10, 8, 6]);
    }

    /// These figures are the client's (see `squadron_capacity`); a silent flip
    /// must fail here rather than in someone's save file.
    #[test]
    fn squadron_capacity_table_is_pinned() {
        // The five tabs of the client's deployment list.
        let tabs: [&[i64]; 5] = [
            &[47, 53, 91],
            &[48],
            &[6, 56],
            &[7, 8, 26, 57, 58],
            &[9, 10, 11, 25, 41, 45, 49, 59, 94],
        ];
        for equip_type in tabs.concat() {
            let expected = match equip_type {
                9 | 10 | 41 | 49 | 59 | 94 => 4,
                53 => 9,
                _ => 18,
            };
            assert_eq!(squadron_capacity(equip_type), Some(expected), "type {equip_type}");
        }
        // 主砲 and 上陸用舟艇 are on no tab.
        for equip_type in [1, 24] {
            assert_eq!(squadron_capacity(equip_type), None, "type {equip_type}");
        }

        assert_eq!(SQUADRON_MAX, 4);
        assert_eq!(AIRUNIT_MAX, 3);
    }
}
