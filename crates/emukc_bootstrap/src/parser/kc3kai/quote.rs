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
	let abyssal_quotes =
		quotes.get("abyssal").ok_or(ParseError::KeyMissing("abyssal".to_string()))?;
	let entries: Vec<u64> = abyssal_quotes.keys().filter_map(|k| k.parse::<u64>().ok()).collect();

	cache.voices.abyssal = entries;

	Ok(())
}
