use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use emukc_db::{
	entity::profile::map_record,
	sea_orm::{ActiveValue, IntoActiveModel, QueryOrder, TransactionTrait, entity::prelude::*},
};
use emukc_model::{
	codex::{
		Codex,
		map::{MapCatalog, MapDefinition, MapResetPolicy},
	},
	kc2::{KcApiEventmap, KcApiMapInfo},
	profile::map_record::MapSelectRank,
};
use emukc_time::{KcTime, chrono::Utc};

use crate::{err::GameplayError, gameplay::HasContext};

use super::{basic::find_profile, fleet::get_fleet_ships_impl};

#[derive(Debug, Clone)]
pub struct EventMapRankSelection {
	pub now_maphp: i64,
	pub max_maphp: i64,
	pub gauge_type: i64,
	pub gauge_num: i64,
	pub sally_flag: [i64; 3],
}

/// A trait for map related gameplay.
#[async_trait]
pub trait MapOps {
	/// Get map records of a profile.
	async fn get_map_records(
		&self,
		profile_id: i64,
	) -> Result<Vec<map_record::Model>, GameplayError>;

	/// Get map info view for KCS API.
	async fn get_map_infos(&self, profile_id: i64) -> Result<Vec<KcApiMapInfo>, GameplayError>;

	/// Select event map rank.
	async fn select_eventmap_rank(
		&self,
		profile_id: i64,
		maparea_id: i64,
		mapinfo_no: i64,
		rank: i64,
	) -> Result<EventMapRankSelection, GameplayError>;

	/// Get current combined fleet type.
	async fn get_combined_type(&self, profile_id: i64) -> Result<i64, GameplayError>;

	/// Set current combined fleet type.
	async fn set_combined_type(
		&self,
		profile_id: i64,
		combined_type: i64,
	) -> Result<i64, GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> MapOps for T {
	async fn get_map_records(
		&self,
		profile_id: i64,
	) -> Result<Vec<map_record::Model>, GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let tx = db.begin().await?;

		ensure_map_records_impl(&tx, codex, profile_id).await?;
		refresh_all_map_records_impl(&tx, codex, profile_id).await?;
		let records = get_map_records_impl(&tx, profile_id).await?;

		tx.commit().await?;
		Ok(records)
	}

	async fn get_map_infos(&self, profile_id: i64) -> Result<Vec<KcApiMapInfo>, GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let tx = db.begin().await?;

		ensure_map_records_impl(&tx, codex, profile_id).await?;
		refresh_all_map_records_impl(&tx, codex, profile_id).await?;
		let records = get_map_records_impl(&tx, profile_id).await?;
		let infos = build_map_infos(codex, records);

		tx.commit().await?;
		Ok(infos)
	}

	async fn select_eventmap_rank(
		&self,
		profile_id: i64,
		maparea_id: i64,
		mapinfo_no: i64,
		rank: i64,
	) -> Result<EventMapRankSelection, GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let tx = db.begin().await?;

		ensure_map_records_impl(&tx, codex, profile_id).await?;
		refresh_all_map_records_impl(&tx, codex, profile_id).await?;
		let definition = find_map_definition(codex, maparea_id, mapinfo_no)?;
		if !definition.is_event {
			return Err(GameplayError::WrongType(format!(
				"map {maparea_id}-{mapinfo_no} is not an event map",
			)));
		}

		let selected_rank = map_record::SelectedRank::from(parse_map_select_rank(rank)?);
		let record = find_map_record_impl(&tx, profile_id, definition.map_id).await?;
		let current_hp = record.current_hp;
		let mut am = record.into_active_model();
		am.selected_rank = ActiveValue::Set(selected_rank);
		am.variant_key = ActiveValue::Set(select_variant_key_for_rank(&definition, rank));
		if definition.max_hp.is_some() && current_hp.is_none() {
			am.current_hp = ActiveValue::Set(definition.max_hp);
		}
		am.gauge_index = ActiveValue::Set(definition.gauge_count.unwrap_or(1));
		am.event_state = ActiveValue::Set(Some(1));
		let updated = am.update(&tx).await?;

		tx.commit().await?;

		Ok(EventMapRankSelection {
			now_maphp: updated.current_hp.unwrap_or(definition.max_hp.unwrap_or(0)),
			max_maphp: definition.max_hp.unwrap_or(updated.current_hp.unwrap_or(0)),
			gauge_type: definition.gauge_type.unwrap_or(2),
			gauge_num: definition.gauge_count.unwrap_or(updated.gauge_index),
			sally_flag: sally_flag_array(&definition),
		})
	}

	async fn get_combined_type(&self, profile_id: i64) -> Result<i64, GameplayError> {
		let db = self.db();
		let profile = find_profile(db, profile_id).await?;
		Ok(profile.combined_type)
	}

	async fn set_combined_type(
		&self,
		profile_id: i64,
		combined_type: i64,
	) -> Result<i64, GameplayError> {
		if !(0..=3).contains(&combined_type) {
			return Err(GameplayError::WrongType(
				format!("invalid combined_type {combined_type}",),
			));
		}

		let db = self.db();
		let tx = db.begin().await?;

		find_profile(&tx, profile_id).await?;
		if combined_type > 0 {
			let fleet1 = get_fleet_ships_impl(&tx, profile_id, 1).await?;
			let fleet2 = get_fleet_ships_impl(&tx, profile_id, 2).await?;
			if fleet1.is_empty() || fleet2.is_empty() {
				return Err(GameplayError::WrongType(
					"combined fleet requires both deck 1 and deck 2 to have ships".to_string(),
				));
			}
		}

		let profile = find_profile(&tx, profile_id).await?;
		let mut am = profile.into_active_model();
		am.combined_type = ActiveValue::Set(combined_type);
		am.update(&tx).await?;

		tx.commit().await?;
		Ok(i64::from(combined_type > 0))
	}
}

