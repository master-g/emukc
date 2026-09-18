use super::*;
use crate::game::battle::sortie::{
    SortieBattleInput, pending_battle, run_day_battle, run_sp_midnight_battle,
};
use crate::game::map_progress::assign_stage_id;
use crate::game::sortie_result::SortieBattleResultSnapshot;
use emukc_battle::BattleContext;
use emukc_bootstrap::prelude::build_final_map_catalog_from_repo_assets;
use emukc_db::{
    entity::profile::{map_record, material as profile_material, ship as profile_ship},
    prelude::new_mem_db,
    sea_orm::{
        ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter,
    },
};
use emukc_model::{
    codex::{
        Codex,
        map::{MapDefinition, MapVariantDefinition},
    },
    kc2::level,
    prelude::ApiMstShip,
};
use emukc_time::chrono::Utc;
use std::collections::BTreeMap;
use std::sync::Arc;

fn sample_ship(codex: &Codex, mst_id: i64, level: i64) -> BattleShipInput {
    let (mut ship, slot_items) = codex.new_ship(mst_id).unwrap();
    let exp_now = level::ship_level_required_exp(level);
    let (_, next_exp) = level::exp_to_ship_level(exp_now);
    ship.api_lv = level;
    ship.api_exp = [exp_now, next_exp, 0];
    codex.cal_ship_status(&mut ship, &slot_items, false).unwrap();
    BattleShipInput {
        ship,
        slot_items,
        effect_list: vec![0],
        married: false,
    }
}

fn weaken_for_midnight(mut ship: BattleShipInput) -> BattleShipInput {
    ship.ship.api_karyoku[0] = 1;
    ship.ship.api_raisou[0] = 0;
    ship.ship.api_soukou[0] = 200;
    ship
}

fn successful_boss_snapshot() -> SortieBattleResultSnapshot {
    SortieBattleResultSnapshot {
        friendly_ship_ids: vec![],
        enemy_ship_ids: vec![],
        friendly_nowhps: vec![],
        enemy_ship_types: vec![],
        enemy_nowhps: vec![],
        win_rank: "S".to_string(),
        get_exp: 0,
        member_lv: 0,
        member_exp: 0,
        get_base_exp: 0,
        mvp: 0,
        get_ship_exp: vec![],
        get_exp_lvup: vec![],
        quest_name: String::new(),
        quest_level: 0,
        enemy_level: 0,
        enemy_rank: String::new(),
        enemy_deck_name: String::new(),
    }
}

#[tokio::test]
async fn sortie_midnight_battle_updates_pending_snapshot() {
    let db = new_mem_db().await.unwrap();
    let mut codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
    // This test asserts real battle outcome (both sides survive → midnight).
    // Disable god-mode debug flags so a local game_config.json with them on
    // (one_hit_kill synthesizes a finishing volley) cannot corrupt the result.
    codex.game_cfg.god_mode = false;
    codex.game_cfg.one_hit_kill = false;
    let context = Ctx::new(Arc::new(db), Arc::new(codex.clone()));
    // Seed the same store the context reads from.
    let store = context.sortie_store.as_ref();
    let profile_id = 42;

    let friend = weaken_for_midnight(sample_ship(&codex, 79, 1));
    let enemy = weaken_for_midnight(sample_ship(&codex, 412, 99));
    let mut rng = ProductionRng;
    let session = run_day_battle(
        store,
        &codex,
        SortieBattleInput {
            profile_id,
            deck_id: 1,
            map_id: 11,
            cell_id: 1,
            context: BattleContext::head_on(
                BattleType::Normal,
                true,
                vec![friend.clone()],
                vec![enemy.clone()],
            ),
        },
        &mut rng,
    );

    assert_eq!(session.packet.midnight_flag, 1);
    store.insert_pending_result(
        profile_id,
        SortieBattleResultSnapshot {
            friendly_ship_ids: session.friendly_ship_ids.clone(),
            enemy_ship_ids: session.enemy_ship_ids.clone(),
            friendly_nowhps: session.friendly.iter().map(|f| f.hp().max(0)).collect(),
            enemy_ship_types: session
                .enemy_ship_ids
                .iter()
                .map(|&id| codex.find::<ApiMstShip>(&id).map(|m| m.api_stype).unwrap_or(0))
                .collect(),
            enemy_nowhps: session.packet.enemy_nowhps.clone(),
            win_rank: session.outcome.win_rank.to_string(),
            get_exp: 0,
            member_lv: 1,
            member_exp: 0,
            get_base_exp: 30,
            mvp: session.outcome.mvp,
            get_ship_exp: vec![],
            get_exp_lvup: vec![],
            quest_name: "test".to_string(),
            quest_level: 1,
            enemy_level: 1,
            enemy_rank: "Test".to_string(),
            enemy_deck_name: "Test".to_string(),
        },
    );

    let response = context.sortie_midnight_battle(profile_id).await.unwrap();
    assert_eq!(response.api_deck_id, 1);
    assert!(response.api_hougeki.is_some());

    let updated_snapshot = store.take_pending_result(profile_id).unwrap();
    assert!(!updated_snapshot.win_rank.is_empty());
    assert!(updated_snapshot.mvp >= 1);

    let stored = pending_battle(store, profile_id).unwrap();
    assert_eq!(stored.packet.midnight_flag, 0);

    let _ = take_day_battle_result(store, profile_id);
    store.clear();
}

