#![allow(missing_docs)]

use crate::profile::quest::QuestProgressStatus;

use super::Kc3rdQuestRequirement;

impl Kc3rdQuestRequirement {
	pub fn calculate_progress(&self, _mst: &Kc3rdQuestRequirement) -> QuestProgressStatus {
		// TODO: implement this
		QuestProgressStatus::Completed
	}
}
