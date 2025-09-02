use crate::{
	codex::Codex,
	kc2::{KcApiQuestClearItemBonusType, KcApiQuestClearItemGetBonus},
};

pub(super) fn use_item_conversion_extra(
	codex: &Codex,
	quest_id: i64,
	bonus: &mut KcApiQuestClearItemGetBonus,
) {
	let Some(item) = bonus.api_item.as_mut() else {
		debug!("no bonus item for quest {}", quest_id);
		return;
	};

	if bonus.api_type != KcApiQuestClearItemBonusType::UseItem as i64 {
		debug!("not a use item bonus for quest {}", quest_id);
		return;
	}

	let Some(item_id) = item.api_id else {
		debug!("no item id for quest {}", quest_id);
		return;
	};

	let Some(useitem_mst) = codex.manifest.find_useitem(item_id) else {
		debug!("no useitem mst for quest {} item {}", quest_id, item_id);
		return;
	};

	let msg = match quest_id {
		646 | 648 | 652 => {
			format!("「{}」を招聘完了！", useitem_mst.api_name)
		}
		651 => {
			format!("「{}」を増産成功！", useitem_mst.api_name)
		}
		_ => format!("extra quest reward info missing for quest {}", quest_id),
	};
	item.api_message = Some(msg);
}

pub(super) fn slot_item_conversion_extra(
	codex: &Codex,
	quest_id: i64,
	bonus: &mut KcApiQuestClearItemGetBonus,
) {
	let Some(item) = bonus.api_item.as_mut() else {
		debug!("no bonus item for quest {}", quest_id);
		return;
	};

	if bonus.api_type != KcApiQuestClearItemBonusType::UseItem as i64 {
		debug!("not a use item bonus for quest {}", quest_id);
		return;
	}

	let Some(item_id) = item.api_id else {
		debug!("no item id for quest {}", quest_id);
		return;
	};

	let Some(useitem_mst) = codex.manifest.find_useitem(item_id) else {
		debug!("no useitem mst for quest {} item {}", quest_id, item_id);
		return;
	};

	let msg = match quest_id {
		646 | 648 | 652 => {
			format!("「{}」を招聘完了！", useitem_mst.api_name)
		}
		651 => {
			format!("「{}」を増産成功！", useitem_mst.api_name)
		}
		_ => format!("extra quest reward info missing for quest {}", quest_id),
	};
	item.api_message = Some(msg);
}
