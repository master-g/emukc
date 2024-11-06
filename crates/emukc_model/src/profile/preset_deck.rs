use serde::{Deserialize, Serialize};

use crate::kc2::{KcApiPresetDeck, KcApiPresetDeckElement};

/// Preset deck item
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PresetDeckItem {
	/// Deck index
	pub index: i64,

	/// Ship id
	pub name: String,

	/// Ships
	pub ships: [i64; 7],
}

/// Preset deck
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PresetDeck {
	/// Max number of records
	pub max_num: i64,

	/// Deck records
	pub records: Vec<PresetDeckItem>,
}

impl From<PresetDeckItem> for KcApiPresetDeckElement {
	fn from(value: PresetDeckItem) -> Self {
		Self {
			api_preset_no: value.index,
			api_name: value.name,
			api_name_id: "".to_string(),
			api_ship: value.ships,
		}
	}
}

impl From<PresetDeck> for KcApiPresetDeck {
	fn from(value: PresetDeck) -> Self {
		let api_deck = value
			.records
			.into_iter()
			.map(|record| (record.index.to_string(), record.into()))
			.collect();

		Self {
			api_max_num: value.max_num,
			api_deck,
		}
	}
}
