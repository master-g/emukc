use serde::{Deserialize, Serialize};

use crate::kc2::{KcApiAirBase, KcApiDistance, KcApiPlaneInfo};

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
/// Upstream publishes neither list: `api_mst_slotitem_equiptype` carries only
/// a name and a picture-book flag, and the client reads `api_max_count` off the
/// response rather than computing it. Both tables are therefore this project's
/// own reading of the game's rules. Types whose eligibility is genuinely
/// unclear — 水上爆撃機, 対潜哨戒機, 水上戦闘機, オートジャイロ — are excluded
/// rather than guessed in.
pub const fn squadron_capacity(equip_type: i64) -> Option<i64> {
    match equip_type {
        // Reconnaissance flies in fours.
        9 | 10 | 49 | 59 | 94 => Some(4),
        // A flying boat is a single aircraft.
        41 => Some(1),
        // Fighters, bombers and the land-based line fly full squadrons.
        6 | 7 | 8 | 47 | 48 | 53 | 56 | 57 | 58 | 91 => Some(18),
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

    /// plane condition
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
            api_cond: assigned.then_some(value.condition),
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

    /// These figures are this project's own (see `squadron_capacity`), so a
    /// silent flip must fail here rather than in someone's save file.
    #[test]
    fn squadron_capacity_table_is_pinned() {
        // 陸上攻撃機, 局地戦闘機, 艦上戦闘機, 艦上攻撃機 — full squadrons.
        for equip_type in [47, 48, 6, 8] {
            assert_eq!(squadron_capacity(equip_type), Some(18), "type {equip_type}");
        }
        // 艦上偵察機, 水上偵察機, 陸上偵察機 — four aircraft.
        for equip_type in [9, 10, 49] {
            assert_eq!(squadron_capacity(equip_type), Some(4), "type {equip_type}");
        }
        assert_eq!(squadron_capacity(41), Some(1), "大型飛行艇 flies alone");

        // 水上爆撃機, 対潜哨戒機, 水上戦闘機, オートジャイロ, 主砲 — not deployable.
        for equip_type in [11, 26, 45, 25, 1] {
            assert_eq!(squadron_capacity(equip_type), None, "type {equip_type}");
        }

        assert_eq!(SQUADRON_MAX, 4);
        assert_eq!(AIRUNIT_MAX, 3);
    }
}