#[tokio::test]
async fn sortie_sp_midnight_battle_runs_night_only() {
    use crate::game::sortie_store::GLOBAL_SORTIE_STORE;
    let store = &*GLOBAL_SORTIE_STORE;
    store.clear();

    let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
    let profile_id = 84;

    let friend = weaken_for_midnight(sample_ship(&codex, 79, 1));
    let enemy = weaken_for_midnight(sample_ship(&codex, 412, 99));

    let mut rng = ProductionRng;
    let (day_session, night_session) = run_sp_midnight_battle(
        store,
        &codex,
        SortieBattleInput {
            profile_id,
            deck_id: 1,
            map_id: 11,
            cell_id: 1,
            context: BattleContext::head_on(
                BattleType::Normal,
                true,
                vec![friend.clone()],
                vec![enemy.clone()],
            ),
        },
        1,
        &mut rng,
    );

    // Day packet should have no combat phases (sp_midnight skips day battle)
    assert!(day_session.packet.kouku.is_none());
    assert!(day_session.packet.hougeki1.is_none());
    assert!(day_session.packet.opening_taisen.is_none());
    assert_eq!(day_session.packet.hourai_flag, [0, 0, 0, 0]);

    // Night battle should have run
    assert!(night_session.packet.hougeki.is_some());
    assert_eq!(night_session.profile_id, profile_id);

    // The stored session should have been updated with night results
    let stored = pending_battle(store, profile_id).unwrap();
    assert_eq!(stored.packet.midnight_flag, 0); // no further midnight allowed

    clear_pending_sortie_runtime_state(store, profile_id);
}

#[tokio::test]
async fn sortie_god_mode_keeps_friendly_at_full_hp_end_to_end() {
    use crate::game::sortie_store::GLOBAL_SORTIE_STORE;
    let store = &*GLOBAL_SORTIE_STORE;
    store.clear();

    let mut codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
    codex.game_cfg.god_mode = true;
    let profile_id = 770_001;

    // A fragile flagship against a high-firepower enemy: sinking protection alone
    // only prevents the kill, not the damage — so without god_mode the flagship
    // would end below full HP. god_mode must restore it to full entry HP.
    let friend = sample_ship(&codex, 1, 1);
    let entry_hp = friend.ship.api_nowhp;
    let mut enemy = sample_ship(&codex, 412, 99);
    enemy.ship.api_karyoku[0] = 200;

    let mut rng = ProductionRng;
    let session = run_day_battle(
        store,
        &codex,
        SortieBattleInput {
            profile_id,
            deck_id: 1,
            map_id: 11,
            cell_id: 1,
            context: BattleContext::head_on(BattleType::Normal, true, vec![friend], vec![enemy]),
        },
        &mut rng,
    );

    // god_mode invariant — holds under any RNG outcome.
    assert_eq!(
        session.friendly[0].hp(),
        entry_hp,
        "god_mode must restore the friendly to full entry HP end-to-end"
    );
    assert_eq!(session.packet.friendly_nowhps[0], entry_hp);

    let _ = take_day_battle_result(store, profile_id);
    store.clear();
}

#[tokio::test]
async fn sortie_one_hit_kill_clears_enemies_and_rejects_night_battle() {
    let mut codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
    codex.game_cfg.one_hit_kill = true;
    let context = Ctx::new(Arc::new(new_mem_db().await.unwrap()), Arc::new(codex.clone()));
    // Seed the same store the context reads from.
    let store = context.sortie_store.as_ref();
    let profile_id = 770_002;

    // Tanky friendly + two tanky enemies with weak attack: a normal day battle
    // leaves both sides alive, so a night battle would be offered. one_hit_kill
    // must sink the enemies and force the night gate shut.
    let mut friend = sample_ship(&codex, 79, 1);
    friend.ship.api_soukou[0] = 200;
    friend.ship.api_nowhp = 200;
    friend.ship.api_maxhp = 200;

    let mut enemy_a = sample_ship(&codex, 412, 99);
    enemy_a.ship.api_karyoku[0] = 1;
    enemy_a.ship.api_soukou[0] = 200;
    enemy_a.ship.api_nowhp = 200;
    enemy_a.ship.api_maxhp = 200;
    let mut enemy_b = sample_ship(&codex, 412, 99);
    enemy_b.ship.api_karyoku[0] = 1;
    enemy_b.ship.api_soukou[0] = 200;
    enemy_b.ship.api_nowhp = 200;
    enemy_b.ship.api_maxhp = 200;

    let mut rng = ProductionRng;
    let session = run_day_battle(
        store,
        &codex,
        SortieBattleInput {
            profile_id,
            deck_id: 1,
            map_id: 11,
            cell_id: 1,
            context: BattleContext::head_on(
                BattleType::Normal,
                true,
                vec![friend],
                vec![enemy_a, enemy_b],
            ),
        },
        &mut rng,
    );

    // one_hit_kill invariants — every enemy dead, midnight forced shut.
    assert!(
        session.packet.enemy_nowhps.iter().all(|&hp| hp == 0),
        "one_hit_kill must sink every enemy"
    );
    assert!(!session.outcome.can_midnight, "one_hit_kill must clear can_midnight");
    assert_eq!(session.packet.midnight_flag, 0);

    // End-to-end: the night-battle request is rejected by the can_midnight gate.
    let err = context.sortie_midnight_battle(profile_id).await.unwrap_err();
    assert!(
        matches!(err, crate::err::GameplayError::WrongType(_)),
        "night battle must be rejected after one_hit_kill, got {err:?}"
    );

    let _ = take_day_battle_result(store, profile_id);
    store.clear();
}

