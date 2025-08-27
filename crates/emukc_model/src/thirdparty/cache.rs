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
	/// List of event quotes, for resources in `kc9997`
	pub event: Vec<u64>,
	/// List of NPC quotes, for resources in `kc9999`
	pub npc: Vec<u64>,
}
