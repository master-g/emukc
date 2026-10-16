//! Ship remodel (改造) requirements, worked out the way the client works them out.
//!
//! The client (`ShipUpgradeModelHolder.getRequires`) takes the level, ammo and steel from the
//! ship before the remodel (`api_afterlv`, `api_afterbull`, `api_afterfuel`) and the use items
//! from its `api_mst_shipupgrade` row. 開発資材, 高速建造材, 新型砲熕兵装資材, 改修資材 and
//! 工廠資材 are not in start2: the client hard-codes them, and `main-decoder` reads those
//! tables into the `remodel_tables` asset (`make remodel-tables-oracle` checks them against
//! the client).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{Codex, CodexError};

/// A count that applies to the remodel into `after`, or only to the one from `before`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemodelTableEntry {
    /// Ship after the remodel.
    pub after: i64,
    /// Ship before the remodel, when the count applies to one route only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<i64>,
    /// How many.
    pub count: i64,
}

/// 開発資材 by the ship before the remodel.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevkitTable {
    /// Listed ships.
    pub by_ship: BTreeMap<i64, i64>,
    /// Ships outside the table that pay the steel-based amount even when a blueprint is needed.
    pub blueprint_group: Vec<i64>,
    /// The first tier whose bound the steel cost is below gives the count.
    pub steel_tiers: Vec<SteelTier>,
    /// The count when the steel cost is above every tier.
    pub steel_top: i64,
}

/// One step of the steel-based 開発資材 amount.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SteelTier {
    /// Exclusive upper bound of the steel cost.
    pub below: i64,
    /// 開発資材 below it.
    pub count: i64,
}

/// 高速建造材 by the ship before the remodel; 0 outside the table.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildkitTable {
    /// Listed ships.
    pub by_ship: BTreeMap<i64, i64>,
}

/// The remodel numbers the client hard-codes, as `main-decoder` read them.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemodelTables {
    /// 開発資材.
    pub devkit: DevkitTable,
    /// 高速建造材.
    pub buildkit: BuildkitTable,
    /// 新型砲熕兵装資材 (useitem 75).
    pub artillery: Vec<RemodelTableEntry>,
    /// 改修資材 (useitem 4).
    pub screw: Vec<RemodelTableEntry>,
    /// 工廠資材 (useitem 104).
    pub arsenal: Vec<RemodelTableEntry>,
}

impl RemodelTables {
    /// `_getRequiredDevkitNum(before, blueprint, steel)`.
    pub fn devkit(&self, before: i64, blueprint: i64, steel: i64) -> i64 {
        let table = &self.devkit;
        if let Some(count) = table.by_ship.get(&before) {
            return *count;
        }
        if blueprint != 0 && !table.blueprint_group.contains(&before) {
            return 0;
        }
        table
            .steel_tiers
            .iter()
            .find(|tier| steel < tier.below)
            .map_or(table.steel_top, |tier| tier.count)
    }

    /// `_getRequiredBuildKitNum(before)`.
    pub fn buildkit(&self, before: i64) -> i64 {
        self.buildkit.by_ship.get(&before).copied().unwrap_or(0)
    }
}

fn entry_count(entries: &[RemodelTableEntry], before: i64, after: i64) -> i64 {
    entries
        .iter()
        .find(|entry| entry.after == after && entry.before.is_none_or(|only| only == before))
        .map_or(0, |entry| entry.count)
}

/// What a remodel needs, as the client shows and deducts it.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RemodelRequirement {
    /// Ship after the remodel.
    pub after: i64,
    /// Level needed.
    pub level: i64,
    /// 弾薬.
    pub ammo: i64,
    /// 鋼材.
    pub steel: i64,
    /// 開発資材.
    pub devkit: i64,
    /// 高速建造材.
    pub buildkit: i64,
    /// 改修資材.
    pub screw: i64,
    /// 改装設計図 (58).
    pub blueprint: i64,
    /// 試製甲板カタパルト (65).
    pub catapult: i64,
    /// 戦闘詳報 (78).
    pub report: i64,
    /// 新型砲熕兵装資材 (75).
    pub artillery: i64,
    /// 新型航空兵装資材 (77).
    pub aviation: i64,
    /// 新型兵装資材 (94).
    pub armament: i64,
    /// 新型高温高圧缶, as equipment.
    pub boiler: i64,
    /// 海外艦最新技術 (100).
    pub tech: i64,
    /// 工廠資材 (104).
    pub arsenal: i64,
}

