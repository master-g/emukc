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
	FurnitureCraftsman = 52,
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
	Saury = 68,
	/// 秋刀魚の缶詰, slotitem
	MackerelCan = 69,
	/// 熟練搭乗員
	SkilledCrew = 70,
	/// ネ式エンジン
	NEngine = 71,
	/// お飾り材料
	DecoMaterial = 72,
	/// 設営隊
	ConstCorps = 73,
	/// 新型航空機設計図
	NewAircraftBlueprint = 74,
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
	/// 海外艦最新技術
	OverseasWarshipTechnology = 100,
	/// 夜間熟練搭乗員
	NightSkilledCrew = 101,
	/// 航空特別増加食
	AirSpecialIncreasedRation = 102,
}
