use emukc_internal::prelude::{
    KcApiMapAirSearch, KcApiMapCellData, KcApiMapEnemyDeckInfo, KcApiMapEoItem, KcApiMapHappening,
    KcApiMapItemGet, KcApiMapNext, KcApiMapStart, SortieAirSearch, SortieCellData,
    SortieEnemyDeckPreview, SortieHappening, SortieItemGet, SortieNextResponse,
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
        api_itemget: response.itemget.map(|items| items.into_iter().map(project_itemget).collect()),
        api_happening: response.happening.map(project_happening),
    }
}

pub(super) fn project_next(response: SortieNextResponse) -> KcApiMapNext {
    let escort = response.escort.as_ref();
    KcApiMapNext {
        // The client's `CellTaskAnchor` reads the resource by its number, 1 to 4, and
        // plays the bonus only where `api_usemst` is 5.
        api_itemget_eo_comment: escort.map(|escort| KcApiMapEoItem {
            api_usemst: 4,
            api_id: escort.gain.item.material_id().unwrap_or_default(),
            api_getcount: escort.gain.amount,
        }),
        api_itemget_eo_result: escort.and_then(|escort| escort.bonus_use_item).map(
            |(id, count)| KcApiMapEoItem {
                api_usemst: 5,
                api_id: id,
                api_getcount: count,
            },
        ),
        api_get_eo_rate: escort.map(|escort| escort.rank_points).filter(|points| *points > 0),
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
        api_destruction_battle: response
            .destruction_battle
            .and_then(|raid| serde_json::to_value(raid).ok()),
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

const MATERIAL_NAMES: [&str; 8] =
    ["", "燃料", "弾薬", "鋼材", "ボーキサイト", "高速建造材", "高速修復材", "開発資材"];

/// A material goes by `api_usemst` 4 and its number, which the client looks up in a table
/// of its own; a furniture box by `api_usemst` 5 and its `api_mst_useitem` id.
fn project_itemget(item: SortieItemGet) -> KcApiMapItemGet {
    let (usemst, id, name) = match (item.item.furniture_box(), item.item.material_id()) {
        (Some(use_item), _) => {
            (5, use_item, ["家具箱(小)", "家具箱(中)", "家具箱(大)"][use_item as usize - 10])
        }
        (None, material) => {
            let id = material.unwrap_or_default();
            (4, id, MATERIAL_NAMES[id as usize])
        }
    };
    KcApiMapItemGet {
        api_id: id,
        api_getcount: item.amount,
        api_name: name.to_string(),
        api_icon_id: id,
        api_usemst: usemst,
    }
}

/// The client reads what was lost from `api_mst_id`: 1 for fuel, 2 for ammunition.
fn project_happening(h: SortieHappening) -> KcApiMapHappening {
    let id = h.item.material_id().unwrap_or_default();
    KcApiMapHappening {
        api_type: 1,
        api_count: h.amount,
        api_dentan: i64::from(h.radar_reduced),
        api_mst_id: id,
        api_icon_id: id,
    }
}

#[cfg(test)]
mod tests {
    use emukc_internal::{model::codex::map::CellItem, prelude::SortieEscortSuccess};

    use super::*;

    /// The fields the client's `CellTaskAnchor` and `CellTaskAirReconnaissance` read.
    #[test]
    fn the_arrival_that_empties_the_gauge_of_1_6_is_laid_out_for_the_client() {
        let next = project_next(SortieNextResponse {
            rashin_flg: false,
            rashin_id: 0,
            maparea_id: 1,
            mapinfo_no: 6,
            cell_no: 14,
            color_no: 6,
            event_id: 8,
            event_kind: 0,
            has_next: false,
            boss_cell_no: 0,
            bosscomp: false,
            from_cell_no: 11,
            comment_kind: Some(0),
            production_kind: Some(0),
            airsearch: Some(SortieAirSearch {
                plane_type: 1,
                result: 2,
            }),
            enemy_deck_preview: None,
            limit_state: Some(0),
            itemget: None,
            happening: None,
            destruction_battle: None,
            escort: Some(SortieEscortSuccess {
                gain: SortieItemGet {
                    item: CellItem::Fuel,
                    amount: 1000,
                },
                bonus_use_item: Some((60, 1)),
                rank_points: 75,
            }),
        });
        let json = serde_json::to_value(&next).unwrap();
        assert_eq!(
            json["api_itemget_eo_comment"],
            serde_json::json!({"api_usemst": 4, "api_id": 1, "api_getcount": 1000})
        );
        assert_eq!(
            json["api_itemget_eo_result"],
            serde_json::json!({"api_usemst": 5, "api_id": 60, "api_getcount": 1})
        );
        assert_eq!(json["api_get_eo_rate"], 75);
        assert_eq!(
            json["api_airsearch"],
            serde_json::json!({"api_plane_type": 1, "api_result": 2})
        );
    }
}