#[tokio::test]
async fn maelstrom_drains_ship_resource_without_touching_profile_materials() {
    let db = new_mem_db().await.unwrap();
    let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
    let context = Ctx::new(Arc::new(db), Arc::new(codex));
    let account = context.sign_up("maelstrom-loss", "1234567").await.unwrap();
    let profile =
        context.new_profile(&account.access_token.token, "maelstrom-admin").await.unwrap();
    let session =
        context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
    let profile_id = session.profile.id;
    let ship = context.add_ship(profile_id, 951).await.unwrap();

    let ship_before = profile_ship::Entity::find_by_id(ship.api_id)
        .one(context.db.as_ref())
        .await
        .unwrap()
        .unwrap();
    let materials_before = profile_material::Entity::find()
        .filter(profile_material::Column::ProfileId.eq(profile_id))
        .one(context.db.as_ref())
        .await
        .unwrap()
        .unwrap();

    let cell = MapCellDefinition {
        cell_no: 99,
        color_no: 3,
        event_id: 3,
        event_kind: 0,
        next_cells: vec![],
        node_label: Some("H".to_string()),
        master_cell_id: None,
        distance: None,
    };

    let (itemget, happening) = resolve_non_battle_node_effect(
        context.db.as_ref(),
        context.codex.as_ref(),
        profile_id,
        &cell,
        std::slice::from_ref(&ship_before),
    )
    .await
    .unwrap();
    assert!(itemget.is_none());
    let happening = happening.expect("maelstrom should produce a happening response");
    assert_eq!(happening.resource_type, 1);
    assert!(happening.amount > 0);

    let ship_after = profile_ship::Entity::find_by_id(ship.api_id)
        .one(context.db.as_ref())
        .await
        .unwrap()
        .unwrap();
    let materials_after = profile_material::Entity::find()
        .filter(profile_material::Column::ProfileId.eq(profile_id))
        .one(context.db.as_ref())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(materials_after, materials_before);
    assert_eq!(ship_after.ammo, ship_before.ammo);
    assert!(ship_after.fuel < ship_before.fuel);
    assert_eq!(ship_before.fuel - ship_after.fuel, happening.amount);
}

#[tokio::test]
async fn maelstrom_drains_ammo_when_color_no_is_4() {
    let db = new_mem_db().await.unwrap();
    let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
    let context = Ctx::new(Arc::new(db), Arc::new(codex));
    let account = context.sign_up("maelstrom-ammo", "1234567").await.unwrap();
    let profile =
        context.new_profile(&account.access_token.token, "maelstrom-ammo-admin").await.unwrap();
    let session =
        context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
    let profile_id = session.profile.id;
    let ship = context.add_ship(profile_id, 951).await.unwrap();

    let ship_before = profile_ship::Entity::find_by_id(ship.api_id)
        .one(context.db.as_ref())
        .await
        .unwrap()
        .unwrap();
    let materials_before = profile_material::Entity::find()
        .filter(profile_material::Column::ProfileId.eq(profile_id))
        .one(context.db.as_ref())
        .await
        .unwrap()
        .unwrap();

    let cell = MapCellDefinition {
        cell_no: 99,
        color_no: 4,
        event_id: 3,
        event_kind: 0,
        next_cells: vec![],
        node_label: Some("H".to_string()),
        master_cell_id: None,
        distance: None,
    };

    let (itemget, happening) = resolve_non_battle_node_effect(
        context.db.as_ref(),
        context.codex.as_ref(),
        profile_id,
        &cell,
        std::slice::from_ref(&ship_before),
    )
    .await
    .unwrap();

    assert!(itemget.is_none());
    let happening = happening.expect("maelstrom ammo drain should produce a happening");
    assert_eq!(happening.resource_type, 2, "color_no=4 should drain ammo");
    assert!(happening.amount > 0);

    let ship_after = profile_ship::Entity::find_by_id(ship.api_id)
        .one(context.db.as_ref())
        .await
        .unwrap()
        .unwrap();
    let materials_after = profile_material::Entity::find()
        .filter(profile_material::Column::ProfileId.eq(profile_id))
        .one(context.db.as_ref())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(materials_after, materials_before);
    assert_eq!(ship_after.fuel, ship_before.fuel, "ammo maelstrom must not touch fuel");
    assert!(ship_after.ammo < ship_before.ammo);
    assert_eq!(ship_before.ammo - ship_after.ammo, happening.amount);
}

async fn equip_radar_on_ship(
    db: &emukc_db::sea_orm::DatabaseConnection,
    codex: &Codex,
    profile_id: i64,
    ship_api_id: i64,
) -> profile_ship::Model {
    use crate::game::slot_item::add_slot_item_impl;
    use emukc_db::entity::profile::item::slot_item;

    let radar_mst_id = 27; // 13号対空電探, type3=12
    let radar = add_slot_item_impl(db, codex, profile_id, radar_mst_id, 0, 0).await.unwrap();

    let ship_model = profile_ship::Entity::find_by_id(ship_api_id).one(db).await.unwrap().unwrap();
    let mut am = ship_model.into_active_model();
    am.slot_1 = ActiveValue::Set(radar.id);
    am.update(db).await.unwrap();

    let radar_am = slot_item::ActiveModel {
        id: ActiveValue::Set(radar.id),
        equip_on: ActiveValue::Set(ship_api_id),
        ..radar.into_active_model()
    };
    radar_am.update(db).await.unwrap();

    profile_ship::Entity::find_by_id(ship_api_id).one(db).await.unwrap().unwrap()
}

