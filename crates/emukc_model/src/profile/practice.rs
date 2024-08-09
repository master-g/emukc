use serde::{Deserialize, Serialize};

use crate::kc2::{
	KcApiPracticeEnemyDeck, KcApiPracticeEnemyInfo, KcApiPracticeEnemyShip, KcApiPracticeRival,
	UserHQRank,
};

/// Rival ship info
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct RivalShip {
	/// profile id
	pub id: i64,

	/// ship instance id
	pub instance_id: i64,

	/// ship mst id
	pub mst_id: i64,

	/// ship level
	pub level: i64,

	/// ship star, indicates the modernization level
	pub star: i64,
}

/// Rival detail
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct RivalDetail {
	/// profile id
	pub id: i64,

	/// experience now
	pub exp_now: i64,

	/// experience next
	pub exp_next: i64,

	/// friend
	pub friend: i64,

	/// current ship count
	pub current_ship_count: i64,

	/// ship capacity
	pub ship_capacity: i64,

	/// current slot item count
	pub current_slot_item_count: i64,

	/// slot item capacity
	pub slot_item_capacity: i64,

	/// furniture
	pub furniture: i64,

	/// deck name
	pub deck_name: String,

	/// ships
	pub ships: Vec<RivalShip>,
}

/// Rival flag
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug, Default)]
pub enum RivalFlag {
	#[default]
	Bronze = 1,
	Silver = 2,
	Gold = 3,
}

#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug, Default)]
pub enum RivalStatus {
	#[default]
	Untouched = 0,
	LostRankE = 1,
	LostRankD = 2,
	LostRankC = 3,
	VictoryRankB = 4,
	VictoryRankA = 5,
	VictoryRankS = 6,
}

/// Rival info
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Rival {
	/// profile id this rival belongs to
	pub id: i64,

	/// rival profile id
	pub rival_profile_id: i64,

	/// rival index
	pub index: i64,

	/// name
	pub name: String,

	/// comment
	pub comment: String,

	/// level
	pub level: i64,

	/// rank
	pub rank: UserHQRank,

	/// flag
	pub flag: RivalFlag,

	/// status
	pub status: RivalStatus,

	/// medals
	pub medals: i64,

	/// details
	pub details: RivalDetail,
}

/// Rival type
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug, Default)]
pub enum RivalType {
	/// First group, whales and chads
	#[default]
	FirstGroup = 0,
	/// Secondary group, casuals and f2p
	SecondaryGroup = 1,
	/// All
	All = 2,
}

/// Practice config
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct PracticeConfig {
	/// profile id
	pub id: i64,

	/// selected type
	pub selected_type: RivalType,

	/// rival info generated with
	pub generated_type: RivalType,
}

impl From<Rival> for KcApiPracticeRival {
	fn from(value: Rival) -> Self {
		Self {
			api_enemy_id: value.id,
			api_enemy_name: value.name,
			api_enemy_name_id: "".to_string(),
			api_enemy_level: value.level,
			api_enemy_rank: value.rank.get_name().to_owned(),
			api_enemy_flag: value.flag as i64,
			api_enemy_flag_ship: value.details.ships.first().map(|ship| ship.mst_id).unwrap_or(0),
			api_enemy_comment: value.comment,
			api_enemy_comment_id: "".to_string(),
			api_state: value.status as i64,
			api_medals: value.medals,
		}
	}
}

impl From<Rival> for KcApiPracticeEnemyInfo {
	fn from(value: Rival) -> Self {
		Self {
			api_member_id: value.id,
			api_nickname: value.name,
			api_nickname_id: "".to_string(),
			api_cmt: value.comment,
			api_cmt_id: "".to_string(),
			api_level: value.level,
			api_rank: value.rank as i64,
			api_experience: vec![value.details.exp_now, value.details.exp_next],
			api_friend: value.details.friend,
			api_ship: vec![value.details.current_ship_count, value.details.ship_capacity],
			api_slotitem: vec![
				value.details.current_slot_item_count,
				value.details.slot_item_capacity,
			],
			api_furniture: value.details.furniture,
			api_deckname: value.details.deck_name,
			api_deckname_id: "".to_string(),
			api_deck: KcApiPracticeEnemyDeck {
				api_ships: (0..6)
					.map(|i| {
						if let Some(ship) = value.details.ships.get(i) {
							KcApiPracticeEnemyShip {
								api_id: ship.instance_id,
								api_ship_id: Some(ship.mst_id),
								api_level: Some(ship.level),
								api_star: Some(ship.star),
							}
						} else {
							KcApiPracticeEnemyShip {
								api_id: -1,
								api_ship_id: None,
								api_level: None,
								api_star: None,
							}
						}
					})
					.collect(),
			},
		}
	}
}