impl Codex {
    /// What remodelling `before` needs and what it becomes (`getNextID` and `getRequires`).
    ///
    /// The `api_mst_shipupgrade` row for `before` names the target when there is one, else
    /// `api_aftershipid` does. A ship without an `api_aftershipid` cannot be remodelled.
    pub fn remodel_requirement(&self, before: i64) -> Result<RemodelRequirement, CodexError> {
        let mst = self
            .manifest
            .find_ship(before)
            .ok_or_else(|| CodexError::NotFound(format!("ship manifest ID: {before}")))?;
        let after_ship_id =
            mst.api_aftershipid.as_deref().map(str::parse::<i64>).transpose()?.unwrap_or(0);
        if after_ship_id <= 0 {
            return Err(CodexError::NotFound(format!("remodel of ship {before}")));
        }
        // The client keeps the first row for a ship.
        let row =
            self.manifest.api_mst_shipupgrade.iter().find(|row| row.api_current_ship_id == before);
        let after = row.map_or(after_ship_id, |row| row.api_id);
        let steel = mst.api_afterfuel.unwrap_or(0);
        let blueprint = row.map_or(0, |row| row.api_drawing_count);
        let tables = &self.remodel_tables;
        let mut requirement = RemodelRequirement {
            after,
            level: mst.api_afterlv.unwrap_or(0),
            ammo: mst.api_afterbull.unwrap_or(0),
            steel,
            devkit: tables.devkit(before, blueprint, steel),
            buildkit: tables.buildkit(before),
            ..Default::default()
        };
        if let Some(row) = row {
            requirement.screw = entry_count(&tables.screw, before, after);
            requirement.blueprint = blueprint;
            requirement.catapult = row.api_catapult_count;
            requirement.report = row.api_report_count;
            requirement.artillery = entry_count(&tables.artillery, before, after);
            requirement.aviation = row.api_aviation_mat_count;
            requirement.armament = row.api_arms_mat_count;
            requirement.boiler = row.api_boiler_count.unwrap_or(0);
            requirement.tech = row.api_tech_count.unwrap_or(0);
            requirement.arsenal = entry_count(&tables.arsenal, before, after);
        }
        Ok(requirement)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use serde_json::Value;

    use super::*;

    fn repo(path: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").join(path)
    }

    fn read_json(path: &str) -> Value {
        serde_json::from_str(&fs::read_to_string(repo(path)).unwrap()).unwrap()
    }

    fn int(value: &Value, field: &str) -> i64 {
        match &value[field] {
            Value::String(text) => text.parse().unwrap_or(0),
            other => other.as_i64().unwrap_or(0),
        }
    }

    /// `getRequires` and `getNextID` of the client's `ShipUpgradeModelHolder`, written against
    /// the raw kcwiki start2 and the synced asset rather than the codex.
    fn client_requirement(
        start2: &Value,
        tables: &Value,
        before: i64,
    ) -> Option<RemodelRequirement> {
        let ship = start2["api_mst_ship"]
            .as_array()
            .unwrap()
            .iter()
            .find(|ship| int(ship, "api_id") == before)?;
        let next = int(ship, "api_aftershipid");
        if next <= 0 {
            return None;
        }
        let row = start2["api_mst_shipupgrade"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| int(row, "api_current_ship_id") > 0)
            .find(|row| int(row, "api_current_ship_id") == before);
        let after = row.map_or(next, |row| int(row, "api_id"));
        let steel = int(ship, "api_afterfuel");
        let blueprint = row.map_or(0, |row| int(row, "api_drawing_count"));

        let devkit = &tables["devkit"];
        let devkit = match devkit["byShip"].get(before.to_string()) {
            Some(count) => count.as_i64().unwrap(),
            None if blueprint != 0
                && !devkit["blueprintGroup"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|ship| ship.as_i64() == Some(before)) =>
            {
                0
            }
            None => devkit["steelTiers"]
                .as_array()
                .unwrap()
                .iter()
                .find(|tier| steel < int(tier, "below"))
                .map_or(int(devkit, "steelTop"), |tier| int(tier, "count")),
        };
        let buildkit =
            tables["buildkit"]["byShip"].get(before.to_string()).map_or(0, |n| n.as_i64().unwrap());
        let by_after = |name: &str| {
            tables[name]
                .as_array()
                .unwrap()
                .iter()
                .find(|entry| {
                    int(entry, "after") == after
                        && entry.get("before").is_none_or(|only| only.as_i64() == Some(before))
                })
                .map_or(0, |entry| int(entry, "count"))
        };
        let from_row = |field: &str| row.map_or(0, |row| int(row, field));
        let row_table = |name: &str| {
            if row.is_some() {
                by_after(name)
            } else {
                0
            }
        };

        Some(RemodelRequirement {
            after,
            level: int(ship, "api_afterlv"),
            ammo: int(ship, "api_afterbull"),
            steel,
            devkit,
            buildkit,
            screw: row_table("screw"),
            blueprint,
            catapult: from_row("api_catapult_count"),
            report: from_row("api_report_count"),
            artillery: row_table("artillery"),
            aviation: from_row("api_aviation_mat_count"),
            armament: from_row("api_arms_mat_count"),
            boiler: from_row("api_boiler_count"),
            tech: from_row("api_tech_count"),
            arsenal: row_table("arsenal"),
        })
    }

    /// Every ship's remodel, which covers every `api_mst_shipupgrade` row and every
    /// `api_aftershipid`, is what the client works out. No exceptions.
    #[test]
    fn every_remodel_needs_what_the_client_says() {
        let start2 = read_json(".data/temp/start2.json");
        let start2 = start2.get("api_data").unwrap_or(&start2);
        let tables = read_json("crates/emukc_bootstrap/assets/remodel_tables.json");
        let codex = Codex::load_without_cache_source(repo(".data/codex")).unwrap();

        let ships = start2["api_mst_ship"].as_array().unwrap();
        let mut routes = 0;
        for ship in ships {
            let before = int(ship, "api_id");
            let want = client_requirement(start2, &tables, before);
            let got = codex.remodel_requirement(before).ok();
            assert_eq!(got, want, "ship {before}");
            routes += usize::from(want.is_some());
        }
        let rows = start2["api_mst_shipupgrade"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| int(row, "api_current_ship_id") > 0)
            .count();
        assert!(routes >= rows, "{routes} routes cover fewer than the {rows} shipupgrade rows");
    }
}