#[tokio::test]
async fn maelstrom_radar_reduces_fuel_loss_across_all_tiers() {
    // Exercises every arm of the `radar_ship_count` match in
    // `resolve_non_battle_node_effect` (sortie/mod.rs): 0, 1, 2, 3, 4, 5, 6+ ships.
    //
    // Each iteration builds a fresh 6-ship fleet with fuel overridden to 1000 so
    // per-tier losses are distinguishable:
    //   N=0 r=0.00 -> per_ship=300 -> total=1800
    //   N=1 r=0.25 -> per_ship=225 -> total=1350
    //   N=2 r=0.40 -> per_ship=180 -> total=1080
    //   N=3 r=0.50 -> per_ship=150 -> total= 900
    //   N=4 r=0.55 -> per_ship=135 -> total= 810
    //   N=5 r=0.58 -> per_ship=126 -> total= 756
    //   N=6 r=0.60 -> per_ship=120 -> total= 720
    let db = new_mem_db().await.unwrap();
    let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
    let context = Ctx::new(Arc::new(db), Arc::new(codex.clone()));
    let account = context.sign_up("maelstrom-radar", "1234567").await.unwrap();
    let profile =
        context.new_profile(&account.access_token.token, "maelstrom-radar-admin").await.unwrap();
    let session =
        context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
    let profile_id = session.profile.id;

    let cell = MapCellDefinition {
        cell_no: 99,
        color_no: 3,
        event_id: 3,
        event_kind: 0,
        next_cells: vec![],
        node_label: Some("H".to_string()),
        master_cell_id: None,
        distance: None,
    };

    const STOCK: i64 = 1000;
    let expected_totals = [1800_i64, 1350, 1080, 900, 810, 756, 720];

    for n_radars in 0usize..=6 {
        // Fresh 6-ship fleet per iteration avoids needing to unequip slot items.
        let mut ship_ids = Vec::with_capacity(6);
        for _ in 0..6 {
            let s = context.add_ship(profile_id, 951).await.unwrap();
            ship_ids.push(s.api_id);
        }

        // Equip radars on the first N ships of this fleet.
        for &api_id in &ship_ids[..n_radars] {
            equip_radar_on_ship(context.db.as_ref(), &codex, profile_id, api_id).await;
        }

        // Override fuel to STOCK on all 6 ships AFTER equip (equip_radar_on_ship
        // rewrites the ship row and would otherwise reset `fuel`).
        let mut fleet = Vec::with_capacity(6);
        for &api_id in &ship_ids {
            let model = profile_ship::Entity::find_by_id(api_id)
                .one(context.db.as_ref())
                .await
                .unwrap()
                .unwrap();
            let mut am = model.into_active_model();
            am.fuel = ActiveValue::Set(STOCK);
            am.update(context.db.as_ref()).await.unwrap();
            let fresh = profile_ship::Entity::find_by_id(api_id)
                .one(context.db.as_ref())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(fresh.fuel, STOCK);
            fleet.push(fresh);
        }

        let (itemget, happening) = resolve_non_battle_node_effect(
            context.db.as_ref(),
            context.codex.as_ref(),
            profile_id,
            &cell,
            &fleet,
        )
        .await
        .unwrap();
        assert!(itemget.is_none());
        let happening = happening.expect("maelstrom always emits a happening");
        assert_eq!(happening.resource_type, 1);
        assert_eq!(
            happening.amount, expected_totals[n_radars],
            "tier N={n_radars}: expected total fuel loss {}",
            expected_totals[n_radars]
        );
        assert_eq!(happening.radar_reduced, n_radars > 0);

        // Integration: fuel loss is persisted per-ship, not only reported.
        let per_ship_loss = expected_totals[n_radars] / 6;
        for &api_id in &ship_ids {
            let after = profile_ship::Entity::find_by_id(api_id)
                .one(context.db.as_ref())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(
                after.fuel,
                STOCK - per_ship_loss,
                "tier N={n_radars}: per-ship fuel should drop by {per_ship_loss}"
            );
        }
    }
}

