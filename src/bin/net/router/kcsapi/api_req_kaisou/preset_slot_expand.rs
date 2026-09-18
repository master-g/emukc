use serde::{Deserialize, Serialize};

use crate::net::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
struct Resp {
    api_max_num: i64,
}

pub(super) async fn handler(state: AppState, Pid(pid): Pid) -> KcApiResult {
    let api_max_num = state.expand_preset_slot_capacity(pid).await?;

    Ok(KcApiResponse::success(&Resp {
        api_max_num,
    }))
}
