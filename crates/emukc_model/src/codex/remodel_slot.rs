//! Improvement arsenal (改修工廠) recipe lookup for `Codex`.
//!
//! The recipe data itself lives on each `Kc3rdSlotItem` as `improvement`; this
//! module turns it into addressable recipes and answers the questions the
//! gameplay layer asks: which stage of consumption applies at a given star
//! level, and how likely the attempt is to succeed.
//!
//! Rules are sourced from
//! [wikiwiki.jp/kancolle/改修工廠](https://wikiwiki.jp/kancolle/%E6%94%B9%E4%BF%AE%E5%B7%A5%E5%BB%A0).

use crate::thirdparty::{
    Kc3rdSlotItemImproveBaseConsumption, Kc3rdSlotItemImprovePerLevelConsumption,
    Kc3rdSlotItemImproveRequirements,
};

use super::{Codex, CodexError};

/// `mst_id` of 明石, the ship that must be flagship to use the arsenal.
pub const AKASHI_MST_ID: i64 = 182;

/// `mst_id` of 明石改, which improves the success rate at high star levels.
pub const AKASHI_KAI_MST_ID: i64 = 187;

/// Highest star level an equipment can hold.
pub const MAX_STARS: i64 = 10;

/// Maximum number of remodel variants any equipment declares, which bounds the
/// recipe id encoding below.
const MAX_VARIANTS: i64 = 9;

/// Which consumption stage a recipe is at, decided by the equipment's current
/// star level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemodelStage {
    /// ★0 through ★5 — `first_half`.
    FirstHalf,
    /// ★6 through ★9 — `second_half`.
    SecondHalf,
    /// ★10, converting the equipment into its variant — `remodel`.
    Upgrade,
}

impl RemodelStage {
    /// The stage that applies to an equipment currently at `stars`.
    ///
    /// Returns `None` above ★10, which no equipment should reach.
    pub const fn from_stars(stars: i64) -> Option<Self> {
        match stars {
            0..=5 => Some(Self::FirstHalf),
            6..=9 => Some(Self::SecondHalf),
            10 => Some(Self::Upgrade),
            _ => None,
        }
    }
}

/// One addressable improvement recipe: an equipment plus, optionally, the
/// variant it upgrades into at ★10.
#[derive(Debug, Clone)]
pub struct RemodelRecipe {
    /// Synthetic recipe id, stable across restarts. See
    /// [`encode_recipe_id`].
    pub recipe_id: i64,
    /// `mst_id` of the equipment being improved.
    pub slot_item_id: i64,
    /// Resource cost charged on every attempt.
    pub base_consumption: Kc3rdSlotItemImproveBaseConsumption,
    /// Per-stage consumption and the secretary ships that unlock this recipe.
    pub requirements: Kc3rdSlotItemImproveRequirements,
    /// `(mst_id, initial_stars)` this recipe upgrades into at ★10. `None` for a
    /// plain improvement recipe, which stops at ★10.
    pub upgrade_to: Option<(i64, i64)>,
}

impl RemodelRecipe {
    /// Consumption for the stage `stars` falls into, or `None` when this recipe
    /// does not define that stage.
    pub fn consumption_at(&self, stars: i64) -> Option<&Kc3rdSlotItemImprovePerLevelConsumption> {
        match RemodelStage::from_stars(stars)? {
            RemodelStage::FirstHalf => self.requirements.first_half.as_ref(),
            RemodelStage::SecondHalf => self.requirements.second_half.as_ref(),
            RemodelStage::Upgrade => self.requirements.remodel.as_ref(),
        }
    }

    /// Whether `secretary_mst_id` unlocks this recipe on `weekday`.
    ///
    /// `weekday` is 0 = Monday through 6 = Sunday, matching
    /// `chrono::Weekday::num_days_from_monday`.
    pub fn allows_secretary(&self, secretary_mst_id: i64, weekday: u32) -> bool {
        self.requirements.secretary.iter().any(|s| {
            s.id == secretary_mst_id
                && match weekday {
                    0 => s.monday,
                    1 => s.tuesday,
                    2 => s.wednesday,
                    3 => s.thursday,
                    4 => s.friday,
                    5 => s.saturday,
                    6 => s.sunday,
                    _ => false,
                }
        })
    }
}

