use axum::{Extension, Router, routing::post};
use serde::{Deserialize, Serialize};

use emukc_internal::prelude::*;

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
};

pub(super) fn router() -> Router {
	Router::new().route("/mxltvkpyuklh", post(handler))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resp {
	api_count: i64,
	api_disp_page: i64,
	api_list: Vec<Item>,
	api_page_count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
	// comment
	api_itbrdpdbkynm: String,
	// comment id
	api_itslcqtmrxtf: i64,
	// name
	api_mtjmdcwtvhdr: String,
	// rank
	api_mxltvkpyuklh: i64,
	// ??
	api_pbgkfylkbjuy: i64,
	// ??
	api_pcumlrymlujh: i64,
	// uid
	api_wuhnhojjxmke: i64,
}

async fn handler(state: AppState, Extension(session): Extension<GameSession>) -> KcApiResult {
	let pid = session.profile.id;
	let (_, basic) = state.get_user_basic(pid).await?;

	let me = Item {
		api_itbrdpdbkynm: basic.api_comment,
		api_itslcqtmrxtf: 1001,
		api_mtjmdcwtvhdr: basic.api_nickname,
		api_mxltvkpyuklh: 1,
		api_pbgkfylkbjuy: 0,
		api_pcumlrymlujh: 3,
		api_wuhnhojjxmke: basic.api_member_id,
	};

	let api_list = (2..=10).map(|rank| {
		let uid = pid + 1001 + rank;
		let api_mtjmdcwtvhdr = format!("Anonymous {}", uid);
		let api_itbrdpdbkynm = format!("Comments from {}", uid);
		Item {
			api_itbrdpdbkynm,
			api_itslcqtmrxtf: 1001 - rank,
			api_mtjmdcwtvhdr,
			api_mxltvkpyuklh: rank,
			api_pbgkfylkbjuy: 0,
			api_pcumlrymlujh: 3,
			api_wuhnhojjxmke: uid,
		}
	});
	let api_list: Vec<Item> = std::iter::once(me).chain(api_list).collect();

	let resp = Resp {
		api_count: 1000,
		api_disp_page: 1,
		api_list,
		api_page_count: 100,
	};

	Ok(KcApiResponse::success(&resp))
}
