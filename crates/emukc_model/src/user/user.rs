use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
	kc2::{
		KcApiDeckPort, KcApiKDock, KcApiMapRecord, KcApiMaterialElement, KcApiNDock, KcApiUserItem,
		MaterialCategory,
	},
	KcApiAirBase, KcApiAirBaseExpandedInfo,
};

use super::{Kc3rdQuestPeriod, Kc3rdQuestRequirement};

pub const PRIMARY_RESOURCE_HARD_CAP: i64 = 35000;
pub const SPECIAL_RESOURCE_CAP: i64 = 3000;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Material {
	pub fuel: i64,
	pub ammo: i64,
	pub steel: i64,
	pub bauxite: i64,
	pub torch: i64,
	pub bucket: i64,
	pub devmat: i64,
	pub screw: i64,
}

impl Default for Material {
	fn default() -> Self {
		Self {
			fuel: 1000,
			ammo: 1000,
			steel: 1000,
			bauxite: 1000,
			torch: 3,
			bucket: 3,
			devmat: 5,
			screw: 0,
		}
	}
}

impl Material {
	pub fn into_api(self, uid: i64) -> Vec<KcApiMaterialElement> {
		vec![
			KcApiMaterialElement {
				api_member_id: uid,
				api_id: MaterialCategory::Fuel as i64,
				api_value: self.fuel,
			},
			KcApiMaterialElement {
				api_member_id: uid,
				api_id: MaterialCategory::Ammo as i64,
				api_value: self.ammo,
			},
			KcApiMaterialElement {
				api_member_id: uid,
				api_id: MaterialCategory::Steel as i64,
				api_value: self.steel,
			},
			KcApiMaterialElement {
				api_member_id: uid,
				api_id: MaterialCategory::Bauxite as i64,
				api_value: self.bauxite,
			},
			KcApiMaterialElement {
				api_member_id: uid,
				api_id: MaterialCategory::Torch as i64,
				api_value: self.torch,
			},
			KcApiMaterialElement {
				api_member_id: uid,
				api_id: MaterialCategory::Bucket as i64,
				api_value: self.bucket,
			},
			KcApiMaterialElement {
				api_member_id: uid,
				api_id: MaterialCategory::DevMat as i64,
				api_value: self.devmat,
			},
			KcApiMaterialElement {
				api_member_id: uid,
				api_id: MaterialCategory::Screw as i64,
				api_value: self.screw,
			},
		]
	}

