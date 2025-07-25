//! Querying trait for objects in the codex.

use crate::{
	kc2::KcApiMusicListElement,
	prelude::{
		ApiMstFurniture, ApiMstPayitem, ApiMstShip, ApiMstSlotitem, ApiMstUseitem, Kc3rdQuest,
		Kc3rdShip, Kc3rdSlotItem,
	},
};

use super::{Codex, CodexError};

/// Trait for objects that can be found in Codex.
pub trait FoundInCodex {
	/// The key type for the object.
	type Key: ?Sized + Copy;

	/// Find an object in the codex by key.
	///
	/// # Parameters
	///
	/// - `codex`: The codex.
	/// - `id`: The ID of the object.
	fn find_in_codex<'a>(codex: &'a Codex, key: &'a Self::Key) -> Result<&'a Self, CodexError>;
}

impl Codex {
	/// Find an object in the codex by ID.
	///
	/// # Parameters
	///
	/// - `id`: The ID of the object.
	pub fn find<'a, T>(&'a self, key: &'a T::Key) -> Result<&'a T, CodexError>
	where
		T: FoundInCodex,
	{
		let v = T::find_in_codex(self, key)?;
		Ok(v)
	}
}

// ApiMstFurniture
impl FoundInCodex for ApiMstFurniture {
	type Key = i64;

	fn find_in_codex<'a>(codex: &'a Codex, key: &'a Self::Key) -> Result<&'a Self, CodexError> {
		codex
			.manifest
			.find_furniture(*key)
			.ok_or_else(|| CodexError::NotFound(format!("furniture manifest ID: {key}")))
	}
}

// ApiMstSlotitem
impl FoundInCodex for ApiMstSlotitem {
	type Key = i64;

	fn find_in_codex<'a>(codex: &'a Codex, key: &'a Self::Key) -> Result<&'a Self, CodexError> {
		codex
			.manifest
			.find_slotitem(*key)
			.ok_or_else(|| CodexError::NotFound(format!("slot item manifest ID: {key}")))
	}
}

// ApiMstShip
impl FoundInCodex for ApiMstShip {
	type Key = i64;

	fn find_in_codex<'a>(codex: &'a Codex, key: &'a Self::Key) -> Result<&'a Self, CodexError> {
		codex
			.manifest
			.find_ship(*key)
			.ok_or_else(|| CodexError::NotFound(format!("ship manifest ID: {key}")))
	}
}

// Kc3rdShip
impl FoundInCodex for Kc3rdShip {
	type Key = i64;

	fn find_in_codex<'a>(codex: &'a Codex, key: &'a Self::Key) -> Result<&'a Self, CodexError> {
		codex.find_ship_extra(*key)
	}
}

// ApiUseItem
impl FoundInCodex for ApiMstUseitem {
	type Key = i64;

	fn find_in_codex<'a>(codex: &'a Codex, key: &'a Self::Key) -> Result<&'a Self, CodexError> {
		codex
			.manifest
			.find_useitem(*key)
			.ok_or_else(|| CodexError::NotFound(format!("use item manifest ID: {key}")))
	}
}

// ApiPayItem
impl FoundInCodex for ApiMstPayitem {
	type Key = i64;

	fn find_in_codex<'a>(codex: &'a Codex, key: &'a Self::Key) -> Result<&'a Self, CodexError> {
		codex
			.manifest
			.find_payitem(*key)
			.ok_or_else(|| CodexError::NotFound(format!("pay item manifest ID: {key}")))
	}
}

// Kc3rdSlotItemExtraInfo
impl FoundInCodex for Kc3rdSlotItem {
	type Key = i64;

	fn find_in_codex<'a>(codex: &'a Codex, key: &'a Self::Key) -> Result<&'a Self, CodexError> {
		codex
			.slotitem_extra_info
			.get(key)
			.ok_or_else(|| CodexError::NotFound(format!("slot item extra info ID: {key}")))
	}
}

// Kc3rdQuest
impl FoundInCodex for Kc3rdQuest {
	type Key = i64;

	fn find_in_codex<'a>(codex: &'a Codex, key: &'a Self::Key) -> Result<&'a Self, CodexError> {
		codex.quest.get(key).ok_or_else(|| CodexError::NotFound(format!("quest ID: {key}")))
	}
}

// KcApiMusicListElement
impl FoundInCodex for KcApiMusicListElement {
	type Key = i64;

	fn find_in_codex<'a>(codex: &'a Codex, key: &'a Self::Key) -> Result<&'a Self, CodexError> {
		codex
			.music_list
			.iter()
			.find(|v| v.api_id == *key)
			.ok_or_else(|| CodexError::NotFound(format!("music list ID: {key}")))
	}
}
