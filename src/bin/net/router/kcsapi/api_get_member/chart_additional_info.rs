use crate::net::prelude::*;

pub(super) async fn handler(state: AppState, Pid(pid): Pid) -> KcApiResult {
    let fleets = state.get_fleets(pid).await?;

    let mut deck_params = Vec::new();

    for _ in fleets {
        let param = KcApiDeckParam {
            api_seiku_value: 0,
            api_tp_value: 0,
            api_atp_value: None,
        };
        deck_params.push(param);
    }

    Ok(KcApiResponse::success(&KcApiChartAdditionalInfo {
        api_deck_param: deck_params,
    }))
}
