use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, enumn::N)]
pub enum KcPayItemType {
	/// 燃料パック250
	FuelPack = 1,
	/// 弾薬パック250
	AmmoPack = 2,
	/// 鋼材パック200
	SteelPack = 3,
	/// ボーキサイトパック150
	BauxitePack = 4,
	/// 開発資材パック7
	DevMaterialPack = 5,
	/// 高速建造材パック6
	TorchPack = 6,
	/// 高速修復材パック6
	BucketPack = 7,
	/// お買得！工廠セット
	FactorySet = 8,
	/// お買得！出撃セット
	SortieSet = 9,
	/// ドック増設セット
	DockExpansionSet = 10,
	/// 応急修理要員
	RepairTeam = 11,
	/// ダメコン特盛セット
	RepairSpecialSet = 12,
	/// 八八資源セット
	EightyEightResourceSet = 13,
	/// 応急修理女神
	RepairGoddess = 14,
	/// 特注家具職人
	FurnitureCraftman = 15,
	/// 母港拡張
	PortExpansion = 16,
	/// タンカー徴用
	TankerRequisition = 17,
	/// 給糧艦「間宮」
	Mamiya = 18,
	/// アルミ大量産
	AluminumMassProduction = 19,
	/// 書類一式＆指輪
	Ring = 20,
	/// 艦娘へのクッキー
	Cookie = 21,
	/// 改修資材パック10
	Screw10 = 22,
	/// 給糧艦「伊良湖」5
	Irako5 = 23,
	/// 戦闘糧食
	BattleRation = 24,
	/// 洋上補給
	Resupplier = 25,
	/// 補強増設
	ReinforceExpansion = 26,
	/// 設営隊
	ConstCorps = 27,
	/// 「緊急修理資材」セット
	EmergencyRepairMaterialSet = 28,
	/// 潜水艦補給物資パック
	SubmarineSupplyMaterialPack = 29,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, enumn::N)]
pub enum KcUseItemType {
	/// 高速修復材
	Bucket = 1,
	/// 高速建造材
	Torch = 2,
	/// 開発資材
	DevMaterial = 3,
	/// 改修資材
	Screw = 4,
	/// 家具箱（小）
	FCoinBox200 = 10,
	/// 家具箱（中）
	FCoinBox400 = 11,
	/// 家具箱（大）
	FCoinBox700 = 12,
	/// 燃料
	Fuel = 31,
	/// 弾薬
	Ammo = 32,
	/// 鋼材
	Steel = 33,
	/// ボーキサイト
	Bauxite = 34,
	/// 家具コイン
	FCoin = 44,
	/// ドック開放キー
	DockKey = 49,
	/// 特注家具職人
	FurnitureCraftman = 52,
	/// 母港拡張
	PortExpansion = 53,
	/// 給糧艦「間宮」
	Mamiya = 54,
	/// 書類一式＆指輪
	Ring = 55,
	/// 艦娘からのチョコ
	Chocolate = 56,
	/// 勲章
	Medal = 57,
	/// 改装設計図
	Blueprint = 58,
	/// 給糧艦「伊良湖」
	Irako = 59,
	/// プレゼント箱
	Presents = 60,
	/// 甲種勲章
	FirstClassMedal = 61,
	/// 菱餅
	Hishimochi = 62,
	/// 司令部要員
	HQPersonnel = 63,
	/// 補強増設
	ReinforceExpansion = 64,
	/// 試製甲板カタパルト
	ProtoCatapult = 65,
	/// 戦闘糧食, slotitem
	Ration = 66,
	/// 洋上補給, slotitem
	Resuppiler = 67,
	/// 秋刀魚
	Meckerel = 68,
	/// 秋刀魚の缶詰, slotitem
	MeckerelCan = 69,
	/// 熟練搭乗員
	SkilledCrew = 70,
	/// ネ式エンジン
	NEngine = 71,
	/// お飾り材料
	DecoMaterial = 72,
	/// 設営隊
	ConstCorps = 73,
	/// 新型航空機設計図
	NewAricraftBlueprint = 74,
	/// 新型砲熕兵装資材
	NewArtilleryMaterial = 75,
	/// 戦闘糧食(特別なおにぎり), slotitem
	RationSpecial = 76,
	/// 新型航空兵装資材
	NewAviationMaterial = 77,
	/// 戦闘詳報
	ActionReport = 78,
	/// 海峡章
	StraitMedal = 79,
	/// Xmas Select Gift Box
	XMasGiftBox = 80,
	/// 捷号章
	ShogoMedalHard = 81,
	/// 捷号章
	ShogoMedalNormal = 82,
	/// 捷号章
	ShogoMedalEasy = 83,
	/// 捷号章
	ShogoMedalCasual = 84,
	/// お米
	Rice = 85,
	/// 梅干
	Umeboshi = 86,
	/// 海苔
	Nori = 87,
	/// お茶
	Tea = 88,
	/// 鳳翔さんの夕食券
	DinnerTicket = 89,
	/// 節分の豆
	SetsubunBeans = 90,
	/// 緊急修理資材
	EmergencyRepair = 91,
	/// 新型噴進装備開発資材
	NewRocketDevMaterial = 92,
	/// 鰯
	Sardine = 93,
	/// 新型兵装資材
	NewArmamentMaterial = 94,
	/// 潜水艦補給物資
	SubmarineSupplyMaterial = 95,
	/// 南瓜
	Pumpkin = 96,
	/// てるてる坊主
	TeruteruBouzu = 97,
	/// 海色リボン
	BlueRibbon = 98,
	/// 白たすき
	WhiteRibbon = 99,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, enumn::N)]
