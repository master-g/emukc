use std::collections::HashMap;

use emukc_model::thirdparty::CacheSource;
use serde::{Deserialize, Serialize};

use crate::parser::error::ParseError;

pub type Quotes = HashMap<String, HashMap<String, QuoteValue>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum QuoteValue {
	Integer(i64),
	String(String),
	StringMap(HashMap<String, String>),
}

pub(super) fn parse(raw: &str, cache: &mut CacheSource) -> Result<(), ParseError> {
	let quotes = serde_json::from_str::<Quotes>(raw)?;
	// kc9998 abyssal quotes
	{
		let abyssal_quotes =
			quotes.get("abyssal").ok_or(ParseError::KeyMissing("abyssal".to_string()))?;
		let entries: Vec<u64> =
			abyssal_quotes.keys().filter_map(|k| k.parse::<u64>().ok()).collect();
		cache.voices.abyssal = entries;
	}

	// kc9997 event quotes
	{
		let event_quotes =
			quotes.get("event").ok_or(ParseError::KeyMissing("event".to_string()))?;
		let entries: Vec<u64> = event_quotes.keys().filter_map(|k| k.parse::<u64>().ok()).collect();
		cache.voices.event = entries;
	}

	// kc9999 npc quotes
	{
		let npc_quotes = quotes.get("npc").ok_or(ParseError::KeyMissing("npc".to_string()))?;
		let entries: Vec<u64> = npc_quotes.keys().filter_map(|k| k.parse::<u64>().ok()).collect();
		cache.voices.npc = entries;
	}

	Ok(())
}
