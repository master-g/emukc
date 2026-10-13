//! What the resource cells give and the maelstroms take (issue #96).
//!
//! The figures are wikiwiki's, by way of `map_cell_effects.json`.

#[cfg(test)]
mod tests {
    use emukc_internal::{
        crypto::rng,
        db::{
            entity::profile::{item::slot_item, ship},
            sea_orm::{ActiveModelTrait, ActiveValue, EntityTrait, IntoActiveModel},
        },
        gameplay::game::{SortieItemGet, SortieNextResponse},
        model::{codex::map::CellItem, profile::material::Material},
    };

    /// ドラム缶(輸送用).
    const DRUM: i64 = 75;
    /// 大発動艇.
    const DAIHATSU: i64 = 68;
    /// 特大発動艇+戦車第11連隊, which is no landing craft at a resource cell.
    const TANK_REGIMENT: i64 = 230;
    /// 13号対空電探.
    const RADAR: i64 = 27;

    async fn profile(context: &crate::TestContext, name: &str) -> i64 {
        let account = context.sign_up(name, "1234567").await.unwrap();
        let profile = context.new_profile(&account.access_token.token, name).await.unwrap();
        context
            .start_game(&account.access_token.token, profile.profile.id)
            .await
            .unwrap()
            .profile
            .id
    }

    /// `count` ships in the first fleet.
    async fn fleet(context: &crate::TestContext, pid: i64, count: usize) -> Vec<i64> {
        fleet_of(context, pid, &vec![951; count]).await
    }

    /// The first fleet, made of these ships.
    async fn fleet_of(context: &crate::TestContext, pid: i64, ships: &[i64]) -> Vec<i64> {
        let mut ids = Vec::new();
        let mut slots = [-1; 6];
        for (slot, &mst) in slots.iter_mut().zip(ships) {
            *slot = context.add_ship(pid, mst).await.unwrap().api_id;
            ids.push(*slot);
        }
        context.update_fleet_ships(pid, 1, &slots).await.unwrap();
        ids
    }

    /// What the fleet met on arriving, and what headquarters held just before.
    struct Arrival {
        next: SortieNextResponse,
        before: Material,
    }

    /// Bring the fleet to node `label` of `map_id`: park it on a cell that leads there and
    /// step, as often as the compass takes it elsewhere. `prepare` runs before each try.
    async fn arrive(
        context: &crate::TestContext,
        pid: i64,
        map_id: i64,
        label: &str,
        prepare: impl AsyncFn(),
    ) -> Arrival {
        let definition = context.codex().maps.map_definition(map_id).unwrap();
        let stage = definition.stage(&definition.default_variant).unwrap();
        let targets: Vec<i64> = stage
            .cells
            .iter()
            .filter(|cell| cell.node_label.as_deref() == Some(label))
            .map(|cell| cell.cell_no)
            .collect();
        let from: Vec<i64> = stage
            .cells
            .iter()
            .filter(|cell| cell.next_cells.iter().any(|next| targets.contains(next)))
            .map(|cell| cell.cell_no)
            .collect();
        assert!(!from.is_empty(), "nothing leads to {map_id} {label}");
        for &from in from.iter().cycle().take(600) {
            context.clear_sortie_state_if_any(pid).await;
            context.start_sortie(pid, 1, 1, 1).await.unwrap();
            let store = context.sortie_store();
            let mut active = store.get_active(pid).unwrap();
            active.map_id = map_id;
            active.stage_id = definition.default_variant.clone();
            active.current_cell_id = from;
            active.boss_cell_id = stage.boss_cell_no;
            active.visited_cell_ids = [from].into();
            active.locked_enemy_composition = None;
            let _ = store.insert_active(pid, active);
            prepare().await;
            let before = context.get_materials(pid).await.unwrap();
            let next = context.next_sortie(pid, None).await.unwrap();
            if targets.contains(&next.cell_no) {
                return Arrival {
                    next,
                    before,
                };
            }
        }
        panic!("the compass never led to {map_id} {label}");
    }

    fn gains(arrival: &Arrival) -> &[SortieItemGet] {
        arrival.next.itemget.as_deref().unwrap_or_default()
    }

