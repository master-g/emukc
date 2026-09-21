use std::collections::BTreeMap;

use serde::{
    Deserialize, Serialize,
    de::{Deserializer as _, MapAccess, Visitor},
};

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

/// Key prefix that opens a new quest record.
const ID_PREFIX: &str = "_quest_id_";

/// A lone entry ending in one of these reads as a description, not a title.
const DESC_SUFFIXES: [char; 2] = ['！', '。'];

/// A lone entry longer than this many characters reads as a description too.
const DESC_MAX_TITLE_CHARS: usize = 30;

/// Collects the object's entries in document order.
///
/// `serde` hands a map's entries to `visit_map` in the order they appear, so this
/// needs neither `serde_json`'s `preserve_order` feature (which would reorder every
/// `serde_json::Map` in the workspace) nor a hand-rolled scan that has to redo JSON
/// unescaping.
struct OrderedEntries;

impl<'de> Visitor<'de> for OrderedEntries {
    type Value = Vec<(String, String)>;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("a JSON object mapping Japanese strings to their translations")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut entries = Vec::with_capacity(map.size_hint().unwrap_or_default());
        while let Some(entry) = map.next_entry::<String, String>()? {
            entries.push(entry);
        }
        Ok(entries)
    }
}

/// Parse the quest info from the raw string.
///
/// The file is a JSON object whose keys are the Japanese strings and whose values are
/// the English translations. A `_quest_id_N` key opens a record; the entries after it,
/// up to the next id, are that quest's name and description. Most quests have both,
/// 13 have only one, and one carries a `"dummy": "forNoComma"` sentinel.
///
/// # Arguments
///
/// * `raw` - The raw string to parse.
///
/// # Returns
///
/// A map of quest id to quest info.
pub fn parse(raw: &str) -> Result<BTreeMap<i64, KccpQuestInfo>, ParseError> {
    let entries = serde_json::Deserializer::from_str(raw).deserialize_map(OrderedEntries)?;

    let mut result: BTreeMap<i64, KccpQuestInfo> = BTreeMap::new();
    let mut current: Option<(i64, Vec<String>)> = None;

    for (key, value) in entries {
        if key == "dummy" && value == "forNoComma" {
            continue;
        }

        if let Some(id) = key.strip_prefix(ID_PREFIX).and_then(|n| n.parse::<i64>().ok()) {
            if let Some((prev_id, keys)) = current.replace((id, Vec::new())) {
                insert_quest(&mut result, prev_id, keys);
            }
        } else if let Some((_, keys)) = current.as_mut() {
            keys.push(key);
        }
    }

    if let Some((id, keys)) = current {
        insert_quest(&mut result, id, keys);
    }

    Ok(result)
}