pub enum KcShipType {
	Unknown = 0,
	/// 海防艦
	DE = 1,
	/// 駆逐艦
	DD = 2,
	/// 軽巡洋艦
	CL = 3,
	/// 重雷装巡洋艦
	CLT = 4,
	/// 重巡洋艦
	CA = 5,
	/// 航空巡洋艦
	CAV = 6,
	/// 軽空母
	CVL = 7,
	/// 巡洋戦艦
	FBB = 8,
	/// 戦艦
	BB = 9,
	/// 航空戦艦
	BBV = 10,
	/// 正規空母
	CV = 11,
	/// 超弩級戦艦, not used
	XBB = 12,
	/// 潜水艦
	SS = 13,
	/// 潜水空母
	SSV = 14,
	/// 補給艦, on enemy side
	AP = 15,
	/// 水上機母艦
	AV = 16,
	/// 揚陸艦
	LHA = 17,
	/// 装甲空母
	CVB = 18,
	/// 工作艦
	AR = 19,
	/// 潜水母艦
	AS = 20,
	/// 練習巡洋艦
	CT = 21,
	/// 補給艦
	AO = 22,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, enumn::N)]
pub enum UserHQRank {
	MarshalAdmiral = 1,
	Admiral = 2,
	ViceAdmiral = 3,
	RearAdmiral = 4,
	Captain = 5,
	Commander = 6,
	JuniorCommander = 7,
	LieutenantCommander = 8,

	/// this is a special rank for the game, should be 8
	ViceLieutenantCommander = 9,
	JuniorLieutenantCommander = 10,
}

impl UserHQRank {
	pub fn get_name(&self) -> &str {
		match self {
			UserHQRank::MarshalAdmiral => "元帥",
			UserHQRank::Admiral => "大将",
			UserHQRank::ViceAdmiral => "中将",
			UserHQRank::RearAdmiral => "少将",
			UserHQRank::Captain => "大佐",
			UserHQRank::Commander => "中佐",
			UserHQRank::JuniorCommander => "新米中佐",
			UserHQRank::LieutenantCommander => "少佐",
			UserHQRank::ViceLieutenantCommander => "中堅少佐",
			UserHQRank::JuniorLieutenantCommander => "新米少佐",
		}
	}
}

impl From<i64> for UserHQRank {
	fn from(value: i64) -> Self {
		match value {
			1 => UserHQRank::MarshalAdmiral,
			2 => UserHQRank::Admiral,
			3 => UserHQRank::ViceAdmiral,
			4 => UserHQRank::RearAdmiral,
			5 => UserHQRank::Captain,
			6 => UserHQRank::Commander,
			7 => UserHQRank::JuniorCommander,
			_ => UserHQRank::LieutenantCommander,
		}
	}
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum KcSortieResultRank {
	S,
	A,
	B,
	C,
	D,
	E,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum KcSortieResult {
	Any,
	Clear,
	Ranked(KcSortieResultRank),
}
