## 1. Investigation

- [ ] 1.1 Read practice battle result handler to verify pending_battle/pending_result cleanup
- [ ] 1.2 Inspect codex map data for map 1-3: verify next_cells and routing_rules against wikiwiki

## 2. Sortie State Cleanup

- [ ] 2.1 Add defensive cleanup at start of `start_sortie_impl`: remove_active_sortie + take_pending_result + take_pending_battle before inserting new state
- [ ] 2.2 Fix practice battle result handler to clear pending_battle and pending_result from SortieStore after processing (if missing)
- [ ] 2.3 Add test: start sortie after incomplete previous sortie — verify no stale state
- [ ] 2.4 Add test: start sortie after practice — verify no practice enemy data leaks

## 3. Map 1-3 Routing Fix

- [ ] 3.1 Fix 1-3 codex map data if next_cells/routing_rules are incorrect (depends on 1.2 findings)
- [ ] 3.2 If routing logic is at fault, fix select_route_from_cells to handle multi-edge cells correctly
- [ ] 3.3 Add test: 1-3 sortie routing follows valid edges only
- [ ] 3.4 Run `cargo test --test gameplay_tests` for integration pass