/// Encode `(slot_item_id, variant_index)` into a recipe id.
///
/// Variant index 0 is the plain improvement recipe; 1..=n address
/// `remodel_variants[index - 1]`. The encoding is injective because the variant
/// index never reaches 10 — the busiest equipment declares 3.
pub const fn encode_recipe_id(slot_item_id: i64, variant_index: i64) -> i64 {
    slot_item_id * (MAX_VARIANTS + 1) + variant_index
}

/// Inverse of [`encode_recipe_id`].
pub const fn decode_recipe_id(recipe_id: i64) -> (i64, i64) {
    (recipe_id / (MAX_VARIANTS + 1), recipe_id % (MAX_VARIANTS + 1))
}

/// Success rate, in percent, of improving from `stars` to `stars + 1`.
///
/// 明石改 as flagship beats plain 明石 from ★4 upward; below that both are
/// certain. Values come from the 艦これ改 teardown reproduced on wikiwiki,
/// whose figures the community's own sampling matches to within a point.
/// ★10 is the upgrade attempt (★10→更新).
pub const fn remodel_success_rate(stars: i64, flagship_is_kai: bool) -> i64 {
    if flagship_is_kai {
        match stars {
            0..=4 => 100,
            5 => 95,
            6 => 90,
            7 => 82,
            8 => 77,
            9 => 67,
            10 => 62,
            _ => 0,
        }
    } else {
        match stars {
            0..=3 => 100,
            4 => 95,
            5 => 90,
            6 => 80,
            7 => 77,
            8 => 72,
            9 => 60,
            10 => 50,
            _ => 0,
        }
    }
}

impl Codex {
    /// Every improvement recipe the game data declares.
    ///
    /// An equipment contributes one recipe per `remodel_variants` entry, plus
    /// one plain recipe when it declares `level_consumption`. Eight equipments
    /// declare both.
    pub fn remodel_recipes(&self) -> Vec<RemodelRecipe> {
        let mut recipes = Vec::new();

        for item in self.slotitem_extra_info.values() {
            let Some(improvement) = item.improvement.as_ref() else {
                continue;
            };

            if let Some(requirements) = improvement.level_consumption.as_ref() {
                recipes.push(RemodelRecipe {
                    recipe_id: encode_recipe_id(item.api_id, 0),
                    slot_item_id: item.api_id,
                    base_consumption: improvement.base_consumption.clone(),
                    requirements: requirements.clone(),
                    upgrade_to: None,
                });
            }

            for (index, variant) in improvement.remodel_variants.iter().flatten().enumerate() {
                let variant_index = i64::try_from(index).unwrap_or(0) + 1;
                if variant_index > MAX_VARIANTS {
                    continue;
                }
                recipes.push(RemodelRecipe {
                    recipe_id: encode_recipe_id(item.api_id, variant_index),
                    slot_item_id: item.api_id,
                    base_consumption: improvement.base_consumption.clone(),
                    requirements: variant.requirements.clone(),
                    upgrade_to: Some((variant.slot_item_id, variant.initial_stars)),
                });
            }
        }

        recipes.sort_by_key(|r| r.recipe_id);
        recipes
    }

