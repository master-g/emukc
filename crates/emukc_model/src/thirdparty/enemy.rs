use serde::{Deserialize, Serialize};

/// Enemy ship extra information map.
pub type Kc3rdEnemyShipMap = std::collections::BTreeMap<i64, Kc3rdEnemyShip>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdEnemyShipSlotInfo {
    /// initial equipment manifest id
    pub item_id: i64,

    /// how many plane the slot can hold
    pub onslot: i64,
}

/// Enemy ship bootstrap information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kc3rdEnemyShip {
    /// `api_id`, ship manifest id
    pub api_id: i64,

    /// ship name
    pub name: String,

    /// ship reading
    pub yomi: String,

    /// ship type id
    pub stype: i64,

    /// ship class type id
    pub ctype: i64,

    /// HP
    pub hp: i64,

    /// firepower
    pub firepower: i64,

    /// torpedo
    pub torpedo: i64,

    /// anti-air
    pub aa: i64,

    /// armor
    pub armor: i64,

    /// evasion
    pub evasion: i64,

    /// anti-submarine
    pub asw: i64,

    /// line of sight
    pub los: i64,

    /// luck
    pub luck: i64,

    /// speed
    pub speed: i64,

    /// range
    pub range: i64,

    /// rarity / back image group
    pub rarity: i64,

    /// ship background image
    pub backs: i64,

    /// number of slots
    pub slot_num: i64,

    /// aircraft capacity for each slot.
    pub maxeq: [i64; 5],

    /// equipped slot items in order.
    pub slots: Vec<Kc3rdEnemyShipSlotInfo>,
}
