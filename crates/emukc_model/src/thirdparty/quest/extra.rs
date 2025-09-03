use crate::{
	codex::Codex,
	kc2::{KcApiQuestClearItemBonusType, KcApiQuestClearItemGetBonus},
	prelude::Kc3rdQuest,
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
	quest: &Kc3rdQuest,
	bonus: &mut KcApiQuestClearItemGetBonus,
) -> bool {
	if bonus.api_type != KcApiQuestClearItemBonusType::SlotItem as i64 {
		debug!("not a slot item bonus for quest {}", quest.api_no);
		return false;
	}
	let Some(item) = bonus.api_item.as_mut() else {
		debug!("no bonus item for quest {}", quest.api_no);
		return false;
	};

	let Some((from_id, to_id)) = quest.extract_model_conversion_info() else {
		debug!("no conversion info for quest {}", quest.api_no);
		return false;
	};

	let Some(from_mst) = codex.manifest.find_slotitem(from_id) else {
		debug!("no from slotitem mst for quest {} item {}", quest.api_no, from_id);
		return false;
	};

	let Some(to_mst) = codex.manifest.find_slotitem(to_id) else {
		debug!("no to slotitem mst for quest {} item {}", quest.api_no, to_id);
		return false;
	};

	item.api_id_from = Some(from_id);
	item.api_id_to = Some(to_id);

	let msg = if from_mst.api_type[4] != 0 && to_mst.api_type[4] != 0 {
		// air craft conversion
		match quest.api_no {
			620 => format!(
				"第一航空戦隊「流星改」精鋭艦攻隊 <br>「{}」編成完了！出撃開始！",
				to_mst.api_name
			),
			626 | 629 | 630 | 631 | 659 | 660 => {
				format!("{}の一部隊が、<br>{}に部隊再編完了！", from_mst.api_name, to_mst.api_name)
			}
			643 => {
				format!("「{}」を新規調達完了！", to_mst.api_name)
			}
			644 | 1111 => {
				format!("「{}」を配備完了！", to_mst.api_name)
			}
			649 => {
				format!("「{}」を開発完了！", to_mst.api_name)
			}
			654 => {
				format!("精鋭飛行隊「{}」配備完了！", to_mst.api_name)
			}
			656 => {
				format!("第一潜水隊運用航空隊：「{}」の新編成を<br>完了しました！", to_mst.api_name)
			}
			666 => {
				format!("精鋭「瑞雲」隊、{}を編成完了！", to_mst.api_name)
			}
			669 | 670 => {
				format!("夜戦型艦上戦闘機「{}」を開発完了！", to_mst.api_name)
			}
			671 => {
				format!("夜間作戦型艦上攻撃機「{}」を開発完了！", to_mst.api_name)
			}
			678 => "新型量産主力艦戦への更新を完了しました！".to_string(),
			684 => {
				format!("精鋭航空戦艦艦爆隊「{}」編成完了！", to_mst.api_name)
			}
			695 => {
				format!("「{}」戦力化成功！", to_mst.api_name)
			}
			696 => {
				format!("最精鋭「瑞雲」隊、「{}」編成完了！", to_mst.api_name)
			}
			698 => {
				format!("新鋭対潜哨戒航空戦力、「{}」配備完了！", to_mst.api_name)
			}
			1106 => {
				format!("精鋭三座水偵隊「{}」配備完了！", to_mst.api_name)
			}
			1113 => {
				format!("「{}」、実戦配備完了！", to_mst.api_name)
			}
			1117 | 1141 => {
				format!("改修型最新水偵「{}」開発完了！", from_mst.api_name)
			}
			1123 => {
				format!("改良三座水偵隊「{}」増備完了！", to_mst.api_name)
			}
			1142 => {
				format!("夜間作戦可能艦攻隊「{}」配備！", to_mst.api_name)
			}
			_ => {
				format!("{}の一部隊が、<br>{}に機種転換完了！", from_mst.api_name, to_mst.api_name)
			}
		}
	} else {
		match quest.api_no {
			621 => format!("代替対潜兵装「{}」開発完了！", to_mst.api_name),
			639 => format!("試製兵装「{}」を獲得しました！", to_mst.api_name),
			641 => format!("「{}」を調達完了！", to_mst.api_name),
			_ => {
				format!("「{}」を「{}」に改造完了！", from_mst.api_name, to_mst.api_name)
			}
		}
	};

	item.api_message = Some(msg);

	true
}
