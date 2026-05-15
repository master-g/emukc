use emukc_model::kc2::KcSortieResultRank;

use crate::types::BattleRuntimeShip;

/// Calculate the MVP (1-based index) from friendly ships.
///
/// Returns the 1-based fleet index of the ship that dealt the most damage,
/// or -1 if the fleet is empty.
pub fn calculate_mvp(friendly: &[BattleRuntimeShip]) -> i64 {
    friendly
        .iter()
        .enumerate()
        .max_by_key(|(_, ship)| ship.damage_dealt)
        .map(|(idx, _)| idx as i64 + 1)
        .unwrap_or(-1)
}

/// Calculate the battle win rank from friendly and enemy ship states.
pub fn calculate_win_rank(
    friendly: &[BattleRuntimeShip],
    enemy: &[BattleRuntimeShip],
) -> KcSortieResultRank {
    let enemy_total_hp: i64 = enemy.iter().map(|ship| ship.ship.api_maxhp).sum();
    let enemy_remaining_hp: i64 = enemy.iter().map(|ship| ship.hp().max(0)).sum();
    let friend_total_hp: i64 = friendly.iter().map(|ship| ship.ship.api_maxhp).sum();
    let friend_remaining_hp: i64 = friendly.iter().map(|ship| ship.hp().max(0)).sum();
    let enemy_all_sunk = enemy.iter().all(BattleRuntimeShip::is_sunk);
    let friend_all_sunk = friendly.iter().all(BattleRuntimeShip::is_sunk);
    let friend_sunk_count = friendly.iter().filter(|ship| ship.is_sunk()).count();
    let friend_count = friendly.len();
    let enemy_damage_rate =
        (enemy_total_hp - enemy_remaining_hp) as f64 / enemy_total_hp.max(1) as f64;
    let friend_damage_rate =
        (friend_total_hp - friend_remaining_hp) as f64 / friend_total_hp.max(1) as f64;

    if friend_all_sunk {
        KcSortieResultRank::E
    } else if enemy_all_sunk && friend_sunk_count == 0 {
        KcSortieResultRank::S
    } else if enemy_all_sunk {
        // All enemy sunk but we lost ships → downgrade to A
        KcSortieResultRank::A
    } else if friend_sunk_count * 2 >= friend_count && friend_count > 1 {
        // Half or more friendly ships sunk → D
        KcSortieResultRank::D
    } else if enemy_damage_rate >= 0.7 {
        if friend_sunk_count > 0 {
            KcSortieResultRank::B
        } else {
            KcSortieResultRank::A
        }
    } else if enemy_damage_rate > friend_damage_rate {
        KcSortieResultRank::B
    } else {
        KcSortieResultRank::C
    }
}

/// Post-simulation integrity check: verifies that protected friendly ships
/// (non-taiha at entry + flagship) have HP >= 1.
/// Panics in debug builds, logs error in release builds.
pub(crate) fn verify_protected_ships_alive(ships: &[BattleRuntimeShip]) {
    for (idx, ship) in ships.iter().enumerate() {
        if !ship.is_friendly || !ship.is_sortie {
            continue;
        }
        let was_taiha = ship.entry_hp * 4 <= ship.ship.api_maxhp;
        let is_protected = idx == 0 || !was_taiha;
        if is_protected && ship.hp() < 1 {
            let msg = format!(
                "BUG: protected ship at index {} has hp={}, entry_hp={}, maxhp={}",
                idx,
                ship.hp(),
                ship.entry_hp,
                ship.ship.api_maxhp
            );
            cfg_select! {
                debug_assertions => panic!("{msg}"),
                _ => tracing::error!("{msg}")
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    #[test]
    fn win_rank_s_requires_no_friendly_sinking() {
        let friendly = vec![make_test_ship(40, 40, 30, 40)];
        let enemy = vec![make_test_ship(40, 40, 0, 40)];
        assert_eq!(calculate_win_rank(&friendly, &enemy), KcSortieResultRank::S);
    }

    #[test]
    fn win_rank_downgraded_to_a_when_friendly_sunk() {
        let friendly = vec![make_test_ship(40, 40, 30, 40), make_test_ship(30, 30, 0, 30)];
        let enemy = vec![make_test_ship(40, 40, 0, 40)];
        assert_eq!(calculate_win_rank(&friendly, &enemy), KcSortieResultRank::A);
    }

    #[test]
    fn win_rank_e_when_all_friendly_sunk() {
        let friendly = vec![make_test_ship(40, 40, 0, 40)];
        let enemy = vec![make_test_ship(40, 40, 20, 40)];
        assert_eq!(calculate_win_rank(&friendly, &enemy), KcSortieResultRank::E);
    }

    #[test]
    fn win_rank_d_when_half_friendly_sunk() {
        let friendly = vec![make_test_ship(40, 40, 30, 40), make_test_ship(30, 30, 0, 30)];
        let enemy = vec![make_test_ship(40, 40, 35, 40)];
        assert_eq!(calculate_win_rank(&friendly, &enemy), KcSortieResultRank::D);
    }
}
