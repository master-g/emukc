//! One account captured from the live server by `scripts/fetch_live_api.py`.
//!
//! Only what the first import slice reads. Each field is taken out of `api_data` by
//! name instead of parsing a whole response, because the local `api_port/port` response
//! type does not match what the live server sends (`api_plane_info` and
//! `api_furniture_affect_items` are declared wrong).

use serde::de::DeserializeOwned;
use serde_json::Value;

use super::{
    KcApiDeckPort, KcApiFurniture, KcApiKDock, KcApiMaterialElement, KcApiNDock, KcApiOssSetting,
    KcApiShip, KcApiSlotItem, KcApiUserBasic, KcApiUserItem,
};

const PORT: &str = "api_port/port";
const REQUIRE_INFO: &str = "api_get_member/require_info";

/// An account's state from the `api_data` of `api_port/port` and `api_get_member/require_info`.
#[derive(Debug)]
pub struct LiveSnapshot {
    /// Port `api_basic`.
    pub basic: KcApiUserBasic,
    /// Port `api_ship`.
    pub ships: Vec<KcApiShip>,
    /// Port `api_deck_port`.
    pub decks: Vec<KcApiDeckPort>,
    /// Port `api_ndock`.
    pub ndocks: Vec<KcApiNDock>,
    /// Port `api_material`.
    pub materials: Vec<KcApiMaterialElement>,
    /// Port `api_p_bgm_id`.
    pub port_bgm: i64,
    /// Port `api_parallel_quest_count`.
    pub parallel_quest_count: i64,
    /// Require-info `api_slot_item`.
    pub slot_items: Vec<KcApiSlotItem>,
    /// Require-info `api_kdock`.
    pub kdocks: Vec<KcApiKDock>,
    /// Require-info `api_useitem`.
    pub use_items: Vec<KcApiUserItem>,
    /// Require-info `api_furniture`.
    pub furniture: Vec<KcApiFurniture>,
    /// Require-info `api_oss_setting`.
    pub oss: KcApiOssSetting,
    /// Require-info `api_skin_id`.
    pub skin_id: i64,
    /// Require-info `api_position_id`.
    pub position_id: i64,
    /// Require-info `api_extra_supply`: `[expedition, sortie]`.
    pub extra_supply: [i64; 2],
}

/// Why a capture could not be read as a [`LiveSnapshot`].
#[derive(Debug, thiserror::Error)]
pub enum LiveSnapshotError {
    /// The field is absent from the endpoint's `api_data`.
    #[error("{endpoint}: `{field}` is missing")]
    Missing {
        /// The endpoint.
        endpoint: &'static str,
        /// The field.
        field: &'static str,
    },
    /// The field is there but not of the expected shape.
    #[error("{endpoint}: `{field}` does not parse: {source}")]
    Invalid {
        /// The endpoint.
        endpoint: &'static str,
        /// The field.
        field: &'static str,
        /// The parse error.
        source: serde_json::Error,
    },
}

impl LiveSnapshot {
    /// Read a snapshot from the two endpoints' `api_data` objects.
    pub fn from_api_data(port: &Value, require_info: &Value) -> Result<Self, LiveSnapshotError> {
        Ok(Self {
            basic: take(port, PORT, "api_basic")?,
            ships: take(port, PORT, "api_ship")?,
            decks: take(port, PORT, "api_deck_port")?,
            ndocks: take(port, PORT, "api_ndock")?,
            materials: take(port, PORT, "api_material")?,
            port_bgm: take(port, PORT, "api_p_bgm_id")?,
            parallel_quest_count: take(port, PORT, "api_parallel_quest_count")?,
            slot_items: take(require_info, REQUIRE_INFO, "api_slot_item")?,
            kdocks: take(require_info, REQUIRE_INFO, "api_kdock")?,
            use_items: take(require_info, REQUIRE_INFO, "api_useitem")?,
            furniture: take(require_info, REQUIRE_INFO, "api_furniture")?,
            oss: take(require_info, REQUIRE_INFO, "api_oss_setting")?,
            skin_id: take(require_info, REQUIRE_INFO, "api_skin_id")?,
            position_id: take(require_info, REQUIRE_INFO, "api_position_id")?,
            extra_supply: take(require_info, REQUIRE_INFO, "api_extra_supply")?,
        })
    }
}

fn take<T: DeserializeOwned>(
    data: &Value,
    endpoint: &'static str,
    field: &'static str,
) -> Result<T, LiveSnapshotError> {
    let value = data.get(field).ok_or(LiveSnapshotError::Missing {
        endpoint,
        field,
    })?;
    T::deserialize(value).map_err(|source| LiveSnapshotError::Invalid {
        endpoint,
        field,
        source,
    })
}
