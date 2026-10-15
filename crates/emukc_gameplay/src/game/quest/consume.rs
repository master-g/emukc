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
                // Use items 1 to 4 live in the material table.
                let material = match item.api_id {
                    1 => Some(MaterialCategory::Bucket),
                    2 => Some(MaterialCategory::Torch),
                    3 => Some(MaterialCategory::DevMat),
                    4 => Some(MaterialCategory::Screw),
                    _ => None,
                };
                match material {
                    Some(material) => {
                        deduct_material_impl(c, profile_id, &[(material, item.amount)]).await?;
                    }
                    None => {
                        deduct_use_item_impl(c, profile_id, item.api_id, item.amount).await?;
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use emukc_db::prelude::new_mem_db;
    use emukc_model::{
        codex::Codex,
        kc2::MaterialCategory,
        thirdparty::{Kc3rdQuestConditionConsumption, Kc3rdQuestConditionUseItemConsumption},
    };

    use super::handle_consumption;
    use crate::gameplay::Ctx;

    /// 658 and 693 take 開発資材 as use item 3, which the material table
    /// holds: the claim must take it from there.
    #[tokio::test]
    async fn a_material_backed_use_item_is_taken_from_the_materials() {
        let db = new_mem_db().await.unwrap();
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let context = Ctx::new(Arc::new(db), Arc::new(codex));
        let account = context.sign_up("quest-devmat", "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "quest-devmat").await.unwrap();
        let pid = context
            .start_game(&account.access_token.token, profile.profile.id)
            .await
            .unwrap()
            .profile
            .id;
        let before = context.get_materials(pid).await.unwrap().devmat;
        context.add_material(pid, &[(MaterialCategory::DevMat, 20)]).await.unwrap();

        let consumption = Kc3rdQuestConditionConsumption::UseItemConsumption(vec![
            Kc3rdQuestConditionUseItemConsumption {
                api_id: 3,
                amount: 20,
            },
        ]);
        handle_consumption(context.db.as_ref(), pid, &consumption).await.unwrap();

        assert_eq!(context.get_materials(pid).await.unwrap().devmat, before);
    }
}
