//! Integration tests: a sortie quest's composition is judged at each battle,
//! on the fleet that fought it, not on the fleet held when the list is read
//! (#114).
//!
//! - 206 B6 — CL flagship and three DD, one battle anywhere: sailed for real.
//! - 997 B191 — three composition and sortie pairs, any one pair completes it.
//! - 317 C15 — exercises, then a sortie in the composition after them.
//! - 101 A1 — a composition with no sortie is still read off the current fleet.
//!
//! 997 and 317 run the codex's own conditions through
//! `apply_event_to_conditions` and `calculate_progress`: their maps want S and
//! A ranks at bosses a test fleet cannot reliably win.

#[cfg(test)]
mod tests {
    use emukc_internal::{
        db::entity::profile::quest::progress,
        gameplay::scenario::{Scenario, ShipSpec, apply_scenario},
        model::profile::quest::QuestProgressStatus,
        prelude::*,
    };

    /// The first player ship of one of the ship types.
    fn ship_of_type(stype: i64) -> i64 {
        crate::CODEX
            .manifest
            .api_mst_ship
            .iter()
            .find(|mst| mst.api_id < 1500 && mst.api_stype == stype)
            .map(|mst| mst.api_id)
            .unwrap()
    }

    /// The first player ship of one of the ship classes.
    fn ship_of_class(ctypes: &[i64]) -> i64 {
        crate::CODEX
            .manifest
            .api_mst_ship
            .iter()
            .find(|mst| mst.api_id < 1500 && ctypes.contains(&mst.api_ctype))
            .map(|mst| mst.api_id)
            .unwrap()
    }

    const DE: i64 = 1;
    const DD: i64 = 2;
    const CL: i64 = 3;

    fn fleet(mst_ids: &[i64]) -> Vec<FleetShipSnapshot> {
        mst_ids
            .iter()
            .enumerate()
            .map(|(idx, &mst_id)| FleetShipSnapshot {
                mst_id,
                level: 50,
                position: idx as i64 + 1,
            })
            .collect()
    }

    fn boss(
        area: i64,
        number: i64,
        win_rank: KcSortieResultRank,
        ships: &[i64],
    ) -> QuestActionEvent {
        QuestActionEvent::SortieBattleCompleted {
            maparea_id: area,
            mapinfo_no: number,
            gauge_phase: 1,
            boss_cell: true,
            node: None,
            win_rank,
            fleet_id: 1,
            fleet_ships: fleet(ships),
        }
    }

    fn master(quest_id: i64) -> Kc3rdQuestRequirement {
        crate::CODEX.find::<Kc3rdQuest>(&quest_id).unwrap().requirements.clone()
    }

    fn conditions(requirement: &Kc3rdQuestRequirement) -> &[Kc3rdQuestCondition] {
        let (Kc3rdQuestRequirement::And(conditions)
        | Kc3rdQuestRequirement::OneOf(conditions)
        | Kc3rdQuestRequirement::Sequential(conditions)) = requirement;
        conditions
    }

    /// `quest_id`'s conditions after `events`, and the progress they make.
    fn run(quest_id: i64, events: &[QuestActionEvent]) -> (Vec<i64>, QuestProgressStatus) {
        let requirement = master(quest_id);
        let mut current = conditions(&requirement).to_vec();
        for event in events {
            apply_event_to_conditions(
                &mut current,
                Some(conditions(&requirement)),
                event,
                Some(&crate::CODEX),
            );
        }
        let sorties = current
            .iter()
            .filter_map(|c| match c {
                Kc3rdQuestCondition::Sortie(sortie) => Some(sortie.times),
                _ => None,
            })
            .collect();
        let progress = match &requirement {
            Kc3rdQuestRequirement::And(_) => Kc3rdQuestRequirement::And(current),
            Kc3rdQuestRequirement::OneOf(_) => Kc3rdQuestRequirement::OneOf(current),
            Kc3rdQuestRequirement::Sequential(_) => Kc3rdQuestRequirement::Sequential(current),
        }
        .calculate_progress(&requirement);
        (sorties, progress)
    }

    /// Two S-rank wins at each of 1-3, 1-5 and 2-2's bosses with `ships`.
    fn b191_cycles(ships: &[i64]) -> Vec<QuestActionEvent> {
        let maps = [(1, 3), (1, 5), (2, 2)];
        maps.iter()
            .chain(&maps)
            .map(|&(area, number)| boss(area, number, KcSortieResultRank::S, ships))
            .collect()
    }

    #[test]
    fn b191_counts_each_composition_toward_its_own_sortie() {
        let dd = ship_of_type(DD);
        let (sorties, progress) = run(997, &b191_cycles(&[dd, dd, dd, dd]));

        assert_eq!(sorties, vec![2, 0, 2], "four DD are the second composition only");
        assert_eq!(progress, QuestProgressStatus::Completed);
    }

    #[test]
    fn b191_counts_nothing_for_a_fleet_of_none_of_its_compositions() {
        let cl = ship_of_type(CL);
        let (sorties, progress) = run(997, &b191_cycles(&[cl, cl, cl, cl]));

        assert_eq!(sorties, vec![2, 2, 2]);
        assert_eq!(progress, QuestProgressStatus::Empty);
    }

    #[test]
    fn b191_with_three_de_completes_the_first_pair() {
        let de = ship_of_type(DE);
        let (sorties, progress) = run(997, &b191_cycles(&[de, de, de]));

        assert_eq!(sorties, vec![0, 2, 2]);
        assert_eq!(progress, QuestProgressStatus::Completed);
    }

