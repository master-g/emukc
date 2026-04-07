#![allow(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};

mod debug;
mod merge;
mod types;

use crate::{
	kc2::start2::{ApiManifest, ApiMstMapinfo},
	profile::map_record::{DEFAULT_MAP_RECORDS, MapRefreshType},
};

pub use types::*;

use merge::merge_definition as merge_definition_impl;
impl MapCatalog {
	pub fn from_manifest(manifest: &ApiManifest) -> Self {
		let defaults = DEFAULT_MAP_RECORDS
			.iter()
			.map(|record| (record.id, record))
			.collect::<BTreeMap<_, _>>();
		let mut maps = BTreeMap::new();
		let area_types = manifest
			.api_mst_maparea
			.iter()
			.map(|area| (area.api_id, area.api_type))
			.collect::<BTreeMap<_, _>>();

		for map in &manifest.api_mst_mapinfo {
			let overlay = defaults.get(&map.api_id);
			let (reset_policy, airbase_count, gauge_type, gauge_count, required_defeat_count) =
				if let Some(record) = overlay {
					let defeat_ctx = record.defeat_ctx.as_ref();
					(
						defeat_ctx
							.map(|ctx| match ctx.refresh_type {
								MapRefreshType::Monthly => MapResetPolicy::Monthly,
								MapRefreshType::Never => MapResetPolicy::Never,
							})
							.unwrap_or(MapResetPolicy::Never),
						record.airbase_count,
						defeat_ctx.map(|ctx| ctx.gauge_type as i64),
						defeat_ctx.map(|ctx| ctx.gauge_num),
						defeat_ctx.map(|ctx| ctx.defeat_required),
					)
				} else {
					(MapResetPolicy::Never, None, None, None, map.api_required_defeat_count)
				};
			let is_event = area_types.get(&map.api_maparea_id).copied().unwrap_or_default() == 1;
			maps.insert(
				map.api_id,
				MapDefinition {
					map_id: map.api_id,
					maparea_id: map.api_maparea_id,
					mapinfo_no: map.api_no,
					name: map.api_name.clone(),
					level: map.api_level,
					sally_flag: map.api_sally_flag.clone(),
					is_event,
					reset_policy,
					airbase_count,
					gauge_type,
					gauge_count,
					required_defeat_count,
					max_hp: extract_max_hp(map),
					default_variant: String::new(),
					rank_stage_ids: BTreeMap::new(),
					variants: BTreeMap::new(),
				},
			);
		}

		let mut catalog = Self {
			maps,
		};
		catalog.ensure_synthetic_variants();
		catalog
	}

	pub fn map_definition(&self, map_id: i64) -> Option<&MapDefinition> {
		self.maps.get(&map_id)
	}

	pub fn map_definition_by_area_no(
		&self,
		maparea_id: i64,
		mapinfo_no: i64,
	) -> Option<&MapDefinition> {
		self.maps.get(&compose_map_id(maparea_id, mapinfo_no))
	}

	pub fn known_maps(&self) -> Vec<&MapDefinition> {
		self.maps.values().collect()
	}

	pub fn merge_missing_from(&mut self, other: Self) {
		for (map_id, other_definition) in other.maps {
			match self.maps.entry(map_id) {
				std::collections::btree_map::Entry::Vacant(entry) => {
					entry.insert(other_definition);
				}
				std::collections::btree_map::Entry::Occupied(mut entry) => {
					merge_definition_impl(entry.get_mut(), other_definition);
				}
			}
		}
		self.ensure_synthetic_variants();
	}

	pub fn to_debug_json(&self, manifest: &ApiManifest) -> serde_json::Value {
		debug::to_debug_json(self, manifest)
	}