    #[tokio::test]
    async fn a_resource_cell_gives_its_own_resource_within_its_range() {
        let context = crate::TestContext::new().await;
        let pid = profile(&context, "cell-kind").await;
        fleet(&context, pid, 6).await;

        // 1-2 B: 弾薬 10 to 20; 2-2 A and F: ボーキサイト 10 to 20 and 15 to 35; 1-4 C: 鋼材
        // 10 to 20.
        for (map_id, label, item, range) in [
            (12, "B", CellItem::Ammo, 10..=20),
            (22, "A", CellItem::Bauxite, 10..=20),
            (22, "F", CellItem::Bauxite, 15..=35),
            (14, "C", CellItem::Steel, 10..=20),
        ] {
            let arrival = arrive(&context, pid, map_id, label, async || {}).await;
            let [gain] = gains(&arrival) else {
                panic!("{map_id} {label}: {:?}", arrival.next.itemget);
            };
            assert_eq!(gain.item, item, "{map_id} {label}");
            assert!(range.contains(&gain.amount), "{map_id} {label}: {}", gain.amount);
            assert_eq!(gain.amount % 5, 0, "{map_id} {label}: {}", gain.amount);
        }
    }

    /// Put a new piece of equipment in slot `slot` (1 to 5) of a ship.
    async fn equip(context: &crate::TestContext, pid: i64, ship_id: i64, slot: usize, mst: i64) {
        let item = context.add_slot_item(pid, mst, 0, 0).await.unwrap().api_id;
        let row = ship::Entity::find_by_id(ship_id).one(&*context.db).await.unwrap().unwrap();
        let mut am = row.into_active_model();
        match slot {
            1 => am.slot_1 = ActiveValue::Set(item),
            2 => am.slot_2 = ActiveValue::Set(item),
            3 => am.slot_3 = ActiveValue::Set(item),
            4 => am.slot_4 = ActiveValue::Set(item),
            _ => am.slot_5 = ActiveValue::Set(item),
        }
        am.update(&*context.db).await.unwrap();
        let row = slot_item::Entity::find_by_id(item).one(&*context.db).await.unwrap().unwrap();
        let mut am = row.into_active_model();
        am.equip_on = ActiveValue::Set(ship_id);
        am.update(&*context.db).await.unwrap();
    }

    /// Leave each ship with the fuel and ammunition named for her.
    async fn stock(context: &crate::TestContext, ships: &[i64], left: &[(i64, i64)]) {
        for (&ship_id, &(fuel, ammo)) in ships.iter().zip(left) {
            let row = ship::Entity::find_by_id(ship_id).one(&*context.db).await.unwrap().unwrap();
            let mut am = row.into_active_model();
            am.fuel = ActiveValue::Set(fuel);
            am.ammo = ActiveValue::Set(ammo);
            am.update(&*context.db).await.unwrap();
        }
    }

    async fn left(context: &crate::TestContext, ships: &[i64]) -> Vec<(i64, i64)> {
        let mut left = Vec::new();
        for &ship_id in ships {
            let row = ship::Entity::find_by_id(ship_id).one(&*context.db).await.unwrap().unwrap();
            left.push((row.fuel, row.ammo));
        }
        left
    }

    #[tokio::test]
    async fn drums_and_landing_craft_add_to_what_a_resource_cell_gives() {
        let context = crate::TestContext::new().await;
        let pid = profile(&context, "cell-bonus").await;
        let ships = fleet(&context, pid, 6).await;
        equip(&context, pid, ships[0], 1, DRUM).await;
        equip(&context, pid, ships[1], 1, DAIHATSU).await;
        equip(&context, pid, ships[2], 1, TANK_REGIMENT).await;

        // 弾薬: 2 for a drum and 3 for a landing craft, so 5 more.
        let arrival = arrive(&context, pid, 12, "B", async || {}).await;
        let amount = gains(&arrival)[0].amount;
        assert!([15, 20, 25].contains(&amount), "1-2 B: {amount}");

        // ボーキサイト: 1.5 and 2, and the 3.5 is rounded down to 3 more.
        let arrival = arrive(&context, pid, 22, "F", async || {}).await;
        let amount = gains(&arrival)[0].amount;
        assert!([18, 23, 28, 33, 38].contains(&amount), "2-2 F: {amount}");
    }

