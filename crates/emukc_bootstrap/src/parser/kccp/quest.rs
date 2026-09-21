use std::collections::BTreeMap;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::parser::error::ParseError;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KccpQuestInfo {
    pub id: i64,
    pub name: String,
    pub desc: String,
}

impl Default for KccpQuestInfo {
    fn default() -> Self {
        Self {
            id: 0,
            name: "n/a".to_string(),
            desc: "n/a".to_string(),
        }
    }
}

enum ParserStatus {
    Id,
    Name,
    Desc,
}

/// Parse the quest info from the raw string.
///
/// # Arguments
///
/// * `raw` - The raw string to parse.
///
/// # Returns
///
/// A map of quest id to quest info.
pub fn parse(raw: &str) -> Result<BTreeMap<i64, KccpQuestInfo>, ParseError> {
    let reg_id = Regex::new(r"_quest_id_(\d+)").unwrap();
    let reg_desc = Regex::new(r#""([^"]+)""#).unwrap();

    let mut status = ParserStatus::Id;
    let mut result: BTreeMap<i64, KccpQuestInfo> = BTreeMap::new();

    let mut quest_id: Option<i64> = None;
    let mut quest_name: Option<String> = None;

    for line in raw.lines() {
        match status {
            ParserStatus::Id => {
                if let Some(caps) = reg_id.captures(line)
                    && let Some(matched) = caps.get(1)
                {
                    quest_id = Some(matched.as_str().parse().unwrap());
                    status = ParserStatus::Name;
                }
            }
            ParserStatus::Name => {
                // 	"【節分任務:鬼】南西方面節分作戦二〇二四": "[Setsubun] Southwestern Area Setsubun Operation 2024",
                if let Some(name) = line.split("\":").next() {
                    quest_name = Some(name.trim_start().replace('"', ""));
                    status = ParserStatus::Desc;
                }
            }
            ParserStatus::Desc => {
                let mut matches = reg_desc.captures_iter(line);
                if let Some(cap) = matches.next() {
                    let desc = cap
                        .get(1)
                        .unwrap()
                        .as_str()
                        .trim_start()
                        .replace('"', "")
                        .replace("\\n", "<br>");

                    if let (Some(id), Some(name)) = (quest_id, &quest_name) {
                        result.insert(
                            id,
                            KccpQuestInfo {
                                id,
                                name: name.clone(),
                                desc,
                            },
                        );
                    }

                    status = ParserStatus::Id;
                }
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = include_str!("../../../tests/fixtures/kccp/quests_sample.json");

    fn parsed() -> BTreeMap<i64, KccpQuestInfo> {
        parse(SAMPLE).unwrap()
    }

    #[test]
    fn well_formed_quests_get_their_japanese_name_and_desc() {
        let result = parsed();

        let q101 = result.get(&101).unwrap();
        assert_eq!(q101.name, "はじめての「編成」！");
        assert_eq!(q101.desc, "２隻以上の艦で構成される「艦隊」を編成せよ！");

        let q102 = result.get(&102).unwrap();
        assert_eq!(q102.name, "「駆逐隊」を編成せよ！");
        assert_eq!(q102.desc, "駆逐艦４隻以上で構成される「駆逐隊」を編成せよ！");
    }

    #[test]
    fn a_quest_without_a_name_line_swallows_the_next_quests_id() {
        let result = parsed();

        // 当前行为，计划 004 会翻转：615 的 desc 应为日文描述，616 应存在
        let q615 = result.get(&615).unwrap();
        assert_eq!(
            q615.name,
            "「九九式艦爆(江草隊)」搭載空母を秘書艦にした状態で新たに「彗星」を2つ廃棄！"
        );
        assert_eq!(q615.desc, "_quest_id_616");
        assert!(!result.contains_key(&616));
    }

    #[test]
    fn a_quest_without_a_desc_line_swallows_the_next_quests_id() {
        let result = parsed();

        // 当前行为，计划 004 会翻转：1124 的 desc 应为空或缺省，103 应存在
        let q1124 = result.get(&1124).unwrap();
        assert_eq!(q1124.name, "【早春限定任務】夜間航空作戦能力の増強");
        assert_eq!(q1124.desc, "_quest_id_103");
        assert!(!result.contains_key(&103));
    }

    #[test]
    fn the_dummy_sentinel_group_parses_normally() {
        let result = parsed();

        let q1169 = result.get(&1169).unwrap();
        assert_eq!(q1169.name, "【期間限定任務・拡張任務】端午の節句工廠【II】");
        assert_eq!(
            q1169.desc,
            "第一艦隊の旗艦及び二番艦に「山汐丸改」または「第四号海防艦」「第三〇号海防艦」を配備。<br>練度maxの「九七式艦攻」x5廃棄。工廠資源x2、開発資材x155、燃料と弾薬x各550を準備せよ！"
        );
    }

    #[test]
    fn the_sample_yields_five_of_its_seven_quests() {
        let result = parsed();

        // 当前行为，计划 004 会翻转：样本里有 7 个 id，616 和 103 被前一条吞掉
        assert_eq!(result.len(), 5);
        assert_eq!(result.keys().copied().collect::<Vec<_>>(), vec![101, 102, 615, 1124, 1169]);
    }
}
