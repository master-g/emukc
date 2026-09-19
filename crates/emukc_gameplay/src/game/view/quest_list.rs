//! The `questlist` view.

use emukc_db::entity::profile::quest::progress;
use emukc_model::{
    kc2::{KcApiQuestListRewardItem, KcApiQuestType},
    thirdparty::{Kc3rdQuest, Kc3rdQuestPeriod},
};

use crate::{err::GameplayError, gameplay::Ctx};

/// Everything the `questlist` view shows.
#[derive(Debug)]
pub struct QuestListView {
    /// Whether any activated quest is ready to claim.
    pub completed_kind: i64,

    /// Number of activated quests.
    pub exec_count: i64,

    /// Quests on the requested tab.
    pub items: Vec<QuestListItem>,
}

/// One quest on the `questlist` view.
#[derive(Debug)]
pub struct QuestListItem {
    /// Quest number.
    pub no: i64,

    /// Quest category.
    pub category: i64,

    /// Quest type, from the quest period.
    pub quest_type: i64,

    /// Quest tab label.
    pub label_type: i64,

    /// 1 = idle, 2 = activated, 3 = ready to claim.
    pub state: i64,

    /// Quest title.
    pub title: String,

    /// Quest detail.
    pub detail: String,

    /// Badges consumed on claim.
    pub lost_badges: i64,

    /// Material rewards, in fuel / ammo / steel / bauxite order.
    pub reward_materials: Vec<i64>,

    /// Selectable rewards.
    pub select_rewards: Option<Vec<Vec<KcApiQuestListRewardItem>>>,

    /// 1 = normal reward, 2 = ship reward.
    pub bonus_flag: i64,

    /// Progress bucket, 0 once the quest is ready to claim.
    pub progress_flag: i64,
}

impl Ctx {
    /// Read the quests shown on one `questlist` tab.
    ///
    /// # Parameters
    ///
    /// - `profile_id`: The profile ID.
    /// - `tab_id`: The tab: 0 = all, 9 = activated, 1 = daily, 2 = weekly,
    ///   3 = monthly, 4 = oneshot, 5 = other (quarterly and yearly).
    pub async fn quest_list_view(
        &self,
        profile_id: i64,
        tab_id: i64,
    ) -> Result<QuestListView, GameplayError> {
        let codex = self.codex.as_ref();

        let quests = self.get_quest_records(profile_id).await?;

        let mut completed_kind = 0;
        let mut exec_count = 0;

        let items: Vec<QuestListItem> = quests
            .iter()
            .filter_map(|model| {
                if tab_id == 9 && model.status != progress::Status::Activated {
                    return None;
                }

                let mst = codex.find::<Kc3rdQuest>(&model.quest_id).ok()?;

                if !tab_shows(tab_id, mst.label_type) {
                    return None;
                }

                if model.status == progress::Status::Activated {
                    exec_count += 1;
                    if model.progress == progress::Progress::Completed {
                        completed_kind = 1;
                    }
                }

                Some(QuestListItem {
                    no: mst.api_no,
                    category: mst.category as i64,
                    quest_type: match mst.period {
                        Kc3rdQuestPeriod::Oneshot => KcApiQuestType::Oneshot as i64,
                        Kc3rdQuestPeriod::Daily
                        | Kc3rdQuestPeriod::Daily3rd7th0th
                        | Kc3rdQuestPeriod::Daily2nd8th => KcApiQuestType::Daily as i64,
                        Kc3rdQuestPeriod::Weekly => KcApiQuestType::Weekly as i64,
                        Kc3rdQuestPeriod::Monthly => KcApiQuestType::Monthly as i64,
                        Kc3rdQuestPeriod::Quarterly | Kc3rdQuestPeriod::Annual => {
                            KcApiQuestType::Other as i64
                        }
                        Kc3rdQuestPeriod::Unknown => KcApiQuestType::Other as i64,
                    },
                    label_type: mst.label_type,
                    state: if model.status == progress::Status::Idle {
                        1
                    } else if model.progress == progress::Progress::Completed {
                        3
                    } else {
                        2
                    },
                    title: mst.name.clone(),
                    detail: mst.detail.clone(),
                    lost_badges: mst.requirements.lost_badges(),
                    reward_materials: vec![
                        mst.reward_fuel,
                        mst.reward_ammo,
                        mst.reward_steel,
                        mst.reward_bauxite,
                    ],
                    select_rewards: mst.to_api_reward_selection(),
                    bonus_flag: mst.bonus_flag(),
                    progress_flag: if model.progress == progress::Progress::Completed {
                        0
                    } else {
                        model.progress as i64
                    },
                })
            })
            .collect();

        Ok(QuestListView {
            completed_kind,
            exec_count,
            items,
        })
    }
}

/// Whether a quest with `label_type` belongs on `questlist` tab `tab_id`.
///
/// Tab ids follow the client's tab bar; label types follow `api_label_type`
/// (1 = oneshot, 2 = daily, 3 = weekly, 6 = monthly, 7 = quarterly, 101..=112
/// = yearly by month), which is also what the client draws as the row label.
fn tab_shows(tab_id: i64, label_type: i64) -> bool {
    match tab_id {
        0 | 9 => true,
        1 => label_type == 2,
        2 => label_type == 3,
        3 => label_type == 6,
        4 => label_type == 1,
        5 => label_type == 7 || (101..=112).contains(&label_type),
        _ => false,
    }
}
