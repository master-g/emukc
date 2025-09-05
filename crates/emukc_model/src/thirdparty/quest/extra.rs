use crate::{
	codex::Codex,
	kc2::{KcApiQuestClearItemBonusType, KcApiQuestClearItemGetBonus},
	prelude::Kc3rdQuest,
};

pub(super) fn add_extra_to_conversion_or_exchange_bonus(
	codex: &Codex,
	quest: &Kc3rdQuest,
	bonus: &mut [KcApiQuestClearItemGetBonus],
) {
	if [614, 615, 616, 622, 623, 626, 627, 628, 629, 630, 631, 632, 633, 659, 660]
		.contains(&quest.api_no)
	{
		handle_aircraft_conversion(codex, quest, bonus);
	} else if [
		620, 621, 639, 641, 642, 643, 644, 645, 649, 650, 654, 656, 658, 666, 668, 669, 670, 671,
		672, 678, 683, 684, 685, 686, 687, 695, 696, 698, 1103, 1104, 1106, 1108, 1109, 1111, 1113,
		1118, 1123, 1127, 1128, 1129, 1132, 1133, 1134, 1137, 1142, 1143, 1145, 1153,
	]
	.contains(&quest.api_no)
	{
		let Some((to_id, b)) = find_first_slotitem_bonus(bonus) else {
			error!("no slotitem bonus for quest {}", quest.api_no);
			return;
		};

		let Some(to_mst) = codex.manifest.find_slotitem(to_id) else {
			error!("no to slotitem mst for quest {} item {}", quest.api_no, to_id);
			return;
		};

		let msg = match quest.api_no {
			620 => format!(
				"第一航空戦隊「流星改」精鋭艦攻隊 <br>「{}」編成完了！出撃開始！",
				to_mst.api_name
			),
			621 => format!("代替対潜兵装「{}」開発完了！", to_mst.api_name),
			639 => format!("試製兵装「{}」を獲得しました！", to_mst.api_name),
			641 => format!("「{}」を調達完了！", to_mst.api_name),
			642 => format!("「{}」を追加調達完了！", to_mst.api_name),
			643 => {
				format!("「{}」を新規調達完了！", to_mst.api_name)
			}
			644 | 1111 => {
				format!("「{}」を配備完了！", to_mst.api_name)
			}
			645 => {
				format!(
					"「{}」物資を調達完了！<br>(※使用には補給艦による運用が必要です) ",
					to_mst.api_name
				)
			}
			649 | 650 => {
				format!("「{}」を開発完了！", to_mst.api_name)
			}
			654 => {
				format!("精鋭飛行隊「{}」配備完了！", to_mst.api_name)
			}
			656 => {
				format!("第一潜水隊運用航空隊：「{}」の新編成を<br>完了しました！", to_mst.api_name)
			}
			658 | 668 | 1134 => format!("「{}」開発完了！", to_mst.api_name),
			666 => {
				format!("精鋭「瑞雲」隊、{}を編成完了！", to_mst.api_name)
			}
			669 | 670 => {
				format!("夜戦型艦上戦闘機「{}」を開発完了！", to_mst.api_name)
			}
			671 => {
				format!("夜間作戦型艦上攻撃機「{}」を開発完了！", to_mst.api_name)
			}
			672 => format!("「{}」創設完了しました！", to_mst.api_name),
			678 => "新型量産主力艦戦への更新を完了しました！".to_string(),
			683 => format!("新型砲熕兵装「{}」を開発完了！", to_mst.api_name),
			684 => {
				format!("精鋭航空戦艦艦爆隊「{}」編成完了！", to_mst.api_name)
			}
			685..=687 => format!("「{}」に改修完了！", to_mst.api_name),
			695 => {
				format!("「{}」戦力化成功！", to_mst.api_name)
			}
			696 => {
				format!("最精鋭「瑞雲」隊、「{}」編成完了！", to_mst.api_name)
			}
			698 => {
				format!("新鋭対潜哨戒航空戦力、「{}」配備完了！", to_mst.api_name)
			}
			1103 => "「新型潜水艦兵装」配備完了！".to_string(),
			1104 => "「潜水艦電子兵装」配備完了！".to_string(),
			1106 => {
				format!("精鋭三座水偵隊「{}」配備完了！", to_mst.api_name)
			}
			1108 => format!("調整改良型水中探信儀「{}」増産完了！", to_mst.api_name),
			1109 => format!("上陸支援用小型戦闘艇「{}」配備完了！", to_mst.api_name),
			1113 => {
				format!("「{}」、実戦配備完了！", to_mst.api_name)
			}
			1118 => format!("対潜迫撃砲「{}」配備完了！", to_mst.api_name),
			1123 => {
				format!("改良三座水偵隊「{}」増備完了！", to_mst.api_name)
			}
			1127 => format!("改良型艦載煙幕発生装備「{}」開発完了！", to_mst.api_name),
			1128 => format!("改金剛型搭載用改修砲「{}」開発完了！", to_mst.api_name),
			1129 | 1133 => format!("「{}」配備完了！", to_mst.api_name),
			1132 => format!("改夕雲型駆逐艦搭載用「{}」配備完了！", to_mst.api_name),
			1137 => format!("対潜兵装「{}」配備完了！", to_mst.api_name),
			1142 => format!("夜間作戦可能艦攻隊「{}」配備！", to_mst.api_name),
			1143 => format!("金剛型改装丙型主砲「{}」開発完了！", to_mst.api_name),
			1145 => format!("潜水艦搭載用水機「{}」増加配備！", to_mst.api_name),
			1153 => format!("「{}」増備完了！", to_mst.api_name),
			_ => {
				error!("missing extra quest reward info for quest {}", quest.api_no);
				return;
			}
		};
		if let Some(item) = b.api_item.as_mut() {
			item.api_message = Some(msg);
			b.api_type = KcApiQuestClearItemBonusType::ModelChange as i64;
		} else {
			error!("no bonus item for quest {}", quest.api_no);
		}
	} else if [1117, 1141].contains(&quest.api_no) {
		let Some((from_id, _)) = quest.extract_model_conversion_info() else {
			error!("no conversion info for quest {}", quest.api_no);
			return;
		};
		let Some(from_mst) = codex.manifest.find_slotitem(from_id) else {
			error!("no from slotitem mst for quest {} item {}", quest.api_no, from_id);
			return;
		};
		let Some((_, b)) = find_first_slotitem_bonus(bonus) else {
			error!("no slotitem bonus for quest {}", quest.api_no);
			return;
		};
		if let Some(item) = b.api_item.as_mut() {
			item.api_message = Some(format!("改修型最新水偵「{}」開発完了！", from_mst.api_name));
			b.api_type = KcApiQuestClearItemBonusType::ModelChange as i64;
		} else {
			error!("no bonus item for quest {}", quest.api_no);
		}
	} else if [637, 646, 648, 651, 652].contains(&quest.api_no) {
		let Some((to_id, b)) = find_first_useitem_bonus(bonus) else {
			error!("no useitem bonus for quest {}", quest.api_no);
			return;
		};
		let Some(to_mst) = codex.manifest.find_useitem(to_id) else {
			error!("no to useitem mst for quest {} item {}", quest.api_no, to_id);
			return;
		};
		let msg = match quest.api_no {
			637 => {
				format!("新たな「{}」を獲得しました！", to_mst.api_name)
			}
			646 | 648 | 652 => {
				format!("「{}」を招聘完了！", to_mst.api_name)
			}
			651 => {
				format!("「{}」を増産成功！", to_mst.api_name)
			}
			_ => {
				return;
			}
		};
		if let Some(item) = b.api_item.as_mut() {
			item.api_message = Some(msg);
			b.api_type = KcApiQuestClearItemBonusType::UseItem as i64;
		} else {
			error!("no bonus item for quest {}", quest.api_no);
		}
	}
}

