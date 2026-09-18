//! Practice battle API response construction.

use super::PracticeBattleResultResponse;

/// Build the practice battle result API response from a snapshot.
pub fn build_result_response(
    snapshot: super::PracticeBattleResultSnapshot,
) -> PracticeBattleResultResponse {
    PracticeBattleResultResponse {
        api_ship_id: snapshot.enemy_ship_ids,
        api_win_rank: snapshot.win_rank.to_string(),
        api_get_exp: snapshot.get_exp,
        api_member_lv: snapshot.member_lv,
        api_member_exp: snapshot.member_exp,
        api_get_base_exp: snapshot.get_base_exp,
        api_mvp: snapshot.mvp,
        api_get_ship_exp: snapshot.get_ship_exp,
        api_get_exp_lvup: snapshot.get_exp_lvup,
        api_enemy_info: super::PracticeBattleEnemyInfo {
            api_user_name: String::new(),
            api_level: snapshot.enemy_level,
            api_rank: snapshot.enemy_rank,
            api_deck_name: snapshot.enemy_deck_name,
        },
    }
}

/// Calculate practice base experience from rival level.
pub fn calculate_base_exp(rival: &emukc_model::profile::practice::Rival) -> i64 {
    (rival.level.max(1) * 9).clamp(100, 1200)
}
