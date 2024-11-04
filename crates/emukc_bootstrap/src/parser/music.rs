use emukc_model::kc2::KcApiMusicListElement;

use super::error::ParseError;

const DEFAULT_MUSIC_LIST: &str = r#"
[
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 101,
    "api_description": "母港BGM設定可能",
    "api_id": 5,
    "api_loops": 1,
    "api_name": "母港",
    "api_use_coin": 100
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 217,
    "api_description": "母港BGM設定可能",
    "api_id": 6,
    "api_loops": 2,
    "api_name": "武蔵の帰投",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 216,
    "api_description": "母港BGM設定可能",
    "api_id": 7,
    "api_loops": 2,
    "api_name": "桃の節句と艦娘",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 263,
    "api_description": "母港BGM設定可能",
    "api_id": 8,
    "api_loops": 2,
    "api_name": "北大西洋の風",
    "api_use_coin": 1500
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 233,
    "api_description": "母港BGM設定可能",
    "api_id": 9,
    "api_loops": 2,
    "api_name": "噴式の胎動",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 210,
    "api_description": "母港BGM設定可能",
    "api_id": 10,
    "api_loops": 2,
    "api_name": "連合艦隊の出撃",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 225,
    "api_description": "試製新曲先行公開",
    "api_id": 11,
    "api_loops": 2,
    "api_name": "加賀岬",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 226,
    "api_description": "同曲母港BGMバージョン",
    "api_id": 12,
    "api_loops": 2,
    "api_name": "加賀岬改(母港ver)",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 255,
    "api_description": "母港BGM設定可能",
    "api_id": 13,
    "api_loops": 2,
    "api_name": "Valentine’s Sea",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 215,
    "api_description": "母港BGM設定可能",
    "api_id": 14,
    "api_loops": 2,
    "api_name": "艦娘のお菓子作り",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 265,
    "api_description": "母港BGM設定可能",
    "api_id": 15,
    "api_loops": 2,
    "api_name": "村雨と峯雲の出撃",
    "api_use_coin": 1500
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 245,
    "api_description": "母港BGM設定可能",
    "api_id": 16,
    "api_loops": 2,
    "api_name": "瑞雲の空",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 223,
    "api_description": "視聴ロングバージョン",
    "api_id": 17,
    "api_loops": 1,
    "api_name": "提督との絆",
    "api_use_coin": 700
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 218,
    "api_description": "母港BGM設定可能",
    "api_id": 18,
    "api_loops": 2,
    "api_name": "雨音の鎮守府",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 219,
    "api_description": "母港BGM設定可能",
    "api_id": 19,
    "api_loops": 2,
    "api_name": "雨とお酒と艦娘",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 240,
    "api_description": "母港BGM設定可能",
    "api_id": 20,
    "api_loops": 2,
    "api_name": "雨とお酒と艦娘(第二夜)",
    "api_use_coin": 1500
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 213,
    "api_description": "母港BGM設定可能",
    "api_id": 21,
    "api_loops": 2,
    "api_name": "士魂の護り",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 274,
    "api_description": "母港BGM設定可能",
    "api_id": 22,
    "api_loops": 2,
    "api_name": "海に吹く碧の風",
    "api_use_coin": 2000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 246,
    "api_description": "母港BGM設定可能",
    "api_id": 23,
    "api_loops": 2,
    "api_name": "二水戦の航跡",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 237,
    "api_description": "試製先行ショートmode",
    "api_id": 24,
    "api_loops": 1,
    "api_name": "月夜海(つきよみ)",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 254,
    "api_description": "母港BGM設定可能",
    "api_id": 25,
    "api_loops": 2,
    "api_name": "頌春令和の海",
    "api_use_coin": 1500
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 229,
    "api_description": "母港BGM設定可能",
    "api_id": 26,
    "api_loops": 2,
    "api_name": "提督と艦娘の食卓",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 266,
    "api_description": "母港BGM設定可能",
    "api_id": 27,
    "api_loops": 2,
    "api_name": "地中海の潮風",
    "api_use_coin": 1500
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 239,
    "api_description": "試製涼月ソロ&ショートmode",
    "api_id": 28,
    "api_loops": 1,
    "api_name": "月夜海 涼月mode",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 250,
    "api_description": "母港BGM設定可能",
    "api_id": 29,
    "api_loops": 2,
    "api_name": "トラック目指して",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 264,
    "api_description": "母港BGM設定可能",
    "api_id": 30,
    "api_loops": 2,
    "api_name": "沖に立つ波",
    "api_use_coin": 1500
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 273,
    "api_description": "母港BGM設定可能",
    "api_id": 31,
    "api_loops": 2,
    "api_name": "海原へ",
    "api_use_coin": 1500
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 276,
    "api_description": "母港BGM設定可能",
    "api_id": 32,
    "api_loops": 2,
    "api_name": "八戸の盾",
    "api_use_coin": 2000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 259,
    "api_description": "母港BGM設定可能",
    "api_id": 33,
    "api_loops": 2,
    "api_name": "根室沖の輝き",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 275,
    "api_description": "母港BGM設定可能",
    "api_id": 34,
    "api_loops": 2,
    "api_name": "抜錨！鵜来型海防艦",
    "api_use_coin": 1500
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 241,
    "api_description": "母港BGM設定可能",
    "api_id": 35,
    "api_loops": 2,
    "api_name": "梅雨明けの白露",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 253,
    "api_description": "母港BGM設定可能",
    "api_id": 36,
    "api_loops": 2,
    "api_name": "雪風の奇跡",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 257,
    "api_description": "母港BGM設定可能",
    "api_id": 37,
    "api_loops": 2,
    "api_name": "ペデスタル作戦",
    "api_use_coin": 1500
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 249,
    "api_description": "BGMバージョン",
    "api_id": 38,
    "api_loops": 1,
    "api_name": "八駆の迎撃",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 236,
    "api_description": "母港BGM設定可能",
    "api_id": 39,
    "api_loops": 2,
    "api_name": "連合艦隊旗艦",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 268,
    "api_description": "母港BGM設定可能",
    "api_id": 40,
    "api_loops": 2,
    "api_name": "艦隊10周年の抜錨",
    "api_use_coin": 1500
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 269,
    "api_description": "母港BGM設定可能",
    "api_id": 41,
    "api_loops": 2,
    "api_name": "夜の祈り",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 148,
    "api_description": "母港BGM設定可能",
    "api_id": 42,
    "api_loops": 2,
    "api_name": "眼下の伊号",
    "api_use_coin": 1500
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 244,
    "api_description": "母港BGM設定可能",
    "api_id": 43,
    "api_loops": 2,
    "api_name": "長波、駆ける",
    "api_use_coin": 1500
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 235,
    "api_description": "母港BGM設定可能",
    "api_id": 44,
    "api_loops": 2,
    "api_name": "師走の鎮守府",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 260,
    "api_description": "母港BGM設定可能",
    "api_id": 45,
    "api_loops": 1,
    "api_name": "Fleet for the end of the year",
    "api_use_coin": 2021
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 238,
    "api_description": "母港BGM設定可能",
    "api_id": 46,
    "api_loops": 2,
    "api_name": "祈り",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 262,
    "api_description": "母港BGM設定可能",
    "api_id": 47,
    "api_loops": 2,
    "api_name": "令和桃の節句",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 208,
    "api_description": "母港BGM設定可能",
    "api_id": 48,
    "api_loops": 2,
    "api_name": "海上護衛戦",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 214,
    "api_description": "母港BGM設定可能",
    "api_id": 49,
    "api_loops": 2,
    "api_name": "特型駆逐艦",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 261,
    "api_description": "母港BGM設定可能",
    "api_id": 50,
    "api_loops": 2,
    "api_name": "新しい年の護り",
    "api_use_coin": 2024
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 212,
    "api_description": "母港BGM設定可能",
    "api_id": 51,
    "api_loops": 2,
    "api_name": "迎春の鎮守府",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 243,
    "api_description": "母港BGM設定可能",
    "api_id": 52,
    "api_loops": 2,
    "api_name": "節分の鎮守府",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 224,
    "api_description": "母港BGM設定可能",
    "api_id": 53,
    "api_loops": 2,
    "api_name": "浜辺の艦娘",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 230,
    "api_description": "母港BGM設定可能",
    "api_id": 54,
    "api_loops": 2,
    "api_name": "水着の出撃",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 231,
    "api_description": "母港BGM設定可能",
    "api_id": 55,
    "api_loops": 2,
    "api_name": "鎮守府秋刀魚祭り改",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 242,
    "api_description": "母港BGM設定可能",
    "api_id": 56,
    "api_loops": 2,
    "api_name": "鎮守府秋刀魚祭り改三",
    "api_use_coin": 1500
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 205,
    "api_description": "母港BGM設定可能",
    "api_id": 57,
    "api_loops": 2,
    "api_name": "秋月の空",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 206,
    "api_description": "母港BGM設定可能",
    "api_id": 58,
    "api_loops": 2,
    "api_name": "明石の工廠",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 271,
    "api_description": "母港BGM設定可能",
    "api_id": 59,
    "api_loops": 2,
    "api_name": "清霜の朝",
    "api_use_coin": 1500
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 247,
    "api_description": "BGMバージョン",
    "api_id": 60,
    "api_loops": 2,
    "api_name": "佐世保の時雨",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 248,
    "api_description": "母港BGM設定可能",
    "api_id": 61,
    "api_loops": 2,
    "api_name": "遥かなるウルシー泊地",
    "api_use_coin": 2000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 256,
    "api_description": "母港BGM設定可能",
    "api_id": 62,
    "api_loops": 2,
    "api_name": "西村艦隊の戦い",
    "api_use_coin": 2000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 251,
    "api_description": "母港BGM設定可能",
    "api_id": 63,
    "api_loops": 2,
    "api_name": "加賀の征く海",
    "api_use_coin": 2000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 228,
    "api_description": "母港BGM設定可能",
    "api_id": 64,
    "api_loops": 2,
    "api_name": "聖夜の母港",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 234,
    "api_description": "母港BGM設定可能",
    "api_id": 65,
    "api_loops": 2,
    "api_name": "粉雪の降る夜",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 211,
    "api_description": "母港BGM設定可能",
    "api_id": 66,
    "api_loops": 2,
    "api_name": "冬の艦隊",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 0,
    "api_bgm_id": 222,
    "api_description": "視聴ロングバージョン",
    "api_id": 67,
    "api_loops": 1,
    "api_name": "華の二水戦",
    "api_use_coin": 700
  },
  {
    "api_bgm_flag": 0,
    "api_bgm_id": 220,
    "api_description": "視聴ロングバージョン",
    "api_id": 68,
    "api_loops": 1,
    "api_name": "暁の水平線に",
    "api_use_coin": 700
  },
  {
    "api_bgm_flag": 0,
    "api_bgm_id": 221,
    "api_description": "視聴ロングバージョン",
    "api_id": 69,
    "api_loops": 1,
    "api_name": "鎮守府の朝",
    "api_use_coin": 700
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 272,
    "api_description": "母港BGM設定可能",
    "api_id": 70,
    "api_loops": 2,
    "api_name": "ももちと新しき朝",
    "api_use_coin": 2000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 227,
    "api_description": "母港BGM設定可能",
    "api_id": 71,
    "api_loops": 2,
    "api_name": "鎮守府の秋祭り",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 270,
    "api_description": "母港BGM設定可能",
    "api_id": 72,
    "api_loops": 2,
    "api_name": "Fleet Halloween Pumpkin",
    "api_use_coin": 1500
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 258,
    "api_description": "母港BGM設定可能",
    "api_id": 73,
    "api_loops": 2,
    "api_name": "Trick or Fleet!",
    "api_use_coin": 1500
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 252,
    "api_description": "母港BGM設定可能",
    "api_id": 74,
    "api_loops": 2,
    "api_name": "秋雲の描くスケッチ",
    "api_use_coin": 2000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 232,
    "api_description": "母港BGM設定可能",
    "api_id": 75,
    "api_loops": 2,
    "api_name": "艦娘音頭",
    "api_use_coin": 1000
  },
  {
    "api_bgm_flag": 1,
    "api_bgm_id": 267,
    "api_description": "母港BGM設定可能",
    "api_id": 76,
    "api_loops": 1,
    "api_name": "未来(いま)",
    "api_use_coin": 2000
  }
]
"#;

pub fn get() -> Result<Vec<KcApiMusicListElement>, ParseError> {
	let result: Vec<KcApiMusicListElement> = serde_json::from_str(DEFAULT_MUSIC_LIST)?;

	Ok(result)
}
