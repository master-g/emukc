use axum::{Form, Router, routing::post};
use serde::{Deserialize, Serialize};

use emukc::bootstrap::prelude::port_api_seed;

use crate::net::prelude::*;

pub(super) fn router() -> Router {
    Router::new().route("/mxltvkpyuklh", post(handler))
}

#[derive(Debug, Deserialize)]
struct Params {
    /// The page asked for; absent (0) for the asker's own.
    #[serde(default)]
    api_pageno: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resp {
    api_count: i64,
    api_disp_page: i64,
    api_list: Vec<Item>,
    api_page_count: i64,
}

/// One line, under the names the client reads (`main.js` `SetAll`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    /// Comment.
    api_itbrdpdbkynm: String,
    /// 勲章, encoded ([`encode_medals`]).
    api_itslcqtmrxtf: i64,
    /// Name.
    api_mtjmdcwtvhdr: String,
    /// Place.
    api_mxltvkpyuklh: i64,
    /// Flag type; the client does not draw it.
    api_pbgkfylkbjuy: i64,
    /// Headquarters rank (`api_rank`), which picks the flag.
    api_pcumlrymlujh: i64,
    /// 戦果, encoded ([`encode_war_result`]).
    api_wuhnhojjxmke: i64,
    /// Scores the client keeps and does not draw.
    api_xlqcmisdyfiu: i64,
    api_mcouotbbbzpx: i64,
}

/// The client's keys by place (`__drawRanking`), `rank % 13`.
const RANK_KEYS: [i64; 13] =
    [8931, 1201, 1156, 5061, 4569, 4732, 3779, 4568, 5695, 4619, 4912, 5669, 6586];
/// What the client shows as `raw / RANK_KEYS[rank % 13] / port_seed - 91`, where
/// `port_seed` is `PORT_API_SEED[viewer % 10]` without its last two digits.
fn encode_war_result(war_result: i64, rank: i64, viewer: i64) -> i64 {
    let port_seed = port_api_seed()[viewer as usize % 10] / 100;
    (war_result + 91) * RANK_KEYS[rank as usize % 13] * port_seed
}

/// What the client shows as `raw / (RANK_KEYS[rank % 13] + 1853) - 157`.
fn encode_medals(medals: i64, rank: i64) -> i64 {
    (medals + 157) * (RANK_KEYS[rank as usize % 13] + 1853)
}

async fn handler(state: AppState, Pid(pid): Pid, Form(params): Form<Params>) -> KcApiResult {
    let page = state.ranking(pid, params.api_pageno).await?;

    let api_list = page
        .entries
        .into_iter()
        .map(|entry| Item {
            api_itbrdpdbkynm: entry.profile.comment,
            api_itslcqtmrxtf: encode_medals(entry.profile.medals, entry.rank),
            api_mtjmdcwtvhdr: entry.profile.name,
            api_mxltvkpyuklh: entry.rank,
            api_pbgkfylkbjuy: 0,
            api_pcumlrymlujh: entry.profile.hq_rank,
            api_wuhnhojjxmke: encode_war_result(entry.war_result, entry.rank, pid),
            api_xlqcmisdyfiu: 0,
            api_mcouotbbbzpx: 0,
        })
        .collect();

    let resp = Resp {
        api_count: page.count,
        api_disp_page: page.page,
        api_list,
        api_page_count: page.page_count,
    };

    Ok(KcApiResponse::success(&resp))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The client's decoding, as `__drawRanking` and `_getPortSeed` write it, with the
    /// seed table the decoded client exports.
    fn shown(raw: i64, rank: i64, viewer: i64) -> f64 {
        let key = RANK_KEYS[rank as usize % 13] as f64;
        let seed = port_api_seed()[viewer as usize % 10];
        let seed = ((seed - seed % 100) / 100) as f64;
        raw as f64 / key / seed - 91.0
    }

    #[test]
    fn the_client_decodes_what_is_encoded() {
        for (war_result, rank, viewer) in [(0, 1, 1), (75, 13, 7), (1234, 101, 1000), (9, 26, 9)] {
            let raw = encode_war_result(war_result, rank, viewer);
            assert_eq!(shown(raw, rank, viewer), war_result as f64);
        }
        for (medals, rank) in [(0, 1), (3, 12)] {
            let key = RANK_KEYS[rank as usize % 13] as f64;
            let decoded = encode_medals(medals, rank) as f64 / (key + 1853.0) - 157.0;
            assert_eq!(decoded, medals as f64);
        }
    }
}
