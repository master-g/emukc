//! Experience calculation for practice battles.

use emukc_battle::BattleRuntimeShip;
use emukc_model::kc2::level;

/// Calculate admiral experience from base exp and win rank.
pub(crate) fn calculate_admiral_exp(base_exp: i64, win_rank: &str) -> i64 {
    match win_rank {
        "S" => (base_exp as f64 * 0.12).round() as i64,
        "A" => (base_exp as f64 * 0.1).round() as i64,
        "B" => (base_exp as f64 * 0.08).round() as i64,
        "C" => (base_exp as f64 * 0.05).round() as i64,
        _ => (base_exp as f64 * 0.03).round() as i64,
    }
}

/// Calculate ship experience gains for a practice battle.
pub(crate) fn calculate_ship_exp(
    friendly: &[BattleRuntimeShip],
    base_exp: i64,
    mvp_idx: i64,
    ct_flagship: bool,
    ct_exp_boost: f64,
    practice_exp_boost: f64,
) -> (Vec<i64>, Vec<Vec<i64>>) {
    let mut exp = vec![-1];
    let mut lvup = Vec::with_capacity(friendly.len());
    let ct_mult = if ct_flagship {
        ct_exp_boost
    } else {
        1.0
    };

    for (idx, ship) in friendly.iter().enumerate() {
        let gain = if !ship.married && ship.ship.api_lv >= 99 {
            0
        } else if idx as i64 + 1 == mvp_idx {
            (base_exp as f64 * 2.0 * ct_mult * practice_exp_boost).floor() as i64
        } else if idx == 0 {
            (base_exp as f64 * 1.5 * ct_mult * practice_exp_boost).floor() as i64
        } else {
            (base_exp as f64 * ct_mult * practice_exp_boost).floor() as i64
        };
        exp.push(gain);

        let new_exp = ship.ship.api_exp[0] + gain;
        lvup.push(build_exp_lvup_vector(ship.ship.api_exp[0], new_exp));
    }

    (exp, lvup)
}

fn build_exp_lvup_vector(before_exp: i64, after_exp: i64) -> Vec<i64> {
    let mut result = vec![before_exp];
    let (_, mut next_exp) = level::exp_to_ship_level(before_exp);
    if next_exp <= 0 {
        result.push(-1);
        return result;
    }
    result.push(next_exp);

    while next_exp > 0 && after_exp >= next_exp {
        let (_, candidate_next) = level::exp_to_ship_level(next_exp);
        if candidate_next <= 0 {
            result.push(-1);
            break;
        }
        if candidate_next == next_exp {
            break;
        }
        result.push(candidate_next);
        next_exp = candidate_next;
    }

    result
}
