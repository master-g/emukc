//! Slot item extension for `Codex`

use crate::prelude::ApiMstSlotitem;

use super::{Codex, CodexError};

impl Codex {
	pub(super) fn find_slotitem_mst(
		&self,
		slotitem_id: i64,
	) -> Result<&ApiMstSlotitem, CodexError> {
		self.manifest
			.find_slotitem(slotitem_id)
			.ok_or(CodexError::NotFound(format!("slot item manifest ID: {}", slotitem_id)))
	}
}