/// Turn one quest's entries into a record. Anything but the plain two-entry shape is
/// warned about, so a change in the upstream file shows up in the log instead of
/// silently reshaping the output.
fn insert_quest(result: &mut BTreeMap<i64, KccpQuestInfo>, id: i64, mut keys: Vec<String>) {
    if keys.is_empty() {
        warn!("kccp quest {id} has no entries of its own, skipping it");
        return;
    }

    if keys.len() > 2 {
        warn!("kccp quest {id} has {} entries, keeping only the first two", keys.len());
        keys.truncate(2);
    }

    let fallback = KccpQuestInfo::default();
    let (name, desc) = if keys.len() == 2 {
        let desc = keys.pop().unwrap();
        (keys.pop().unwrap(), desc)
    } else {
        let only = keys.pop().unwrap();
        if only.ends_with(DESC_SUFFIXES) || only.chars().count() > DESC_MAX_TITLE_CHARS {
            warn!("kccp quest {id} has a single entry, reading it as the description");
            (fallback.name, only)
        } else {
            warn!("kccp quest {id} has a single entry, reading it as the name");
            (only, fallback.desc)
        }
    };

    result.insert(
        id,
        KccpQuestInfo {
            id,
            name,
            desc: desc.replace('\n', "<br>"),
        },
    );
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
    fn a_quest_without_a_name_keeps_its_desc_and_leaves_the_next_quest_alone() {
        let result = parsed();

        let q615 = result.get(&615).unwrap();
        assert_eq!(q615.name, "n/a");
        assert_eq!(
            q615.desc,
            "「九九式艦爆(江草隊)」搭載空母を秘書艦にした状態で新たに「彗星」を2つ廃棄！"
        );

        let q616 = result.get(&616).unwrap();
        assert_eq!(q616.name, "n/a");
        assert_eq!(
            q616.desc,
            "「零戦52型丙(六〇一空)」搭載空母を秘書艦にした状態で新たに「試製烈風 後期型」を2つ廃棄！"
        );
    }

    #[test]
    fn a_quest_without_a_desc_keeps_its_name_and_leaves_the_next_quest_alone() {
        let result = parsed();

        let q1124 = result.get(&1124).unwrap();
        assert_eq!(q1124.name, "【早春限定任務】夜間航空作戦能力の増強");
        assert_eq!(q1124.desc, "n/a");

        let q103 = result.get(&103).unwrap();
        assert_eq!(q103.name, "「水雷戦隊」を編成せよ！");
        assert_eq!(
            q103.desc,
            "軽巡洋艦を旗艦とし、数隻の駆逐艦で構成される「水雷戦隊」を編成せよ！"
        );
    }

    #[test]
    fn the_dummy_sentinel_is_dropped() {
        let result = parsed();

        let q1169 = result.get(&1169).unwrap();
        assert_eq!(q1169.name, "【期間限定任務・拡張任務】端午の節句工廠【II】");
        assert_eq!(
            q1169.desc,
            "第一艦隊の旗艦及び二番艦に「山汐丸改」または「第四号海防艦」「第三〇号海防艦」を配備。<br>練度maxの「九七式艦攻」x5廃棄。工廠資源x2、開発資材x155、燃料と弾薬x各550を準備せよ！"
        );
        assert!(result.values().all(|q| q.name != "dummy" && q.desc != "dummy"));
    }

    #[test]
    fn every_id_in_the_sample_yields_a_record() {
        let result = parsed();

        assert_eq!(
            result.keys().copied().collect::<Vec<_>>(),
            vec![101, 102, 103, 615, 616, 1124, 1169]
        );
        assert!(
            result
                .values()
                .all(|q| !q.name.starts_with(ID_PREFIX) && !q.desc.starts_with(ID_PREFIX)),
            "no field may hold an internal id key"
        );
    }

    #[test]
    fn a_trailing_id_with_no_entries_is_skipped_rather_than_panicking() {
        let raw = r#"{
	"_quest_id_101": "_quest_code_A1",
	"はじめての「編成」！": "The First Attempt at Fleet Organization!",
	"２隻以上の艦で構成される「艦隊」を編成せよ！": "Have 2 ships in your main fleet.",
	"_quest_id_999": "_quest_code_Z9"
}"#;

        let result = parse(raw).unwrap();

        assert!(result.contains_key(&101));
        assert!(!result.contains_key(&999));
    }

    #[test]
    fn grouping_follows_entry_order_not_line_position() {
        // the same three entries, all on one line: a line-driven parser would break here
        let raw = "{\"_quest_id_101\": \"_quest_code_A1\", \
                   \"はじめての「編成」！\": \"The First Attempt at Fleet Organization!\", \
                   \"２隻以上の艦で構成される「艦隊」を編成せよ！\": \"Have 2 ships in your main fleet.\"}";

        let result = parse(raw).unwrap();

        let q101 = result.get(&101).unwrap();
        assert_eq!(q101.name, "はじめての「編成」！");
        assert_eq!(q101.desc, "２隻以上の艦で構成される「艦隊」を編成せよ！");
    }
}
