use axum::{Extension, Form};
use rand::{RngCore, SeedableRng, rngs::SmallRng, seq::IndexedRandom};
use serde::{Deserialize, Serialize};

use emukc_internal::prelude::*;

use crate::net::{
	AppState,
	auth::GameSession,
	err::ApiError,
	resp::{KcApiResponse, KcApiResult},
};

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct Params {
	/// fuel
	api_item1: i64,
	/// ammo
	api_item2: i64,
	/// steel
	api_item3: i64,
	/// bauxite
	api_item4: i64,
	/// 0: normal, 1: three in a row
	api_multiple_flag: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resp {
	api_create_flag: i64,
	api_get_items: Vec<GetItem>,
	api_material: Vec<i64>,
	api_unset_items: Option<Vec<UnsetItem>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetItem {
	api_id: i64,
	api_slotitem_id: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnsetItem {
	api_slot_list: Vec<i64>,
	api_type3: i64,
}

#[axum_macros::debug_handler]
pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;
	let codex = state.codex();

	let costs = if params.api_multiple_flag == 0 {
		vec![
			(MaterialCategory::Fuel, params.api_item1),
			(MaterialCategory::Ammo, params.api_item2),
			(MaterialCategory::Steel, params.api_item3),
			(MaterialCategory::Bauxite, params.api_item4),
			(MaterialCategory::DevMat, 1),
		]
	} else {
		vec![
			(MaterialCategory::Fuel, params.api_item1 * 3),
			(MaterialCategory::Ammo, params.api_item2 * 3),
			(MaterialCategory::Steel, params.api_item3 * 3),
			(MaterialCategory::Bauxite, params.api_item4 * 3),
			(MaterialCategory::DevMat, 3),
		]
	};

	let pool: Vec<Kc3rdSlotItem> = codex
		.slotitem_extra_info
		.iter()
		.filter_map(|(_, info)| {
			if !info.craftable {
				None
			} else {
				Some(info.clone())
			}
		})
		.collect();

	let mut crafted_mst_ids: Vec<i64> = Vec::new();

	let upper = if params.api_multiple_flag == 1 {
		3
	} else {
		1
	};

	let mut r = SmallRng::from_os_rng();

	for _ in 0..upper {
		let next_u32 = r.next_u32() % 100;
		if next_u32 % 100 > 30 {
			let item = pool.choose(&mut r).ok_or(ApiError::Internal(
				"cannot pick random from slotitem crafting pool".to_owned(),
			))?;
			crafted_mst_ids.push(item.api_id);
		} else {
			crafted_mst_ids.push(-1);
		}
	}

	let (ids, material) = state.create_slotitem(pid, &crafted_mst_ids, &costs).await?;

	let api_create_flag = if ids.iter().any(|v| *v > 0) {
		1
	} else {
		0
	};
	let api_material: Vec<KcApiMaterialElement> = material.into();
	let api_material: Vec<i64> = api_material.into_iter().map(|v| v.api_value).collect();
	let api_get_items: Vec<GetItem> = crafted_mst_ids
		.iter()
		.zip(ids)
		.map(|(mst_id, id)| GetItem {
			api_id: id,
			api_slotitem_id: *mst_id,
		})
		.collect();

	let api_unset_items = if api_create_flag != 0 {
		let crafted_types = crafted_mst_ids
			.iter()
			.filter_map(|id| {
				if *id > 0 {
					codex.find::<ApiMstSlotitem>(id).map(|mst| mst.api_type[2]).ok()
				} else {
					None
				}
			})
			.collect::<Vec<i64>>();
		let unset_items = state.get_unset_slot_items(pid).await?;
		let unset_items = unset_items
			.iter()
			.filter_map(|item| {
				let mst = codex.find::<ApiMstSlotitem>(&item.api_slotitem_id).ok()?;
				if crafted_types.contains(&mst.api_type[2]) {
					Some((mst.api_type[2], item.api_id))
				} else {
					None
				}
			})
			.fold(Vec::<UnsetItem>::new(), |mut acc, (t, id)| {
				if let Some(item) = acc.iter_mut().find(|v| v.api_type3 == t) {
					item.api_slot_list.push(id);
				} else {
					acc.push(UnsetItem {
						api_slot_list: vec![id],
						api_type3: t,
					});
				}
				acc
			});
		Some(unset_items)
	} else {
		None
	};

	Ok(KcApiResponse::success(&Resp {
		api_create_flag,
		api_get_items,
		api_material,
		api_unset_items,
	}))
}
