/// Embedded real `api_req_map/start` captures used to generate public map overlays.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealMapStartAsset {
    /// Display name for the asset (e.g. `"map_1-1.json"`).
    pub name: &'static str,
    raw_json: &'static str,
}

impl RealMapStartAsset {
    /// Create a new asset from a name and raw JSON string.
    #[cfg(test)]
    pub(crate) fn new(name: &'static str, raw_json: &'static str) -> Self {
        Self {
            name,
            raw_json,
        }
    }

    /// Return the raw JSON body of the captured `api_req_map/start` response.
    pub fn raw_json(&self) -> &'static str {
        self.raw_json
    }
}

macro_rules! real_map_start_assets {
	($($name:literal),+ $(,)?) => {
		&[
			$(
				RealMapStartAsset {
					name: $name,
					raw_json: include_str!(concat!("../assets/real_map_start_data/", $name)),
				},
			)+
		]
	};
}

/// All embedded real map start assets, compiled into the binary via `include_str!`.
pub const EMBEDDED_REAL_MAP_START_ASSETS: &[RealMapStartAsset] = real_map_start_assets!(
    "map_1-1.json",
    "map_1-2.json",
    "map_1-3.json",
    "map_1-4.json",
    "map_1-5.json",
    "map_2-1.json",
    "map_2-2.json",
    "map_2-3.json",
    "map_2-4.json",
    "map_2-5.json",
    "map_3-1.json",
    "map_3-2.json",
    "map_3-3.json",
    "map_3-4.json",
    "map_3-5.json",
    "map_4-1.json",
    "map_4-2.json",
    "map_4-3.json",
    "map_4-4.json",
    "map_4-5.json",
    "map_5-1.json",
    "map_5-2.json",
    "map_5-3.json",
    "map_5-4.json",
    "map_5-5.json",
    "map_6-1.json",
    "map_6-2.json",
    "map_6-3.json",
    "map_6-4.json",
    "map_6-5.json",
    "map_7-1.json",
    "map_7-2.json",
    "map_7-3.json",
    "map_7-3-part2.json",
    "map_7-4.json",
    "map_7-5.json",
);