	fn ensure_synthetic_variants(&mut self) {
		for definition in self.maps.values_mut() {
			if definition.variants.is_empty() {
				definition.variants.insert(
					String::new(),
					MapVariantDefinition {
						variant_key: String::new(),
						boss_cell_no: 1,
						cells: vec![
							MapCellDefinition {
								cell_no: 0,
								color_no: 0,
								event_id: 0,
								event_kind: 0,
								next_cells: vec![1],
								node_label: Some("Start".to_string()),
								master_cell_id: None,
								distance: None,
							},
							MapCellDefinition {
								cell_no: 1,
								color_no: 5,
								event_id: 5,
								event_kind: 1,
								next_cells: vec![],
								node_label: None,
								master_cell_id: None,
								distance: None,
							},
						],
						routing_rules: BTreeMap::new(),
						enemy_fleets: BTreeMap::new(),
						ship_drops: BTreeMap::new(),
						required_defeat_count: None,
						clear_to_variant_key: None,
						parse_warnings: Vec::new(),
					},
				);
			}
		}
	}
}

impl MapDefinition {
	pub fn default_stage_id(&self) -> Option<&str> {
		if !self.default_variant.is_empty() && self.variants.contains_key(&self.default_variant) {
			return Some(&self.default_variant);
		}
		if self.variants.contains_key("") {
			return Some("");
		}
		self.variants.keys().next().map(String::as_str)
	}

	pub fn resolve_stage_id_for_rank(&self, rank: i64) -> Option<&str> {
		self.rank_stage_ids.get(&rank).map(String::as_str).or_else(|| self.default_stage_id())
	}

	pub fn active_stage(&self, stage_id: Option<&str>) -> Option<&MapStageDefinition> {
		stage_id
			.and_then(|stage_id| self.variants.get(stage_id))
			.or_else(|| self.default_stage_id().and_then(|stage_id| self.variants.get(stage_id)))
	}

	pub fn stage_for_rank(&self, rank: i64) -> Option<&MapStageDefinition> {
		self.resolve_stage_id_for_rank(rank).and_then(|stage_id| self.active_stage(Some(stage_id)))
	}

	pub fn stage(&self, stage_id: &str) -> Option<&MapStageDefinition> {
		self.active_stage(Some(stage_id))
	}

	pub fn variant(&self, variant_key: &str) -> Option<&MapVariantDefinition> {
		self.variants
			.get(variant_key)
			.or_else(|| self.variants.get(&self.default_variant))
			.or_else(|| self.variants.get(""))
	}
}

impl MapVariantDefinition {
	pub fn cell(&self, cell_no: i64) -> Option<&MapCellDefinition> {
		self.cells.iter().find(|cell| cell.cell_no == cell_no)
	}

	pub fn ship_drops(&self, cell_no: i64) -> Option<&[ShipDropDefinition]> {
		self.ship_drops.get(&cell_no).map(Vec::as_slice)
	}

	pub fn first_progress_cell_no(&self) -> Option<i64> {
		if let Some(rules) = self.routing_rules.get(&0).filter(|rules| !rules.is_empty()) {
			let targets = rules.iter().map(|rule| rule.to_cell_no).collect::<BTreeSet<_>>();
			return (targets.len() == 1).then(|| targets.into_iter().next()).flatten();
		}
		if let Some(start) = self.cell(0) {
			return match start.next_cells.as_slice() {
				[only] => Some(*only),
				_ => None,
			};
		}
		let mut cells = self.cells.iter().filter(|cell| cell.cell_no > 0).map(|cell| cell.cell_no);
		let first = cells.next()?;
		cells.next().is_none().then_some(first)
	}
}

fn compose_map_id(maparea_id: i64, mapinfo_no: i64) -> i64 {
	maparea_id * 10 + mapinfo_no
}

pub fn split_map_id(map_id: i64) -> (i64, i64) {
	(map_id / 10, map_id % 10)
}

fn extract_max_hp(map: &ApiMstMapinfo) -> Option<i64> {
	match map.api_max_maphp.as_ref()? {
		serde_json::Value::Number(value) => value.as_i64(),
		serde_json::Value::String(value) => value.parse::<i64>().ok(),
		serde_json::Value::Array(values) => values.first().and_then(|value| match value {
			serde_json::Value::Number(number) => number.as_i64(),
			serde_json::Value::String(string) => string.parse::<i64>().ok(),
			_ => None,
		}),
		_ => None,
	}
}
