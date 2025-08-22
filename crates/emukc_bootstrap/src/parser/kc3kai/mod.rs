use emukc_model::thirdparty::CacheSource;

use crate::parser::error::ParseError;

pub mod quote;

pub fn parse(raw: &str, cache: &mut CacheSource) -> Result<(), ParseError> {
	quote::parse(raw, cache)?;

	Ok(())
}
