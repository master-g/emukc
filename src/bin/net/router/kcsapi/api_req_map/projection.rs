use emukc_internal::prelude::{
    KcApiMapAirSearch, KcApiMapCellData, KcApiMapEnemyDeckInfo, KcApiMapHappening, KcApiMapItemGet,
    KcApiMapNext, KcApiMapStart, SortieAirSearch, SortieCellData, SortieEnemyDeckPreview,
    SortieHappening, SortieItemGet, SortieNextResponse, SortieStartResponse,
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
        api_itemget: None,
        api_happening: None,
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
        api_itemget: response.itemget.map(|items| items.into_iter().map(project_itemget).collect()),
        api_happening: response.happening.map(project_happening),
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

const RESOURCE_NAMES: [&str; 5] = ["", "燃料", "弾薬", "鋼材", "ボーキサイト"];

fn project_itemget(item: SortieItemGet) -> KcApiMapItemGet {
    let name = RESOURCE_NAMES.get(item.resource_type as usize).unwrap_or(&"").to_string();
    KcApiMapItemGet {
        api_id: item.resource_type,
        api_getcount: item.amount,
        api_name: name,
        api_icon_id: item.resource_type,
        api_usemst: 0,
    }
}

fn project_happening(h: SortieHappening) -> KcApiMapHappening {
    KcApiMapHappening {
        api_type: h.resource_type,
        api_count: h.amount,
        api_dentan: i64::from(h.radar_reduced),
        api_mst_id: 0,
        api_icon_id: h.resource_type,
    }
}
