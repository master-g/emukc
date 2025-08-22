use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheSource {
	/// cache source for voices
	pub voices: VoiceCacheSource,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VoiceCacheSource {
	/// List of abyssal quotes, for resources in `kc9998`
	pub abyssal: Vec<u64>,
}