fn find_first_slotitem_bonus(
	bonus: &mut [KcApiQuestClearItemGetBonus],
) -> Option<(i64, &mut KcApiQuestClearItemGetBonus)> {
	let bonus_item = bonus
		.iter_mut()
		.find(|bonus| bonus.api_type == KcApiQuestClearItemBonusType::SlotItem as i64)?;

	let item_id = bonus_item.api_item.as_ref()?.api_id?;

	Some((item_id, bonus_item))
}

fn find_first_useitem_bonus(
	bonus: &mut [KcApiQuestClearItemGetBonus],
) -> Option<(i64, &mut KcApiQuestClearItemGetBonus)> {
	let bonus_item = bonus
		.iter_mut()
		.find(|bonus| bonus.api_type == KcApiQuestClearItemBonusType::UseItem as i64)?;

	let item_id = bonus_item.api_item.as_ref()?.api_id?;

	Some((item_id, bonus_item))
}

fn handle_aircraft_conversion(
	codex: &Codex,
	quest: &Kc3rdQuest,
	bonus: &mut [KcApiQuestClearItemGetBonus],
) {
	// Handle aircraft conversion quests
	let (from_id, to_id) = match quest.api_no {
		622 => (143, 143), // special case for quest 622 (no conversion info in quest data)
		_ => {
			if let Some((from_id, to_id)) = quest.extract_model_conversion_info() {
				(from_id, to_id)
			} else {
				debug!("no conversion info for quest {}", quest.api_no);
				return;
			}
		}
	};

	let Some(from_mst) = codex.manifest.find_slotitem(from_id) else {
		debug!("no from slotitem mst for quest {} item {}", quest.api_no, from_id);
		return;
	};

	let Some(to_mst) = codex.manifest.find_slotitem(to_id) else {
		debug!("no to slotitem mst for quest {} item {}", quest.api_no, to_id);
		return;
	};

	if let Some(bonus) = bonus
		.iter_mut()
		.find(|v| v.api_item.as_ref().is_some_and(|item| item.api_id == Some(to_id)))
	{
		bonus.api_type = KcApiQuestClearItemBonusType::ModelChange as i64;
		if let Some(item) = bonus.api_item.as_mut() {
			let cat = if quest.api_no == 631 {
				"機種転換"
			} else {
				"部隊再編"
			};

			item.api_id_from = Some(from_id);
			item.api_id_to = Some(to_id);
			item.api_message = Some(format!(
				"{}の一部隊が、<br>{}に{cat}完了！",
				from_mst.api_name, to_mst.api_name
			));
		} else {
			debug!("no bonus item for quest {}", quest.api_no);
		}
	} else {
		debug!("bonus item not found for quest {} in {:?}", quest.api_no, bonus);
	}
}