	pub fn apply_hard_cap(&mut self) {
		self.fuel = self.fuel.min(PRIMARY_RESOURCE_HARD_CAP);
		self.ammo = self.ammo.min(PRIMARY_RESOURCE_HARD_CAP);
		self.steel = self.steel.min(PRIMARY_RESOURCE_HARD_CAP);
		self.bauxite = self.bauxite.min(PRIMARY_RESOURCE_HARD_CAP);
		self.torch = self.torch.min(SPECIAL_RESOURCE_CAP);
		self.bucket = self.bucket.min(SPECIAL_RESOURCE_CAP);
		self.devmat = self.devmat.min(SPECIAL_RESOURCE_CAP);
		self.screw = self.screw.min(SPECIAL_RESOURCE_CAP);
	}
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UserOwnedFurniture {
	pub records: Vec<i64>,
}

impl Default for UserOwnedFurniture {
	fn default() -> Self {
		Self {
			records: vec![1, 38, 72, 102, 133, 164],
		}
	}
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UserDeckPort {
	pub records: Vec<KcApiDeckPort>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UserConstructionDock {
	pub records: Vec<KcApiKDock>,
}

impl Default for UserConstructionDock {
	fn default() -> Self {
		let records = (1..=4)
			.map(|id| KcApiKDock {
				api_id: id,
				api_state: if id < 3 {
					0
				} else {
					-1
				},
				api_created_ship_id: 0,
				api_complete_time: 0,
				api_complete_time_str: "0".to_string(),
				api_item1: 0,
				api_item2: 0,
				api_item3: 0,
				api_item4: 0,
				api_item5: 0,
			})
			.collect();

		Self {
			records,
		}
	}
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UserRepairationDock {
	pub records: Vec<KcApiNDock>,
}

impl UserRepairationDock {
	pub fn new(uid: i64) -> Self {
		let records = (1..=4)
			.map(|id| KcApiNDock {
				api_member_id: uid,
				api_id: id,
				api_state: if id < 3 {
					0
				} else {
					-1
				},
				api_ship_id: 0,
				api_complete_time: 0,
				api_complete_time_str: "0".to_string(),
				api_item1: 0,
				api_item2: 0,
				api_item3: 0,
				api_item4: 0,
			})
			.collect();

		Self {
			records,
		}
	}
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct UserUseItems {
	pub items: Vec<KcApiUserItem>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct UserPayItems {
	pub items: Vec<KcApiUserItem>,
}

#[derive(Error, Debug)]
pub enum UserItemError {
	/// Insufficient item
	#[error("Insufficient item: {id} (current: {current}, required: {required})")]
	InsufficientItem {
		/// Item ID
		id: i64,
		/// Current amount
		current: i64,
		/// Required amount
		required: i64,
	},

	/// Item not found
	#[error("Item not found: {0}")]
	ItemNotFound(i64),
}

pub trait UserItemsTrait {
	fn increase(&mut self, id: i64, amount: i64);
	#[allow(dead_code)]
	fn add(&mut self, items: Vec<KcApiUserItem>);
	#[allow(dead_code)]
	fn decrease(&mut self, id: i64, amount: i64) -> Result<(), UserItemError>;
}

impl UserItemsTrait for Vec<KcApiUserItem> {
	fn increase(&mut self, id: i64, amount: i64) {
		if let Some(item) = self.iter_mut().find(|item| item.api_id == id) {
			item.api_count += amount;
		} else {
			self.push(KcApiUserItem {
				api_id: id,
				api_count: amount,
			});
		}
	}

	fn add(&mut self, items: Vec<KcApiUserItem>) {
		for item in items {
			self.increase(item.api_id, item.api_count);
		}
	}

	fn decrease(&mut self, id: i64, amount: i64) -> Result<(), UserItemError> {
		if let Some(item) = self.iter_mut().find(|item| item.api_id == id) {
			if item.api_count < amount {
				return Err(UserItemError::InsufficientItem {
					id,
					current: item.api_count,
					required: amount,
				});
			}
			item.api_count -= amount;
			if item.api_count == 0 {
				self.retain(|item| item.api_id != id);
			}
		} else {
			return Err(UserItemError::ItemNotFound(id));
		}

		Ok(())
	}
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UserUpdateTimer {
	pub first_3_materials: DateTime<Utc>,
	pub bauxite: DateTime<Utc>,
	pub repair_dock: DateTime<Utc>,
}

impl Default for UserUpdateTimer {
	fn default() -> Self {
		Self {
			first_3_materials: Utc::now(),
			bauxite: Utc::now(),
			repair_dock: Utc::now(),
		}
	}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserMapRecord {
	pub records: Vec<KcApiMapRecord>,
}

impl Default for UserMapRecord {
	fn default() -> Self {
		let map_id_list: Vec<i64> = vec![
			11, 12, 13, 14, 15, // map 1
			21, 22, 23, 24, 25, // map 2
			31, 32, 33, 34, 35, // map 3
			41, 42, 43, 44, 45, // map 4
			51, 52, 53, 54, 55, // map 5
			61, 62, 63, 64, 65, // map 6
			71, 72, 73, // map 7
		];
		let records = map_id_list
			.iter()
			.map(|id| KcApiMapRecord {
				api_id: *id,
				api_cleared: 0,
				api_defeat_count: None,
				api_now_maphp: None,
			})
			.collect();

		Self {
			records,
		}
	}
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct UserAirBase {
	pub records: Vec<KcApiAirBase>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct UserAirBaseExtended {
	pub records: Vec<KcApiAirBaseExpandedInfo>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct UserMissionInfo {
	pub api_id: i64,
	pub state: i64,
	pub since: DateTime<Utc>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct UserMissionRecord {
	pub records: Vec<UserMissionInfo>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct UserSlotItem {
	pub records: Vec<i64>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct UserShip {
	pub records: Vec<i64>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PictureBookShipInfo {
	pub sortno: i64,
	pub damaged: bool,
	pub married: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct PictureBookShip {
	pub records: Vec<PictureBookShipInfo>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct PictureBookSlotItem {
	pub records: Vec<i64>,
}

// practice

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct UserPracticeRivalDetails {
	pub experience: i64,
	pub friend: i64,        // 0: default
	pub ship: Vec<i64>,     // [0]: current value, [1]: capacity
	pub slotitem: Vec<i64>, // [0]: current value, [1]: capacity
	pub furniture: i64,
	pub deckname: String,
	pub deck: UserPracticeRivalDeck,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct UserPracticeRivalDeck {
	pub ships: Vec<UserPracticeRivalShip>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct UserPracticeRivalShip {
	pub api_id: i64, // uuid
	pub ship_id: Option<i64>,
	pub level: Option<i64>,
	pub star: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct UserPracticeRival {
	pub api_id: i64,
	pub name: String,
	pub comment: String,
	pub level: i64,
	pub rank: i64,
	pub flag: i64,  // 1=銅, 2=銀, 3=金
	pub state: i64, // 0=未挑戦, 1=E敗北?, 2=D敗北?, 3=C敗北, 4=B勝利, 5=A勝利, 6=S勝利
	pub medals: i64,
	pub details: UserPracticeRivalDetails,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct UserRivals {
	pub selected_type: i64,
	pub created_type: i64,
	pub created_time: DateTime<Utc>,
	pub records: Vec<UserPracticeRival>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct UserQuests {
	pub records: Vec<UserQuestRecord>,
	pub completed: Vec<i64>,
	pub periodic: Vec<UserPeriodicQuestRecord>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct UserPeriodicQuestRecord {
	pub period: Kc3rdQuestPeriod,
	pub last_updated: DateTime<Utc>,
	pub completed: Vec<i64>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct UserQuestRecord {
	pub api_id: i64,
	pub period: Kc3rdQuestPeriod,
	pub activated: bool,
	pub state: i64,    // 1: not started, 2: in progress, 3: completed
	pub progress: i64, // 0=空白(達成含む), 1=50%以上達成, 2=80%以上達成
	pub requirements: Kc3rdQuestRequirement,
}
