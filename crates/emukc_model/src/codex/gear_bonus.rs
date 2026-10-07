//! Visible equipment bonuses (装備ボーナス): stats a ship gains for carrying particular
//! equipment, on top of the equipment's own stats.
//!
//! The table is converted from `KC3Kai`'s `GearBonus.js` by `main-decoder` and
//! [`GearBonusTable::bonus`] is a port of its reader,
//! `KC3Gear.equipmentTotalStatsOnShipBonus`. Where that reader is quirky the port follows it
//! rather than the apparent intent, so that table and reader stay a matched pair. The one
//! exceptions are about improvement stars: the reader skips star thresholds for entries that
//! do not declare a star record, cannot read the stars of equipment without an entry of its
//! own, and ignores the `isMultiple` it is given. `make gear-bonus-oracle`, which compares
//! the result with the game client's own bonus code, shows the client does none of that, so
//! stars are always read from what is carried and `isMultiple` is honoured.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

fn is_zero(value: &i64) -> bool {
    *value == 0
}

/// A set of stat changes, named as in `api_mst_slotitem`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[expect(missing_docs)]
pub struct GearBonusStats {
    #[serde(skip_serializing_if = "is_zero")]
    pub houg: i64,
    #[serde(skip_serializing_if = "is_zero")]
    pub raig: i64,
    #[serde(skip_serializing_if = "is_zero")]
    pub tyku: i64,
    #[serde(skip_serializing_if = "is_zero")]
    pub souk: i64,
    #[serde(skip_serializing_if = "is_zero")]
    pub houk: i64,
    #[serde(skip_serializing_if = "is_zero")]
    pub tais: i64,
    #[serde(skip_serializing_if = "is_zero")]
    pub saku: i64,
    #[serde(skip_serializing_if = "is_zero")]
    pub houm: i64,
    #[serde(skip_serializing_if = "is_zero")]
    pub leng: i64,
    #[serde(skip_serializing_if = "is_zero")]
    pub soku: i64,
    #[serde(skip_serializing_if = "is_zero")]
    pub baku: i64,
}

impl GearBonusStats {
    fn add(&mut self, other: Option<&Self>, times: i64) {
        let Some(other) = other else {
            return;
        };
        self.houg += other.houg * times;
        self.raig += other.raig * times;
        self.tyku += other.tyku * times;
        self.souk += other.souk * times;
        self.houk += other.houk * times;
        self.tais += other.tais * times;
        self.saku += other.saku * times;
        self.houm += other.houm * times;
        self.leng += other.leng * times;
        self.soku += other.soku * times;
        self.baku += other.baku * times;
    }
}

/// Where the table was converted from.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[expect(missing_docs)]
pub struct GearBonusSource {
    pub repo: String,
    pub commit: String,
    pub files: Vec<String>,
}

/// The whole bonus table.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct GearBonusTable {
    /// Where the table was converted from.
    pub source: GearBonusSource,
    /// Nation name to ship classes. A class in none of the lists counts as `Japan`.
    pub nations: BTreeMap<String, Vec<i64>>,
    /// Counter name to the equipment ids that raise it.
    pub synergy_gears: BTreeMap<String, Vec<i64>>,
    /// In the order the reader visits them; some qualifiers only grant on first visit.
    pub gears: Vec<GearBonusEntry>,
}

/// The rules of one equipment, or of every equipment of one type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GearBonusEntry {
    /// An equipment id, or `t2_<n>` / `t3_<n>` for `api_type[2]` / `api_type[3]`. Several
    /// ids joined by `+` make one entry counting the copies of all of them together.
    pub key: String,
    /// Scoped rules first (by class, then by nation), unscoped ones last.
    pub rules: Vec<GearBonusRule>,
}

/// One condition and what it grants.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
#[expect(missing_docs)]
pub struct GearBonusRule {
    /// The ship class the rule is listed under.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<i64>,
    /// The nation the rule is listed under.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ids: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excludes: Option<Vec<i64>>,
    /// First forms of the remodel chains the rule covers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origins: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classes: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude_classes: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stypes: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude_stypes: Option<Vec<i64>>,
    /// Equipment ids sharing this grant: only the first rule naming the same list applies.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distinct_gears: Option<Vec<i64>>,
    /// Lowest position in the remodel chain, the first form being 0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remodel: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remodel_cap: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_stars: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count_cap: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_cap: Option<i64>,
    /// Granted once.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub single: Option<GearBonusStats>,
    /// Granted per copy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiple: Option<GearBonusStats>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub synergy: Vec<GearBonusSynergy>,
}

