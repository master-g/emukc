use serde::{self, Deserialize, Deserializer, de};

/// Deserialize form kcs api form IDs
pub(crate) fn deserialize_form_ivec<'de, D>(deserializer: D) -> Result<Vec<i64>, D::Error>
where
	D: Deserializer<'de>,
{
	let s = String::deserialize(deserializer)?;
	s.split(',')
		.map(|part| {
			part.trim()
				.parse::<i64>()
				.map_err(|e| de::Error::custom(format!("cannot parse to int: {}", e)))
		})
		.collect()
}
