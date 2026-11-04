use axum::Form;
use serde::{Deserialize, Serialize};

use crate::net::err::ApiError;
use crate::net::prelude::*;

#[derive(Deserialize, Serialize, Debug)]
pub(super) struct Params {
    /// 0: disban all except flagship
    /// x: operate on ship x
    api_ship_idx: i64,
    /// fleet id
    api_id: i64,
    /// -1: for disband
    /// -2: disband all except flagship
    /// x: target ship id
    api_ship_id: i64,
}

#[derive(Serialize, Deserialize, Debug)]
struct Resp {
    #[serde(skip_serializing_if = "Option::is_none")]
    api_change_count: Option<i64>,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    let fleet = state.get_fleet(pid, params.api_id).await?;

    let origin_ship_idx = if params.api_ship_id < 0 {
        params.api_ship_id
    } else {
        let pos = fleet.ships.iter().position(|s| *s == params.api_ship_id);
        pos.map(|p| p as i64).unwrap_or(-1)
    };

    let source_ship_id = fleet.ships.get(params.api_ship_idx as usize).ok_or_else(|| {
        ApiError::NotFound(format!(
            "ship idx {} not found in fleet {}",
            params.api_ship_idx, params.api_id
        ))
    })?;

    let (mut new_fleet, count) = if origin_ship_idx == -2 {
        // disband all except flagship
        let count = fleet.ships.iter().skip(1).fold(0, |acc, &s| {
            if s > 0 {
                acc + 1
            } else {
                acc
            }
        });
        let count = if count > 0 {
            Some(count)
        } else {
            None
        };
        ([fleet.ships[0], -1, -1, -1, -1, -1], count)
    } else {
        let mut nf = fleet.ships;
        nf[params.api_ship_idx as usize] = params.api_ship_id;
        if origin_ship_idx >= 0 {
            // swap
            nf[origin_ship_idx as usize] = *source_ship_id;
        }
        (nf, None)
    };

    new_fleet.move_value_to_end(-1);

    state.update_fleet_ships(pid, params.api_id, &new_fleet).await?;

    Ok(KcApiResponse::success(&Resp {
        api_change_count: count,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::router::kcsapi::test_utils::{app_state, new_test_context};

    #[tokio::test]
    async fn change_moves_removes_and_disbands_ships() {
        let context = new_test_context().await;
        let state = &context.state;
        let pid = context.session.profile.id;
        let mut ids = Vec::new();
        for _ in 0..4 {
            ids.push(state.add_ship(pid, 1).await.unwrap().api_id);
        }
        let [a, b, c, d] = ids[..] else {
            unreachable!()
        };

        let cases = [
            ("flagship to slot 3", 2, a, vec![c, b, a]),
            ("swap in fleet", 0, b, vec![b, a, c]),
            ("remove", 1, -1, vec![a, c]),
            ("keep only flagship", 0, -2, vec![a]),
            ("add from the port", 3, d, vec![a, b, c, d]),
        ];
        for (name, api_ship_idx, api_ship_id, want) in cases {
            state.update_fleet_ships(pid, 1, &[a, b, c, -1, -1, -1]).await.unwrap();
            let params = Params {
                api_ship_idx,
                api_id: 1,
                api_ship_id,
            };
            handler(app_state(state), Pid(pid), Form(params)).await.unwrap();

            let ships = state.get_fleet(pid, 1).await.unwrap().ships;
            let got: Vec<i64> = ships.into_iter().filter(|&id| id > 0).collect();
            assert_eq!(got, want, "{name}");
        }
    }
}