    #[tokio::test]
    async fn a_cell_with_a_formula_of_its_own_follows_it_up_to_its_cap() {
        let context = crate::TestContext::new().await;
        let pid = profile(&context, "cell-formula").await;
        let ships = fleet(&context, pid, 6).await;
        equip(&context, pid, ships[0], 1, DRUM).await;
        equip(&context, pid, ships[0], 2, DRUM).await;
        equip(&context, pid, ships[1], 1, DAIHATSU).await;

        // 5-5 E: 燃料 40, 15 for a drum and 10 for a landing craft.
        let arrival = arrive(&context, pid, 55, "E", async || {}).await;
        assert_eq!(
            gains(&arrival),
            [SortieItemGet {
                item: CellItem::Fuel,
                amount: 80
            }]
        );
        // 1-6 G: 弾薬 20 and 5 for a landing craft, drums nothing; M: 燃料 40 and 10.
        let arrival = arrive(&context, pid, 16, "G", async || {}).await;
        assert_eq!(gains(&arrival)[0].amount, 25);
        let arrival = arrive(&context, pid, 16, "M", async || {}).await;
        assert_eq!(gains(&arrival)[0].amount, 50);
        // 5-4 I: 弾薬 60, 10 for a drum and 15 for a landing craft.
        let arrival = arrive(&context, pid, 54, "I", async || {}).await;
        assert_eq!(
            gains(&arrival),
            [SortieItemGet {
                item: CellItem::Ammo,
                amount: 95
            }]
        );

        // Nine more drums and four more landing craft run both into their caps.
        for (index, &ship_id) in ships.iter().enumerate().skip(1) {
            for slot in 2..=3 {
                equip(&context, pid, ship_id, slot, DRUM).await;
            }
            if index > 1 {
                equip(&context, pid, ship_id, 1, DAIHATSU).await;
            }
        }
        let arrival = arrive(&context, pid, 55, "E", async || {}).await;
        assert_eq!(gains(&arrival)[0].amount, 180);
        let arrival = arrive(&context, pid, 16, "G", async || {}).await;
        assert_eq!(gains(&arrival)[0].amount, 40);
        let arrival = arrive(&context, pid, 16, "M", async || {}).await;
        assert_eq!(gains(&arrival)[0].amount, 80);
        let arrival = arrive(&context, pid, 54, "I", async || {}).await;
        assert_eq!(gains(&arrival)[0].amount, 180);
    }

    #[tokio::test]
    async fn the_port_cell_of_7_4_gives_fuel_and_bauxite_together() {
        let context = crate::TestContext::new().await;
        let pid = profile(&context, "cell-port").await;
        // Three submarines, as the route to the port asks.
        let ships = fleet_of(&context, pid, &[126, 127, 128]).await;
        // zekamashi's example: four landing craft and two drums bring 84 and 72.
        for (ship, slot, mst) in [
            (0, 1, DAIHATSU),
            (0, 2, DAIHATSU),
            (1, 1, DAIHATSU),
            (1, 2, DAIHATSU),
            (2, 1, DRUM),
            (2, 2, DRUM),
            // 彩雲: the branch before the port turns back a fleet that sees too little.
            (2, 3, 54),
            (2, 4, 54),
        ] {
            equip(&context, pid, ships[ship], slot, mst).await;
        }

        let arrival = arrive(&context, pid, 74, "O", async || {}).await;
        assert_eq!(
            gains(&arrival),
            [
                SortieItemGet {
                    item: CellItem::Fuel,
                    amount: 84
                },
                SortieItemGet {
                    item: CellItem::Bauxite,
                    amount: 72
                },
            ]
        );
    }

    async fn use_item_count(context: &crate::TestContext, pid: i64, id: i64) -> i64 {
        let items = context.get_use_items(pid).await.unwrap();
        items.iter().find(|item| item.api_id == id).map_or(0, |item| item.api_count)
    }

