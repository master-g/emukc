use axum::Extension;
use serde::Serialize;

use crate::net::{
    AppState,
    auth::GameSession,
    resp::{KcApiResponse, KcApiResult},
    router::kcs2::GIT_HASH,
};
use emukc_internal::prelude::*;

#[derive(Serialize)]
struct Resp {
    api_material: Vec<KcApiMaterialElement>,
    api_deck_port: Vec<KcApiDeckPort>,
    api_ndock: Vec<KcApiNDock>,
    api_ship: Vec<KcApiShip>,
    api_basic: KcApiUserBasic,
    api_log: Vec<KcApiLogElement>,
    api_combined_flag: i64,
    api_p_bgm_id: i64,
    // api_event_object: KcApiEventObject,
    api_parallel_quest_count: i64,
    api_dest_ship_slot: i64,
    // api_plane_info: Vec<KcApiPlaneInfo>,
    // api_furniture_affect_items: Vec<i64>,
    api_c_flags: Vec<i64>,
    api_c_flag2: i64,
}

pub(super) async fn handler(
    state: AppState,
    Extension(session): Extension<GameSession>,
) -> KcApiResult {
    let pid = session.profile.id;

    // Clear stale sortie state from mid-sortie disconnects.
    state.clear_sortie_state_if_any(pid).await;

    let resp = build_port_response(state.0.as_ref(), pid).await?;

    Ok(KcApiResponse::success(&resp))
}

async fn build_port_response<T: GameOps + ?Sized>(
    state: &T,
    pid: i64,
) -> Result<Resp, GameplayError> {
    let (_, api_basic) = state.get_user_basic(pid).await?;

    state.update_materials(pid).await?;

    // TODO(#0): update quests here

    let api_material = state.get_materials(pid).await?;
    let api_material: Vec<KcApiMaterialElement> = api_material.into();

    let api_deck_port = state.get_fleets(pid).await?;
    let api_deck_port: Vec<KcApiDeckPort> =
        api_deck_port.into_iter().map(std::convert::Into::into).collect();
    let api_dest_ship_slot = 1;
    let api_ndock = state.get_ndocks(pid).await?;
    let api_ndock: Vec<KcApiNDock> = api_ndock.into_iter().map(std::convert::Into::into).collect();

    let api_ship = state.get_ships(pid).await?;

    let ver = format!("Welcome to EmuKC {}-{}", VERSION, GIT_HASH.to_uppercase());
    let api_log = vec![KcApiLogElement {
        api_no: 0,
        api_type: "10".to_string(),
        api_state: "0".to_string(),
        api_message: ver,
    }];

    // log type
    // 1: ndock
    // 2: factory
    // 3: expedition
    // 4: provision
    // 5: practice
    // 6: medal
    // 7: sortie
    // 8: quest
    // 9: apply
    // 10: promotion
    // 11: picturebook
    // 12: complete
    // 13: n/a
    // 14: sortie
    // 15: remodel

    let settings = state.get_game_settings(pid).await?;
    let api_p_bgm_id = settings.api_p_bgm_id;
    let api_combined_flag = state.get_combined_type(pid).await?;
    let api_parallel_quest_count = api_basic.api_max_quests;
    let api_c_flags: Vec<i64> = vec![0]; // event functional flags
    let api_c_flag2 = 0; // mini event item usage lock flag

    Ok(Resp {
        api_material,
        api_deck_port,
        api_dest_ship_slot,
        api_ndock,
        api_ship,
        api_basic,
        api_log,
        api_p_bgm_id,
        api_parallel_quest_count,
        api_c_flags,
        api_c_flag2,
        api_combined_flag,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use emukc_internal::{
        db::{
            entity::profile::quest,
            prelude::new_mem_db,
            sea_orm::{ActiveModelTrait, ActiveValue},
        },
        time::chrono::Utc,
    };
    use std::path::PathBuf;

    async fn new_game_session() -> ((emukc_internal::db::sea_orm::DbConn, Codex), StartGameInfo) {
        let db = new_mem_db().await.unwrap();
        let codex_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".data/codex");
        let codex = Codex::load_without_cache_source(codex_root).unwrap();
        let context = (db, codex);

        let account = context.sign_up("test", "1234567").await.unwrap();
        let profile = context.new_profile(&account.access_token.token, "admin").await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();

        (context, session)
    }

    async fn insert_completed_quest(
        context: &(emukc_internal::db::sea_orm::DbConn, Codex),
        profile_id: i64,
        quest_id: i64,
    ) {
        let quest_manifest = context.1.quest.get(&quest_id).unwrap();
        let (requirements, requirement_type) = match &quest_manifest.requirements {
            Kc3rdQuestRequirement::And(conditions) => {
                (conditions.clone(), quest::progress::RequirementType::And)
            }
            Kc3rdQuestRequirement::OneOf(conditions) => {
                (conditions.clone(), quest::progress::RequirementType::OneOf)
            }
            Kc3rdQuestRequirement::Sequential(conditions) => {
                (conditions.clone(), quest::progress::RequirementType::Sequential)
            }
        };

        quest::progress::ActiveModel {
            id: ActiveValue::NotSet,
            profile_id: ActiveValue::Set(profile_id),
            quest_id: ActiveValue::Set(quest_id),
            status: ActiveValue::Set(quest::progress::Status::Activated),
            progress: ActiveValue::Set(quest::progress::Progress::Completed),
            period: ActiveValue::Set(quest_manifest.period.into()),
            start_since: ActiveValue::Set(Utc::now()),
            requirements: ActiveValue::Set(serde_json::to_value(requirements).unwrap()),
            requirement_type: ActiveValue::Set(requirement_type),
        }
        .insert(&context.0)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn quest_reward_claim_updates_materials_and_persists_slotitem_reward() {
        let (context, session) = new_game_session().await;
        let pid = session.profile.id;
        let quest_id = 103;

        insert_completed_quest(&context, pid, quest_id).await;

        let before_materials = context.get_materials(pid).await.unwrap();
        let before_slotitems = context.get_slot_items(pid).await.unwrap();

        let reward_resp = context.quest_clear_and_claim_reward(pid, quest_id, None).await.unwrap();
        assert_eq!(reward_resp.api_material, [40, 40, 0, 40]);

        let after_materials = context.get_materials(pid).await.unwrap();
        let after_slotitems = context.get_slot_items(pid).await.unwrap();
        assert_eq!(after_materials.fuel, before_materials.fuel + 40);
        assert_eq!(after_materials.ammo, before_materials.ammo + 40);
        assert_eq!(after_materials.bauxite, before_materials.bauxite + 40);
        assert_eq!(after_slotitems.len(), before_slotitems.len() + 1);
        assert!(after_slotitems.iter().any(|item| item.api_slotitem_id == 42));

        let port = build_port_response(&context, pid).await.unwrap();
        assert_eq!(
            port.api_material.iter().map(|entry| entry.api_value).collect::<Vec<_>>()[..4],
            [
                after_materials.fuel,
                after_materials.ammo,
                after_materials.steel,
                after_materials.bauxite
            ]
        );
    }
}
