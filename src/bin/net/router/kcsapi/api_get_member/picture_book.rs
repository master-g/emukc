use std::collections::BTreeMap;

use axum::{Extension, Form};
use serde::{Deserialize, Serialize};

use emukc_internal::{model::profile::picture_book::PictureBookShip, prelude::*};

use crate::net::{
	auth::GameSession,
	err::ApiError,
	resp::{KcApiError, KcApiResponse, KcApiResult},
	AppState,
};

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct Params {
	/// book type, 1 for ship, 2 for equipment
	api_type: i64,

	// page
	api_no: i64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ShipItem {
	api_index_no: i64,
	/// [][5], 0: reigster, 1: medium damaged, 2: married, 3: ?, 4: ?
	api_state: Vec<[i64; 5]>,
	api_q_voice_info: Vec<KcApiShipQVoiceInfo>,
	api_table_id: Vec<i64>,
	api_name: String,
	api_yomi: String,
	api_stype: i64,
	api_ctype: i64,
	api_cnum: i64,
	#[serde(skip_serializing_if = "Option::is_none")]
	api_taik: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	api_souk: Option<i64>,
	api_kaih: i64,
	#[serde(skip_serializing_if = "Option::is_none")]
	api_houg: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	api_raig: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	api_tyku: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	api_tais: Option<i64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	api_leng: Option<i64>,
	api_sinfo: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct SlotItem {
	api_index_no: i64,
	api_state: Vec<i64>,
	api_table_id: Vec<i64>,
	api_name: String,
	api_type: [i64; 5],
	api_souk: i64,
	api_houg: i64,
	api_raig: i64,
	api_soku: i64,
	api_baku: i64,
	api_tyku: i64,
	api_tais: i64,
	api_houm: i64,
	api_houk: i64,
	api_saku: i64,
	api_leng: i64,
	api_flag: Vec<i64>,
	api_info: String,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let start_index = (params.api_no - 1) * 70 + 1;
	let end_index = start_index + 70;

	let pid = session.profile.id;

	match params.api_type {
		1 => ship_book(state, pid, start_index, end_index).await,
		2 => item_book(state, pid, start_index, end_index).await,
		_ => Err(KcApiError::from(ApiError::Unknown(format!(
			"unknown book type {}",
			params.api_type
		)))),
	}
}

async fn ship_book(state: AppState, pid: i64, start_index: i64, end_index: i64) -> KcApiResult {
	let codex = state.codex();

	let records = state.get_ship_picturebook(pid).await?;
	let mut map: BTreeMap<i64, PictureBookShip> = BTreeMap::new();
	for r in records {
		map.insert(r.sort_num, r);
	}

	let mut data: Vec<ShipItem> = Vec::new();
	for sort_no in start_index..end_index {
		let mst = codex.manifest.api_mst_ship.iter().find(|m| m.api_sortno == Some(sort_no));
		let Some(mst) = mst else {
			warn!("ship mst sortno {} not found", sort_no);
			continue;
		};

		let basic = match codex.find_ship_extra(mst.api_id) {
			Ok(basic) => basic,
			Err(_) => &{
				warn!("ship extra id {} not found or cannot be loaded", mst.api_id);
				Kc3rdShip {
					api_id: mst.api_id,
					kaih: [-1, -1],
					tais: [-1, -1],
					saku: [-1, -1],
					luck: [-1, -1],
					cnum: 1,
					slots: vec![],
					luck_bonus: 0f64,
					armor_bonus: 0,
					buildable: false,
					buildable_lsc: false,
					remodel: None,
					remodel_back_to: 0,
					remodel_back_requirement: None,
				}
				// continue;
			},
		};

		let Ok(picturebook_info) = codex.find_ship_picturebook(mst.api_id) else {
			warn!("ship picturebook id {} not found or cannot be loaded", mst.api_id);
			continue;
		};

		let record = if codex.picturebook_extra.unlock_all_ships {
			// force unlock all
			(true, true)
		} else if let Some(record) = map.get(&sort_no) {
			// unlock based on record
			(record.damaged, record.married)
		} else {
			// skip if not locked
			continue;
		};

		let voices = codex.picturebook_extra.voice_map.get(&sort_no).cloned();
		let api_q_voice_info = voices.unwrap_or_default();

		if sort_no == 566 {
			info!("ship sort no 566 found!");
		}

		data.push(ShipItem {
			api_index_no: sort_no,
			api_state: vec![[1, record.0 as i64, record.1 as i64, 0, 0]],
			api_q_voice_info,
			api_table_id: vec![mst.api_id],
			api_name: mst.api_name.clone(),
			api_yomi: mst.api_yomi.clone(),
			api_stype: mst.api_stype,
			api_ctype: mst.api_ctype,
			api_cnum: basic.cnum,
			api_taik: mst.api_taik.as_ref().map(|v| v[0]),
			api_souk: mst.api_souk.as_ref().map(|v| v[0]),
			api_kaih: basic.kaih[0],
			api_houg: mst.api_houg.as_ref().map(|v| v[0]),
			api_raig: mst.api_raig.as_ref().map(|v| v[0]),
			api_tyku: mst.api_tyku.as_ref().map(|v| v[0]),
			api_tais: mst.api_tais.as_ref().map(|v| v[0]),
			api_leng: mst.api_leng,
			api_sinfo: picturebook_info.info.clone(),
		});
	}

	Ok(KcApiResponse::success_json(serde_json::json!({
		"api_list": data,
	})))
}

async fn item_book(state: AppState, pid: i64, start_index: i64, end_index: i64) -> KcApiResult {
	let codex = state.codex();
	let records = state.get_slot_item_picturebook(pid).await?;
	let records: Vec<i64> = records.into_iter().map(|v| v.sort_num).collect();

	let mut data: Vec<SlotItem> = Vec::new();
	for sort_no in start_index..end_index {
		let mst = codex.manifest.api_mst_slotitem.iter().find(|m| m.api_sortno == sort_no);
		let Some(mst) = mst else {
			continue;
		};
		let Ok(extra) = codex.find::<Kc3rdSlotItem>(&mst.api_id) else {
			warn!("slotitem extra id {} not found or cannot be loaded", mst.api_id);
			continue;
		};
		if !codex.picturebook_extra.unlock_all_slotitems && !records.contains(&sort_no) {
			continue;
		}

		data.push(SlotItem {
			api_index_no: sort_no,
			api_state: vec![1, 0, 0, 0, 0],
			api_table_id: vec![sort_no],
			api_name: mst.api_name.clone(),
			api_type: mst.api_type,
			api_souk: mst.api_souk,
			api_houg: mst.api_houg,
			api_raig: mst.api_raig,
			api_soku: mst.api_soku,
			api_baku: mst.api_baku,
			api_tyku: mst.api_tyku,
			api_tais: mst.api_tais,
			api_houm: mst.api_houm,
			api_houk: mst.api_houk,
			api_saku: mst.api_saku,
			api_leng: mst.api_leng,
			api_flag: vec![0, 0, 0, 0, 0, 0, 0, 0],
			api_info: extra.info.clone(),
		});
	}

	Ok(KcApiResponse::success_json(serde_json::json!({
		"api_list": data,
	})))
}
