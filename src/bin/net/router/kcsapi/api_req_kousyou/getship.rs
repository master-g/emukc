use axum::Form;
use serde::{Deserialize, Serialize};

use crate::net::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct Params {
    /// construction dock id
    api_kdock_id: i64,
}

#[derive(Serialize, Deserialize, Debug)]
struct Resp {
    /// ship instance id
    api_id: i64,

    /// ship manifest id
    api_ship_id: i64,

    /// construction dock
    api_kdock: Vec<KcApiKDock>,

    /// ship
    api_ship: KcApiShip,

    /// slot items
    api_slotitem: Option<Vec<KcApiSlotItem>>,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    let (ship, slot_items) = state.complete_ship_construction(pid, params.api_kdock_id).await?;

    let kdocks = state.get_kdocks(pid).await?;
    let api_kdock: Vec<KcApiKDock> = kdocks.into_iter().map(Into::into).collect();

    Ok(KcApiResponse::success(&Resp {
        api_id: ship.api_id,
        api_ship_id: ship.api_ship_id,
        api_kdock,
        api_ship: ship,
        api_slotitem: if slot_items.is_empty() {
            None
        } else {
            Some(slot_items)
        },
    }))
}
