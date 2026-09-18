use crate::net::prelude::*;

pub(super) async fn handler(state: AppState, Pid(pid): Pid) -> KcApiResult {
    let fleets = state.get_fleets(pid).await?;
    let deck_ports: Vec<KcApiDeckPort> = fleets.into_iter().map(std::convert::Into::into).collect();

    Ok(KcApiResponse::success(&deck_ports))
}