/// An extra grant for carrying other equipment alongside.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
#[expect(missing_docs)]
pub struct GearBonusSynergy {
    /// Counters that all have to be above zero. `<name>Nonexist` is above zero when
    /// `<name>` is zero.
    pub flags: Vec<String>,
    /// Other equipment that has to be carried as well; every item has to be met.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<GearBonusRequirement>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub single: Option<GearBonusStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiple: Option<GearBonusStats>,
    /// Index into `flags` of the counter `multiple` scales with; the equipment's own count
    /// otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count_flag: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count_cap: Option<i64>,
    /// Granted once however many rules name the same flags.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distinct: Option<GearBonusStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by_count: Option<GearBonusByCount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by_stars: Option<GearBonusByStars>,
}

/// "At least `min_count` of these, each with `min_stars` or more." The source cannot say
/// this; corrections use it where the client asks for it.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct GearBonusRequirement {
    /// Equipment ids, counted together.
    pub gears: Vec<i64>,
    /// Stars a copy needs to count.
    #[serde(skip_serializing_if = "is_zero")]
    pub min_stars: i64,
    /// Copies needed; one when absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_count: Option<i64>,
}

/// A grant looked up by how many copies of something are carried.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GearBonusByCount {
    /// A counter name, or `this` for the equipment's own count.
    pub gear: String,
    /// Granted once however many rules name the same flags.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub distinct: bool,
    /// Exact count to grant.
    pub table: BTreeMap<i64, GearBonusStats>,
}

/// A grant depending on the stars of another equipment.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct GearBonusByStars {
    /// The entry whose star record is read.
    pub gear_id: String,
    /// No grant when a copy with fewer stars is carried.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_stars_less_than: Option<usize>,
    /// Grant each row once per copy reaching it instead of once.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_multiple: bool,
    /// Every row whose star threshold some copy reaches is granted.
    pub table: Vec<GearBonusStarRow>,
}

/// One row of [`GearBonusByStars`].
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[expect(missing_docs)]
pub struct GearBonusStarRow {
    pub min_stars: usize,
    pub stats: GearBonusStats,
}

/// The ship a bonus is worked out for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GearBonusShip {
    /// `api_mst_ship` id.
    pub id: i64,
    /// `api_ctype`.
    pub class: i64,
    /// `api_stype`.
    pub stype: i64,
    /// Every form of the ship from the first one on, in remodel order.
    pub remodel_chain: Vec<i64>,
}

/// One carried equipment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct GearBonusGear {
    /// `api_mst_slotitem` id.
    pub id: i64,
    /// `api_type[2]`.
    pub type2: i64,
    /// `api_type[3]`.
    pub type3: i64,
    /// Improvement level.
    pub stars: i64,
    /// `api_saku`.
    pub saku: i64,
    /// `api_tyku`.
    pub tyku: i64,
    /// `api_houm`.
    pub houm: i64,
}

/// Copies of the equipment one entry covers, by stars.
#[derive(Default, Clone)]
struct Carried {
    count: i64,
    stars: [i64; 11],
}

impl Carried {
    fn with_stars_from(&self, min: usize) -> i64 {
        self.stars.iter().skip(min).sum()
    }
}

fn truthy(value: Option<i64>) -> Option<i64> {
    value.filter(|value| *value != 0)
}

impl GearBonusTable {
    /// Whether any carried equipment has an entry, which is what makes [`Self::bonus`]
    /// worth calling.
    pub fn covers(&self, gears: &[GearBonusGear]) -> bool {
        gears
            .iter()
            .map(GearBonusGear::keys)
            .any(|keys| self.gears.iter().any(|entry| entry.covers(&keys)))
    }

