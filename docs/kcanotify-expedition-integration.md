# KCanotify 远征数据集成方案

本文档描述如何将 KCanotify 远征数据集成到 EmuKC 的 bootstrap 流程中。

## 目录

- [1. 架构概览](#1-架构概览)
- [2. 文件变更清单](#2-文件变更清单)
- [3. 详细实现](#3-详细实现)
  - [3.1 添加数据源](#31-添加数据源)
  - [3.2 定义 KCanotify 数据模型](#32-定义-kcanotify-数据模型)
  - [3.3 定义内部模型](#33-定义内部模型)
  - [3.4 解析器实现](#34-解析器实现)
  - [3.5 更新解析模块入口](#35-更新解析模块入口)
  - [3.6 更新 Codex 结构](#36-更新-codex-结构)
- [4. 使用示例](#4-使用示例)
- [5. 数据验证清单](#5-数据验证清单)
- [6. 测试建议](#6-测试建议)

---

## 1. 架构概览

```
┌─────────────────────────────────────────────────────────────────┐
│                      Bootstrap 数据流                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐         │
│  │   RES_LIST  │───▶│ download_all│───▶│  .data/     │         │
│  │  (添加URL)  │    │  (并发下载)  │    │expedition.json        │
│  └─────────────┘    └─────────────┘    └──────┬──────┘         │
│                                               │                 │
│  ┌─────────────┐    ┌─────────────┐    ┌─────▼──────┐          │
│  │   Codex     │◀───│   parse     │◀───│  parser/   │          │
│  │(添加字段)    │    │partial_codex│    │kcanotify/  │          │
│  └──────┬──────┘    └─────────────┘    └────────────┘          │
│         │                                                       │
│  ┌──────▼──────┐                                               │
│  │  codex.save │───▶ .data/codex/expedition_condition.json      │
│  └─────────────┘                                               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 数据流说明

1. **Download**: `download_all()` 从 `RES_LIST` 并发下载 JSON 文件
2. **Parse**: `parse_partial_codex()` 调用 `kcanotify::expedition::parse()` 解析数据
3. **Model**: 解析后的数据填充到 `Kc3rdExpeditionConditionMap`
4. **Store**: `Codex::save()` 将数据序列化为 JSON 保存到 `.data/codex/`

---

## 2. 文件变更清单

| 文件路径 | 变更类型 | 说明 |
|----------|----------|------|
| `crates/emukc_bootstrap/src/res.rs` | 修改 | 添加 KCanotify 远征数据 URL |
| `crates/emukc_bootstrap/src/parser/kcanotify/mod.rs` | 新建 | KCanotify 解析器模块入口 |
| `crates/emukc_bootstrap/src/parser/kcanotify/types.rs` | 新建 | KCanotify 原始数据模型 |
| `crates/emukc_bootstrap/src/parser/kcanotify/expedition.rs` | 新建 | 远征数据解析实现 |
| `crates/emukc_bootstrap/src/parser/mod.rs` | 修改 | 添加 kcanotify 模块和解析调用 |
| `crates/emukc_model/src/thirdparty/expedition.rs` | 新建 | 远征条件内部模型 |
| `crates/emukc_model/src/thirdparty/mod.rs` | 修改 | 导出 expedition 模块 |
| `crates/emukc_model/src/codex/mod.rs` | 修改 | 添加 expedition_conditions 字段和序列化 |

---

## 3. 详细实现

### 3.1 添加数据源

**文件**: `crates/emukc_bootstrap/src/res.rs`

```rust
pub static RES_LIST: LazyLock<Vec<Resource<'static>>> = LazyLock::new(|| {
	vec![
		// ... 现有资源 ...
		
		Resource {
			// category: ResourceCategory::KCanotifyExpedition,
			url: "https://antest1.github.io/kcanotify-gamedata/files/expedition.json",
			save_as: "kcanotify_expedition.json",
			unzip_to: None,
		},
	]
});
```

### 3.2 定义 KCanotify 数据模型

**文件**: `crates/emukc_bootstrap/src/parser/kcanotify/types.rs`

```rust
//! KCanotify expedition data types

use serde::{Deserialize, Serialize};

/// KCanotify 远征数据根结构
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KCanotifyExpedition {
	/// 远征编号 (1-46, 100-115, 等)
	#[serde(rename = "no")]
	pub id: String,
	
	/// 远征代码 (如 "1", "A1", "B2")
	pub code: String,
	
	/// 海域编号 (1-7, 99)
	pub area: i64,
	
	/// 多语言名称
	pub name: KCanotifyExpeditionName,
	
	/// 远征时间 (分钟)
	pub time: i64,
	
	/// 资源奖励 [燃料, 弹药, 钢材, 铝土]
	pub resource: [i64; 4],
	
	/// 物品奖励 [[物品ID, 数量], ...]
	pub reward: Vec<[i64; 2]>,
	
	/// 经验值 [提督经验, 舰队经验]
	pub exp: [i64; 2],
	
	/// 舰队舰船数量要求
	#[serde(rename = "total-num")]
	pub total_num: i64,
	
	/// 旗舰等级要求
	#[serde(rename = "flag-lv")]
	pub flagship_level: i64,
	
	/// 舰队总等级要求 (可选)
	#[serde(rename = "total-lv")]
	pub total_level: Option<i64>,
	
	/// 旗舰类型要求 (可选)
	#[serde(rename = "flag-cond")]
	pub flagship_type: Option<String>,
	
	/// 编成条件表达式 (可选)
	/// 格式: "舰种ID-数量|舰种ID,舰种ID-数量/..."
	/// 示例: "3-1|1,2-2" = 轻巡1艘 或 (任意1艘+驱逐2艘)
	#[serde(rename = "total-cond")]
	pub total_condition: Option<String>,
	
	/// 舰队火力要求 (可选)
	#[serde(rename = "total-firepower")]
	pub total_firepower: Option<i64>,
	
	/// 舰队火力要求别名 (可选)
	#[serde(rename = "total-fp")]
	pub total_fp: Option<i64>,
	
	/// 舰队对潜要求 (可选)
	#[serde(rename = "total-asw")]
	pub total_asw: Option<i64>,
	
	/// 舰队索敌要求 (可选)
	#[serde(rename = "total-los")]
	pub total_los: Option<i64>,
	
	/// 携带桶的舰船数量要求 (可选)
	#[serde(rename = "drum-ship")]
	pub drum_ship: Option<i64>,
	
	/// 桶的总数量要求 (可选)
	#[serde(rename = "drum-num")]
	pub drum_num: Option<i64>,
	
	/// 桶数量要求 (可选，用于远征24等)
	#[serde(rename = "drum-num-optional")]
	pub drum_num_optional: Option<i64>,
}

/// 多语言远征名称
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KCanotifyExpeditionName {
	pub jp: String,
	pub ko: String,
	pub en: String,
	pub scn: String,
	pub tcn: String,
}

/// 舰种ID映射 (用于解析 total-cond)
pub mod ship_type {
	pub const ANY: i64 = 1;           // 任意舰
	pub const DESTROYER: i64 = 2;     // 驱逐
	pub const LIGHT_CRUISER: i64 = 3; // 轻巡
	pub const HEAVY_CRUISER: i64 = 5; // 重巡
	pub const CARRIER: &[i64] = &[7, 11, 16, 18]; // 航母系
	pub const SUBMARINE: &[i64] = &[13, 14]; // 潜水艇
	pub const SEAPLANE_TENDER: i64 = 16; // 水母
	pub const SUBMARINE_TENDER: i64 = 20; // 潜水母舰
	pub const TRAINING_CRUISER: i64 = 21; // 练习巡洋舰
	pub const LIGHT_CARRIER: i64 = 27;    // 轻空母
}
```

### 3.3 定义内部模型

**文件**: `crates/emukc_model/src/thirdparty/expedition.rs`

```rust
//! Expedition condition models for internal use

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// 远征条件映射表
pub type Kc3rdExpeditionConditionMap = HashMap<i64, Kc3rdExpeditionCondition>;

/// 远征编成条件
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdExpeditionCondition {
	/// 远征ID
	pub api_id: i64,
	
	/// 远征代码 (如 "1", "A1")
	pub code: String,
	
	/// 海域编号
	pub area: i64,
	
	/// 多语言名称
	pub name: Kc3rdExpeditionName,
	
	/// 远征时间 (分钟)
	pub time_minutes: i64,
	
	/// 资源奖励 [燃料, 弹药, 钢材, 铝土]
	pub resource_reward: [i64; 4],
	
	/// 物品奖励
	pub item_rewards: Vec<Kc3rdExpeditionItemReward>,
	
	/// 提督经验值
	pub admiral_exp: i64,
	
	/// 舰队经验值
	pub fleet_exp: i64,
	
	/// 编成条件
	pub requirements: Kc3rdExpeditionRequirements,
}

/// 远征物品奖励
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdExpeditionItemReward {
	/// 物品ID
	pub item_id: i64,
	/// 数量
	pub count: i64,
}

/// 多语言名称
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdExpeditionName {
	pub ja: String,
	pub ko: String,
	pub en: String,
	pub zh_cn: String,
	pub zh_tw: String,
}

/// 远征编成要求
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdExpeditionRequirements {
	/// 舰队舰船数量
	pub ship_count: i64,
	
	/// 旗舰等级要求
	pub flagship_level: i64,
	
	/// 舰队总等级要求 (可选)
	pub fleet_level: Option<i64>,
	
	/// 旗舰类型要求 (可选)
	pub flagship_type: Option<i64>,
	
	/// 编成条件 (OR条件列表)
	pub composition: Vec<Kc3rdCompositionAlternative>,
	
	/// 舰队火力要求 (可选)
	pub total_firepower: Option<i64>,
	
	/// 舰队对潜要求 (可选)
	pub total_asw: Option<i64>,
	
	/// 舰队索敌要求 (可选)
	pub total_los: Option<i64>,
	
	/// 运输桶要求 (可选)
	pub drum_requirements: Option<Kc3rdDrumRequirements>,
}

/// 编成条件分支 (OR中的一个选项)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdCompositionAlternative {
	/// AND条件列表 (必须同时满足)
	pub conditions: Vec<Kc3rdShipTypeRequirement>,
}

/// 舰种数量要求
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdShipTypeRequirement {
	/// 允许的舰种ID列表 (OR)
	pub ship_types: Vec<i64>,
	/// 所需数量
	pub count: i64,
}

/// 运输桶要求
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdDrumRequirements {
	/// 携带桶的舰船数量
	pub ship_count: i64,
	/// 桶的总数量
	pub total_count: i64,
	/// 是否可选
	pub optional: bool,
}
```

**文件**: `crates/emukc_model/src/thirdparty/mod.rs`

```rust
//! Thirdparty data from other sources.

#[doc(hidden)]
mod cache;
#[doc(hidden)]
pub mod expedition;  // 新增
#[doc(hidden)]
mod picturebook;
#[doc(hidden)]
mod quest;
#[doc(hidden)]
mod ship;
#[doc(hidden)]
mod slotitem;

// Re-export
#[doc(inline)]
#[allow(unused_imports)]
pub use cache::*;

#[doc(inline)]
#[allow(unused_imports)]
pub use expedition::*;  // 新增

#[doc(inline)]
#[allow(unused_imports)]
pub use quest::*;

#[doc(inline)]
#[allow(unused_imports)]
pub use ship::*;

#[doc(inline)]
#[allow(unused_imports)]
pub use slotitem::*;

#[doc(inline)]
#[allow(unused_imports)]
pub use picturebook::*;
```

### 3.4 解析器实现

**文件**: `crates/emukc_bootstrap/src/parser/kcanotify/mod.rs`

```rust
//! KCanotify data parser

pub mod expedition;
pub mod types;
```

**文件**: `crates/emukc_bootstrap/src/parser/kcanotify/expedition.rs`

```rust
//! KCanotify expedition data parser

use std::collections::HashMap;
use std::path::Path;

use super::types::KCanotifyExpedition;
use crate::parser::error::ParseError;
use emukc_model::thirdparty::{
	Kc3rdCompositionAlternative, Kc3rdDrumRequirements, Kc3rdExpeditionCondition,
	Kc3rdExpeditionConditionMap, Kc3rdExpeditionItemReward, Kc3rdExpeditionName,
	Kc3rdExpeditionRequirements, Kc3rdShipTypeRequirement,
};

/// 解析 KCanotify 远征数据文件
pub fn parse(path: impl AsRef<Path>) -> Result<Kc3rdExpeditionConditionMap, ParseError> {
	let raw = std::fs::read_to_string(path)?;
	let kcanotify_data: Vec<KCanotifyExpedition> = serde_json::from_str(&raw)?;
	
	let mut result = HashMap::new();
	
	for expedition in kcanotify_data {
		let condition = convert_to_internal_model(&expedition)?;
		result.insert(condition.api_id, condition);
	}
	
	Ok(result)
}

/// 将 KCanotify 数据转换为内部模型
fn convert_to_internal_model(
	src: &KCanotifyExpedition,
) -> Result<Kc3rdExpeditionCondition, ParseError> {
	let api_id = src.id.parse::<i64>()
		.map_err(|e| ParseError::IntParse(e.to_string()))?;
	
	Ok(Kc3rdExpeditionCondition {
		api_id,
		code: src.code.clone(),
		area: src.area,
		name: convert_name(&src.name),
		time_minutes: src.time,
		resource_reward: src.resource,
		item_rewards: convert_rewards(&src.reward),
		admiral_exp: src.exp[0],
		fleet_exp: src.exp[1],
		requirements: Kc3rdExpeditionRequirements {
			ship_count: src.total_num,
			flagship_level: src.flagship_level,
			fleet_level: src.total_level,
			flagship_type: parse_flagship_type(&src.flagship_type),
			composition: parse_composition(&src.total_condition)?,
			total_firepower: src.total_firepower.or(src.total_fp),
			total_asw: src.total_asw,
			total_los: src.total_los,
			drum_requirements: parse_drum_requirements(
				src.drum_ship,
				src.drum_num,
				src.drum_num_optional,
			),
		},
	})
}

/// 转换多语言名称
fn convert_name(src: &super::types::KCanotifyExpeditionName) -> Kc3rdExpeditionName {
	Kc3rdExpeditionName {
		ja: src.jp.clone(),
		ko: src.ko.clone(),
		en: src.en.clone(),
		zh_cn: src.scn.clone(),
		zh_tw: src.tcn.clone(),
	}
}

/// 转换物品奖励
fn convert_rewards(rewards: &[[i64; 2]]) -> Vec<Kc3rdExpeditionItemReward> {
	rewards
		.iter()
		.filter(|r| r[0] != 0) // 过滤掉 [0, 0] 的空奖励
		.map(|r| Kc3rdExpeditionItemReward {
			item_id: r[0],
			count: r[1],
		})
		.collect()
}

/// 解析旗舰类型
fn parse_flagship_type(cond: &Option<String>) -> Option<i64> {
	cond.as_ref().and_then(|s| s.parse().ok())
}

/// 解析编成条件表达式
/// 
/// 格式: "舰种ID-数量|舰种ID,舰种ID-数量/..."
/// 示例: 
///   - "3-1|1,2-2" → 轻巡1艘 或 (任意1艘+驱逐2艘)
///   - "3-1|2-2" → 轻巡1艘 或 驱逐2艘
///   - "1,2-3" → (任意或驱逐)3艘
fn parse_composition(
	cond: &Option<String>,
) -> Result<Vec<Kc3rdCompositionAlternative>, ParseError> {
	let Some(cond_str) = cond else {
		return Ok(vec![]);
	};
	
	if cond_str.is_empty() {
		return Ok(vec![]);
	}
	
	let mut alternatives = vec![];
	
	// 按 | 分割 OR 条件
	for alt_str in cond_str.split('|') {
		let mut conditions = vec![];
		
		// 按 / 分割 AND 条件
		for and_str in alt_str.split('/') {
			let req = parse_ship_type_requirement(and_str.trim())?;
			conditions.push(req);
		}
		
		alternatives.push(Kc3rdCompositionAlternative { conditions });
	}
	
	Ok(alternatives)
}

/// 解析单个舰种要求 (如 "1,2-3" 或 "3-1")
fn parse_ship_type_requirement(s: &str) -> Result<Kc3rdShipTypeRequirement, ParseError> {
	let parts: Vec<&str> = s.split('-').collect();
	if parts.len() != 2 {
		return Err(ParseError::Generic(format!(
			"Invalid ship type requirement format: {}",
			s
		)));
	}
	
	// 解析舰种ID列表 (逗号分隔)
	let ship_types: Vec<i64> = parts[0]
		.split(',')
		.map(|t| t.parse::<i64>())
		.collect::<Result<Vec<_>, _>>()
		.map_err(|e| ParseError::IntParse(e.to_string()))?;
	
	// 解析数量
	let count = parts[1]
		.parse::<i64>()
		.map_err(|e| ParseError::IntParse(e.to_string()))?;
	
	Ok(Kc3rdShipTypeRequirement { ship_types, count })
}

/// 解析运输桶要求
fn parse_drum_requirements(
	ship_count: Option<i64>,
	total_count: Option<i64>,
	optional_count: Option<i64>,
) -> Option<Kc3rdDrumRequirements> {
	// 优先使用 required，否则使用 optional
	let (count, optional) = if let (Some(ships), Some(total)) = (ship_count, total_count) {
		(Some((ships, total)), false)
	} else if let Some(opt) = optional_count {
		// optional 时，假设 ship_count = 1, total = opt
		(Some((1, opt)), true)
	} else {
		(None, false)
	};
	
	count.map(|(ships, total)| Kc3rdDrumRequirements {
		ship_count: ships,
		total_count: total,
		optional,
	})
}

#[cfg(test)]
mod tests {
	use super::*;
	
	#[test]
	fn test_parse_composition_simple() {
		let result = parse_composition(&Some("3-1".to_string())).unwrap();
		assert_eq!(result.len(), 1);
		assert_eq!(result[0].conditions.len(), 1);
		assert_eq!(result[0].conditions[0].ship_types, vec![3]);
		assert_eq!(result[0].conditions[0].count, 1);
	}
	
	#[test]
	fn test_parse_composition_or() {
		let result = parse_composition(&Some("3-1|2-2".to_string())).unwrap();
		assert_eq!(result.len(), 2);
		// 第一分支: 轻巡1艘
		assert_eq!(result[0].conditions[0].ship_types, vec![3]);
		// 第二分支: 驱逐2艘
		assert_eq!(result[1].conditions[0].ship_types, vec![2]);
	}
	
	#[test]
	fn test_parse_composition_complex() {
		// 远征4: 轻巡1艘 或 (任意1艘+驱逐2艘)
		let result = parse_composition(&Some("3-1|1,2-2".to_string())).unwrap();
		assert_eq!(result.len(), 2);
		// 第一分支
		assert_eq!(result[0].conditions.len(), 1);
		assert_eq!(result[0].conditions[0].ship_types, vec![3]);
		// 第二分支
		assert_eq!(result[1].conditions.len(), 1);
		assert_eq!(result[1].conditions[0].ship_types, vec![1, 2]);
	}
	
	#[test]
	fn test_parse_composition_multi_and() {
		// 模拟更复杂的: 驱逐2艘 / 轻巡1艘 (AND关系)
		let result = parse_composition(&Some("2-2/3-1".to_string())).unwrap();
		assert_eq!(result.len(), 1);
		assert_eq!(result[0].conditions.len(), 2);
		assert_eq!(result[0].conditions[0].ship_types, vec![2]);
		assert_eq!(result[0].conditions[0].count, 2);
		assert_eq!(result[0].conditions[1].ship_types, vec![3]);
		assert_eq!(result[0].conditions[1].count, 1);
	}
}
```

### 3.5 更新解析模块入口

**文件**: `crates/emukc_bootstrap/src/parser/mod.rs`

```rust
//! Parsers for various data sources.

pub mod error;
pub mod kcanotify;  // 新增
pub mod kc3kai;
pub mod kccp;
pub mod kcwiki;
pub mod kcwikizh_kcdata;
pub mod music;
pub mod tsunkit_quest;

use std::str::FromStr;

use emukc_model::{codex::game_config::GameConfig, kc2::navy::KcNavy, prelude::*};

use error::ParseError;
pub use kc3kai::parse as parse_kc3kai;
pub use kccp::quest::parse as parse_kccp_quests;
pub use kcwiki::parse as parse_kcwiki;
pub use kcwikizh_kcdata::parse as parse_kcdata;
pub use tsunkit_quest::parse as parse_tsunkit_quests;

/// Parse a partial codex from the given directory.
pub fn parse_partial_codex(dir: impl AsRef<std::path::Path>) -> Result<Codex, ParseError> {
	let dir = dir.as_ref();
	let manifest = {
		let path = dir.join("start2.json");
		let raw = std::fs::read_to_string(&path)?;
		debug!("Parsing manifest from {:?}", path);
		ApiManifest::from_str(&raw)?
	};

	let (ship_extra, slotitem_extra_info) = parse_kcwiki(dir, &manifest)?;

	let (ship_picturebook, ship_class_name) = parse_kcdata(dir.join("kc_data"), &manifest)?;
	let kccp_quests = {
		let path = dir.join("kccp_quests.json");
		let raw = std::fs::read_to_string(&path)?;
		debug!("Parsing kccp quests from {:?}", path);
		parse_kccp_quests(&raw)?
	};
	let quest = parse_tsunkit_quests(dir.join("tsunkit_quests.json"), &manifest, &kccp_quests)?;
	
	// 新增: 解析 KCanotify 远征数据
	let expedition_conditions = {
		let path = dir.join("kcanotify_expedition.json");
		debug!("Parsing KCanotify expedition data from {:?}", path);
		kcanotify::expedition::parse(&path)?
	};

	let music_list = music::get()?;

	let mut cache_source = CacheSource::default();
	{
		let path = dir.join("kc3kai_jp_quotes.json");
		let raw = std::fs::read_to_string(&path)?;
		let cleaned = raw
			.trim_start_matches('\u{FEFF}') // UTF-8 BOM
			.trim_start_matches('\u{FFFE}') // UTF-16 BOM
			.trim_start_matches(['\0', '\x01', '\x02', '\x03', '\x04', '\x05']) // controls
			.trim_start(); // whitespace
		parse_kc3kai(cleaned, &mut cache_source)?;
	}

	Ok(Codex {
		manifest,
		ship_extra,
		ship_class_name,
		ship_picturebook,
		slotitem_extra_info,
		quest,
		expedition_conditions, // 新增
		picturebook_extra: Kc3rdPicturebookExtra::default(),
		navy: KcNavy::default(),
		game_cfg: GameConfig::default(),
		music_list,
		cache_source: Some(cache_source),
	})
}
```

### 3.6 更新 Codex 结构

**文件**: `crates/emukc_model/src/codex/mod.rs`

```rust
//! All the data need for running the game logic

use std::{fs::create_dir_all, str::FromStr};

use game_config::GameConfig;
use thiserror::Error;

use crate::{
	kc2::{self, KcApiMusicListElement},
	prelude::{CacheSource, Kc3rdPicturebookExtra, Kc3rdPicturebookRW},
	thirdparty,
};

pub mod furniture;
pub mod game_config;
pub mod group;
pub mod incentive;
pub mod query;
pub mod repair;
pub mod ship;
pub mod slot_item;

/// Error type for `Codex`
#[derive(Error, Debug)]
pub enum CodexError {
	/// Entry already exists
	#[error("file {0} already exists")]
	AlreadyExist(String),

	/// IO error
	#[error("IO error: {0}")]
	Io(#[from] std::io::Error),

	/// Parse error
	#[error("Parse error: {0}")]
	Parse(#[from] std::num::ParseIntError),

	/// Serde error
	#[error("Serde error: {0}")]
	Serde(#[from] serde_json::Error),

	/// Entry not found
	#[error("Entry not found: {0}")]
	NotFound(String),
}

/// The `Codex` struct holds almost all the game data needed for the `EmuKC` project.
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct Codex {
	/// KC2 API manifest.
	pub manifest: kc2::start2::ApiManifest,

	/// thirdparty ship extra info map.
	pub ship_extra: thirdparty::Kc3rdShipMap,

	/// thirdparty ship class name map.
	pub ship_class_name: thirdparty::Kc3rdShipClassNameMap,

	/// thirdparty ship picturebook info map.
	pub ship_picturebook: thirdparty::Kc3rdShipPicturebookInfoMap,

	/// thirdparty slot item extra info map.
	pub slotitem_extra_info: thirdparty::Kc3rdSlotItemMap,

	/// thirdparty picturebook extra info.
	pub picturebook_extra: thirdparty::Kc3rdPicturebookExtra,

	/// navy info.
	pub navy: kc2::navy::KcNavy,

	/// thirdparty quest info map.
	pub quest: thirdparty::Kc3rdQuestMap,

	/// thirdparty expedition condition info map.
	pub expedition_conditions: thirdparty::Kc3rdExpeditionConditionMap,

	/// game config
	pub game_cfg: GameConfig,

	/// Music list
	pub music_list: Vec<KcApiMusicListElement>,

	/// Cache source.
	pub cache_source: Option<CacheSource>,
}

const PATH_START2: &str = "start2.json";
const PATH_SHIP_EXTRA: &str = "ship_extra.json";
const PATH_SHIP_CLASS_NAME: &str = "ship_class_name.json";
const PATH_SHIP_PICTUREBOOK: &str = "ship_picturebook.json";
const PATH_SLOTITEM_EXTRA_INFO: &str = "slotitem_extra_info.json";
const PATH_PICTUREBOOK_EXTRA_INFO: &str = "picturebook_extra_info.json";
const PATH_NAVY: &str = "navy.json";
const PATH_QUEST: &str = "quest.json";
const PATH_EXPEDITION_CONDITION: &str = "expedition_condition.json";
const PATH_MUSIC_LIST: &str = "music_list.json";
const PATH_GAME_CFG: &str = "game_config.json";
const PATH_CACHE_SOURCE: &str = "cache_source.json";

impl Codex {
	/// Load `Codex` instance from directory.
	pub fn load(dir: impl AsRef<std::path::Path>) -> Result<Self, CodexError> {
		let dir = dir.as_ref();
		let codex_dir = dir.join("codex");

		let manifest = {
			let path = codex_dir.join(PATH_START2);
			let raw = std::fs::read_to_string(&path)?;
			serde_json::from_str(&raw)?
		};

		let ship_extra = {
			let path = codex_dir.join(PATH_SHIP_EXTRA);
			let raw = std::fs::read_to_string(&path)?;
			serde_json::from_str(&raw)?
		};

		let ship_class_name = {
			let path = codex_dir.join(PATH_SHIP_CLASS_NAME);
			let raw = std::fs::read_to_string(&path)?;
			serde_json::from_str(&raw)?
		};

		let ship_picturebook = {
			let path = codex_dir.join(PATH_SHIP_PICTUREBOOK);
			let raw = std::fs::read_to_string(&path)?;
			serde_json::from_str(&raw)?
		};

		let slotitem_extra_info = {
			let path = codex_dir.join(PATH_SLOTITEM_EXTRA_INFO);
			let raw = std::fs::read_to_string(&path)?;
			serde_json::from_str(&raw)?
		};

		let picturebook_extra = {
			let path = codex_dir.join(PATH_PICTUREBOOK_EXTRA_INFO);
			let raw = std::fs::read_to_string(&path)?;
			serde_json::from_str(&raw)?
		};

		let navy = {
			let path = codex_dir.join(PATH_NAVY);
			let raw = std::fs::read_to_string(&path)?;
			serde_json::from_str(&raw)?
		};

		let quest = {
			let path = codex_dir.join(PATH_QUEST);
			let raw = std::fs::read_to_string(&path)?;
			serde_json::from_str(&raw)?
		};

		// 新增: 加载远征条件数据
		let expedition_conditions = {
			let path = codex_dir.join(PATH_EXPEDITION_CONDITION);
			let raw = std::fs::read_to_string(&path)?;
			serde_json::from_str(&raw)?
		};

		let game_cfg = {
			let path = codex_dir.join(PATH_GAME_CFG);
			let raw = std::fs::read_to_string(&path)?;
			serde_json::from_str(&raw)?
		};

		let music_list = {
			let path = codex_dir.join(PATH_MUSIC_LIST);
			let raw = std::fs::read_to_string(&path)?;
			serde_json::from_str(&raw)?
		};

		let cache_source = {
			let path = codex_dir.join(PATH_CACHE_SOURCE);
			if path.exists() {
				let raw = std::fs::read_to_string(&path)?;
				serde_json::from_str(&raw).ok()
			} else {
				None
			}
		};

		Ok(Self {
			manifest,
			ship_extra,
			ship_class_name,
			ship_picturebook,
			slotitem_extra_info,
			picturebook_extra,
			navy,
			quest,
			expedition_conditions, // 新增
			game_cfg,
			music_list,
			cache_source,
		})
	}

	/// Save `Codex` instance to directory.
	pub fn save(&self, dir: impl AsRef<std::path::Path>) -> Result<(), CodexError> {
		let dir = dir.as_ref();
		let codex_dir = dir.join("codex");

		if !codex_dir.exists() {
			create_dir_all(&codex_dir)?;
		}

		{
			let path = codex_dir.join(PATH_START2);
			let raw = serde_json::to_string_pretty(&self.manifest)?;
			std::fs::write(&path, raw)?;
		}

		{
			let path = codex_dir.join(PATH_SHIP_EXTRA);
			let raw = serde_json::to_string_pretty(&self.ship_extra)?;
			std::fs::write(&path, raw)?;
		}

		{
			let path = codex_dir.join(PATH_SHIP_CLASS_NAME);
			let raw = serde_json::to_string_pretty(&self.ship_class_name)?;
			std::fs::write(&path, raw)?;
		}

		{
			let path = codex_dir.join(PATH_SHIP_PICTUREBOOK);
			let raw = serde_json::to_string_pretty(&self.ship_picturebook)?;
			std::fs::write(&path, raw)?;
		}

		{
			let path = codex_dir.join(PATH_SLOTITEM_EXTRA_INFO);
			let raw = serde_json::to_string_pretty(&self.slotitem_extra_info)?;
			std::fs::write(&path, raw)?;
		}

		{
			let path = codex_dir.join(PATH_PICTUREBOOK_EXTRA_INFO);
			let raw = serde_json::to_string_pretty(&self.picturebook_extra)?;
			std::fs::write(&path, raw)?;
		}

		{
			let path = codex_dir.join(PATH_NAVY);
			let raw = serde_json::to_string_pretty(&self.navy)?;
			std::fs::write(&path, raw)?;
		}

		{
			let path = codex_dir.join(PATH_QUEST);
			let raw = serde_json::to_string_pretty(&self.quest)?;
			std::fs::write(&path, raw)?;
		}

		// 新增: 保存远征条件数据
		{
			let path = codex_dir.join(PATH_EXPEDITION_CONDITION);
			let raw = serde_json::to_string_pretty(&self.expedition_conditions)?;
			std::fs::write(&path, raw)?;
		}

		{
			let path = codex_dir.join(PATH_GAME_CFG);
			let raw = serde_json::to_string_pretty(&self.game_cfg)?;
			std::fs::write(&path, raw)?;
		}

		{
			let path = codex_dir.join(PATH_MUSIC_LIST);
			let raw = serde_json::to_string_pretty(&self.music_list)?;
			std::fs::write(&path, raw)?;
		}

		if let Some(ref cache_source) = self.cache_source {
			let path = codex_dir.join(PATH_CACHE_SOURCE);
			let raw = serde_json::to_string_pretty(cache_source)?;
			std::fs::write(&path, raw)?;
		}

		Ok(())
	}
}
```

---

## 4. 使用示例

### 4.1 在 Gameplay 中验证远征条件

```rust
use emukc_model::prelude::*;

pub async fn validate_expedition(
	codex: &Codex,
	fleet: &Fleet,
	mission_id: i64,
) -> Result<(), GameplayError> {
	// 1. 获取远征条件
	let condition = codex.expedition_conditions
		.get(&mission_id)
		.ok_or_else(|| GameplayError::NotFound(format!("远征 {} 不存在", mission_id)))?;
	
	// 2. 验证旗舰等级
	if fleet.flagship.level < condition.requirements.flagship_level {
		return Err(GameplayError::InvalidInput(
			format!("旗舰等级不足: {} < {}", 
				fleet.flagship.level, 
				condition.requirements.flagship_level
			)
		));
	}
	
	// 3. 验证编成条件 (OR条件)
	let mut composition_satisfied = condition.requirements.composition.is_empty();
	for alternative in &condition.requirements.composition {
		let mut satisfied = true;
		for req in &alternative.conditions {
			let actual_count = fleet.ships.iter()
				.filter(|s| req.ship_types.contains(&s.ship_type_id))
				.count() as i64;
			if actual_count < req.count {
				satisfied = false;
				break;
			}
		}
		if satisfied {
			composition_satisfied = true;
			break;
		}
	}
	
	if !composition_satisfied {
		return Err(GameplayError::InvalidInput("舰队编成不满足远征要求".into()));
	}
	
	// 4. 验证运输桶要求
	if let Some(drum_req) = &condition.requirements.drum_requirements {
		const DRUM_CANISTER_ID: i64 = 75; // 运输桶物品ID
		
		let ships_with_drum = fleet.ships.iter()
			.filter(|s| s.equipment.iter().any(|e| e.item_id == DRUM_CANISTER_ID))
			.count() as i64;
		
		if ships_with_drum < drum_req.ship_count {
			return Err(GameplayError::InvalidInput(
				format!("携带运输桶的舰船数量不足: {} < {}", 
					ships_with_drum, 
					drum_req.ship_count
				)
			));
		}
		
		// 检查总桶数
		let total_drums: i64 = fleet.ships.iter()
			.map(|s| s.equipment.iter()
				.filter(|e| e.item_id == DRUM_CANISTER_ID)
				.count() as i64
			)
			.sum();
		
		if total_drums < drum_req.total_count {
			return Err(GameplayError::InvalidInput(
				format!("运输桶总数不足: {} < {}", 
					total_drums, 
					drum_req.total_count
				)
			));
		}
	}
	
	Ok(())
}
```

### 4.2 计算远征成功率

```rust
/// 计算远征成功率
fn calculate_expedition_success_rate(
	codex: &Codex,
	fleet: &Fleet,
	mission_id: i64,
) -> (f64, f64) {  // (成功率, 大成功率)
	let condition = match codex.expedition_conditions.get(&mission_id) {
		Some(c) => c,
		None => return (0.0, 0.0),
	};
	
	// 基础成功率: 50%
	let base_rate = 0.50;
	
	// 旗舰等级加成: 每级 +0.1%, 上限 6%
	let flagship_bonus = (fleet.flagship.level as f64 * 0.001).min(0.06);
	
	let success_rate = base_rate + flagship_bonus;
	
	// 大成功率: 16.67% × 闪舰数量 (morale >= 50)
	const SPARKLE_THRESHOLD: i64 = 50;
	let sparkled_count = fleet.ships.iter()
		.filter(|s| s.morale >= SPARKLE_THRESHOLD)
		.count() as f64;
	
	let great_success_rate = (sparkled_count * 0.1667).min(1.0);
	
	(success_rate, great_success_rate)
}
```

---

## 5. 数据验证清单

实现后应验证以下数据点：

### 5.1 基础数据完整性

- [ ] 远征 1-46 (常规远征) 全部可解析
- [ ] 远征 A1-A6 (100-105) 编成条件正确
- [ ] 远征 B1-B6 (110-115) 编成条件正确
- [ ] 远征 D1-D3 (131-133) 编成条件正确
- [ ] 远征 E1-E2 (141-142) 编成条件正确
- [ ] 远征 S1-S2 (203-204) 支援远征数据正确

### 5.2 特殊远征验证

- [ ] 远征 21, 37, 38 (东京急行) drum-ship 和 drum-num 字段正确
- [ ] 远征 24 (北方航路) drum-num-optional 正确识别
- [ ] 远征 40 (水上机前线运输) drum-num-optional 正确识别
- [ ] 远征 115 (精鋭水雷戦隊夜襲) 复杂编成条件正确解析
- [ ] 远征 43 (MI船団護衛) 多分支旗舰类型要求正确解析

### 5.3 条件表达式解析验证

| 远征 | 条件表达式 | 预期解析结果 |
|------|-----------|-------------|
| 4 | `3-1\|1,2-2` | 轻巡1艘 或 (任意+驱逐)2艘 |
| 5 | `3-1\|1,2-2/2-1\|1-3` | 复杂多分支 |
| 100 | `1,2-3` | 驱逐/任意 3艘 |
| 115 | `3-1\|2-5` | 轻巡旗舰+驱逐5艘 |

### 5.4 多语言名称验证

- [ ] 日文名称完整 (jp)
- [ ] 简体中文名称完整 (scn)
- [ ] 繁体中文名称完整 (tcn)
- [ ] 英文名称完整 (en)
- [ ] 韩文名称完整 (ko)

---

## 6. 测试建议

### 6.1 单元测试

```rust
// crates/emukc_bootstrap/src/parser/kcanotify/expedition.rs

#[cfg(test)]
mod tests {
	use super::*;
	
	/// 测试简单编成条件解析
	#[test]
	fn test_parse_simple_composition() {
		let result = parse_composition(&Some("3-1".to_string())).unwrap();
		assert_eq!(result.len(), 1);
		assert_eq!(result[0].conditions[0].ship_types, vec![3]);
		assert_eq!(result[0].conditions[0].count, 1);
	}
	
	/// 测试远征4 (対潜警戒任務)
	#[test]
	fn test_expedition_4() {
		// 条件: 轻巡1艘 或 (驱逐2艘 + 任意1艘)
		let result = parse_composition(&Some("3-1\|1,2-2".to_string())).unwrap();
		assert_eq!(result.len(), 2);
	}
	
	/// 测试完整数据加载
	#[test]
	fn test_load_expedition_data() {
		let map = parse(".data/kcanotify_expedition.json").unwrap();
		assert_eq!(map.len(), 65); // 应包含65条远征
		
		// 验证远征1
		let exp1 = map.get(&1).unwrap();
		assert_eq!(exp1.code, "1");
		assert_eq!(exp1.time_minutes, 15);
		
		// 验证远征37 (东京急行)
		let exp37 = map.get(&37).unwrap();
		assert!(exp37.requirements.drum_requirements.is_some());
	}
}
```

### 6.2 集成测试

```rust
// tests/expedition_tests.rs

use emukc_bootstrap::parser::parse_partial_codex;

#[test]
fn test_expedition_conditions_in_codex() {
	let codex = parse_partial_codex(".data").unwrap();
	
	// 验证所有远征条件都已加载
	assert!(!codex.expedition_conditions.is_empty());
	
	// 验证特定远征
	let exp1 = codex.expedition_conditions.get(&1).unwrap();
	assert_eq!(exp1.name.zh_cn, "练习航海");
}
```

### 6.3 手动验证命令

```bash
# 1. 下载数据
cargo run -- bootstrap

# 2. 验证远征数据文件
jq '. | length' .data/kcanotify_expedition.json
# 预期输出: 65

# 3. 验证远征1数据
jq '.[0]' .data/kcanotify_expedition.json

# 4. 验证远征37 (东京急行) 运输桶要求
jq '.[] | select(.code == "37") | {name: .name.scn, drum_ship, drum_num}' .data/kcanotify_expedition.json

# 5. 检查 Codex 是否包含远征数据
jq '. | keys' .data/codex/expedition_condition.json | head -20
```

---

## 附录: 舰种ID参考表

| ID | 舰种 | 英文 |
|----|------|------|
| 1 | 任意舰 | Any |
| 2 | 驱逐 | Destroyer |
| 3 | 轻巡 | Light Cruiser |
| 5 | 重巡 | Heavy Cruiser |
| 7,11,16,18 | 航母系 | Carrier |
| 13,14 | 潜水艇 | Submarine |
| 16 | 水母 | Seaplane Tender |
| 20 | 潜水母舰 | Submarine Tender |
| 21 | 练习巡洋舰 | Training Cruiser |
| 27 | 轻空母 | Light Carrier |

---

## 参考链接

- KCanotify 数据源: https://antest1.github.io/kcanotify-gamedata/files/expedition.json
- ElectronicObserver 验证逻辑: https://github.com/andanteyk/ElectronicObserver/blob/develop/ElectronicObserver/Data/MissionClearCondition.cs
- 远征系统实现方案: ./expedition-system-plan.md