    /// Look up a single recipe by its id.
    pub fn find_remodel_recipe(&self, recipe_id: i64) -> Result<RemodelRecipe, CodexError> {
        let (slot_item_id, variant_index) = decode_recipe_id(recipe_id);

        let item = self
            .slotitem_extra_info
            .get(&slot_item_id)
            .ok_or_else(|| CodexError::NotFound(format!("slot item {slot_item_id}")))?;
        let improvement = item.improvement.as_ref().ok_or_else(|| {
            CodexError::NotFound(format!("slot item {slot_item_id} has no improvement data"))
        })?;

        if variant_index == 0 {
            let requirements = improvement.level_consumption.as_ref().ok_or_else(|| {
                CodexError::NotFound(format!("recipe {recipe_id} has no level consumption"))
            })?;
            return Ok(RemodelRecipe {
                recipe_id,
                slot_item_id,
                base_consumption: improvement.base_consumption.clone(),
                requirements: requirements.clone(),
                upgrade_to: None,
            });
        }

        let variant = improvement
            .remodel_variants
            .as_ref()
            .and_then(|variants| variants.get((variant_index - 1) as usize))
            .ok_or_else(|| CodexError::NotFound(format!("recipe {recipe_id}")))?;

        Ok(RemodelRecipe {
            recipe_id,
            slot_item_id,
            base_consumption: improvement.base_consumption.clone(),
            requirements: variant.requirements.clone(),
            upgrade_to: Some((variant.slot_item_id, variant.initial_stars)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipe_id_round_trips() {
        for slot_item_id in [1, 122, 293, 1990] {
            for variant_index in 0..=MAX_VARIANTS {
                let id = encode_recipe_id(slot_item_id, variant_index);
                assert_eq!(decode_recipe_id(id), (slot_item_id, variant_index));
            }
        }
    }

    /// Distinct (equipment, variant) pairs must never collide, or a client
    /// round-tripping a recipe id would improve the wrong equipment.
    #[test]
    fn recipe_ids_are_unique_across_equipments() {
        let mut seen = std::collections::HashSet::new();
        for slot_item_id in 1..=1990 {
            for variant_index in 0..=MAX_VARIANTS {
                assert!(
                    seen.insert(encode_recipe_id(slot_item_id, variant_index)),
                    "collision at {slot_item_id}/{variant_index}"
                );
            }
        }
    }

    #[test]
    fn stage_boundaries_match_the_consumption_fields() {
        assert_eq!(RemodelStage::from_stars(0), Some(RemodelStage::FirstHalf));
        assert_eq!(RemodelStage::from_stars(5), Some(RemodelStage::FirstHalf));
        assert_eq!(RemodelStage::from_stars(6), Some(RemodelStage::SecondHalf));
        assert_eq!(RemodelStage::from_stars(9), Some(RemodelStage::SecondHalf));
        assert_eq!(RemodelStage::from_stars(10), Some(RemodelStage::Upgrade));
        assert_eq!(RemodelStage::from_stars(11), None);
        assert_eq!(RemodelStage::from_stars(-1), None);
    }

    /// Both columns of the wikiwiki success table, cell by cell.
    #[test]
    fn success_rate_table_matches_reference() {
        let akashi = [
            (0, 100),
            (1, 100),
            (2, 100),
            (3, 100),
            (4, 95),
            (5, 90),
            (6, 80),
            (7, 77),
            (8, 72),
            (9, 60),
            (10, 50),
        ];
        for (stars, want) in akashi {
            assert_eq!(remodel_success_rate(stars, false), want, "明石 ★{stars}");
        }

        let kai = [
            (0, 100),
            (1, 100),
            (2, 100),
            (3, 100),
            (4, 100),
            (5, 95),
            (6, 90),
            (7, 82),
            (8, 77),
            (9, 67),
            (10, 62),
        ];
        for (stars, want) in kai {
            assert_eq!(remodel_success_rate(stars, true), want, "明石改 ★{stars}");
        }
    }

    /// 明石改 is never worse than 明石, and strictly better from ★4 up — the
    /// whole reason to remodel her.
    #[test]
    fn kai_is_never_worse_than_plain_akashi() {
        for stars in 0..=MAX_STARS {
            let plain = remodel_success_rate(stars, false);
            let kai = remodel_success_rate(stars, true);
            assert!(kai >= plain, "★{stars}: kai {kai} < plain {plain}");
            if stars >= 4 {
                assert!(kai > plain, "★{stars}: kai must beat plain akashi");
            }
        }
    }
}
