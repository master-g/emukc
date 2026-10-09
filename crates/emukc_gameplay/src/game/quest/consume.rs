use emukc_db::sea_orm::ConnectionTrait;
use emukc_model::{kc2::MaterialCategory, thirdparty::Kc3rdQuestConditionConsumption};

use crate::{
    err::GameplayError,
    game::{material::deduct_material_impl, use_item::deduct_use_item_impl},
};

pub(super) async fn handle_consumption<C>(
    c: &C,
    profile_id: i64,
    consumption: &Kc3rdQuestConditionConsumption,
) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    match consumption {
        Kc3rdQuestConditionConsumption::Resources(res) => {
            let mats = vec![
                (MaterialCategory::Fuel, res.fuel),
                (MaterialCategory::Ammo, res.ammo),
                (MaterialCategory::Steel, res.steel),
                (MaterialCategory::Bauxite, res.bauxite),
            ];
            deduct_material_impl(c, profile_id, &mats).await?;
        }
        // Resolved and taken with the rest of the quest's equipment (`holding`).
        Kc3rdQuestConditionConsumption::SlotItemConsumption(_) => {}
        Kc3rdQuestConditionConsumption::UseItemConsumption(items) => {
            for item in items {
                deduct_use_item_impl(c, profile_id, item.api_id, item.amount).await?;
            }
        }
    }
    Ok(())
}
