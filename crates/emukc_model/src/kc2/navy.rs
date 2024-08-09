//! Navy manifest, used to determine which navy a ship belongs to

/*
let list = vec![
299, // Scamp
433, // Saratoga
440, // Iowa
544, // Gambier Bay
549, // Intrepid
561, // Samuel B. Roberts
562, // Johnston
595, // Houston
596, // Fletcher
597, // Atlanta
598, // Honolulu
601, // Colorado
602, // South Dakota
603, // Hornet
615, // Helena
654, // Washington
655, // Northampton
891, // Salmon
892, // Drum
896, // Brooklyn
913, // Maryland
923, // Tuscaloosa
924, // Nevada
925, // Langley
931, // Ranger
933, // Massachusetts
941, // Heywood L. Edwards
];

let ids: Vec<i64> = list
   .iter()
   .filter_map(|id| {
	   let mst = codex.find_ship_mst(*id).unwrap();
	   debug!("looking for all submodels of: {} {}", id, mst.api_name);
	   codex.ship_and_after(*id).ok()
   })
   .flat_map(|s| {
	   s.iter().for_each(|id| {
		   let mst = codex.find_ship_mst(*id).unwrap();
		   info!("\tall subships: {}, {}", mst.api_id, mst.api_name);
	   });
	   s.into_iter()
   })
   .collect();

info!("USN: {:?}", ids);

ids.iter().for_each(|id| {
   let mst = codex.find_ship_mst(*id).unwrap();
   info!("ship: {}, {}", mst.api_id, mst.api_name);
});
*/

use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// United States Navy
const USN: [i64; 61] = [
	299, 715, 433, 438, 545, 550, 440, 360, 544, 396, 707, 549, 397, 561, 681, 920, 562, 689, 595,
	600, 596, 692, 628, 629, 597, 696, 598, 711, 601, 1496, 602, 697, 603, 704, 615, 620, 654, 659,
	655, 660, 891, 897, 892, 732, 896, 722, 913, 918, 923, 928, 924, 929, 936, 925, 930, 931, 723,
	933, 938, 941, 726,
];

/*
let list = [
439, // Warspite
514, // Sheffield
515, // Ark Royal
519, // Jervis
520, // Janus
571, // Nelson
572, // Rodney
885, // Victorious
901, // Javelin
];
 */

/// Royal Navy
const RN: [i64; 18] =
	[439, 364, 514, 705, 515, 393, 519, 394, 520, 893, 571, 576, 572, 577, 885, 713, 901, 906];

/*
613, // Perth
 */
/// Royal Australian Navy
const RAN: [i64; 2] = [613, 618];

/*
604, // De Ruyter
 */
/// Royal Netherlands Navy
const RNN: [i64; 2] = [604, 609];

/// KC Navy manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KcNavy {
	/// United States Navy
	pub usn: Vec<i64>,

	/// Royal Navy
	pub rn: Vec<i64>,

	/// Royal Australian Navy
	pub ran: Vec<i64>,

	/// Royal Netherlands Navy
	pub rnn: Vec<i64>,
}

impl Default for KcNavy {
	fn default() -> Self {
		Self {
			usn: USN.to_vec(),
			rn: RN.to_vec(),
			ran: RAN.to_vec(),
			rnn: RNN.to_vec(),
		}
	}
}

impl FromStr for KcNavy {
	type Err = serde_json::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let data: KcNavy = serde_json::from_str(s)?;

		Ok(data)
	}
}

impl KcNavy {
	/// Check if the ship is in the USN
	///
	/// # Arguments
	///
	/// * `id` - The ship ID
	///
	/// # Returns
	///
	/// * `bool` - If the ship is in the USN
	pub fn is_usn(&self, id: i64) -> bool {
		self.usn.contains(&id)
	}

	/// Check if the ship is in the RN
	///
	/// # Arguments
	///
	/// * `id` - The ship ID
	///
	/// # Returns
	///
	/// * `bool` - If the ship is in the RN
	pub fn is_rn(&self, id: i64) -> bool {
		self.rn.contains(&id)
	}

	/// Check if the ship is in the RAN
	///
	/// # Arguments
	///
	/// * `id` - The ship ID
	///
	/// # Returns
	///
	/// * `bool` - If the ship is in the RAN
	pub fn is_ran(&self, id: i64) -> bool {
		self.ran.contains(&id)
	}

	/// Check if the ship is in the RNN
	///
	/// # Arguments
	///
	/// * `id` - The ship ID
	///
	/// # Returns
	///
	/// * `bool` - If the ship is in the RNN
	pub fn is_rnn(&self, id: i64) -> bool {
		self.rnn.contains(&id)
	}
}