    #[tokio::test]
    async fn special_materials_and_furniture_boxes_are_given_at_once() {
        let context = crate::TestContext::new().await;
        let pid = profile(&context, "cell-special").await;
        fleet(&context, pid, 6).await;

        // 2-1 E: one 高速建造材.
        let arrival = arrive(&context, pid, 21, "E", async || {}).await;
        assert_eq!(
            gains(&arrival),
            [SortieItemGet {
                item: CellItem::Torch,
                amount: 1
            }]
        );
        let after = context.get_materials(pid).await.unwrap();
        assert_eq!(after.torch, arrival.before.torch + 1);

        // 3-3 H: one 家具箱（大）.
        let boxes = use_item_count(&context, pid, 12).await;
        let arrival = arrive(&context, pid, 33, "H", async || {}).await;
        assert_eq!(
            gains(&arrival),
            [SortieItemGet {
                item: CellItem::BoxLarge,
                amount: 1
            }]
        );
        assert_eq!(use_item_count(&context, pid, 12).await, boxes + 1);
    }

    #[tokio::test]
    async fn fuel_ammunition_steel_and_bauxite_reach_headquarters_in_port() {
        let context = crate::TestContext::new().await;
        let pid = profile(&context, "cell-port-credit").await;
        fleet(&context, pid, 6).await;

        let arrival = arrive(&context, pid, 12, "B", async || {}).await;
        let gained = gains(&arrival)[0].amount;
        let at_sea = context.get_materials(pid).await.unwrap();
        assert_eq!(at_sea.ammo, arrival.before.ammo, "nothing reaches headquarters at sea");

        let port = context.port_view(pid).await.unwrap();
        assert_eq!(port.materials.ammo, arrival.before.ammo + gained);
        let again = context.port_view(pid).await.unwrap();
        assert_eq!(again.materials.ammo, arrival.before.ammo + gained, "and only once");
    }

    /// The client asks for `require_info` when the page loads and at no other time.
    #[tokio::test]
    async fn a_page_reloaded_at_sea_forfeits_what_the_fleet_gathered() {
        let context = crate::TestContext::new().await;
        let pid = profile(&context, "cell-reload").await;
        fleet(&context, pid, 6).await;

        let arrival = arrive(&context, pid, 12, "B", async || {}).await;
        assert!(gains(&arrival)[0].amount > 0);

        context.require_info_view(pid).await.unwrap();
        let port = context.port_view(pid).await.unwrap();
        assert_eq!(port.materials.ammo, arrival.before.ammo);
    }

    #[tokio::test]
    async fn a_maelstrom_takes_what_its_cell_says() {
        let context = crate::TestContext::new().await;
        let pid = profile(&context, "maelstrom-kind").await;
        let ships = fleet(&context, pid, 6).await;
        let full = [(100, 100); 6];

        // 3-2 G: 35% of fuel, 30 at most. 6-2 D: 40% of ammunition or more, 15 at most.
        for (map_id, label, item, after) in
            [(32, "G", CellItem::Fuel, (70, 100)), (62, "D", CellItem::Ammo, (100, 85))]
        {
            let arrival =
                arrive(&context, pid, map_id, label, async || stock(&context, &ships, &full).await)
                    .await;
            let happening = arrival.next.happening.expect("a maelstrom");
            assert_eq!(happening.item, item, "{map_id} {label}");
            assert_eq!(happening.amount, 100 - after.0.min(after.1), "{map_id} {label}");
            assert!(!happening.radar_reduced);
            assert_eq!(left(&context, &ships).await, [after; 6], "{map_id} {label}");
            assert!(arrival.next.itemget.is_none());
        }
    }

