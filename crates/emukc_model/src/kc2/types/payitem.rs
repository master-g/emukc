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
	FurnitureCraftsman = 15,
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
	OffShoreResupply = 25,
	/// 補強増設
	ReinforceExpansion = 26,
	/// 設営隊
	ConstCorps = 27,
	/// 「緊急修理資材」セット
	EmergencyRepairMaterialSet = 28,
	/// 潜水艦補給物資パック
	SubmarineSupplyMaterialPack = 29,
}
