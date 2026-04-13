//! Composition quest activation integration tests.

use emukc_db::{
    entity::profile::quest::progress,
    prelude::new_mem_db,
    sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter},
};
use emukc_gameplay::prelude::*;
use emukc_model::{
    codex::{Codex, query::FoundInCodex},
    kc2::KcShipType,
    thirdparty::{
        Kc3rdQuest, Kc3rdQuestCondition, Kc3rdQuestConditionComposition, Kc3rdQuestConditionShip,
        Kc3rdQuestConditionShipGroup, Kc3rdQuestRequirement, Kc3rdQuestShipAmount,
    },
};
use emukc_time::chrono::Utc;

async fn mock_context() -> (emukc_db::sea_orm::DbConn, Codex) {
    let db = new_mem_db().await.unwrap();
    let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
    (db, codex)
}

async fn new_game_session() -> ((emukc_db::sea_orm::DbConn, Codex), StartGameInfo) {
    let context = mock_context().await;

    let account = context.sign_up("test", "1234567").await.unwrap();
    let profile = context.new_profile(&account.access_token.token, "admin").await.unwrap();
    let session =
        context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();

    (context, session)
}

fn first_ship_mst_by_type(codex: &Codex, ship_type: KcShipType) -> i64 {
    codex
        .manifest
        .api_mst_ship
        .iter()
        .find(|mst| KcShipType::n(mst.api_stype) == Some(ship_type))
        .map(|mst| mst.api_id)
        .unwrap()
}

#[tokio::test]
async fn activating_satisfied_composition_quest_completes_it_immediately() {
    let (context, session) = new_game_session().await;
    let pid = session.profile.id;
    let dd_mst = first_ship_mst_by_type(&context.1, KcShipType::DD);
    let ship = context.add_ship(pid, dd_mst).await.unwrap();
    context.update_fleet_ships(pid, 1, &[ship.api_id, -1, -1, -1, -1, -1]).await.unwrap();

    let conditions = vec![Kc3rdQuestCondition::Composition(Kc3rdQuestConditionComposition {
        groups: vec![Kc3rdQuestConditionShipGroup {
            ship: Kc3rdQuestConditionShip::ShipType(vec![KcShipType::DD as i64]),
            amount: Kc3rdQuestShipAmount::exact(1),
            lv: 1,
            position: 0,
            other_ships: false,
            white_list: None,
        }],
        disallowed: None,
        fleet_id: 1,
    })];

    progress::ActiveModel {
        id: ActiveValue::NotSet,
        profile_id: ActiveValue::Set(pid),
        quest_id: ActiveValue::Set(999001),
        status: ActiveValue::Set(progress::Status::Idle),
        progress: ActiveValue::Set(progress::Progress::Empty),
        period: ActiveValue::Set(emukc_db::entity::profile::quest::Period::Oneshot),
        start_since: ActiveValue::Set(Utc::now()),
        requirement_type: ActiveValue::Set(progress::RequirementType::And),
        requirements: ActiveValue::Set(serde_json::to_value(conditions).unwrap()),
    }
    .insert(&context.0)
    .await
    .unwrap();

    context.quest_start(pid, 999001).await.unwrap();

    let quest = progress::Entity::find()
        .filter(progress::Column::ProfileId.eq(pid))
        .filter(progress::Column::QuestId.eq(999001))
        .one(&context.0)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(quest.status, progress::Status::Activated);
    assert_eq!(quest.progress, progress::Progress::Completed);
}

