use emukc_internal::prelude::{
	KcApiMapAirSearch, KcApiMapCellData, KcApiMapEnemyDeckInfo, KcApiMapNext, KcApiMapStart,
	SortieAirSearch, SortieCellData, SortieEnemyDeckPreview, SortieNextResponse,
	SortieStartResponse,
};

pub(super) fn project_start(response: SortieStartResponse) -> KcApiMapStart {
	KcApiMapStart {
		api_cell_data: response.cell_data.into_iter().map(project_cell_data).collect(),
		api_rashin_flg: i64::from(response.rashin_flg),
		api_rashin_id: response.rashin_id,
		api_maparea_id: response.maparea_id,
		api_mapinfo_no: response.mapinfo_no,
		api_no: response.cell_no,
		api_color_no: response.color_no,
		api_event_id: response.event_id,
		api_event_kind: response.event_kind,
		api_next: i64::from(response.has_next),
		api_bosscell_no: response.boss_cell_no,
		api_bosscomp: i64::from(response.bosscomp),
		api_from_no: response.from_cell_no,
		api_limit_state: response.limit_state,
		api_eventmap: None,
		api_airsearch: response.airsearch.map(project_airsearch),
		api_e_deck_info: response
			.enemy_deck_preview
			.map(|preview| preview.into_iter().map(project_enemy_deck_preview).collect()),
	}
}

pub(super) fn project_next(response: SortieNextResponse) -> KcApiMapNext {
	KcApiMapNext {
		api_rashin_flg: i64::from(response.rashin_flg),
		api_rashin_id: response.rashin_id,
		api_maparea_id: response.maparea_id,
		api_mapinfo_no: response.mapinfo_no,
		api_no: response.cell_no,
		api_color_no: response.color_no,
		api_event_id: response.event_id,
		api_event_kind: response.event_kind,
		api_next: i64::from(response.has_next),
		api_bosscell_no: response.boss_cell_no,
		api_bosscomp: i64::from(response.bosscomp),
		api_from_no: response.from_cell_no,
		api_comment_kind: response.comment_kind,
		api_production_kind: response.production_kind,
		api_airsearch: response.airsearch.map(project_airsearch),
		api_e_deck_info: response
			.enemy_deck_preview
			.map(|preview| preview.into_iter().map(project_enemy_deck_preview).collect()),
		api_limit_state: response.limit_state,
	}
}

fn project_cell_data(cell: SortieCellData) -> KcApiMapCellData {
	KcApiMapCellData {
		api_id: cell.master_cell_id,
		api_no: cell.cell_no,
		api_color_no: cell.color_no,
		api_passed: i64::from(cell.passed),
		api_distance: cell.distance,
	}
}

fn project_airsearch(airsearch: SortieAirSearch) -> KcApiMapAirSearch {
	KcApiMapAirSearch {
		api_plane_type: airsearch.plane_type,
		api_result: airsearch.result,
	}
}

fn project_enemy_deck_preview(preview: SortieEnemyDeckPreview) -> KcApiMapEnemyDeckInfo {
	KcApiMapEnemyDeckInfo {
		api_kind: preview.kind,
		api_ship_ids: preview.ship_ids,
	}
}
