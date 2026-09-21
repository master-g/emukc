use axum::Form;
use serde::{Deserialize, Serialize};

use crate::net::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
    /// Recipe id, as handed out by `remodel_slotlist`.
    pub(super) api_id: i64,
    /// Instance id of the equipment to improve.
    pub(super) api_slot_id: i64,
    /// Pay extra to guarantee success.
    #[serde(deserialize_with = "crate::net::router::kcsapi::form_utils::deserialize_form_flag")]
    pub(super) api_certain_flag: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resp {
    api_remodel_flag: i64,
    /// `[before, after]` equipment master id.
    api_remodel_id: [i64; 2],
    api_after_material: Vec<i64>,
    api_voice_ship_id: i64,
    api_voice_id: i64,
    /// Absent on a failed attempt, per the upstream contract.
    #[serde(skip_serializing_if = "Option::is_none")]
    api_after_slot: Option<KcApiSlotItem>,
    /// Present only when the attempt ate equipment.
    #[serde(skip_serializing_if = "Option::is_none")]
    api_use_slot_id: Option<Vec<i64>>,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    let result =
        state.remodel_slot(pid, params.api_id, params.api_slot_id, params.api_certain_flag).await?;

    let api_after_material: Vec<KcApiMaterialElement> = result.after_material.into();
    let api_after_material: Vec<i64> =
        api_after_material.into_iter().map(|v| v.api_value).collect();

    Ok(KcApiResponse::success(&Resp {
        api_remodel_flag: i64::from(result.success),
        api_remodel_id: result.remodel_id,
        api_after_material,
        api_voice_ship_id: result.voice_ship_id,
        api_voice_id: result.voice_id,
        api_after_slot: result.after_slot,
        api_use_slot_id: (!result.used_slot_ids.is_empty()).then_some(result.used_slot_ids),
    }))
}
