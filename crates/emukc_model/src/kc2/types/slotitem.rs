use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, enumn::N)]
pub enum KcSlotItemType3 {
	/// 小口径主砲
	SmallCaliberMainGun = 1,
	/// 中口径主砲
	MediumCaliberMainGun = 2,
	/// 大口径主砲
	LargeCaliberMainGun = 3,
	/// 副砲
	SecondaryGun = 4,
	/// 魚雷
	Torpedo = 5,
	/// 艦上戦闘機
	CarrierBasedFighter = 6,
	/// 艦上爆撃機
	CarrierBasedDiveBomber = 7,
	/// 艦上攻撃機
	CarrierBasedTorpedoBomber = 8,
	/// 艦上偵察機
	CarrierBasedRecon = 9,
	/// 水上偵察機
	SeaBasedRecon = 10,
	/// 水上爆撃機
	SeaBasedBomber = 11,
	/// 小型電探
	SmallRadar = 12,
	/// 大型電探
	LargeRadar = 13,
	/// ソナー
	Sonar = 14,
	/// 爆雷
	DepthCharge = 15,
	/// 追加装甲
	ExtraArmor = 16,
	/// 機関部強化
	EngineBoost = 17,
	/// 対空強化弾
	AntiAircraftShell = 18,
	/// 対艦強化弾
	ArmorPiercingShell = 19,
	/// VT信管
	VTFuse = 20,
	/// 対空機銃
	AntiAircraftGun = 21,
	/// 特殊潜航艇
	SpecialSubmarineVessel = 22,
	/// 応急修理要員
	DamageControl = 23,
	/// 上陸用舟艇
	LandingCraft = 24,
	/// オートジャイロ
	AutoGyro = 25,
	/// 対潜哨戒機
	AntiSubmarinePatrol = 26,
	/// 追加装甲(中型)
	ExtraArmorMedium = 27,
	/// 追加装甲(大型)
	ExtraArmorLarge = 28,
	/// 探照灯
	Searchlight = 29,
	/// 簡易輸送部材
	TransportContainer = 30,
	/// 艦艇修理施設
	ShipRepairFacility = 31,
	/// 潜水艦魚雷
	SubmarineTorpedo = 32,
	/// 照明弾
	Flare = 33,
	/// 司令部施設
	CommandFacility = 34,
	/// 航空要員
	AviationPersonnel = 35,
	/// 高射装置
	AntiAircraftGunMount = 36,
	/// 対地装備
	AntiGroundEquipment = 37,
	/// 大口径主砲(II)
	LargeCaliberMainGun2 = 38,
	/// 水上艦要員
	SeaplanePersonnel = 39,
	/// 大型ソナー
	LargeSonar = 40,
	/// 大型飛行艇
	LargeFlyingBoat = 41,
	/// 大型探照灯
	LargeSearchlight = 42,
	/// 戦闘糧食
	BattleRation = 43,
	/// 補給物資
	SupplyMaterial = 44,
	/// 水上戦闘機
	SeaplaneFighter = 45,
	/// 特型内火艇
	SpecialTypeAmphibiousTank = 46,
	/// 陸上攻撃機
	LandBasedAttacker = 47,
	/// 局地戦闘機
	LocalFighter = 48,
	/// 陸上偵察機
	LandBasedRecon = 49,
	/// 輸送機材
	TransportEquipment = 50,
	/// 潜水艦装備
	SubmarineEquipment = 51,
	/// 陸戦部隊
	LandBattleUnit = 52,
	/// 大型陸上機
	LargeLandBasedAircraft = 53,
	/// 水上艦装備
	SeaplaneEquipment = 54,
	/// 噴式戦闘機
	JetFighter = 56,
	/// 噴式戦闘爆撃機
	JetFighterBomber = 57,
	/// 噴式攻撃機
	JetAttacker = 58,
	/// 噴式偵察機
	JetRecon = 59,
	/// 大型電探（II）
	LargeRadar2 = 93,
	/// 艦上偵察機（II）
	CarrierBasedRecon2 = 94,
	/// 副砲（II）
	SecondaryGun2 = 95,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, enumn::N)]
pub enum KcSlotItemCommonId {
	/// 応急修理要員
	RepairTeam = 42,
	/// 応急修理女神
	RepairGoddess = 43,
	/// 戦闘糧食
	BattleRation = 145,
	/// 洋上補給
	OffShoreResupply = 146,
}
