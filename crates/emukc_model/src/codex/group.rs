//! Grouping ships.

use std::collections::HashSet;

use super::Codex;

/// Group DE ships parameters.
#[derive(Debug, Clone, Copy)]
pub struct DeGroupParam {
	/// Ship instance ID.
	pub id: i64,

	/// Ship manifest ID.
	pub mst_id: i64,

	/// Ship class.
	pub ctype: i64,
}

/// Group DE ships response.
#[derive(Debug, Clone)]
pub struct DeGroupResp {
	/// Same class different type pairs, which can be used as HP bonus.
	pub hp_pairs: Vec<(i64, i64)>,
	/// Other pairs.
	pub other_pairs: Vec<(i64, i64)>,
	/// Rest of the ships.
	pub rest: Vec<i64>,
}

impl Codex {
	/// Group DE ships.
	///
	/// result.0: same class different type pairs, which can be used as HP bonus.
	/// result.1: rest of the ships.
	///
	/// # Arguments
	///
	/// * `params` - Group DE ships parameters.
	pub fn group_de_ships(&self, params: &[DeGroupParam]) -> DeGroupResp {
		let mut hp_pairs: Vec<(i64, i64)> = Vec::new();
		let mut other_pairs: Vec<(i64, i64)> = Vec::new();
		let mut rest: Vec<i64> = Vec::new();

		if params.len() < 2 {
			rest.extend(params.iter().map(|p| p.id));
			return DeGroupResp {
				hp_pairs,
				other_pairs,
				rest,
			};
		}

		// record paired ship IDs
		let mut marked: HashSet<i64> = HashSet::new();

		for i in 0..params.len() {
			if marked.contains(&params[i].id) {
				continue;
			}

			let mut found = false;

			for j in i + 1..params.len() {
				if marked.contains(&params[j].id) {
					continue;
				}

				let ship_i = &params[i];
				let ship_j = &params[j];

				let before_and_after =
					self.ships_before_and_after(ship_j.mst_id).unwrap_or_default();

				if before_and_after.contains(&ship_i.mst_id) {
					other_pairs.push((ship_i.id, ship_j.id));
					marked.insert(ship_i.id);
					marked.insert(ship_j.id);
					found = true;
					break;
				} else if ship_i.ctype == ship_j.ctype {
					// same class, check if they are different types
					hp_pairs.push((ship_i.id, ship_j.id));
					marked.insert(ship_i.id);
					marked.insert(ship_j.id);
					found = true;
					break;
				}
			}

			if !found {
				rest.push(params[i].id);
			}
		}

		DeGroupResp {
			hp_pairs,
			other_pairs,
			rest,
		}
	}
}