pub(crate) async fn get_map_records_impl<C>(
	c: &C,
	profile_id: i64,
) -> Result<Vec<map_record::Model>, GameplayError>
where
	C: ConnectionTrait,
{
	let records = map_record::Entity::find()
		.filter(map_record::Column::ProfileId.eq(profile_id))
		.order_by_asc(map_record::Column::MapId)
		.all(c)
		.await?;

	Ok(records)
}

pub(crate) async fn find_map_record_impl<C>(
	c: &C,
	profile_id: i64,
	map_id: i64,
) -> Result<map_record::Model, GameplayError>
where
	C: ConnectionTrait,
{
	map_record::Entity::find()
		.filter(map_record::Column::ProfileId.eq(profile_id))
		.filter(map_record::Column::MapId.eq(map_id))
		.one(c)
		.await?
		.ok_or_else(|| {
			GameplayError::EntryNotFound(format!(
				"map_record for map {map_id} not found for profile {profile_id}",
			))
		})
}

pub(crate) async fn ensure_map_records_impl<C>(
	c: &C,
	codex: &Codex,
	profile_id: i64,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	let existing = get_map_records_impl(c, profile_id)
		.await?
		.into_iter()
		.map(|record| record.map_id)
		.collect::<BTreeSet<_>>();
	let now = Utc::now();

	for definition in active_map_catalog(codex).known_maps() {
		if existing.contains(&definition.map_id) {
			continue;
		}

		map_record::ActiveModel {
			id: ActiveValue::NotSet,
			profile_id: ActiveValue::Set(profile_id),
			map_id: ActiveValue::Set(definition.map_id),
			cleared: ActiveValue::Set(false),
			last_cleared_at: ActiveValue::Set(None),
			last_reset_at: ActiveValue::Set(Some(now)),
			defeat_count: ActiveValue::Set(definition.required_defeat_count.map(|_| 0)),
			current_hp: ActiveValue::Set(definition.max_hp),
			gauge_index: ActiveValue::Set(definition.gauge_count.unwrap_or(1)),
			variant_key: ActiveValue::Set(select_variant_key_for_rank(definition, 0)),
			selected_rank: ActiveValue::Set(map_record::SelectedRank::NotSet),
			event_state: ActiveValue::Set(definition.max_hp.map(|_| 1)),
		}
		.insert(c)
		.await?;
	}

	Ok(())
}

pub(crate) async fn refresh_all_map_records_impl<C>(
	c: &C,
	codex: &Codex,
	profile_id: i64,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	let now = Utc::now();
	let refresh_boundary = KcTime::jst_0500_of_nth_day(1);
	let definitions = active_map_catalog(codex)
		.known_maps()
		.into_iter()
		.map(|definition| (definition.map_id, definition.clone()))
		.collect::<BTreeMap<_, _>>();

	for record in get_map_records_impl(c, profile_id).await? {
		let Some(definition) = definitions.get(&record.map_id) else {
			continue;
		};
		let selected_rank = record.selected_rank.clone() as i64;
		if definition.reset_policy == MapResetPolicy::Monthly
			&& now >= refresh_boundary
			&& record.last_reset_at.is_none_or(|ts| ts < refresh_boundary)
		{
			let mut am = record.into_active_model();
			am.cleared = ActiveValue::Set(false);
			am.last_cleared_at = ActiveValue::Set(None);
			am.last_reset_at = ActiveValue::Set(Some(now));
			am.defeat_count = ActiveValue::Set(definition.required_defeat_count.map(|_| 0));
			am.current_hp = ActiveValue::Set(definition.max_hp);
			am.gauge_index = ActiveValue::Set(definition.gauge_count.unwrap_or(1));
			am.variant_key =
				ActiveValue::Set(select_variant_key_for_rank(definition, selected_rank));
			am.event_state = ActiveValue::Set(definition.max_hp.map(|_| 1));
			am.update(c).await?;
		}
	}

	Ok(())
}