    /// The stats `ship` gains for carrying `gears`.
    pub fn bonus(&self, ship: &GearBonusShip, gears: &[GearBonusGear]) -> GearBonusStats {
        // Five counters go by master data rather than by their id lists, as in the source:
        // the lists are the same thing precomputed and miss equipment whose stats changed.
        let counters: HashMap<&str, i64> = self
            .synergy_gears
            .iter()
            .map(|(name, ids)| {
                let counts = |gear: &&GearBonusGear| {
                    let radar = matches!(gear.type2, 12 | 13);
                    match name.as_str() {
                        "surfaceRadar" => radar && gear.saku >= 5,
                        "airRadar" => radar && gear.tyku >= 2,
                        "highAccuracyRadar" => radar && gear.houm >= 8,
                        "rotorcraft" => gear.type2 == 25,
                        "aaMachineGun" => gear.type2 == 21,
                        _ => ids.contains(&gear.id),
                    }
                };
                (name.as_str(), gears.iter().filter(counts).count() as i64)
            })
            .collect();
        // How often a grant shared between rules has come up.
        let mut seen: HashMap<String, i64> = HashMap::new();
        let keys: Vec<[String; 3]> = gears.iter().map(GearBonusGear::keys).collect();
        let carried: Vec<Carried> = self
            .gears
            .iter()
            .map(|entry| {
                let mut carried = Carried::default();
                for (gear, keys) in gears.iter().zip(&keys) {
                    if entry.covers(keys) {
                        carried.count += 1;
                        carried.stars[gear.stars.clamp(0, 10) as usize] += 1;
                    }
                }
                carried
            })
            .collect();
        let carried_of = |key: &str| {
            self.gears.iter().position(|entry| entry.key == key).map(|index| &carried[index])
        };

        let nation = self
            .nations
            .iter()
            .find(|(_, classes)| classes.contains(&ship.class))
            .map_or("Japan", |(name, _)| name.as_str());
        let origin = ship.remodel_chain.first().copied();
        let remodel_index = ship.remodel_chain.iter().position(|id| *id == ship.id).unwrap_or(0);

        let mut total = GearBonusStats::default();
        for (entry, own) in self.gears.iter().zip(&carried).filter(|(_, own)| own.count > 0) {
            for rule in &entry.rules {
                if rule.class.is_some_and(|class| class != ship.class)
                    || rule.nation.as_deref().is_some_and(|name| name != nation)
                {
                    continue;
                }
                let within = |list: &Option<Vec<i64>>, value: i64| {
                    list.as_ref().is_none_or(|list| list.contains(&value))
                };
                let outside = |list: &Option<Vec<i64>>, value: i64| {
                    list.as_ref().is_none_or(|list| !list.contains(&value))
                };
                if !(within(&rule.ids, ship.id)
                    && outside(&rule.excludes, ship.id)
                    && rule
                        .origins
                        .as_ref()
                        .is_none_or(|list| origin.is_some_and(|id| list.contains(&id)))
                    && within(&rule.classes, ship.class)
                    && outside(&rule.exclude_classes, ship.class)
                    && within(&rule.stypes, ship.stype)
                    && outside(&rule.exclude_stypes, ship.stype))
                {
                    continue;
                }
                if let Some(shared) = &rule.distinct_gears {
                    let key = format!("countOnceIds{}", join(shared));
                    let times = seen.entry(key).or_default();
                    *times += 1;
                    if *times > 1 {
                        continue;
                    }
                }
                if truthy(rule.remodel).is_some() || truthy(rule.remodel_cap).is_some() {
                    let index = remodel_index as i64;
                    if rule.remodel.is_some_and(|min| index < min)
                        || rule.remodel_cap.is_some_and(|max| index > max)
                    {
                        continue;
                    }
                }
                let mut count = own.count;
                if let Some(min) = truthy(rule.min_stars) {
                    count = own.with_stars_from(min as usize);
                    if count == 0 {
                        continue;
                    }
                }
                if let Some(min) = truthy(rule.min_count) {
                    let required = match &rule.distinct_gears {
                        // The source's sum starts over at an id without an entry.
                        Some(shared) => shared.iter().fold(0, |sum, id| {
                            carried_of(&id.to_string()).map_or(0, |other| sum + other.count)
                        }),
                        None => count,
                    };
                    if required < min {
                        continue;
                    }
                }

                let capped = |cap: Option<i64>, amount: i64| match truthy(cap) {
                    Some(cap) => cap.min(amount),
                    None => amount,
                };
                total.add(rule.single.as_ref(), 1);
                total.add(rule.multiple.as_ref(), capped(rule.count_cap, count));
                for synergy in &rule.synergy {
                    let flag = |name: &str| match name.strip_suffix("Nonexist") {
                        Some(base) => i64::from(counters.get(base).copied().unwrap_or(0) == 0),
                        None => counters.get(name).copied().unwrap_or(0),
                    };
                    let met = |requirement: &GearBonusRequirement| {
                        let copies = gears
                            .iter()
                            .filter(|gear| requirement.gears.contains(&gear.id))
                            .filter(|gear| gear.stars >= requirement.min_stars)
                            .count() as i64;
                        copies >= requirement.min_count.unwrap_or(1)
                    };
                    if !synergy.flags.iter().all(|name| flag(name) > 0)
                        || !synergy.requires.iter().all(met)
                    {
                        continue;
                    }
                    total.add(synergy.single.as_ref(), 1);
                    let scale = match synergy.count_flag {
                        Some(index) => synergy.flags.get(index).map_or(0, |name| flag(name)),
                        None => count,
                    };
                    total.add(synergy.multiple.as_ref(), capped(synergy.count_cap, scale));
                    let by_count = synergy.by_count.as_ref().map(|by_count| {
                        let amount = if by_count.gear == "this" {
                            count
                        } else {
                            flag(&by_count.gear)
                        };
                        (by_count, by_count.table.get(&amount))
                    });

                    let applied = format!("{}Applied", synergy.flags.join("_"));
                    let mut first_time = || {
                        let times = seen.entry(applied.clone()).or_default();
                        *times += 1;
                        *times < 2
                    };
                    if synergy.distinct.is_some() && first_time() {
                        total.add(synergy.distinct.as_ref(), 1);
                    }
                    if let Some((by_count, row)) = by_count
                        && (!by_count.distinct || first_time())
                    {
                        total.add(row, 1);
                    }
                    if let Some(by_stars) = &synergy.by_stars {
                        // The source reads the star record of the other equipment's entry
                        // and so finds nothing when there is no such entry, and it never
                        // reads `is_multiple`. The client does neither; see the module doc.
                        let mut other = Carried::default();
                        for gear in gears {
                            if gear.id.to_string() == by_stars.gear_id {
                                other.count += 1;
                                other.stars[gear.stars.clamp(0, 10) as usize] += 1;
                            }
                        }
                        let lower = by_stars.no_stars_less_than.unwrap_or(0);
                        if other.stars.iter().take(lower).sum::<i64>() == 0 {
                            for row in &by_stars.table {
                                let copies = other.with_stars_from(row.min_stars);
                                let times = if by_stars.is_multiple {
                                    copies
                                } else {
                                    copies.min(1)
                                };
                                total.add(Some(&row.stats), times);
                            }
                        }
                    }
                }
                if let Some(cap) = truthy(rule.speed_cap) {
                    total.soku = total.soku.min(cap);
                }
            }
        }
        total
    }
}

