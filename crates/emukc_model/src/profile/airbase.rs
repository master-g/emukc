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
        // An unassigned slot carries neither a count nor a condition —
        // `docs/apilist.txt` marks all three as 未配属なら存在しない.
        let assigned = !matches!(value.state, PlaneState::Unassigned);

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
