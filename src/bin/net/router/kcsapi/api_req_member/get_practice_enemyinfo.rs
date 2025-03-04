use axum::{Extension, Form};
use emukc::{
	crypto::SimpleHash,
	prelude::{
		KcApiPracticeEnemyDeck, KcApiPracticeEnemyInfo, KcApiPracticeEnemyShip, PracticeOps,
	},
};
use serde::Deserialize;

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
};
// use emukc_internal::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
	api_member_id: i64,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;

	let m = state.get_practice_rival_details(pid, params.api_member_id).await?;

	let resp = KcApiPracticeEnemyInfo {
		api_member_id: m.id,
		api_nickname: m.name.to_owned(),
		api_nickname_id: m.name.simple_hash(),
		api_cmt: m.comment.to_owned(),
		api_cmt_id: m.comment.simple_hash(),
		api_level: m.level,
		api_rank: m.rank as i64,
		api_experience: [m.details.exp_now, m.details.exp_next],
		api_friend: m.details.friend,
		api_ship: [m.details.current_ship_count, m.details.ship_capacity],
		api_slotitem: [m.details.current_slot_item_count, m.details.slot_item_capacity],
		api_furniture: m.details.furniture,
		api_deckname: m.details.deck_name.to_owned(),
		api_deckname_id: m.details.deck_name.simple_hash(),
		api_deck: KcApiPracticeEnemyDeck {
			api_ships: (0..6)
				.map(|i| {
					m.details.ships.get(i).map_or(
						KcApiPracticeEnemyShip {
							api_id: -1,
							api_ship_id: None,
							api_level: None,
							api_star: None,
						},
						|v| KcApiPracticeEnemyShip {
							api_id: v.id,
							api_ship_id: Some(v.mst_id),
							api_level: Some(v.level),
							api_star: Some(v.star),
						},
					)
				})
				.collect(),
		},
	};

	Ok(KcApiResponse::success(&resp))
}