#[tokio::test]
async fn activating_a1_with_more_than_two_ships_completes_immediately() {
    let (context, session) = new_game_session().await;
    let pid = session.profile.id;
    let dd_mst = first_ship_mst_by_type(&context.1, KcShipType::DD);
    let ship1 = context.add_ship(pid, dd_mst).await.unwrap();
    let ship2 = context.add_ship(pid, dd_mst).await.unwrap();
    let ship3 = context.add_ship(pid, dd_mst).await.unwrap();
    context
        .update_fleet_ships(pid, 1, &[ship1.api_id, ship2.api_id, ship3.api_id, -1, -1, -1])
        .await
        .unwrap();

    let quest_manifest = Kc3rdQuest::find_in_codex(&context.1, &101).unwrap();
    let (requirement_type, requirements) = match &quest_manifest.requirements {
        Kc3rdQuestRequirement::And(conditions) => {
            (progress::RequirementType::And, conditions.clone())
        }
        Kc3rdQuestRequirement::OneOf(conditions) => {
            (progress::RequirementType::OneOf, conditions.clone())
        }
        Kc3rdQuestRequirement::Sequential(conditions) => {
            (progress::RequirementType::Sequential, conditions.clone())
        }
    };

    progress::ActiveModel {
        id: ActiveValue::NotSet,
        profile_id: ActiveValue::Set(pid),
        quest_id: ActiveValue::Set(101),
        status: ActiveValue::Set(progress::Status::Idle),
        progress: ActiveValue::Set(progress::Progress::Empty),
        period: ActiveValue::Set(emukc_db::entity::profile::quest::Period::Oneshot),
        start_since: ActiveValue::Set(Utc::now()),
        requirement_type: ActiveValue::Set(requirement_type),
        requirements: ActiveValue::Set(serde_json::to_value(requirements).unwrap()),
    }
    .insert(&context.0)
    .await
    .unwrap();

    context.quest_start(pid, 101).await.unwrap();

    let quest = progress::Entity::find()
        .filter(progress::Column::ProfileId.eq(pid))
        .filter(progress::Column::QuestId.eq(101))
        .one(&context.0)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(quest.status, progress::Status::Activated);
    assert_eq!(quest.progress, progress::Progress::Completed);
}

#[tokio::test]
async fn activating_a1_with_stale_stored_requirements_still_completes() {
    let (context, session) = new_game_session().await;
    let pid = session.profile.id;
    let dd_mst = first_ship_mst_by_type(&context.1, KcShipType::DD);
    let ship1 = context.add_ship(pid, dd_mst).await.unwrap();
    let ship2 = context.add_ship(pid, dd_mst).await.unwrap();
    let ship3 = context.add_ship(pid, dd_mst).await.unwrap();
    context
        .update_fleet_ships(pid, 1, &[ship1.api_id, ship2.api_id, ship3.api_id, -1, -1, -1])
        .await
        .unwrap();

    // Simulate an existing old-profile quest record created before the parser/validator fix.
    let stale_conditions = vec![Kc3rdQuestCondition::Composition(Kc3rdQuestConditionComposition {
        groups: vec![Kc3rdQuestConditionShipGroup {
            ship: Kc3rdQuestConditionShip::Any,
            amount: Kc3rdQuestShipAmount::exact(2),
            lv: 0,
            position: 0,
            other_ships: false,
            white_list: None,
        }],
        disallowed: None,
        fleet_id: 0,
    })];

    progress::ActiveModel {
        id: ActiveValue::NotSet,
        profile_id: ActiveValue::Set(pid),
        quest_id: ActiveValue::Set(101),
        status: ActiveValue::Set(progress::Status::Idle),
        progress: ActiveValue::Set(progress::Progress::Empty),
        period: ActiveValue::Set(emukc_db::entity::profile::quest::Period::Oneshot),
        start_since: ActiveValue::Set(Utc::now()),
        requirement_type: ActiveValue::Set(progress::RequirementType::And),
        requirements: ActiveValue::Set(serde_json::to_value(stale_conditions).unwrap()),
    }
    .insert(&context.0)
    .await
    .unwrap();

    context.quest_start(pid, 101).await.unwrap();

    let quest = progress::Entity::find()
        .filter(progress::Column::ProfileId.eq(pid))
        .filter(progress::Column::QuestId.eq(101))
        .one(&context.0)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(quest.status, progress::Status::Activated);
    assert_eq!(quest.progress, progress::Progress::Completed);
}
