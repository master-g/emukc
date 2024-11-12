use emukc_db::prelude::new_mem_db;
use emukc_db::sea_orm::DbConn;
use emukc_gameplay::prelude::*;
use emukc_model::codex::group::DeGroupParam;
use emukc_model::codex::Codex;
use emukc_model::kc2::{
	KcApiIncentiveItem, KcApiIncentiveMode, KcApiIncentiveType, MaterialCategory,
};
use emukc_model::prelude::{ApiMstShip, Kc3rdQuest, Kc3rdShip, Kc3rdSlotItem};
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

	println!("{:?}", ship);

	let slot_items = context.get_slot_items(pid).await.unwrap();
	assert_eq!(slot_items.len(), 3);

	let unset_slots = context.get_unset_slot_items(pid).await.unwrap();
	assert!(unset_slots.is_empty());

	let ships = context.get_ships(pid).await.unwrap();
	assert_eq!(ships.len(), 1);
}

#[tokio::test]
async fn ship_incentive() {
	let (context, _, _) = new_game_session().await;

	let incentive = context.1.new_incentive_with_ship(951).unwrap();
	println!("{:?}", incentive);
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

	let (ids, material) = context.create_slotitem(pid, &ids, &costs).await.unwrap();

	println!("{:?}", ids);
	println!("{:?}", material);
}

#[tokio::test]
async fn material() {
	let (context, _, session) = new_game_session().await;

	let old = context.get_materials(session.profile.id).await.unwrap();

	context.add_material(session.profile.id, &[(MaterialCategory::Fuel, 100)]).await.unwrap();

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

#[tokio::test]
async fn practice() {
	let (context, _, session) = new_game_session().await;
	let pid = session.profile.id;

	let rivals = context.get_practice_rivals(pid).await.unwrap();
	println!("{:?}", rivals);
}

#[tokio::test]
async fn quests() {
	let (context, _, session) = new_game_session().await;
	let pid = session.profile.id;

	let quests = context.get_quest_records(pid).await.unwrap();
	let quest_ids: Vec<i64> = quests.iter().map(|m| m.quest_id).collect();
	println!("{:?}", quest_ids);
}

#[tokio::test]
async fn c_list_quests() {
	let ((_, codex), _, _) = new_game_session().await;
	[
		506, 641, 642, 643, 644, 645, 646, 647, 648, 649, 650, 651, 652, 653, 658, 666, 668, 966,
		1101, 1102, 1103, 1104, 1105, 1110, 1111, 1112, 1113, 1114, 1115, 1116, 1117, 1119, 1120,
		1121, 1122, 1126, 1127, 1128, 1129, 1130, 1131, 1132, 1133, 1134, 1135,
	]
	.iter()
	.for_each(|&id| {
		if let Some(mst) = codex.find::<Kc3rdQuest>(&id).ok() {
			println!("--- {} | {} ---", id, mst.name);
			println!("category: {:?}", mst.category);
			println!("requirements: {:?}", mst.requirements);
		} else {
			println!("--- {} ---", id);
			println!("NOT FOUND");
		}
	});
}

#[tokio::test]
async fn ship_before_and_after() {
	let ((_, codex), _, _) = new_game_session().await;
	let list = codex.ships_before_and_after(518).unwrap();
	list.iter().filter_map(|id| codex.find::<ApiMstShip>(id).ok()).for_each(|mst| {
		println!("--- {} | {} ---", mst.api_id, mst.api_name);
	});
}

#[tokio::test]
async fn de_group() {
	let ((_, codex), _, _) = new_game_session().await;

	{
		let grouped = codex.group_de_ships(&[]);
		assert!(grouped.hp_pairs.is_empty());
		assert!(grouped.other_pairs.is_empty());
		assert!(grouped.rest.is_empty());
	}
	{
		let grouped = codex.group_de_ships(&[DeGroupParam {
			id: 1,
			mst_id: 518,
			ctype: 74,
		}]);
		assert!(grouped.hp_pairs.is_empty());
		assert!(grouped.other_pairs.is_empty());
		assert_eq!(grouped.rest.len(), 1);
	}
	{
		let grouped = codex.group_de_ships(&[
			DeGroupParam {
				id: 1,
				mst_id: 518,
				ctype: 74,
			},
			DeGroupParam {
				id: 2,
				mst_id: 377,
				ctype: 74,
			},
		]);
		assert!(grouped.hp_pairs.is_empty());
		assert_eq!(grouped.other_pairs.len(), 1);
		assert!(grouped.rest.is_empty())
	}
	{
		let grouped = codex.group_de_ships(&[
			DeGroupParam {
				id: 1,
				mst_id: 518,
				ctype: 74,
			},
			DeGroupParam {
				id: 2,
				mst_id: 517,
				ctype: 74,
			},
		]);
		assert_eq!(grouped.hp_pairs.len(), 1);
		assert!(grouped.other_pairs.is_empty());
		assert!(grouped.rest.is_empty());
	}
	{
		let grouped = codex.group_de_ships(&[
			DeGroupParam {
				id: 1,
				mst_id: 518,
				ctype: 74,
			},
			DeGroupParam {
				id: 2,
				mst_id: 517,
				ctype: 74,
			},
			DeGroupParam {
				id: 3,
				mst_id: 524,
				ctype: 77,
			},
			DeGroupParam {
				id: 4,
				mst_id: 525,
				ctype: 77,
			},
		]);
		assert_eq!(grouped.hp_pairs.len(), 2);
		assert!(grouped.other_pairs.is_empty());
		assert!(grouped.rest.is_empty());
	}
}
