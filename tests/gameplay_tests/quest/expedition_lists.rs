//! Integration tests: expedition quest lists name expeditions by code and mean
//! "each of", except 410 and 411 (#121).
//!
//! Runs the codex's own quest conditions (so the parser's expansion is under
//! test too) through `apply_event_with_context` with the codex, which is what
//! resolves `A1` to mission 100.

#[cfg(test)]
mod tests {
    use emukc_internal::prelude::*;

    /// The quest's `Expedition` condition as the codex holds it, plus its master.
    fn expedition_condition(quest_id: i64) -> Kc3rdQuestCondition {
        let quest = crate::CODEX.find::<Kc3rdQuest>(&quest_id).unwrap();
        let Kc3rdQuestRequirement::And(conditions) = &quest.requirements else {
            panic!("quest {quest_id} is not an And quest");
        };
        conditions
            .iter()
            .find(|c| matches!(c, Kc3rdQuestCondition::Expedition(_)))
            .cloned()
            .expect("quest has an Expedition condition")
    }

    /// Return `missions` from successful expeditions, in order.
    fn run(quest_id: i64, missions: &[i64]) -> Kc3rdQuestCondition {
        let master = expedition_condition(quest_id);
        let mut current = master.clone();
        for &mission_id in missions {
            current.apply_event_with_context(
                &QuestActionEvent::ExpeditionCompleted {
                    mission_id,
                    result: ExpeditionResult::Success,
                    fleet_id: 2,
                },
                Some(&master),
                Some(&crate::CODEX),
            );
        }
        current
    }

    fn done(quest_id: i64, missions: &[i64]) -> bool {
        run(quest_id, missions).is_satisfied()
    }

    /// A1 is mission 100, A2 101, A3 102.
    const A1: i64 = 100;
    const A2: i64 = 101;
    const A3: i64 = 102;

    #[test]
    fn an_expedition_named_by_its_code_matches() {
        assert!(done(427, &[A1]), "427 asks for one A1");
        assert!(!done(427, &[1]));
    }

    #[test]
    fn a_list_means_each_listed_expedition() {
        assert!(!done(426, &[3]), "expedition 3 alone is not 426");
        assert!(!done(426, &[3, 3, 3, 3]), "one expedition four times is not 426");
        assert!(done(426, &[3, 4, 5, 10]));
    }

    #[test]
    fn each_listed_expedition_needs_its_own_times() {
        assert!(!done(428, &[4, A2, A3, 4, A2]));
        assert!(done(428, &[4, A2, A3, 4, A2, A3]));
    }

    #[test]
    fn the_tokyo_express_quests_take_any_of_their_list() {
        assert!(done(410, &[37]));
        assert!(done(410, &[38]));
        assert!(!done(411, &[37, 38, 37, 38, 37]));
        assert!(done(411, &[37, 38, 37, 38, 37, 37]));
    }

    #[test]
    fn progress_counts_each_listed_expedition() {
        let current = run(426, &[3, 4]);
        let Kc3rdQuestRequirement::And(master) =
            &crate::CODEX.find::<Kc3rdQuest>(&426).unwrap().requirements
        else {
            unreachable!()
        };
        let progress = Kc3rdQuestRequirement::And(vec![current])
            .calculate_progress(&Kc3rdQuestRequirement::And(master.clone()));
        assert_eq!(progress, emukc_internal::model::profile::quest::QuestProgressStatus::Half);
    }
}
