use super::split_map_id;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::{Deserialize, Deserializer as SerdeDeserializer, Serialize, Serializer};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, enumn::N)]
pub enum MapResetPolicy {
    #[default]
    Never = 0,
    Monthly = 1,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteOperator {
    #[default]
    Eq,
    Gte,
    Lte,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpeedClass {
    Slow,
    #[default]
    Fast,
    FastPlus,
    Fastest,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MapCatalog {
    pub maps: BTreeMap<i64, MapDefinition>,
    /// Map ID → prerequisite map ID that must be cleared.
    /// Maps not in this table have no prerequisite (always unlocked, e.g., 1-1).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub prerequisites: HashMap<i64, i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MapDefinition {
    pub map_id: i64,
    pub maparea_id: i64,
    pub mapinfo_no: i64,
    pub name: String,
    pub level: i64,
    pub sally_flag: Vec<i64>,
    pub is_event: bool,
    pub reset_policy: MapResetPolicy,
    pub airbase_count: Option<i64>,
    pub gauge_type: Option<i64>,
    pub gauge_count: Option<i64>,
    pub required_defeat_count: Option<i64>,
    pub max_hp: Option<i64>,
    pub gauge_type_e: Option<i64>,
    pub default_variant: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub rank_stage_ids: BTreeMap<i64, String>,
    pub variants: BTreeMap<String, MapVariantDefinition>,
}

impl MapDefinition {
    /// Create a minimal [`MapDefinition`] with all fields set to defaults/empty.
    ///
    /// `maparea_id` and `mapinfo_no` are derived from `map_id` via integer division.
    pub fn minimal(map_id: i64) -> Self {
        let (maparea_id, mapinfo_no) = split_map_id(map_id);
        Self {
            map_id,
            maparea_id,
            mapinfo_no,
            name: String::new(),
            level: 0,
            sally_flag: Vec::new(),
            is_event: false,
            reset_policy: MapResetPolicy::default(),
            airbase_count: None,
            gauge_type: None,
            gauge_count: None,
            required_defeat_count: None,
            max_hp: None,
            gauge_type_e: None,
            default_variant: String::new(),
            rank_stage_ids: BTreeMap::new(),
            variants: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MapVariantDefinition {
    pub variant_key: String,
    pub boss_cell_no: i64,
    pub cells: Vec<MapCellDefinition>,
    #[serde(default)]
    pub routing_rules: BTreeMap<i64, Vec<RouteRule>>,
    /// Rules choosing among several start cells; `to_cell_no` is a start cell and
    /// `from_cell_no` is unused. Empty when the map has one start or nothing decides it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub start_rules: Vec<RouteRule>,
    pub enemy_fleets: BTreeMap<i64, EnemyFleetDefinition>,
    /// The fleets that raid the air base while this map is sortied. Empty for a map without
    /// raids.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub air_raid_fleets: Vec<EnemyComposition>,
    /// The fleets that raid it instead once one more sinking of the boss would break the gauge.
    /// Empty when the last bar brings the same raids as the rest.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub last_bar_air_raid_fleets: Vec<EnemyComposition>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub ship_drops: BTreeMap<i64, Vec<ShipDropDefinition>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_defeat_count: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clear_to_variant_key: Option<String>,
    /// The gauge is a transport gauge: `required_defeat_count` is its length in transport
    /// points, and a boss win of rank A or better takes what the fleet carried past the
    /// landing cell. Otherwise only a sunk boss flagship moves the gauge, one step a time.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub transport_gauge: bool,
    /// Cells whose arrival moves the map on to `clear_to_variant_key`. A stage with any has
    /// no gauge of its own: it is a route to open, not a boss to sink.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub advance_on_reach: Vec<i64>,
    /// Cells one of which must have been won with an S rank before the emptied gauge moves
    /// the map on to `clear_to_variant_key`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub advance_needs_s_rank_at: Vec<i64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parse_warnings: Vec<String>,
}

impl MapVariantDefinition {
    /// Returns a multi-valued index from `node_label` to all matching `cell_no`s.
    ///
    /// Unlike [`label_to_cell_no`](Self::label_to_cell_no), duplicate labels are preserved
    /// so each label maps to a `Vec` of cell numbers.
    pub fn multi_label_index(&self) -> BTreeMap<String, Vec<i64>> {
        let mut index = BTreeMap::<String, Vec<i64>>::new();
        for cell in &self.cells {
            let Some(label) = cell.node_label.as_ref().filter(|label| !label.is_empty()) else {
                continue;
            };
            index.entry(label.clone()).or_default().push(cell.cell_no);
        }
        index
    }

    /// Returns all cell numbers that share the boss node's `node_label`.
    ///
    /// When a boss node is reachable via multiple incoming routes, each route produces
    /// a separate cell (all carrying the same `node_label`). This helper resolves the
    /// full set so callers can do a label-aware membership test ("is this cell a boss
    /// cell?") instead of an exact scalar match against `boss_cell_no`.
    ///
    /// If the boss cell has no `node_label` (synthetic/skeleton variants), falls back to
    /// `[boss_cell_no]`, preserving behavior for single-incoming-route maps.
    pub fn boss_cell_nos(&self) -> Vec<i64> {
        let boss_label = self
            .cells
            .iter()
            .find(|c| c.cell_no == self.boss_cell_no)
            .and_then(|c| c.node_label.clone());
        let mut cells = match boss_label.filter(|l| !l.is_empty()) {
            Some(label) => self
                .multi_label_index()
                .get(&label)
                .cloned()
                .unwrap_or_else(|| vec![self.boss_cell_no]),
            None => vec![self.boss_cell_no],
        };
        cells.sort();
        cells
    }

    /// Returns a map from `node_label` to `cell_no` for all uniquely-labeled cells.
    ///
    /// If a label appears on two or more cells with different `cell_nos`, that label is excluded.
    pub fn label_to_cell_no(&self) -> BTreeMap<String, i64> {
        let mut labels = BTreeMap::new();
        let mut duplicates = BTreeSet::new();

        for cell in &self.cells {
            let Some(label) = cell.node_label.as_ref().filter(|l| !l.is_empty()) else {
                continue;
            };
            if duplicates.contains(label) {
                continue;
            }
            if let Some(previous) = labels.insert(label.clone(), cell.cell_no)
                && previous != cell.cell_no
            {
                labels.remove(label);
                duplicates.insert(label.clone());
            }
        }

        labels
    }
}

pub type MapStageDefinition = MapVariantDefinition;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MapCellDefinition {
    pub cell_no: i64,
    pub color_no: i64,
    pub event_id: i64,
    pub event_kind: i64,
    pub next_cells: Vec<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub master_cell_id: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distance: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnemyFleetDefinition {
    pub cell_no: i64,
    pub battle_kind: i64,
    pub formations: Vec<i64>,
    pub compositions: Vec<EnemyComposition>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnemyComposition {
    pub comp_id: String,
    pub weight: i64,
    pub ship_ids: Vec<i64>,
    #[serde(default)]
    pub formation: Option<i64>,
    /// Level of each ship, in `ship_ids` order. Empty when the source has none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub levels: Vec<i64>,
    /// The escort fleet of an enemy combined fleet, flagship first. Empty for a single fleet.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub escort_ship_ids: Vec<i64>,
    /// Level of each escort ship, in `escort_ship_ids` order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub escort_levels: Vec<i64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShipDropDefinition {
    /// Master id of the ship, or `0` for the outcome "nothing drops".
    pub ship_id: i64,
    pub raw_ship_name: String,
    pub tags: Vec<String>,
    /// Times this outcome was observed; the chance of it is its share of the cell's total.
    /// `0` is an entry without a count and weighs as `1`.
    pub weight: i64,
    /// The win ranks this outcome was seen at, out of `S`, `A`, `B`. Empty means any.
    pub ranks: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum CompactShipDropDefinition {
    ShipId(i64),
    Detailed {
        ship_id: i64,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        tags: Vec<String>,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        raw_ship_name: String,
        #[serde(default, skip_serializing_if = "is_zero")]
        weight: i64,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        ranks: String,
    },
}

#[expect(clippy::trivially_copy_pass_by_ref)]
fn is_zero(value: &i64) -> bool {
    *value == 0
}

impl Serialize for ShipDropDefinition {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if *self
            == (Self {
                ship_id: self.ship_id,
                ..Default::default()
            })
        {
            CompactShipDropDefinition::ShipId(self.ship_id).serialize(serializer)
        } else {
            CompactShipDropDefinition::Detailed {
                ship_id: self.ship_id,
                tags: self.tags.clone(),
                raw_ship_name: self.raw_ship_name.clone(),
                weight: self.weight,
                ranks: self.ranks.clone(),
            }
            .serialize(serializer)
        }
    }
}

impl<'de> Deserialize<'de> for ShipDropDefinition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: SerdeDeserializer<'de>,
    {
        let compact = CompactShipDropDefinition::deserialize(deserializer)?;
        Ok(match compact {
            CompactShipDropDefinition::ShipId(ship_id) => Self {
                ship_id,
                ..Default::default()
            },
            CompactShipDropDefinition::Detailed {
                ship_id,
                tags,
                raw_ship_name,
                weight,
                ranks,
            } => Self {
                ship_id,
                raw_ship_name,
                tags,
                weight,
                ranks,
            },
        })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RouteRule {
    pub from_cell_no: i64,
    pub to_cell_no: i64,
    pub priority: i64,
    #[serde(default)]
    pub weight: Option<i64>,
    #[serde(default)]
    pub probability_pct: Option<f64>,
    pub predicate: RoutePredicate,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub raw_text: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum RoutePredicate {
    #[default]
    Always,
    VisitedNode {
        cell_nos: Vec<i64>,
        visited: bool,
    },
    VisitedNodeLabel {
        node_labels: Vec<String>,
        visited: bool,
    },
    FleetSize {
        op: RouteOperator,
        value: i64,
    },
    /// Number of ships carrying at least one equipment of the given slotitem type(s).
    /// This counts ships (not individual equipment items).
    EquipmentCount {
        slotitem_types: Vec<i64>,
        op: RouteOperator,
        value: i64,
    },
    ShipTypeCount {
        ship_types: Vec<i64>,
        op: RouteOperator,
        value: i64,
    },
    FlagshipShipType {
        ship_types: Vec<i64>,
    },
    FlagshipShipId {
        ship_ids: Vec<i64>,
    },
    ContainsShipType {
        ship_types: Vec<i64>,
    },
    ContainsShipId {
        ship_ids: Vec<i64>,
    },
    ContainsShipSet {
        ship_types: Vec<i64>,
        ship_ids: Vec<i64>,
    },
    OnlyShipTypes {
        ship_types: Vec<i64>,
    },
    OnlyShipSet {
        ship_types: Vec<i64>,
        ship_ids: Vec<i64>,
    },
    ShipSetCount {
        ship_types: Vec<i64>,
        ship_ids: Vec<i64>,
        op: RouteOperator,
        value: i64,
    },
    ShipSetSpeedCount {
        ship_types: Vec<i64>,
        ship_ids: Vec<i64>,
        speed_op: RouteOperator,
        speed_class: SpeedClass,
        op: RouteOperator,
        value: i64,
    },
    Speed {
        class: SpeedClass,
    },
    LoS {
        formula: Option<String>,
        /// The branch-point coefficient (分岐点係数) the formula-33 score is taken with.
        /// `None` on rules whose source never stated one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        coefficient: Option<i64>,
        op: RouteOperator,
        value: i64,
    },
    /// Compares a weighted sum of fleet counters with a constant, e.g.
    /// `戦艦級 − 低速戦艦 ≥ 2` or `重巡 + 軽巡 + 駆逐 − 艦数 = 0`.
    CountSum {
        terms: Vec<RouteCountTerm>,
        op: RouteOperator,
        value: i64,
    },
    /// Number of ships carrying at least one drum canister. Like
    /// [`RoutePredicate::EquipmentCount`] this counts ships, not equipment: the
    /// wikiwiki conditions all read 「ドラム缶搭載艦の隻数」.
    DrumCanisterCount {
        op: RouteOperator,
        value: i64,
    },
    And(Vec<RoutePredicate>),
    Or(Vec<RoutePredicate>),
    Not(Box<RoutePredicate>),
    FleetSizeWeightedRandom {
        weights: Vec<FleetSizeWeight>,
    },
    Unknown {
        raw_text: String,
    },
    SourceUnknown {
        raw_text: String,
    },
}

/// One addend of a [`RoutePredicate::CountSum`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteCountTerm {
    pub coef: i64,
    pub counter: RouteCounter,
}

/// A per-fleet quantity a routing condition counts. Every variant counts ships.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteCounter {
    /// Ships of any of the given ship types.
    ShipTypes(Vec<i64>),
    /// Ships whose master id is one of the given ids.
    Ships(Vec<i64>),
    /// Every ship in the fleet.
    FleetSize,
    /// Ships carrying at least one equipment with one of the master ids.
    EquipCarriers {
        slotitem_ids: Vec<i64>,
    },
    /// Ships of the given types whose own, unequipped speed is slow.
    SlowShips {
        ship_types: Vec<i64>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetSizeWeight {
    pub fleet_size: i64,
    pub probability_pct: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a cell with a label and `event_id`.
    fn cell_with_label(cell_no: i64, label: &str, event_id: i64) -> MapCellDefinition {
        MapCellDefinition {
            cell_no,
            event_id,
            node_label: Some(label.to_string()),
            ..Default::default()
        }
    }

    fn variant(cells: Vec<MapCellDefinition>, boss_cell_no: i64) -> MapVariantDefinition {
        MapVariantDefinition {
            cells,
            boss_cell_no,
            ..Default::default()
        }
    }

    // ── U1: boss_cell_nos helper scenarios ──

    #[test]
    fn boss_cell_nos_returns_all_cells_sharing_boss_label() {
        // Mirror of map 1-2 node E: two cells (5, 6) share label "E".
        let v = variant(vec![cell_with_label(5, "E", 5), cell_with_label(6, "E", 5)], 5);
        assert_eq!(v.boss_cell_nos(), vec![5, 6]);
    }

    #[test]
    fn boss_cell_nos_single_boss_returns_singleton() {
        // Common case: single-incoming-route boss node (R4 invariant).
        let v = variant(vec![cell_with_label(3, "C", 5)], 3);
        assert_eq!(v.boss_cell_nos(), vec![3]);
    }

    #[test]
    fn boss_cell_nos_fallback_when_label_is_none() {
        let cell = MapCellDefinition {
            cell_no: 5,
            event_id: 5,
            ..Default::default()
        };
        let v = variant(vec![cell], 5);
        assert_eq!(v.boss_cell_nos(), vec![5]);
    }

    #[test]
    fn boss_cell_nos_fallback_when_label_is_empty() {
        let v = variant(vec![cell_with_label(5, "", 5)], 5);
        assert_eq!(v.boss_cell_nos(), vec![5]);
    }

    #[test]
    fn boss_cell_nos_output_is_sorted() {
        // Insertion order is 6 then 5, but output must be sorted.
        let v = variant(vec![cell_with_label(6, "E", 5), cell_with_label(5, "E", 5)], 5);
        assert_eq!(v.boss_cell_nos(), vec![5, 6]);
    }

    // ── U4: regression test pinning equivalence semantics ──

    #[test]
    fn boss_cell_nos_invariant_all_returned_cells_have_boss_event_id() {
        // Every cell sharing the boss label must have event_id == 5.
        let v = variant(vec![cell_with_label(5, "E", 5), cell_with_label(6, "E", 5)], 5);
        for cell_no in v.boss_cell_nos() {
            let cell = v.cells.iter().find(|c| c.cell_no == cell_no).unwrap();
            assert_eq!(
                cell.event_id, 5,
                "cell {cell_no} shares boss label but has event_id {}",
                cell.event_id
            );
        }
    }

    #[test]
    fn boss_cell_nos_does_not_include_non_boss_label_cells() {
        // Cells with different labels must not appear in the boss set.
        let v = variant(
            vec![
                cell_with_label(5, "E", 5),
                cell_with_label(6, "E", 5),
                cell_with_label(4, "D", 4),
            ],
            5,
        );
        let nos = v.boss_cell_nos();
        assert!(nos.contains(&5));
        assert!(nos.contains(&6));
        assert!(!nos.contains(&4));
    }

    // ── Existing ShipDropDefinition tests (preserved) ──

    use super::ShipDropDefinition;

    #[test]
    fn ship_drop_definition_serializes_compactly() {
        let drops = vec![
            ShipDropDefinition {
                ship_id: 1,
                raw_ship_name: "睦月".to_string(),
                tags: Vec::new(),
                ..Default::default()
            },
            ShipDropDefinition {
                ship_id: 2,
                raw_ship_name: "如月".to_string(),
                tags: vec!["limited".to_string()],
                ..Default::default()
            },
        ];

        let json = serde_json::to_value(&drops).unwrap();
        assert_eq!(json[0], serde_json::json!({"ship_id": 1, "raw_ship_name": "睦月"}));
        assert_eq!(
            json[1],
            serde_json::json!({
                "ship_id": 2,
                "tags": ["limited"],
                "raw_ship_name": "如月",
            })
        );
    }

    #[test]
    fn ship_drop_definition_deserializes_compact_and_legacy_forms() {
        let json = serde_json::json!([
            1,
            {
                "ship_id": 2,
                "tags": ["limited"],
            },
            {
                "ship_id": 3,
                "raw_ship_name": "綾波",
                "tags": ["rare"],
            }
        ]);

        let drops = serde_json::from_value::<Vec<ShipDropDefinition>>(json).unwrap();
        assert_eq!(
            drops,
            vec![
                ShipDropDefinition {
                    ship_id: 1,
                    raw_ship_name: String::new(),
                    tags: Vec::new(),
                    ..Default::default()
                },
                ShipDropDefinition {
                    ship_id: 2,
                    raw_ship_name: String::new(),
                    tags: vec!["limited".to_string()],
                    ..Default::default()
                },
                ShipDropDefinition {
                    ship_id: 3,
                    raw_ship_name: "綾波".to_string(),
                    tags: vec!["rare".to_string()],
                    ..Default::default()
                },
            ]
        );
    }
}