#[tokio::test]
async fn maelstrom_zero_resource_ship_skips_loss_without_underflow() {
    let db = new_mem_db().await.unwrap();
    let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
    let context = Ctx::new(Arc::new(db), Arc::new(codex));
    let account = context.sign_up("maelstrom-zero", "1234567").await.unwrap();
    let profile =
        context.new_profile(&account.access_token.token, "maelstrom-zero-admin").await.unwrap();
    let session =
        context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
    let profile_id = session.profile.id;
    let ship = context.add_ship(profile_id, 951).await.unwrap();

    // Drain fuel to 0
    let ship_model = profile_ship::Entity::find_by_id(ship.api_id)
        .one(context.db.as_ref())
        .await
        .unwrap()
        .unwrap();
    let mut am = ship_model.into_active_model();
    am.fuel = ActiveValue::Set(0);
    am.update(context.db.as_ref()).await.unwrap();

    let ship_zero = profile_ship::Entity::find_by_id(ship.api_id)
        .one(context.db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(ship_zero.fuel, 0);

    let cell = MapCellDefinition {
        cell_no: 99,
        color_no: 3,
        event_id: 3,
        event_kind: 0,
        next_cells: vec![],
        node_label: Some("H".to_string()),
        master_cell_id: None,
        distance: None,
    };

    let (itemget, happening) = resolve_non_battle_node_effect(
        context.db.as_ref(),
        context.codex.as_ref(),
        profile_id,
        &cell,
        std::slice::from_ref(&ship_zero),
    )
    .await
    .unwrap();

    assert!(itemget.is_none());
    // event_id=3 always emits a happening; a 0-fuel ship is skipped by the
    // `ship_loss <= 0` guard so total_loss stays 0 but the happening is still Some.
    let happening = happening.expect("maelstrom always emits a happening at event_id=3");
    assert_eq!(happening.resource_type, 1, "color_no=3 drains fuel");
    assert_eq!(happening.amount, 0, "zero-stock ship contributes zero loss");
    assert!(!happening.radar_reduced, "no radars equipped");

    let ship_after = profile_ship::Entity::find_by_id(ship.api_id)
        .one(context.db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(ship_after.fuel, 0, "no underflow");
}

#[test]
fn kouku_and_shelling_combined_sinking_protection_keeps_flagship_alive() {
    use crate::game::sortie_store::GLOBAL_SORTIE_STORE;
    let store = &*GLOBAL_SORTIE_STORE;
    store.clear();

    let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
    let profile_id = 99991;

    // Flagship: DD at taiha HP (1 HP, maxhp ~13-24 depending on level).
    // taiha: entry_hp * 4 <= maxhp → 1*4=4 <= 13 → taiha → flagship protected.
    let mut flagship = sample_ship(&codex, 1, 1);
    flagship.ship.api_soukou[0] = 0; // no armor
    flagship.ship.api_nowhp = 1;
    let flagship_maxhp = flagship.ship.api_maxhp;
    assert!(flagship_maxhp >= 13, "DD maxhp must be >= 13 for taiha check");

    // Enemy: CVL with planes (triggers kouku) + high firepower for shelling.
    let mut enemy_cvl = sample_ship(&codex, 89, 99);
    enemy_cvl.ship.api_karyoku[0] = 200;
    enemy_cvl.ship.api_soukou[0] = 0;

    let mut rng = ProductionRng;
    let session = run_day_battle(
        store,
        &codex,
        SortieBattleInput {
            profile_id,
            deck_id: 1,
            map_id: 11,
            cell_id: 1,
            context: BattleContext::head_on(
                BattleType::Normal,
                true,
                vec![flagship],
                vec![enemy_cvl],
            ),
        },
        &mut rng,
    );

    // Flagship must survive despite taiha HP + kouku + shelling.
    let flagship_hp = session.friendly[0].hp();
    assert!(
        flagship_hp > 0,
        "flagship at taiha HP must survive kouku + shelling under sinking protection"
    );

    // Verify kouku phase ran and dealt some damage to the flagship.
    if let Some(kouku) = &session.packet.kouku {
        let kouku_flagship_damage = kouku.api_stage3.api_fdam[0];
        if kouku_flagship_damage > 0 {
            // Kouku dealt damage but flagship survived → protection must have capped it.
            assert!(
                kouku_flagship_damage < flagship_maxhp,
                "kouku api_fdam should reflect dealt (capped) damage, not raw overkill"
            );
        }
    }

    // Verify at least one shelling phase ran.
    let any_shelling = session.packet.hougeki1.is_some()
        || session.packet.hougeki2.is_some()
        || session.packet.hougeki3.is_some();
    assert!(any_shelling, "at least one shelling phase must run");

    // Final HP in packet must also show survival.
    assert!(
        session.packet.friendly_nowhps[0] > 0,
        "friendly_nowhps[0] must be > 0 after full day battle"
    );

    let _ = take_day_battle_result(store, profile_id);
    store.clear();
}

#[test]
fn start_source_cells_include_nonzero_route_cell_roots() {
    let variant = MapVariantDefinition {
        variant_key: String::new(),
        boss_cell_no: 14,
        cells: vec![
            MapCellDefinition {
                cell_no: 0,
                color_no: 0,
                event_id: 0,
                event_kind: 0,
                next_cells: vec![1, 2],
                node_label: Some("Start".to_string()),
                master_cell_id: None,
                distance: None,
            },
            MapCellDefinition {
                cell_no: 1,
                color_no: 4,
                event_id: 4,
                event_kind: 1,
                next_cells: vec![],
                node_label: Some("A".to_string()),
                master_cell_id: None,
                distance: None,
            },
            MapCellDefinition {
                cell_no: 2,
                color_no: 4,
                event_id: 4,
                event_kind: 1,
                next_cells: vec![],
                node_label: Some("B".to_string()),
                master_cell_id: None,
                distance: None,
            },
            MapCellDefinition {
                cell_no: 13,
                color_no: 4,
                event_id: 4,
                event_kind: 1,
                next_cells: vec![14],
                node_label: Some("M".to_string()),
                master_cell_id: None,
                distance: None,
            },
            MapCellDefinition {
                cell_no: 14,
                color_no: 5,
                event_id: 5,
                event_kind: 1,
                next_cells: vec![],
                node_label: Some("N".to_string()),
                master_cell_id: None,
                distance: None,
            },
            MapCellDefinition {
                cell_no: 22,
                color_no: 0,
                event_id: 0,
                event_kind: 0,
                next_cells: vec![13],
                node_label: Some("Start".to_string()),
                master_cell_id: None,
                distance: None,
            },
        ],
        routing_rules: BTreeMap::new(),
        enemy_fleets: BTreeMap::new(),
        ship_drops: BTreeMap::new(),
        required_defeat_count: None,
        clear_to_variant_key: None,
        parse_warnings: Vec::new(),
    };

    let sources =
        start_source_cells(&variant).into_iter().map(|cell| cell.cell_no).collect::<Vec<_>>();

    assert_eq!(sources, vec![0, 22]);
}

#[tokio::test]
async fn first_gauge_clear_switches_map_variant_without_finishing_map() {
    let db = new_mem_db().await.unwrap();
    let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
    let context = Ctx::new(Arc::new(db), Arc::new(codex));
    let account = context.sign_up("variant-switch", "1234567").await.unwrap();
    let profile = context.new_profile(&account.access_token.token, "variant-admin").await.unwrap();
    let session =
        context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
    let profile_id = session.profile.id;
    let now = Utc::now();
    if let Ok(record) = find_map_record_impl(context.db.as_ref(), profile_id, 73).await {
        let mut am = record.into_active_model();
        am.cleared = ActiveValue::Set(false);
        am.unlocked = ActiveValue::Set(true);
        am.last_cleared_at = ActiveValue::Set(None);
        am.last_reset_at = ActiveValue::Set(Some(now));
        am.defeat_count = ActiveValue::Set(Some(2));
        am.current_hp = ActiveValue::Set(None);
        am.gauge_index = ActiveValue::Set(1);
        assign_stage_id(&mut am, Some("pre_p_unlock".to_string()));
        am.selected_rank = ActiveValue::Set(map_record::SelectedRank::NotSet);
        am.event_state = ActiveValue::Set(None);
        am.update(context.db.as_ref()).await.unwrap();
    } else {
        map_record::ActiveModel {
            id: ActiveValue::NotSet,
            profile_id: ActiveValue::Set(profile_id),
            map_id: ActiveValue::Set(73),
            cleared: ActiveValue::Set(false),
            last_cleared_at: ActiveValue::Set(None),
            last_reset_at: ActiveValue::Set(Some(now)),
            defeat_count: ActiveValue::Set(Some(2)),
            current_hp: ActiveValue::Set(None),
            gauge_index: ActiveValue::Set(1),
            stage_id: ActiveValue::Set(Some("pre_p_unlock".to_string())),
            selected_rank: ActiveValue::Set(map_record::SelectedRank::NotSet),
            event_state: ActiveValue::Set(None),
            unlocked: ActiveValue::Set(true),
        }
        .insert(context.db.as_ref())
        .await
        .unwrap();
    }

    let definition = context.codex.maps.map_definition(73).unwrap().clone();
    assert_eq!(definition.default_variant, "pre_p_unlock");
    assert_eq!(definition.gauge_count, Some(2));
    let variant = definition.variant("pre_p_unlock").unwrap().clone();
    assert_eq!(variant.required_defeat_count, Some(3));
    assert_eq!(variant.clear_to_variant_key.as_deref(), Some("post_p_unlock"));
    let snapshot = successful_boss_snapshot();

    assert_eq!(
        apply_sortie_map_result(
            context.db.as_ref(),
            profile_id,
            &definition,
            &variant,
            true,
            &snapshot
        )
        .await
        .unwrap(),
        0
    );

    let record = map_record::Entity::find()
        .filter(map_record::Column::ProfileId.eq(profile_id))
        .filter(map_record::Column::MapId.eq(73))
        .one(context.db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert!(!record.cleared);
    assert_eq!(record.defeat_count, Some(0));
    assert_eq!(record.gauge_index, 2);
    assert_eq!(record.stage_id.as_deref(), Some("post_p_unlock"));
    assert!(record.last_cleared_at.is_none());
}

#[tokio::test]
async fn start_sortie_returns_post_p_unlock_layout_after_first_gauge_clear() {
    let db = new_mem_db().await.unwrap();
    let mut codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
    codex.maps =
        build_final_map_catalog_from_repo_assets("../../.data/temp", &codex.manifest).unwrap();
    let context = Ctx::new(Arc::new(db), Arc::new(codex));
    let account = context.sign_up("variant-layout", "1234567").await.unwrap();
    let profile =
        context.new_profile(&account.access_token.token, "variant-layout-admin").await.unwrap();
    let session =
        context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
    let profile_id = session.profile.id;
    let now = Utc::now();
    if let Ok(record) = find_map_record_impl(context.db.as_ref(), profile_id, 73).await {
        let mut am = record.into_active_model();
        am.cleared = ActiveValue::Set(false);
        am.unlocked = ActiveValue::Set(true);
        am.last_cleared_at = ActiveValue::Set(None);
        am.last_reset_at = ActiveValue::Set(Some(now));
        am.defeat_count = ActiveValue::Set(Some(2));
        am.current_hp = ActiveValue::Set(None);
        am.gauge_index = ActiveValue::Set(1);
        assign_stage_id(&mut am, Some("pre_p_unlock".to_string()));
        am.selected_rank = ActiveValue::Set(map_record::SelectedRank::NotSet);
        am.event_state = ActiveValue::Set(None);
        am.update(context.db.as_ref()).await.unwrap();
    } else {
        map_record::ActiveModel {
            id: ActiveValue::NotSet,
            profile_id: ActiveValue::Set(profile_id),
            map_id: ActiveValue::Set(73),
            cleared: ActiveValue::Set(false),
            last_cleared_at: ActiveValue::Set(None),
            last_reset_at: ActiveValue::Set(Some(now)),
            defeat_count: ActiveValue::Set(Some(2)),
            current_hp: ActiveValue::Set(None),
            gauge_index: ActiveValue::Set(1),
            stage_id: ActiveValue::Set(Some("pre_p_unlock".to_string())),
            selected_rank: ActiveValue::Set(map_record::SelectedRank::NotSet),
            event_state: ActiveValue::Set(None),
            unlocked: ActiveValue::Set(true),
        }
        .insert(context.db.as_ref())
        .await
        .unwrap();
    }

    let definition = context.codex.maps.map_definition(73).unwrap().clone();
    let variant = definition.variant("pre_p_unlock").unwrap().clone();
    let snapshot = successful_boss_snapshot();
    apply_sortie_map_result(
        context.db.as_ref(),
        profile_id,
        &definition,
        &variant,
        true,
        &snapshot,
    )
    .await
    .unwrap();

    let ship = context.add_ship(profile_id, 951).await.unwrap();
    context.update_fleet_ships(profile_id, 1, &[ship.api_id, -1, -1, -1, -1, -1]).await.unwrap();

    let response = context.start_sortie(profile_id, 1, 7, 3).await.unwrap();
    let cell_nos = response.cell_data.iter().map(|cell| cell.cell_no).collect::<Vec<_>>();

    assert!(cell_nos.iter().any(|cell_no| *cell_no > 16));
    assert!(cell_nos.contains(&25));
    assert_eq!(response.cell_data.first().map(|cell| cell.cell_no), Some(0));
    assert_eq!(response.cell_data.last().map(|cell| cell.cell_no), Some(25));
}

#[tokio::test]
async fn hp_gauge_clear_advances_to_next_gauge_before_marking_map_cleared() {
    let db = new_mem_db().await.unwrap();
    let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
    let context = Ctx::new(Arc::new(db), Arc::new(codex));
    let account = context.sign_up("hp-gauge", "1234567").await.unwrap();
    let profile = context.new_profile(&account.access_token.token, "hp-gauge-admin").await.unwrap();
    let session =
        context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
    let profile_id = session.profile.id;
    let now = Utc::now();
    let definition = MapDefinition {
        map_id: 99011,
        maparea_id: 99,
        mapinfo_no: 11,
        name: "hp gauge".to_string(),
        level: 1,
        sally_flag: vec![],
        is_event: true,
        reset_policy: Default::default(),
        airbase_count: None,
        gauge_type: Some(2),
        gauge_type_e: None,
        gauge_count: Some(2),
        required_defeat_count: None,
        max_hp: Some(1),
        default_variant: String::new(),
        rank_stage_ids: BTreeMap::new(),
        variants: BTreeMap::from([(
            String::new(),
            MapStageDefinition {
                variant_key: String::new(),
                ..Default::default()
            },
        )]),
    };
    let stage = definition.variant("").unwrap().clone();
    map_record::ActiveModel {
        id: ActiveValue::NotSet,
        profile_id: ActiveValue::Set(profile_id),
        map_id: ActiveValue::Set(definition.map_id),
        cleared: ActiveValue::Set(false),
        last_cleared_at: ActiveValue::Set(None),
        last_reset_at: ActiveValue::Set(Some(now)),
        defeat_count: ActiveValue::Set(None),
        current_hp: ActiveValue::Set(Some(1)),
        gauge_index: ActiveValue::Set(1),
        stage_id: ActiveValue::Set(None),
        selected_rank: ActiveValue::Set(map_record::SelectedRank::NotSet),
        event_state: ActiveValue::Set(Some(1)),
        unlocked: ActiveValue::Set(true),
    }
    .insert(context.db.as_ref())
    .await
    .unwrap();

    assert_eq!(
        apply_sortie_map_result(
            context.db.as_ref(),
            profile_id,
            &definition,
            &stage,
            true,
            &successful_boss_snapshot(),
        )
        .await
        .unwrap(),
        0
    );

    let record = map_record::Entity::find()
        .filter(map_record::Column::ProfileId.eq(profile_id))
        .filter(map_record::Column::MapId.eq(definition.map_id))
        .one(context.db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert!(!record.cleared);
    assert_eq!(record.current_hp, Some(1));
    assert_eq!(record.gauge_index, 2);
    assert_eq!(record.event_state, Some(1));
    assert!(record.last_cleared_at.is_none());
}

#[tokio::test]
async fn hp_gauge_clear_switches_stage_before_marking_map_cleared() {
    let db = new_mem_db().await.unwrap();
    let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
    let context = Ctx::new(Arc::new(db), Arc::new(codex));
    let account = context.sign_up("hp-stage", "1234567").await.unwrap();
    let profile = context.new_profile(&account.access_token.token, "hp-stage-admin").await.unwrap();
    let session =
        context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
    let profile_id = session.profile.id;
    let now = Utc::now();
    let definition = MapDefinition {
        map_id: 99012,
        maparea_id: 99,
        mapinfo_no: 12,
        name: "hp stage".to_string(),
        level: 1,
        sally_flag: vec![],
        is_event: true,
        reset_policy: Default::default(),
        airbase_count: None,
        gauge_type: Some(2),
        gauge_type_e: None,
        gauge_count: Some(2),
        required_defeat_count: None,
        max_hp: Some(1),
        default_variant: "pre".to_string(),
        rank_stage_ids: BTreeMap::new(),
        variants: BTreeMap::from([
            (
                "pre".to_string(),
                MapStageDefinition {
                    variant_key: "pre".to_string(),
                    clear_to_variant_key: Some("post".to_string()),
                    ..Default::default()
                },
            ),
            (
                "post".to_string(),
                MapStageDefinition {
                    variant_key: "post".to_string(),
                    ..Default::default()
                },
            ),
        ]),
    };
    let stage = definition.variant("pre").unwrap().clone();
    map_record::ActiveModel {
        id: ActiveValue::NotSet,
        profile_id: ActiveValue::Set(profile_id),
        map_id: ActiveValue::Set(definition.map_id),
        cleared: ActiveValue::Set(false),
        last_cleared_at: ActiveValue::Set(None),
        last_reset_at: ActiveValue::Set(Some(now)),
        defeat_count: ActiveValue::Set(None),
        current_hp: ActiveValue::Set(Some(1)),
        gauge_index: ActiveValue::Set(1),
        stage_id: ActiveValue::Set(Some("pre".to_string())),
        selected_rank: ActiveValue::Set(map_record::SelectedRank::NotSet),
        event_state: ActiveValue::Set(Some(1)),
        unlocked: ActiveValue::Set(true),
    }
    .insert(context.db.as_ref())
    .await
    .unwrap();

    assert_eq!(
        apply_sortie_map_result(
            context.db.as_ref(),
            profile_id,
            &definition,
            &stage,
            true,
            &successful_boss_snapshot(),
        )
        .await
        .unwrap(),
        0
    );

    let record = map_record::Entity::find()
        .filter(map_record::Column::ProfileId.eq(profile_id))
        .filter(map_record::Column::MapId.eq(definition.map_id))
        .one(context.db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert!(!record.cleared);
    assert_eq!(record.current_hp, Some(1));
    assert_eq!(record.gauge_index, 2);
    assert_eq!(record.stage_id.as_deref(), Some("post"));
    assert_eq!(record.event_state, Some(1));
    assert!(record.last_cleared_at.is_none());
}

#[tokio::test]
async fn final_hp_gauge_clear_marks_map_cleared() {
    let db = new_mem_db().await.unwrap();
    let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
    let context = Ctx::new(Arc::new(db), Arc::new(codex));
    let account = context.sign_up("hp-final", "1234567").await.unwrap();
    let profile = context.new_profile(&account.access_token.token, "hp-final-admin").await.unwrap();
    let session =
        context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
    let profile_id = session.profile.id;
    let now = Utc::now();
    let definition = MapDefinition {
        map_id: 99013,
        maparea_id: 99,
        mapinfo_no: 13,
        name: "hp final".to_string(),
        level: 1,
        sally_flag: vec![],
        is_event: true,
        reset_policy: Default::default(),
        airbase_count: None,
        gauge_type: Some(2),
        gauge_type_e: None,
        gauge_count: Some(2),
        required_defeat_count: None,
        max_hp: Some(1),
        default_variant: String::new(),
        rank_stage_ids: BTreeMap::new(),
        variants: BTreeMap::from([(
            String::new(),
            MapStageDefinition {
                variant_key: String::new(),
                ..Default::default()
            },
        )]),
    };
    let stage = definition.variant("").unwrap().clone();
    map_record::ActiveModel {
        id: ActiveValue::NotSet,
        profile_id: ActiveValue::Set(profile_id),
        map_id: ActiveValue::Set(definition.map_id),
        cleared: ActiveValue::Set(false),
        last_cleared_at: ActiveValue::Set(None),
        last_reset_at: ActiveValue::Set(Some(now)),
        defeat_count: ActiveValue::Set(None),
        current_hp: ActiveValue::Set(Some(1)),
        gauge_index: ActiveValue::Set(2),
        stage_id: ActiveValue::Set(None),
        selected_rank: ActiveValue::Set(map_record::SelectedRank::NotSet),
        event_state: ActiveValue::Set(Some(1)),
        unlocked: ActiveValue::Set(true),
    }
    .insert(context.db.as_ref())
    .await
    .unwrap();

    assert_eq!(
        apply_sortie_map_result(
            context.db.as_ref(),
            profile_id,
            &definition,
            &stage,
            true,
            &successful_boss_snapshot(),
        )
        .await
        .unwrap(),
        1
    );

    let record = map_record::Entity::find()
        .filter(map_record::Column::ProfileId.eq(profile_id))
        .filter(map_record::Column::MapId.eq(definition.map_id))
        .one(context.db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert!(record.cleared);
    assert_eq!(record.current_hp, Some(0));
    assert_eq!(record.gauge_index, 2);
    assert_eq!(record.event_state, Some(2));
    assert!(record.last_cleared_at.is_some());
}

#[tokio::test]
async fn clearing_map_1_1_unlocks_dependents_via_cascade() {
    let db = new_mem_db().await.unwrap();
    let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
    let context = Ctx::new(Arc::new(db), Arc::new(codex));
    let account = context.sign_up("cascade-test", "1234567").await.unwrap();
    let profile = context.new_profile(&account.access_token.token, "cascade-tester").await.unwrap();
    let session =
        context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
    let profile_id = session.profile.id;

    let catalog = active_map_catalog(context.codex.as_ref());
    let deps = catalog.dependents_of(11);
    assert!(!deps.is_empty(), "1-1 should have dependents");

    // Verify dependents start locked
    for &dep_id in &deps {
        let rec = find_map_record_impl(context.db.as_ref(), profile_id, dep_id).await.unwrap();
        assert!(!rec.unlocked, "dependent {dep_id} should start locked");
    }

    // Simulate Boss win on 1-1 through the actual cascade
    let definition = catalog.as_ref().map_definition(11).unwrap();
    let stage = definition.stage("").unwrap();
    let snapshot = successful_boss_snapshot();

    let first_clear = apply_sortie_map_result(
        context.db.as_ref(),
        profile_id,
        definition,
        stage,
        true, // boss cell
        &snapshot,
    )
    .await
    .unwrap();
    assert_eq!(first_clear, 1, "first clear should return 1");

    let unlocked = check_and_unlock_dependencies_impl(
        context.db.as_ref(),
        context.codex.as_ref(),
        profile_id,
        11,
    )
    .await
    .unwrap();
    assert!(!unlocked.is_empty(), "should unlock at least one map");

    // Verify dependents are now unlocked
    for &dep_id in &deps {
        let rec = find_map_record_impl(context.db.as_ref(), profile_id, dep_id).await.unwrap();
        assert!(rec.unlocked, "dependent {dep_id} should be unlocked after clearing 1-1");
    }
}

async fn guard_context() -> (Ctx, i64) {
    let db = new_mem_db().await.unwrap();
    let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
    let context = Ctx::new(Arc::new(db), Arc::new(codex));
    let account = context.sign_up("sp-guard", "1234567").await.unwrap();
    let profile = context.new_profile(&account.access_token.token, "sp-guard").await.unwrap();
    let session =
        context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
    (context, session.profile.id)
}

/// An active sortie on map 1-1 parked at the first cell of its default stage that
/// satisfies `pick`, with no battle pending.
fn active_sortie_on_cell(
    codex: &Codex,
    pick: impl Fn(&MapCellDefinition) -> bool,
) -> ActiveSortieState {
    let definition = codex.maps.map_definition(11).unwrap();
    let stage = definition.stage(&definition.default_variant).unwrap();
    let cell = stage.cells.iter().find(|cell| pick(cell)).unwrap();
    ActiveSortieState {
        deck_id: 1,
        map_id: 11,
        map_name: definition.name.clone(),
        map_level: definition.level,
        stage_id: definition.default_variant.clone(),
        current_cell_id: cell.cell_no,
        boss_cell_id: stage.boss_cell_no,
        pending_battle_cell_id: None,
        visited_cell_ids: BTreeSet::from([cell.cell_no]),
        locked_enemy_composition: None,
    }
}

#[tokio::test]
async fn sortie_sp_midnight_battle_rejects_combined_fleet_like_sortie_battle() {
    let (context, pid) = guard_context().await;
    let mut profile = find_profile(context.db.as_ref(), pid).await.unwrap().into_active_model();
    profile.combined_type = ActiveValue::Set(1);
    profile.update(context.db.as_ref()).await.unwrap();
    let store = context.sortie_store.as_ref();
    let _ = store.insert_active(pid, active_sortie_on_cell(&context.codex, |c| c.event_kind == 1));

    let day = context.sortie_battle(pid, 1).await.unwrap_err();
    let night = context.sortie_sp_midnight_battle(pid, 1).await.unwrap_err();

    assert!(
        matches!(day, GameplayError::WrongType(ref msg) if msg.contains("combined")),
        "{day:?}"
    );
    assert_eq!(night.to_string(), day.to_string());
}

#[tokio::test]
async fn sortie_sp_midnight_battle_rejects_non_battle_cell_like_sortie_battle() {
    let (context, pid) = guard_context().await;
    let store = context.sortie_store.as_ref();
    let _ = store.insert_active(pid, active_sortie_on_cell(&context.codex, |c| c.event_kind != 1));

    let day = context.sortie_battle(pid, 1).await.unwrap_err();
    let night = context.sortie_sp_midnight_battle(pid, 1).await.unwrap_err();

    assert!(
        matches!(day, GameplayError::WrongType(ref msg) if msg.contains("is not a battle cell")),
        "{day:?}"
    );
    assert_eq!(night.to_string(), day.to_string());
}