    /// wikiwiki 資材's two worked examples.
    #[tokio::test]
    async fn a_maelstrom_follows_the_worked_examples() {
        let context = crate::TestContext::new().await;
        let pid = profile(&context, "maelstrom-examples").await;
        let ships = fleet(&context, pid, 3).await;

        // 5-5 D, 35%, 80 at most, no radar: 253 left loses 88, cut to 80. The report is
        // of the ship that lost most.
        let arrival = arrive(&context, pid, 55, "D", async || {
            stock(&context, &ships, &[(253, 50), (100, 50), (0, 50)]).await;
        })
        .await;
        assert_eq!(arrival.next.happening.unwrap().amount, 80);
        assert_eq!(left(&context, &ships).await, [(173, 50), (65, 50), (0, 50)]);

        // The other example is 3-3 C's; 3-4 D takes the same 30%. Two ships with radar:
        // 80 left loses 14.
        equip(&context, pid, ships[1], 1, RADAR).await;
        equip(&context, pid, ships[1], 2, RADAR).await;
        equip(&context, pid, ships[2], 1, RADAR).await;
        let arrival = arrive(&context, pid, 34, "D", async || {
            stock(&context, &ships, &[(80, 50), (80, 50), (80, 50)]).await;
        })
        .await;
        let happening = arrival.next.happening.unwrap();
        assert_eq!(happening.amount, 14);
        assert!(happening.radar_reduced);
        assert_eq!(left(&context, &ships).await, [(66, 50); 3]);
    }

    #[tokio::test]
    async fn every_ship_with_radar_spares_the_fleet_more() {
        let context = crate::TestContext::new().await;
        let pid = profile(&context, "maelstrom-radar").await;
        let ships = fleet(&context, pid, 6).await;

        // 3-4 D: 30% and no cap. 25, 40, 50, 55, 58 and 60 in a hundred are spared.
        for (with_radar, lost) in [30, 22, 18, 15, 13, 12, 12].into_iter().enumerate() {
            if with_radar > 0 {
                equip(&context, pid, ships[with_radar - 1], 1, RADAR).await;
            }
            let arrival = arrive(&context, pid, 34, "D", async || {
                stock(&context, &ships, &[(100, 100); 6]).await;
            })
            .await;
            let happening = arrival.next.happening.unwrap();
            assert_eq!(happening.amount, lost, "{with_radar} ships with radar");
            assert_eq!(happening.radar_reduced, with_radar > 0);
            assert_eq!(left(&context, &ships).await, [(100 - lost, 100); 6]);
        }
    }

    #[tokio::test]
    async fn only_a_strong_maelstrom_s_cell_takes_the_great_loss() {
        rng::seed(1);
        let context = crate::TestContext::new().await;
        let pid = profile(&context, "maelstrom-strong").await;
        let ships = fleet(&context, pid, 1).await;

        // 5-2 M: 40%, or 150% once in a while, 100 at most; she has 100 to lose.
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..80 {
            let arrival = arrive(&context, pid, 52, "M", async || {
                stock(&context, &ships, &[(100, 100)]).await;
            })
            .await;
            seen.insert(arrival.next.happening.unwrap().amount);
        }
        assert_eq!(seen, [40, 100].into());

        // 3-4 D is no such cell.
        for _ in 0..80 {
            let arrival = arrive(&context, pid, 34, "D", async || {
                stock(&context, &ships, &[(100, 100)]).await;
            })
            .await;
            assert_eq!(arrival.next.happening.unwrap().amount, 30);
        }
    }

    /// Every resource cell and maelstrom of the regular maps has what it gives or takes,
    /// and no other cell has.
    #[test]
    fn every_resource_cell_and_maelstrom_has_its_entry() {
        let maps = &crate::CODEX.maps;
        let mut entries = 0;
        for map in maps.maps.values().filter(|map| !map.is_event) {
            for cell in map.variants.values().flat_map(|variant| &variant.cells) {
                let effect = maps.cell_effect(map.map_id, cell.cell_no);
                let place = format!("{} {:?}", map.name, cell.node_label);
                match cell.event_id {
                    2 => assert!(effect.is_some_and(|e| !e.gains.is_empty()), "{place}"),
                    3 => assert!(effect.is_some_and(|e| e.maelstrom.is_some()), "{place}"),
                    _ => assert!(effect.is_none(), "{place}"),
                }
                entries += usize::from(effect.is_some());
            }
        }
        let pinned: usize =
            maps.cell_effects.cells.values().map(std::collections::BTreeMap::len).sum();
        assert!(pinned > 0);
        assert!(entries >= pinned, "an entry sits on a cell no map has");
    }
}
