use emukc_db::prelude::new_mem_db;
use emukc_db::sea_orm::DbConn;
use emukc_gameplay::prelude::*;
use emukc_model::codex::Codex;
use emukc_model::kc2::{
	KcApiIncentiveItem, KcApiIncentiveMode, KcApiIncentiveType, MaterialCategory,
};
use emukc_model::prelude::{Kc3rdShip, Kc3rdSlotItem};
use emukc_model::profile::kdock::ConstructionDockStatus;
use emukc_model::profile::ndock::RepairDockStatus;

async fn mock_context() -> (DbConn, Codex) {
	let db = new_mem_db().await.unwrap();
	let codex = Codex::load("../../.data/codex").unwrap();
	(db, codex)
}

async fn new_game_session() -> ((DbConn, Codex), AccountInfo, StartGameInfo) {
	let context = mock_context().await;

	let account = context.sign_up("test", "1234567").await.unwrap();
	let profile = context.new_profile(&account.access_token.token, "admin").await.unwrap();
	let session =
		context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();

	(context, account, session)
}

#[tokio::test]
async fn foo() {
	let (context, _, _) = new_game_session().await;

	let codex = context.codex();
	let basic = codex.find::<Kc3rdShip>(&966).unwrap();
	println!("{:?}", basic);
}

#[tokio::test]
async fn nickname() {
	let (context, _, session) = new_game_session().await;
	let pid = session.profile.id;

	context.update_user_nickname(pid, "ararakikun").await.unwrap();

	let (_, basic) = context.get_user_basic(pid).await.unwrap();

	assert_eq!(basic.api_nickname, "ararakikun");
}

#[tokio::test]
async fn fleet() {
	let (context, _, session) = new_game_session().await;
	let pid = session.profile.id;

	let fleets = context.get_fleets(pid).await.unwrap();
	println!("{:?}", fleets);
	assert_eq!(fleets.len(), 1);

	let fleet = context.unlock_fleet(pid, 2).await.unwrap();
	println!("{:?}", fleet);
	assert_eq!(fleet.index, 2);
}

#[tokio::test]
async fn incentive() {
	let (context, _, session) = new_game_session().await;

	let pid = session.profile.id;

	let codex = &context.1;
	let ship_mst = codex.manifest.find_ship(181).unwrap();

	context
		.add_incentive(
			pid,
			&[KcApiIncentiveItem {
				api_mode: KcApiIncentiveMode::PreRegister as i64,
				api_type: KcApiIncentiveType::Ship as i64,
				api_mst_id: ship_mst.api_id,
				api_getmes: ship_mst.api_getmes.clone(),
				api_slotitem_level: None,
				amount: 1,
				alv: 0,
			}],
		)
		.await
		.unwrap();

	let incentive = context.confirm_incentives(session.profile.id).await.unwrap();

	println!("{:?}", incentive);

	assert!(!incentive.is_empty());

	let incentive = context.confirm_incentives(session.profile.id).await.unwrap();
	assert!(incentive.is_empty());
}

#[tokio::test]
async fn basics() {
	let (context, _, session) = new_game_session().await;
	let pid = session.profile.id;

	let basic = context.get_user_basic(pid).await.unwrap();
	println!("{:?}", basic);
}

#[tokio::test]
async fn add_ship() {
	let (context, _, session) = new_game_session().await;
	let pid = session.profile.id;
	let ship = context.add_ship(pid, 951).await.unwrap();

	assert_eq!(ship.api_id, 1);

	let slot_items = context.get_slot_items(pid).await.unwrap();
	assert_eq!(slot_items.len(), 3);

	let unset_slots = context.get_unuse_slot_items(pid).await.unwrap();
	assert!(unset_slots.is_empty());

	let ships = context.get_ships(pid).await.unwrap();
	assert_eq!(ships.len(), 1);
}

#[tokio::test]
async fn create_slotitems() {
	let (context, _, session) = new_game_session().await;
	let pid = session.profile.id;

	let slot_items = context
		.1
		.slotitem_extra_info
		.iter()
		.filter_map(|(_, v)| {
			if v.craftable {
				Some(v.clone())
			} else {
				None
			}
		})
		.collect::<Vec<Kc3rdSlotItem>>();
	let chunk = slot_items.chunks(3).next().unwrap();

	let costs = vec![
		(MaterialCategory::Fuel, 100),
		(MaterialCategory::Ammo, 100),
		(MaterialCategory::Steel, 100),
		(MaterialCategory::Bauxite, 100),
		(MaterialCategory::DevMat, 1),
	];

	let ids = chunk.iter().map(|v| v.api_id).collect::<Vec<i64>>();

	let (ids, material) = context.create_slotitem(pid, ids, costs).await.unwrap();

	println!("{:?}", ids);
	println!("{:?}", material);
}

#[tokio::test]
async fn material() {
	let (context, _, session) = new_game_session().await;

	let old = context.get_materials(session.profile.id).await.unwrap();

	context.add_material(session.profile.id, MaterialCategory::Fuel, 100).await.unwrap();

	let new = context.get_materials(session.profile.id).await.unwrap();

	assert_eq!(new.fuel, old.fuel + 100);
}

#[tokio::test]
async fn use_item() {
	let (context, _, session) = new_game_session().await;
	let pid = session.profile.id;
	let m = context.add_use_item(pid, 5, 99).await.unwrap();
	assert_eq!(m.api_count, 99);

	let m = context.add_use_item(pid, 5, 1).await.unwrap();
	assert_eq!(m.api_count, 100);
}

#[tokio::test]
async fn kdock() {
	let (context, _, session) = new_game_session().await;
	let pid = session.profile.id;

	let docks = context.get_kdocks(pid).await.unwrap();
	assert_eq!(docks.len(), 4);

	let dock = context.get_kdock(pid, 1).await.unwrap();
	assert_eq!(dock.index, 1);
	assert_eq!(dock.status, ConstructionDockStatus::Idle);

	let dock = context.get_kdock(pid, 2).await.unwrap();
	assert_eq!(dock.index, 2);
	assert_eq!(dock.status, ConstructionDockStatus::Locked);

	let dock = context.unlock_kdock(pid, 2).await.unwrap();
	assert_eq!(dock.status, ConstructionDockStatus::Idle);
}

#[tokio::test]
async fn ndock() {
	let (context, _, session) = new_game_session().await;
	let pid = session.profile.id;

	let docks = context.get_ndocks(pid).await.unwrap();
	assert_eq!(docks.len(), 4);

	let dock = context.get_ndock(pid, 1).await.unwrap();
	assert_eq!(dock.index, 1);
	assert_eq!(dock.status, RepairDockStatus::Idle);

	let dock = context.get_ndock(pid, 2).await.unwrap();
	assert_eq!(dock.index, 2);
	assert_eq!(dock.status, RepairDockStatus::Locked);

	let dock = context.unlock_ndock(pid, 2).await.unwrap();
	assert_eq!(dock.status, RepairDockStatus::Idle);
}