    #[test]
    fn c15_completes_with_its_exercises_and_a_sortie_in_its_composition() {
        let pair = ship_of_class(&[38, 30]);
        let dd = ship_of_type(DD);
        let exercise = QuestActionEvent::ExerciseBattleCompleted {
            fleet_id: 1,
            win_rank: KcSortieResultRank::S,
            fleet_ships: fleet(&[pair, pair, dd]),
        };
        let maps = [(2, 2), (2, 3), (2, 4), (2, 5)];
        let mut events = vec![exercise.clone(), exercise.clone(), exercise];

        let wrong: Vec<_> =
            maps.iter().map(|&(a, n)| boss(a, n, KcSortieResultRank::A, &[dd, dd, dd])).collect();
        events.extend(wrong.iter().cloned());
        let (sorties, progress) = run(317, &events);
        assert_eq!(sorties, vec![1], "the sortie wants two of the classes");
        assert_ne!(progress, QuestProgressStatus::Completed);

        events.extend(
            maps.iter().map(|&(a, n)| boss(a, n, KcSortieResultRank::A, &[pair, pair, dd])),
        );
        let (sorties, progress) = run(317, &events);
        assert_eq!(sorties, vec![0]);
        assert_eq!(progress, QuestProgressStatus::Completed);
    }

    async fn new_profile(context: &crate::TestContext, tag: &str) -> (i64, Vec<i64>) {
        let account = context.sign_up(&format!("test-scomp-{tag}"), "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "sortie-comp").await.unwrap();
        let pid = context
            .start_game(&account.access_token.token, profile.profile.id)
            .await
            .unwrap()
            .profile
            .id;
        let scenario = Scenario {
            fleet: vec![
                ShipSpec::new(ship_of_type(CL), 50),
                ShipSpec::new(ship_of_type(DD), 50),
                ShipSpec::new(ship_of_type(DD), 50),
                ShipSpec::new(ship_of_type(DD), 50),
            ],
            ..Scenario::fresh_1_1()
        };
        let ships = apply_scenario(context, pid, &scenario).await.unwrap();
        (pid, ships)
    }

    async fn set_fleet(context: &crate::TestContext, pid: i64, ships: &[i64]) {
        let mut slots = [-1; 6];
        slots[..ships.len()].copy_from_slice(ships);
        context.update_fleet_ships(pid, 1, &slots).await.unwrap();
    }

    /// Sail 1-1 to its end, fighting every battle cell; returns the battles
    /// fought. The fleet is resupplied first so a sail never runs dry.
    async fn sail_1_1(context: &crate::TestContext, pid: i64, ships: &[i64]) -> i64 {
        context.charge_supply(pid, ships, KcApiChargeKind::Collective, false).await.unwrap();
        let start = context.start_sortie(pid, 1, 1, 1).await.unwrap();
        let (mut event_id, mut has_next) = (start.event_id, start.has_next);
        let mut battles = 0;
        loop {
            if matches!(event_id, 4 | 5) {
                context.sortie_battle(pid, 1).await.unwrap();
                context.sortie_battle_result(pid).await.unwrap();
                battles += 1;
            }
            if !has_next {
                return battles;
            }
            let next = context.next_sortie(pid, None).await.unwrap();
            (event_id, has_next) = (next.event_id, next.has_next);
        }
    }

    async fn record(context: &crate::TestContext, pid: i64, quest_id: i64) -> progress::Model {
        let records = context.get_quest_records(pid).await.unwrap();
        records.into_iter().find(|r| r.quest_id == quest_id).expect("quest has a record")
    }

    #[tokio::test]
    async fn b6_sailed_in_the_wrong_order_does_not_count_afterwards() {
        let context = crate::TestContext::new().await;
        let (pid, ships) = new_profile(&context, "wrong").await;
        context.quest_add(pid, 206).await.unwrap();
        context.quest_start(pid, 206).await.unwrap();

        // A destroyer leads: B6 wants the light cruiser as flagship.
        let wrong = [ships[1], ships[0], ships[2], ships[3]];
        set_fleet(&context, pid, &wrong).await;
        assert!(sail_1_1(&context, pid, &wrong).await >= 1, "1-1 opens with a battle");

        // Putting the cruiser back in front afterwards does not complete it.
        set_fleet(&context, pid, &ships).await;
        let quest = record(&context, pid, 206).await;
        assert_ne!(quest.progress, progress::Progress::Completed);
        assert!(context.quest_clear_and_claim_reward(pid, 206, None).await.is_err());
    }

    #[tokio::test]
    async fn b6_sailed_right_stays_complete_after_the_fleet_changes() {
        let context = crate::TestContext::new().await;
        let (pid, ships) = new_profile(&context, "right").await;
        context.quest_add(pid, 206).await.unwrap();
        context.quest_start(pid, 206).await.unwrap();

        assert!(sail_1_1(&context, pid, &ships).await >= 1, "1-1 opens with a battle");
        set_fleet(&context, pid, &[ships[1], ships[0]]).await;

        let quest = record(&context, pid, 206).await;
        assert_eq!(quest.progress, progress::Progress::Completed);
        context.quest_clear_and_claim_reward(pid, 206, None).await.unwrap();
    }

    #[tokio::test]
    async fn a1_is_still_read_off_the_current_fleet() {
        let context = crate::TestContext::new().await;
        let (pid, ships) = new_profile(&context, "a1").await;
        context.quest_add(pid, 101).await.unwrap();
        context.quest_start(pid, 101).await.unwrap();
        assert_eq!(record(&context, pid, 101).await.progress, progress::Progress::Completed);

        // A1 wants two ships or more; one alone takes it back.
        set_fleet(&context, pid, &ships[..1]).await;
        assert_ne!(record(&context, pid, 101).await.progress, progress::Progress::Completed);
    }
}
