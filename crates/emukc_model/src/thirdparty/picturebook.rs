use serde::{Deserialize, Serialize};

use crate::kc2::KcApiShipQVoiceInfo;

/// Ship extra voice information map, ship sort number -> `KcApiShipQVoiceInfo`
pub type Kc3rdShipVoiceMap = std::collections::BTreeMap<i64, Vec<KcApiShipQVoiceInfo>>;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Kc3rdPicturebookExtra {
	/// extra voice information
	pub voice_map: Kc3rdShipVoiceMap,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Kc3rdShipQVoiceRWItem {
	/// ship sort number
	pub sortno: i64,

	/// voices
	pub voices: Vec<KcApiShipQVoiceInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Kc3rdPicturebookRW {
	/// extra voice information
	pub voices: Vec<Kc3rdShipQVoiceRWItem>,
}

impl From<Kc3rdPicturebookExtra> for Kc3rdPicturebookRW {
	fn from(p: Kc3rdPicturebookExtra) -> Self {
		Self {
			voices: p
				.voice_map
				.into_iter()
				.map(|(k, v)| Kc3rdShipQVoiceRWItem {
					sortno: k,
					voices: v,
				})
				.collect(),
		}
	}
}

impl From<Kc3rdPicturebookRW> for Kc3rdPicturebookExtra {
	fn from(value: Kc3rdPicturebookRW) -> Self {
		Self {
			voice_map: value.voices.into_iter().map(|v| (v.sortno, v.voices)).collect(),
		}
	}
}
