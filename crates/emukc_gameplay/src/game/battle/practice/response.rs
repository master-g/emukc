//! Practice battle API response construction.

use emukc_model::kc2::KcSortieResultRank;

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
        api_get_base_exp: rank_corrected_base_exp(snapshot.get_base_exp, snapshot.win_rank),
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

/// Calculate practice base experience from the rival's flagship and second ship
/// (wikiwiki 演習): the flagship's total exp / 100 plus the second ship's / 300,
/// with only the square root of anything above 500 counted. A lone flagship
/// counts as having a level-1 second ship.
pub fn calculate_base_exp(rival: &emukc_model::profile::practice::Rival) -> i64 {
    use emukc_model::kc2::level::ship_level_required_exp;

    let level_at = |position: usize| rival.details.ships.get(position).map_or(1, |ship| ship.level);
    let raw =
        ship_level_required_exp(level_at(0)) / 100 + ship_level_required_exp(level_at(1)) / 300;
    if raw <= 500 {
        raw
    } else {
        500 + (raw - 500).isqrt()
    }
}

/// The base exp after the win-rank correction, as the result screen shows it.
/// wikiwiki 演習: S 1.2, A and B 1.0, C 0.64, D 0.56, E 0.4.
pub fn rank_corrected_base_exp(base_exp: i64, win_rank: KcSortieResultRank) -> i64 {
    let percent = match win_rank {
        KcSortieResultRank::S => 120,
        KcSortieResultRank::A | KcSortieResultRank::B => 100,
        KcSortieResultRank::C => 64,
        KcSortieResultRank::D => 56,
        KcSortieResultRank::E => 40,
    };
    base_exp * percent / 100
}

#[cfg(test)]
mod tests {
    use emukc_model::profile::practice::{Rival, RivalShip};

    use super::*;

    fn rival_with_levels(levels: &[i64]) -> Rival {
        let mut rival = Rival::default();
        rival.details.ships = levels
            .iter()
            .map(|&level| RivalShip {
                level,
                ..Default::default()
            })
            .collect();
        rival
    }

    /// Cells of wikiwiki 演習's 基本経験値表 (flagship Lv, second ship Lv).
    #[test]
    fn base_exp_matches_the_wiki_table() {
        for (flagship, second, exp) in [
            (1, 5, 3),
            (15, 50, 503),
            (30, 20, 498),
            (30, 25, 505),
            (60, 60, 543),
            (99, 99, 613),
            (120, 120, 627),
            (150, 150, 740),
            (180, 180, 915),
        ] {
            assert_eq!(
                calculate_base_exp(&rival_with_levels(&[flagship, second])),
                exp,
                "flagship {flagship}, second {second}"
            );
        }
    }

    /// A lone flagship counts as having a level-1 second ship; the third ship
    /// and later do not count.
    #[test]
    fn base_exp_reads_only_the_first_two_ships() {
        for (flagship, exp) in [(1, 0), (30, 435), (40, 516)] {
            assert_eq!(calculate_base_exp(&rival_with_levels(&[flagship])), exp);
            assert_eq!(calculate_base_exp(&rival_with_levels(&[flagship, 1, 99, 99])), exp);
        }
    }

    /// wikiwiki 演習's 評価による補正倍率: S 1.2, A and B 1.0, C 0.64, D 0.56, E 0.4.
    #[test]
    fn the_win_rank_scales_the_base_exp() {
        use KcSortieResultRank::*;
        for (rank, exp) in [(S, 612), (A, 510), (B, 510), (C, 326), (D, 285), (E, 204)] {
            assert_eq!(rank_corrected_base_exp(510, rank), exp, "{rank}");
        }
    }
}