pub(crate) fn find_map_definition(
	codex: &Codex,
	maparea_id: i64,
	mapinfo_no: i64,
) -> Result<MapDefinition, GameplayError> {
	active_map_catalog(codex).map_definition_by_area_no(maparea_id, mapinfo_no).cloned().ok_or_else(
		|| {
			GameplayError::EntryNotFound(format!(
				"map definition not found for {maparea_id}-{mapinfo_no}",
			))
		},
	)
}

pub(crate) fn active_map_catalog(codex: &Codex) -> MapCatalog {
	if codex.maps.maps.is_empty() {
		MapCatalog::from_manifest(&codex.manifest)
	} else {
		codex.maps.clone()
	}
}

pub(crate) fn build_map_infos(codex: &Codex, records: Vec<map_record::Model>) -> Vec<KcApiMapInfo> {
	let record_map =
		records.into_iter().map(|record| (record.map_id, record)).collect::<BTreeMap<_, _>>();
	let manifest_ids =
		codex.manifest.api_mst_mapinfo.iter().map(|map| map.api_id).collect::<BTreeSet<_>>();
	let catalog = active_map_catalog(codex);

	let mut infos = catalog
		.known_maps()
		.into_iter()
		.filter(|definition| manifest_ids.contains(&definition.map_id))
		.collect::<Vec<_>>();
	infos.sort_by_key(|definition| definition.map_id);

	infos
		.into_iter()
		.filter_map(|definition| {
			record_map.get(&definition.map_id).map(|record| build_map_info(definition, record))
		})
		.collect()
}

fn build_map_info(definition: &MapDefinition, record: &map_record::Model) -> KcApiMapInfo {
	let mut info = KcApiMapInfo {
		api_id: definition.map_id,
		api_cleared: record.cleared as i64,
		api_defeat_count: definition
			.required_defeat_count
			.map(|_| record.defeat_count.unwrap_or(0)),
		api_gauge_num: definition
			.gauge_count
			.or_else(|| definition.required_defeat_count.map(|_| 1)),
		api_gauge_type: definition.gauge_type,
		api_required_defeat_count: definition.required_defeat_count,
		api_air_base_decks: definition.airbase_count,
		api_eventmap: None,
		api_s_no: None,
		api_m10: None,
		api_sally_flag: None,
	};

	if definition.is_event
		|| definition.max_hp.is_some()
		|| record.selected_rank != map_record::SelectedRank::NotSet
	{
		info.api_eventmap = Some(KcApiEventmap {
			api_now_maphp: record.current_hp.unwrap_or(definition.max_hp.unwrap_or(0)),
			api_max_maphp: definition.max_hp.unwrap_or(record.current_hp.unwrap_or(0)),
			api_selected_rank: record.selected_rank.clone() as i64,
			api_state: record.event_state.unwrap_or(1),
		});
		info.api_sally_flag = Some(sally_flag_array(definition));
		info.api_s_no = Some(definition.mapinfo_no);
		info.api_m10 = Some(0);
	}

	info
}

fn parse_map_select_rank(rank: i64) -> Result<MapSelectRank, GameplayError> {
	MapSelectRank::n(rank as i32)
		.ok_or_else(|| GameplayError::WrongType(format!("invalid map rank {rank}")))
}

fn select_variant_key_for_rank(definition: &MapDefinition, _rank: i64) -> Option<String> {
	(!definition.default_variant.is_empty()).then(|| definition.default_variant.clone())
}

fn sally_flag_array(definition: &MapDefinition) -> [i64; 3] {
	let mut result = [0; 3];
	for (idx, value) in definition.sally_flag.iter().copied().take(3).enumerate() {
		result[idx] = value;
	}
	result
}

pub(super) async fn init<C>(c: &C, codex: &Codex, profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	ensure_map_records_impl(c, codex, profile_id).await
}

pub(super) async fn wipe<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	map_record::Entity::delete_many()
		.filter(map_record::Column::ProfileId.eq(profile_id))
		.exec(c)
		.await?;

	Ok(())
}
