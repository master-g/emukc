//! Ship experience settlement, shared by sortie, practice and expedition.

use emukc_model::kc2::level;

/// The experience state of a ship after a gain has been applied.
///
/// Pure data: settling does not touch the database, because the three callers
/// persist through different carriers (`KcApiShip` for sortie / practice,
/// `ship::Model` for expedition).
pub(crate) struct ShipExpSettlement {
    /// The new ship level, already clamped to the level cap.
    pub level: i64,
    /// The new accumulated experience, pinned to the cap requirement at the cap.
    pub exp_now: i64,
    /// The experience required for the next level, `0` at the cap.
    pub exp_next: i64,
    /// Progress towards the next level in percent, `0` at the cap.
    pub progress: i64,
}

/// Settle an experience `gain` onto a ship currently holding `exp_now`.
///
/// Once the ship reaches its level cap (99 unmarried, 175 married) the
/// experience is pinned to the cap requirement and both the next-level
/// threshold and the progress are zeroed.
pub(crate) fn settle_ship_exp(exp_now: i64, gain: i64, married: bool) -> ShipExpSettlement {
    let raw_exp = exp_now + gain;
    let (level, next_exp) = level::exp_to_ship_level(raw_exp);
    let level_cap = level::ship_level_cap(married);
    let level = level.min(level_cap);

    if level >= level_cap {
        let cap_exp = level::ship_level_required_exp(level_cap);
        return ShipExpSettlement {
            level,
            exp_now: cap_exp,
            exp_next: 0,
            progress: 0,
        };
    }

    let current_level_exp = level::ship_level_required_exp(level);
    let progress = if next_exp > current_level_exp {
        ((raw_exp - current_level_exp) * 100 / (next_exp - current_level_exp)).clamp(0, 99)
    } else {
        0
    };

    ShipExpSettlement {
        level,
        exp_now: raw_exp,
        exp_next: next_exp,
        progress,
    }
}

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

/// Build the `api_get_exp_lvup` vector: the pre-gain exp followed by every
/// level threshold the gain reaches.
pub(crate) fn build_exp_lvup_vector(before_exp: i64, after_exp: i64) -> Vec<i64> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settle_ship_exp_levels_up_below_the_cap() {
        let before = level::ship_level_required_exp(10);
        let gain = level::ship_level_required_exp(11) - before;
        let settled = settle_ship_exp(before, gain, false);

        assert_eq!(settled.level, 11);
        assert_eq!(settled.exp_now, before + gain);
        assert_eq!(settled.exp_next, level::ship_level_required_exp(12));
        assert_eq!(settled.progress, 0);
    }

    #[test]
    fn settle_ship_exp_pins_an_unmarried_ship_crossing_99() {
        let before = level::ship_level_required_exp(99) - 1;
        let settled = settle_ship_exp(before, 15, false);

        assert_eq!(settled.level, 99);
        assert_eq!(settled.exp_now, level::ship_level_required_exp(99));
        assert_eq!(settled.exp_next, 0);
        assert_eq!(settled.progress, 0);
    }

    #[test]
    fn settle_ship_exp_keeps_an_unmarried_ship_already_at_99_pinned() {
        let cap_exp = level::ship_level_required_exp(99);
        let settled = settle_ship_exp(cap_exp, 0, false);

        assert_eq!(settled.level, 99);
        assert_eq!(settled.exp_now, cap_exp);
        assert_eq!(settled.exp_next, 0);
        assert_eq!(settled.progress, 0);
    }

    #[test]
    fn settle_ship_exp_pins_a_married_ship_crossing_175() {
        let before = level::ship_level_required_exp(175) - 1;
        let settled = settle_ship_exp(before, 10_000, true);

        assert_eq!(settled.level, 175);
        assert_eq!(settled.exp_now, level::ship_level_required_exp(175));
        assert_eq!(settled.exp_next, 0);
        assert_eq!(settled.progress, 0);
    }

    #[test]
    fn settle_ship_exp_does_not_pin_a_married_ship_at_99() {
        let before = level::ship_level_required_exp(99) - 1;
        let settled = settle_ship_exp(before, 1, true);

        // Lv.99 and Lv.100 share the same 1,000,000 exp requirement, so a married
        // ship crossing that threshold lands on 100 rather than stopping at 99.
        assert_eq!(settled.level, 100);
        assert_eq!(settled.exp_now, level::ship_level_required_exp(99));
        assert!(settled.exp_next > settled.exp_now);
    }
}
