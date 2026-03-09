use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Holes report for updating hardcoded lists
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct HolesReport {
	pub event_ship_full: BTreeSet<i64>,
	pub event_ship_full_dmg: BTreeSet<i64>,
	pub event_ship_up: BTreeSet<i64>,
	pub event_ship_up_dmg: BTreeSet<i64>,
	pub enemy_ship_full: BTreeSet<i64>,
	pub enemy_ship_full_dmg: BTreeSet<i64>,
	pub furniture_normal: BTreeSet<i64>,
	pub slot_character: BTreeSet<i64>,
}

#[allow(dead_code)]
impl HolesReport {
	/// Create a new empty holes report
	pub fn new() -> Self {
		Self::default()
	}

	/// Add a missing id to the report
	pub fn add_missing(&mut self, category: &str, id: i64) {
		match category {
			"event_ship_full" => self.event_ship_full.insert(id),
			"event_ship_full_dmg" => self.event_ship_full_dmg.insert(id),
			"event_ship_up" => self.event_ship_up.insert(id),
			"event_ship_up_dmg" => self.event_ship_up_dmg.insert(id),
			"enemy_ship_full" => self.enemy_ship_full.insert(id),
			"enemy_ship_full_dmg" => self.enemy_ship_full_dmg.insert(id),
			"furniture_normal" => self.furniture_normal.insert(id),
			"slot_character" => self.slot_character.insert(id),
			_ => false,
		};
	}

	/// Generate Rust code for the missing ids
	pub fn generate_rust_code(&self) -> String {
		let mut output = String::new();

		output.push_str("// Generated holes report - copy to source files\n\n");

		if !self.event_ship_full.is_empty()
			|| !self.event_ship_full_dmg.is_empty()
			|| !self.event_ship_up.is_empty()
			|| !self.event_ship_up_dmg.is_empty()
		{
			output.push_str("// ship.rs - EVENT_SHIP_HOLES\n");
			output.push_str(
				"static EVENT_SHIP_HOLES: LazyLock<ShipEventHoles> = LazyLock::new(|| ShipEventHoles {\n",
			);
			output.push_str(&format!(
				"    full: vec!{:?},\n",
				self.event_ship_full.iter().collect::<Vec<_>>().sort()
			));
			output.push_str(&format!(
				"    full_dmg: vec!{:?},\n",
				self.event_ship_full_dmg.iter().collect::<Vec<_>>().sort()
			));
			output.push_str(&format!(
				"    up: vec!{:?},\n",
				self.event_ship_up.iter().collect::<Vec<_>>().sort()
			));
			output.push_str(&format!(
				"    up_dmg: vec!{:?},\n",
				self.event_ship_up_dmg.iter().collect::<Vec<_>>().sort()
			));
			output.push_str("});\n\n");
		}

		if !self.enemy_ship_full.is_empty() || !self.enemy_ship_full_dmg.is_empty() {
			output.push_str("// ship.rs - ENEMY_SHIP_HOLES\n");
			output.push_str(
				"static ENEMY_SHIP_HOLES: LazyLock<ShipEventHoles> = LazyLock::new(|| ShipEventHoles {\n",
			);
			output.push_str(&format!(
				"    full: vec!{:?},\n",
				self.enemy_ship_full.iter().collect::<Vec<_>>()
			));
			output.push_str(&format!(
				"    full_dmg: vec!{:?},\n",
				self.enemy_ship_full_dmg.iter().collect::<Vec<_>>()
			));
			output.push_str("    up: vec![],\n");
			output.push_str("    up_dmg: vec![],\n");
			output.push_str("});\n\n");
		}

		if !self.furniture_normal.is_empty() {
			output.push_str("// furniture.rs - NORMAL_HOLES\n");
			output.push_str(&format!(
				"static NORMAL_HOLES: LazyLock<Vec<i64>> = LazyLock::new(|| vec!{:?});\n\n",
				self.furniture_normal.iter().collect::<Vec<_>>()
			));
		}

		if !self.slot_character.is_empty() {
			output.push_str("// slot.rs - CHARACTER_HOLES\n");
			output.push_str(&format!(
				"static CHARACTER_HOLES: LazyLock<Vec<i64>> = LazyLock::new(|| vec!{:?});\n\n",
				self.slot_character.iter().collect::<Vec<_>>()
			));
		}

		output
	}
}
