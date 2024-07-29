use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KcShipRemodelRequirement {
	pub id_from: i64,   // 改装前艦船ID
	pub id_to: i64,     // 改装後艦船ID
	pub ammo: i64,      // ammo consumed, `api_afterbull`
	pub steel: i64,     // steel consumed, `api_afterfuel`
	pub drawing: i64,   // 改装設計図
	pub catapult: i64,  // 試製甲板カタパルト
	pub report: i64,    // 戦闘詳報
	pub devmat: i64,    // 開発資材
	pub torch: i64,     // 高速建造材
	pub aviation: i64,  // 新型航空兵装資材
	pub artillery: i64, // 新型砲熕兵装資材
	pub arms: i64,      // 新型兵装資材
	pub boiler: i64,    // 新型高温高圧缶
}
