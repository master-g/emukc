use axum::Form;
use serde::{Deserialize, Serialize};

use crate::net::prelude::*;

#[derive(Deserialize, Serialize, Debug)]
pub(super) struct Params {
    api_type: i64,
    api_no: i64,
    #[serde(
        default,
        deserialize_with = "crate::net::router::kcsapi::form_utils::deserialize_form_flag"
    )]
    api_discount_flag: bool,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    let codex = state.codex();
    let mst = codex.find::<ApiMstFurniture>(&params.api_no)?;

    let price = mst.api_price;
    let needs_craftman = Codex::furniture_needs_craftman(price);
    let consume_craftman = needs_craftman || params.api_discount_flag;

    state.buy_furniture(pid, params.api_no, price, consume_craftman).await?;

    Ok(KcApiResponse::empty())
}
