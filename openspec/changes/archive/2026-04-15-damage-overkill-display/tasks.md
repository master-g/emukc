## 1. Change `apply_damage` Return Type

- [x] 1.1 Change `apply_damage` signature in `core.rs:180` to return `(i64, i64)` — `(raw_damage, effective_damage)`. Sunk ship early return: `(0, 0)`. Normal path: `(raw_damage, raw_damage.min(self.current_hp))`. Protection path: `(raw_damage, proportional)`.
- [x] 1.2 Add a type alias `type DamageResult = (i64, i64)` or a struct `AppliedDamage { raw: i64, effective: i64 }` if tuple proves unclear at call sites. Prefer tuple unless readability suffers.

## 2. Update Shelling Phase Call Sites

- [x] 2.1 Day battle shelling (`simulate_shelling_side`, ~line 885): destructure `(raw, effective)`, use `raw` for `damage.push(vec![raw])` in hougeki recording, use `effective` for `ship.damage_dealt`.
- [x] 2.2 OASW shelling (`simulate_opening_taisen`, ~lines 1427, 1454): same pattern — raw for display, effective for damage_dealt.

## 3. Update Torpedo Phase Call Sites

- [x] 3.1 Opening torpedo (friendly/enemy, ~lines 937, 961): destructure result, pass `raw` to `payload.record_torpedo_hit`, use `effective` for `damage_dealt`.
- [x] 3.2 Closing torpedo (friendly/enemy, ~lines 999, 1023): same pattern.

## 4. Update Airstrike Phase Call Sites

- [x] 4.1 Kouku airstrike (`simulate_kouku`, ~lines 1165, 1182): use `raw` for `api_edam[target_idx]` and `api_fdam[target_idx]`, use `effective` for `damage_dealt`.

## 5. Update Night Battle Call Sites

- [x] 5.1 Night battle hougeki (`simulate_night_hougeki`, ~lines 2375, 2416): use `raw` for `hit_damages.push(raw)` and display, use `effective` for `total_dealt`.

## 6. Update Tests

- [x] 6.1 Update direct `apply_damage` test assertions in `core.rs` tests (~10 tests at lines 3642-3865): destructure `(raw, effective)` from `apply_damage`, assert `raw` >= `effective`, update expected values where overkill now returns raw.
- [x] 6.2 Add new test: `overkill_shows_raw_damage` — ship with 5 HP takes 100 damage, verify `(100, 5)` returned.
- [x] 6.3 Add new test: `protection_shows_raw_damage_but_reduces_hp_proportionally` — protected flagship takes lethal damage, verify raw matches input but HP drop is proportional.

## 7. Validation

- [x] 7.1 Run `cargo test -p emukc_gameplay` — all tests pass
- [x] 7.2 Run `cargo clippy --workspace` — no new warnings
