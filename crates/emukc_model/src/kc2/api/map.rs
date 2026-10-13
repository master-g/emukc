use serde::{Deserialize, Serialize};

use super::KcApiEventmap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KcApiMapCellData {
    pub api_id: i64,
    pub api_no: i64,
    pub api_color_no: i64,
    pub api_passed: i64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_distance: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KcApiMapAirSearch {
    pub api_plane_type: i64,
    pub api_result: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KcApiMapEnemyDeckInfo {
    pub api_kind: i64,
    pub api_ship_ids: Vec<i64>,
}

/// Resource acquisition at a non-battle node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KcApiMapItemGet {
    pub api_id: i64,
    pub api_getcount: i64,
    pub api_name: String,
    pub api_icon_id: i64,
    pub api_usemst: i64,
}

/// Maelstrom (渦潮) resource loss at a non-battle node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KcApiMapHappening {
    /// Always 1.
    pub api_type: i64,
    pub api_count: i64,
    /// 1 if a 電探 (radar) reduced the loss
    pub api_dentan: i64,
    /// What was lost: 1=fuel, 2=ammo. The client reads this one.
    pub api_mst_id: i64,
    pub api_icon_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KcApiMapStart {
    pub api_cell_data: Vec<KcApiMapCellData>,
    pub api_rashin_flg: i64,
    pub api_rashin_id: i64,
    pub api_maparea_id: i64,
    pub api_mapinfo_no: i64,
    pub api_no: i64,
    pub api_color_no: i64,
    pub api_event_id: i64,
    pub api_event_kind: i64,
    pub api_next: i64,
    pub api_bosscell_no: i64,
    pub api_bosscomp: i64,
    pub api_from_no: i64,
    pub api_limit_state: i64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_eventmap: Option<KcApiEventmap>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_airsearch: Option<KcApiMapAirSearch>,

    /// The cells the player may pick from, where this cell is a choice of route.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_select_route: Option<KcApiMapSelectRoute>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_e_deck_info: Option<Vec<KcApiMapEnemyDeckInfo>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_itemget: Option<Vec<KcApiMapItemGet>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_happening: Option<KcApiMapHappening>,
}

/// The ways out of a cell where the player picks the route (能動分岐).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KcApiMapSelectRoute {
    /// Cell numbers, to come back as `api_cell_id`.
    pub api_select_cells: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KcApiMapNext {
    pub api_rashin_flg: i64,
    pub api_rashin_id: i64,
    pub api_maparea_id: i64,
    pub api_mapinfo_no: i64,
    pub api_no: i64,
    pub api_color_no: i64,
    pub api_event_id: i64,
    pub api_event_kind: i64,
    pub api_next: i64,
    pub api_bosscell_no: i64,
    pub api_bosscomp: i64,
    pub api_from_no: i64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_comment_kind: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_production_kind: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_airsearch: Option<KcApiMapAirSearch>,

    /// The cells the player may pick from, where this cell is a choice of route.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_select_route: Option<KcApiMapSelectRoute>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_e_deck_info: Option<Vec<KcApiMapEnemyDeckInfo>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_limit_state: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_itemget: Option<Vec<KcApiMapItemGet>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_happening: Option<KcApiMapHappening>,

    /// The raid on the air base this step brought, laid out as a battle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_destruction_battle: Option<serde_json::Value>,

    /// The resource an escort-success cell gives.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_itemget_eo_comment: Option<KcApiMapEoItem>,

    /// What the arrival that empties the gauge gives besides.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_itemget_eo_result: Option<KcApiMapEoItem>,

    /// The ranking points (戦果) of that arrival.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_get_eo_rate: Option<i64>,
}

/// One thing an escort-success cell (船団護衛成功) gives.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KcApiMapEoItem {
    /// 4 for a material, 5 for a use item.
    pub api_usemst: i64,
    pub api_id: i64,
    pub api_getcount: i64,
}