impl GearBonusEntry {
    fn covers(&self, gear_keys: &[String; 3]) -> bool {
        self.key.split('+').any(|key| gear_keys.iter().any(|gear_key| gear_key == key))
    }
}

impl GearBonusGear {
    /// The entry keys that cover this equipment.
    fn keys(&self) -> [String; 3] {
        [self.id.to_string(), format!("t2_{}", self.type2), format!("t3_{}", self.type3)]
    }
}

fn join(ids: &[i64]) -> String {
    ids.iter().map(ToString::to_string).collect::<Vec<_>>().join("_")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(gears: serde_json::Value) -> GearBonusTable {
        serde_json::from_value(serde_json::json!({
            "nations": { "Sweden": [89] },
            "synergyGears": { "lookouts": [28], "turbine": [33] },
            "gears": gears,
        }))
        .unwrap()
    }

    fn ship(id: i64, class: i64, chain: &[i64]) -> GearBonusShip {
        GearBonusShip {
            id,
            class,
            stype: 2,
            remodel_chain: chain.to_vec(),
        }
    }

    fn gear(id: i64, stars: i64) -> GearBonusGear {
        GearBonusGear {
            id,
            type2: 10,
            type3: 10,
            stars,
            saku: 0,
            tyku: 0,
            houm: 0,
        }
    }

    #[test]
    fn per_copy_and_once_only_grants_add_up_for_the_right_remodel() {
        let table = table(serde_json::json!([{ "key": "371", "rules": [
            { "class": 89, "multiple": { "houg": 4, "saku": 6 } },
            { "class": 89, "remodel": 2, "single": { "houg": 2, "saku": 3 } },
            { "nation": "Sweden", "single": { "houk": 1 } },
            { "nation": "Japan", "single": { "tais": 9 } },
        ] }]));
        let twice = [gear(371, 0), gear(371, 0)];

        let base = table.bonus(&ship(574, 89, &[574, 579, 630]), &twice);
        assert_eq!((base.houg, base.saku, base.houk, base.tais), (8, 12, 1, 0));
        let second_remodel = table.bonus(&ship(630, 89, &[574, 579, 630]), &twice);
        assert_eq!((second_remodel.houg, second_remodel.saku), (10, 15));
        // A class no nation lists is Japanese.
        assert_eq!(table.bonus(&ship(1, 1, &[1]), &twice).tais, 9);
        assert_eq!(table.bonus(&ship(1, 1, &[1]), &[gear(5, 0)]), GearBonusStats::default());
    }

    #[test]
    fn a_star_threshold_counts_only_the_copies_reaching_it() {
        let table = table(serde_json::json!([{ "key": "1", "rules": [
            { "minStars": 7, "multiple": { "tyku": 1 } },
        ] }]));
        let carried = [gear(1, 10), gear(1, 3)];
        let any = ship(1, 1, &[1]);

        assert_eq!(table.bonus(&any, &carried).tyku, 1);
        assert_eq!(table.bonus(&any, &carried[1..]).tyku, 0);
    }

    #[test]
    fn a_joined_entry_counts_the_copies_of_all_its_equipment() {
        let table = table(serde_json::json!([{ "key": "1+2", "rules": [
            { "countCap": 2, "multiple": { "raig": 2 } },
            { "minStars": 10, "multiple": { "houg": 1 } },
        ] }]));
        let any = ship(1, 1, &[1]);

        let mixed = table.bonus(&any, &[gear(1, 10), gear(2, 10), gear(2, 0)]);
        assert_eq!((mixed.raig, mixed.houg), (4, 2));
        assert_eq!(table.bonus(&any, &[gear(2, 0)]).raig, 2);
        assert!(!table.covers(&[gear(3, 0)]));
    }

    #[test]
    fn a_requirement_looks_at_the_stars_of_other_equipment() {
        let table = table(serde_json::json!([{ "key": "1", "rules": [{ "synergy": [{
            "requires": [{ "gears": [2, 3], "minStars": 3 }, { "gears": [4], "minCount": 2 }],
            "single": { "tyku": 4 },
        }] }] }]));
        let any = ship(1, 1, &[1]);

        assert_eq!(table.bonus(&any, &[gear(1, 0), gear(3, 3), gear(4, 0), gear(4, 0)]).tyku, 4);
        assert_eq!(table.bonus(&any, &[gear(1, 0), gear(3, 2), gear(4, 0), gear(4, 0)]).tyku, 0);
        assert_eq!(table.bonus(&any, &[gear(1, 0), gear(2, 9), gear(4, 0)]).tyku, 0);
    }

    #[test]
    fn shared_and_synergy_grants_apply_once() {
        let shared = serde_json::json!({ "distinctGears": [1, 2], "single": { "raig": 3 } });
        let synergy = serde_json::json!({ "synergy": [
            { "flags": ["lookouts"], "distinct": { "houk": 2 }, "single": { "houg": 1 } },
            { "flags": ["turbineNonexist"], "single": { "souk": 5 } },
        ] });
        let table = table(serde_json::json!([
            { "key": "1", "rules": [shared, synergy] },
            { "key": "2", "rules": [shared, synergy] },
            { "key": "t2_10", "rules": [{ "minCount": 3, "multiple": { "saku": 1 } }] },
        ]));
        let any = ship(1, 1, &[1]);

        let alone = table.bonus(&any, &[gear(1, 0), gear(2, 0)]);
        assert_eq!((alone.raig, alone.houg, alone.houk, alone.souk, alone.saku), (3, 0, 0, 10, 0));
        let with_radar = table.bonus(&any, &[gear(1, 0), gear(2, 0), gear(28, 0), gear(33, 0)]);
        assert_eq!(
            (with_radar.houg, with_radar.houk, with_radar.souk, with_radar.saku),
            (2, 2, 0, 4)
        );
    }
}
