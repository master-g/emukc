//! The engagement form (交戦形態) a battle is fought in.

use emukc_battle::{BattleRng, BattleShipInput, EngagementType};

/// 彩雲, 彩雲(東カロリン空) and 彩雲(偵四). The other carrier recon planes
/// (二式艦上偵察機, 試製景雲) do not have the effect.
const SAIUN_IDS: [i64; 3] = [54, 212, 273];

/// Whether the ship has a 彩雲 in a slot with at least one plane left.
pub(crate) fn carries_saiun(ship: &BattleShipInput) -> bool {
    ship.ship.api_slot.iter().zip(ship.ship.api_onslot).any(|(slot_id, planes)| {
        planes > 0
            && ship
                .slot_items
                .iter()
                .any(|item| item.api_id == *slot_id && SAIUN_IDS.contains(&item.api_slotitem_id))
    })
}

/// Draw the engagement of a sortie or practice battle: 同航戦 45%, 反航戦 30%, T字有利 15%,
/// T字不利 10%. With a 彩雲 a T字不利 becomes 反航戦; the other two are not
/// made any likelier.
pub(crate) fn roll_engagement(rng: &mut impl BattleRng, carries_saiun: bool) -> EngagementType {
    match rng.roll_range(0, 100) {
        0..45 => EngagementType::SameCourse,
        45..75 => EngagementType::HeadOn,
        75..90 => EngagementType::TAdvantage,
        _ if carries_saiun => EngagementType::HeadOn,
        _ => EngagementType::TDisadvantage,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use emukc_model::codex::Codex;

    /// Answers every roll with the same number.
    struct Fixed(i64);

    impl BattleRng for Fixed {
        fn random_f64_range(&mut self, min: f64, _max: f64) -> f64 {
            min
        }

        fn roll_range_impl(&mut self, _min: i64, _max: i64) -> i64 {
            self.0
        }
    }

    #[test]
    fn engagement_follows_the_45_30_15_10_split() {
        for (roll, expected) in [
            (0, EngagementType::SameCourse),
            (44, EngagementType::SameCourse),
            (45, EngagementType::HeadOn),
            (74, EngagementType::HeadOn),
            (75, EngagementType::TAdvantage),
            (89, EngagementType::TAdvantage),
            (90, EngagementType::TDisadvantage),
            (99, EngagementType::TDisadvantage),
        ] {
            assert_eq!(roll_engagement(&mut Fixed(roll), false), expected, "roll {roll}");
        }
    }

    #[test]
    fn a_saiun_turns_only_t_disadvantage_into_head_on() {
        for roll in 0..100 {
            let plain = roll_engagement(&mut Fixed(roll), false);
            let expected = if plain == EngagementType::TDisadvantage {
                EngagementType::HeadOn
            } else {
                plain
            };
            assert_eq!(roll_engagement(&mut Fixed(roll), true), expected, "roll {roll}");
        }
    }

    /// 赤城 with her first slot holding `planes` of the given equipment.
    fn carrier_with(codex: &Codex, mst_id: i64, planes: i64) -> BattleShipInput {
        let (mut ship, mut slot_items) = codex.new_ship(83).unwrap();
        slot_items.truncate(1);
        slot_items[0].api_id = 7;
        slot_items[0].api_slotitem_id = mst_id;
        ship.api_slot = [7, -1, -1, -1, -1];
        ship.api_onslot = [planes, 0, 0, 0, 0];
        BattleShipInput {
            ship,
            slot_items,
            effect_list: vec![],
            married: false,
        }
    }

    #[test]
    fn only_a_saiun_with_planes_left_counts() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        assert!(carries_saiun(&carrier_with(&codex, 54, 9)));
        assert!(carries_saiun(&carrier_with(&codex, 273, 1)));
        // Shot down to nothing.
        assert!(!carries_saiun(&carrier_with(&codex, 54, 0)));
        // 二式艦上偵察機 and 試製景雲(艦偵型) are carrier recon planes without the effect.
        assert!(!carries_saiun(&carrier_with(&codex, 61, 9)));
        assert!(!carries_saiun(&carrier_with(&codex, 151, 9)));
    }
}
