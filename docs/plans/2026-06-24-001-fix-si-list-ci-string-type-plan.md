---
title: "fix: api_si_list CI entries must serialize as JSON strings"
status: completed
created: 2026-06-24
type: fix
origin: user-reported CV cut-in display bug + official server data analysis
---

# Fix: api_si_list CI Entries Must Serialize as JSON Strings

## Problem Frame

During CV Cut-In (戦爆連合, `at_type=7`), the client only shows one type of plane instead of the full aircraft formation. Analysis of official server battle data (`/Users/mg/Downloads/kcsapi/battle*.txt`) reveals the root cause: **CI entries in `api_si_list` must be JSON strings, not integers.**

**Client dispatch mechanics (from main.js reverse-engineering):** The client's hougeki renderer dispatches to `_kuboCI` based on `api_at_type == 7` (day) or `api_sp_list == 6` (night), NOT on si_list JSON type. Our code sets `at_type=7` correctly, so the client **does** enter the carrier CI rendering path. The rendering bug occurs *within* that path: `PreloadCutinKubo._loadSlotTextImage` and `CutinKuboDayCanvas.initialize` use the raw si_list values as resource cache keys. In JavaScript, the string `"22"` and the number `22` are different object keys — a resource cached under numeric key `22` will not be found when looking up string key `"22"`, causing plane sprites and equipment card images to be missing.

Our code uses `Vec<Vec<i64>>` for `api_si_list`, which serializes everything as integers. Matching the official server's string format ensures the resource lookup chain works correctly.

## Evidence from Official Server Data

Raw JSON from `battle(2).txt`:

```json
"api_at_type":[7,0,0,0,0,0,0,0],
"api_si_list":[["22","291","112"],[1505],[161],[63],[266],[-1],[-1],[147]]
```

- CI entry (`at_type=7`): `["22","291","112"]` — **strings**, 3 master IDs (FBA pattern)
- Normal entries: `[1505]`, `[161]`, etc. — **integers**, single master ID

Verified mappings from `require_info.txt`:

- `"22"` = fighter (instance 2715 → master 22)
- `"291"` = dive bomber (instance 42130 → master 291)
- `"112"` = torpedo bomber (instance 8888 → master 112)
- `[161]` = main gun (instance 18776 → master 161)

Both CI and normal entries use **master IDs** (`api_slotitem_id`). Our code already uses master IDs correctly. The only bug is the JSON type.

---

## Scope

### In Scope

- Change `api_si_list` packet types to support mixed string/integer serialization
- CI display ID entries serialize as strings; normal entries stay as integers
- Covers day battle (`BattleHougeki`) and night battle (`BattleNightHougeki`)
- Update all call sites that construct `api_si_list` entries
- Update tests and golden transcript assertions

### Out of Scope

- Changing the ID value type (master ID is correct — verified)
- Enemy pseudo-instance ID allocation (enemy `api_id=0` does not affect `api_si_list` since it uses master IDs)
- Other API fields that may have type quirks

#### Deferred to Follow-Up Work

- Confirm night battle CI (`api_sp_list > 0`) string requirement with an official night battle capture — the current dataset only has day battles. The plan applies strings proactively based on main.js analysis showing all CI types share the same server-side serialization path.

---

## Key Technical Decisions

### KTD-1: SiListId enum with `#[serde(untagged)]`

Use an untagged enum to represent each ID in `api_si_list`:

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub(crate) enum SiListId {
    Num(i64),
    Text(String),
}
```

- `Num(i64)` serializes as a JSON number (normal attacks)
- `Text(String)` serializes as a JSON string (CI entries)

`#[serde(untagged)]` ensures clean JSON output without type tags.

**Alternative considered:** `serde_json::Value` — rejected for losing type safety and intent clarity.

### KTD-2: Conversion at each `si_list.push()` call site

The display ID helper functions (`carrier_ci_display_ids`, `night_attack_display_ids`, `day_attack_display_ids`) continue to return `Vec<i64>` internally. Conversion to `SiListId` happens at each `si_list.push()` call in the simulation code (`shelling.rs`, `night.rs`, etc.), not at a single batch conversion at packet construction time.

This means each `push_attack` call site wraps its display IDs into `Vec<SiListId>` before pushing. The `push_attack` function's `si_list: &mut Vec<Vec<i64>>` parameter must also change to `&mut Vec<Vec<SiListId>>`. All four `let mut si_list = Vec::new()` declarations (shelling.rs, asw.rs, night.rs, special_attack.rs) will infer `Vec<Vec<SiListId>>` from the new push type.

Rationale: keeps the helper functions pure and testable; the string/integer distinction is a serialization concern that belongs at the point of data entry into the battle packet.

**Sentinel handling:** The empty-equipment sentinel — `carrier_ci_display_ids` / `day_attack_display_ids` return `vec![-1]` when there is no display equipment — must always convert to `SiListId::Num(-1)`, even inside CI/special-attack branches. The official capture shows `-1` entries as integers (`[-1]`), so `-1` must be special-cased before stringifying. This applies to the simulation branches as well as the `debug_overlay.rs` placeholders.

### KTD-3: String convention for all CI/special attack types

Apply string serialization to all entries where the attack type indicates a special attack:

- Day carrier CI (`at_type == 7`)
- Day artillery spotting CI (`at_type >= 3`)
- Day double attack (`at_type == 2`)
- Night CI (`api_sp_list > 0`)
- Night double attack

**Implementation note:** Match these categories by the attack *branch* at each call site, not by a numeric `api_at_type` comparison. `api_at_type == 7` is overloaded — it is carrier CI inside `shelling.rs` but ASW inside `asw.rs` (which pushes `at_type = 7`). A shared `match at_type` converter would wrongly stringify ASW; ASW entries stay integer. See U2.

**Evidence base:** Official server data confirms strings for carrier CI (`at_type=7`) in `battle(2).txt`. The `battle(3).txt` data (no CI, all `at_type=0`) confirms integers for normal attacks. main.js reverse-engineering shows the hougeki dispatch routes `at_type=7` to `_kuboCI`, which passes 3 si_list values to `CutinKuboDay`/`CutinKuboNight`. The `_createPlanes` method uses `this._attacker.slots` directly, but the cutin canvas (`CutinKuboDayCanvas`) uses the si_list values for equipment card display with `initialize(this._slot_mst_id1, ...)`.

**Rationale for broad scope:** The official server's PHP backend applies a uniform serialization policy — all special-attack si_list entries are stringified, not just carrier CI. This is consistent with how PHP `json_encode` handles mixed-type arrays. Applying strings only to carrier CI would create an inconsistent format that differs from the official server's behavior for other CI types. If testing reveals a specific type that should remain integer, it can be reverted per-type.

---

## Implementation Units

### U1. Add SiListId type and update packet structs

**Goal:** Introduce a serializable type that can emit JSON numbers or strings, and update `BattleHougeki` / `BattleNightHougeki` to use it.

**Dependencies:** None

**Files:**

- `crates/emukc_battle/src/types/packet.rs` — add `SiListId` enum, change `api_si_list` field type in both structs

**Approach:**

- Add `SiListId` enum with `Num(i64)` and `Text(String)` variants, `#[serde(untagged)]`
- Change `api_si_list: Vec<Vec<i64>>` → `api_si_list: Vec<Vec<SiListId>>` in `BattleHougeki`
- Change `api_si_list: Vec<Vec<i64>>` → `api_si_list: Vec<Vec<SiListId>>` in `BattleNightHougeki`
- Add `From<i64>` impl for ergonomic construction: `SiListId::from(161)` → `Num(161)`
- Add helper: `SiListId::text_from_i64(ids: &[i64]) -> Vec<SiListId>` for CI entries

**Patterns to follow:** Existing serde derive patterns in the same file.

**Test scenarios:**

- Serialize `vec![vec![SiListId::Num(161)]]` → assert JSON output contains `[161]` (integer)
- Serialize `vec![vec![SiListId::Text("291".into()), SiListId::Text("112".into())]]` → assert JSON output contains `["291","112"]` (strings)
- Round-trip: a packet with mixed entries serializes to valid JSON matching the official format
- Edge case: `SiListId::Num(-1)` serializes as `-1` (not `"-1"`)

**Verification:** `cargo build -p emukc_battle` compiles; serialization tests pass.

---

### U2. Update shelling simulation to produce correct si_list types

**Goal:** Day battle shelling pushes string-typed entries for CI attacks and integer-typed entries for normal attacks.

**Dependencies:** U1

**Files:**

- `crates/emukc_battle/src/simulation/shelling.rs` — update `push_attack` signature (`si_list` param and `display_ids` param), all push sites, and the local `let mut si_list = Vec::new()` declaration
- `crates/emukc_battle/src/simulation/asw.rs` — update si_list entries (always `SiListId::Num`, no CI), and the local `let mut si_list = Vec::new()` declaration
- `crates/emukc_battle/src/simulation/special_attack.rs` — day special attacks (Nelson Touch, Nagato, etc.) build a day `BattleHougeki` whose `si_list` is `extend`ed into shelling (shelling.rs `si_list.extend(...)`); convert its si_list construction to `SiListId::Text` and update the local `let mut si_list = Vec::new()` declaration. Moved here from U3 because it is day-battle-coupled, not night.

**Approach:**

- Change `push_attack`'s `display_ids` parameter from `Vec<i64>` to `Vec<SiListId>`
- **Drive the Text/Num choice by the call-site attack category, not by the raw `api_at_type` integer** (`api_at_type == 7` means carrier CI here but ASW in `asw.rs` — see KTD-3 implementation note)
- At carrier CI branch: convert `carrier_ci_display_ids` result to `SiListId::Text` variants
- At artillery spotting CI branch (`MainSecCI`/`MainRadarCI`/`MainApSecCI`/`MainApMainCI`, i.e. `at_type` 3-6): convert `day_attack_display_ids` result to `SiListId::Text` variants
- At double attack branch (`at_type == 2`): convert to `SiListId::Text` variants
- At normal attack branch (`at_type == 0`): convert to `SiListId::Num` variants
- At ASW branch (`asw.rs`, despite `at_type == 7`): convert to `SiListId::Num` variants (no CI)
- Special attack path (`special_attack.rs`): entries should be `SiListId::Text` for special attacks

**Per-call-site mapping** — pick the variant where each `si_list` entry is built, keyed on the helper + attack category (not raw `api_at_type`); `push_attack` itself just forwards the already-typed `Vec<SiListId>`:

- `carrier_ci_display_ids` (carrier CI, `at_type == 7` in shelling.rs) → `Text`
- `day_attack_display_ids` in a CI / double-attack branch (`at_type` 2 or 3-6) → `Text`
- `day_attack_display_ids` in a normal-attack branch (`at_type == 0`) → `Num`
- `day_attack_display_ids` in `asw.rs` (ASW, `at_type == 7`) → `Num`
- `special_attack.rs` entries (merged into shelling via `si_list.extend`) → `Text`
- In every branch, the `-1` empty-equipment sentinel stays `Num(-1)` per KTD-2

**Test scenarios:**

- Carrier CI (at_type=7): si_list entry contains string variants — assert serialized JSON shows `["22","291","112"]` pattern for FBA
- Normal attack (at_type=0): si_list entry contains number variants — assert serialized JSON shows `[161]`
- ASW attack: si_list entry contains number variants
- Double attack (at_type=2): si_list entry contains string variants
- Special attack: si_list entries contain string variants

**Verification:** `cargo test -p emukc_battle` passes; golden transcript still matches (after U4 update).

---

### U3. Update night battle and debug overlay

**Goal:** Night battle si_list entries follow the same string/integer convention. Debug overlay and transcript rendering also updated.

**Dependencies:** U1

**Files:**

- `crates/emukc_battle/src/simulation/night.rs` — update `night_attack_display_ids` push sites
- `crates/emukc_battle/src/debug_overlay.rs` — update si_list construction (2 sites)
- `crates/emukc_battle/src/transcript.rs` — update **test fixtures only** (lines ~372, ~383, ~476) to use `SiListId`-compatible `api_si_list` values. The rendering functions (`render_hougeki`, `render_night_hougeki`) do **not** reference `api_si_list` and require no changes.

**Approach:**

- Night battle: when `api_sp_list > 0` (CI), push `SiListId::Text` variants; otherwise `SiListId::Num`
- Night double attack: push `SiListId::Text` variants
- Debug overlay: convert existing `vec![-1]` placeholders to `SiListId::Num(-1)`
- Transcript: no rendering changes — `render_hougeki` / `render_night_hougeki` do not reference `api_si_list`; only the test-fixture `api_si_list` literals listed under **Files** are touched (consistent with the Files note)

**Test scenarios:**

- Night CI (sp_list > 0): si_list entry contains string variants
- Night normal (sp_list = 0): si_list entry contains number variants
- Night double attack: si_list entry contains string variants
- Debug overlay produces valid JSON with mixed types
- Transcript test: `night_battle_renders_midnight_and_cutin` still passes

**Verification:** `cargo test -p emukc_battle` passes; all night battle tests green.

---

### U4. Update tests and golden transcript

**Goal:** All existing tests pass with the new si_list type; golden transcript assertions updated if needed.

**Dependencies:** U2, U3

**Files:**

Files that actually reference `api_si_list` (verified by grep) — these need real updates:

- `crates/emukc_battle/src/simulation/shelling.rs` — update unit tests that construct/assert `api_si_list`
- `crates/emukc_battle/src/simulation/asw.rs` — update the `taisen` unit test asserting si_list
- `crates/emukc_battle/src/simulation/night.rs` — update night-attack unit tests
- `crates/emukc_battle/src/simulation/day_cutin.rs` — `carrier_ci_display_ids` return type is unchanged; add downstream conversion tests only if useful
- `crates/emukc_battle/src/transcript.rs` — test fixtures only (the `vec![]` / `vec![vec![], vec![]]` literals already compile as `Vec<Vec<SiListId>>`; touch only if a value needs a concrete variant)

Format-correctness coverage comes from the **new serialization test** added in U1/U2, not from the integration suites. `tests/gameplay_tests/battle_golden.rs`, `crates/emukc_gameplay/tests/sortie_battle.rs`, `tests/gameplay_tests/map/sortie_battle.rs`, and `crates/emukc_gameplay/tests/practice_battle.rs` contain **no** `api_si_list` assertions (verified by grep), so they only need to keep compiling and passing — no si_list edits expected.

**Approach:**

- Any test that directly constructs `api_si_list` with `Vec<Vec<i64>>` must use `Vec<Vec<SiListId>>`
- Tests that check serialized JSON output should verify the string/integer distinction
- Golden transcript: `render_hougeki` / `render_night_hougeki` do not reference `api_si_list`, so the golden string is unaffected — just confirm the existing golden assertions (damage / rank / HP) still pass; no si_list-driven rendering changes (consistent with U3)

**Test scenarios:**

- All existing `cargo test -p emukc_battle` tests pass
- All existing `cargo test --test gameplay_tests` tests pass
- New test: serialized battle packet with carrier CI contains string-typed si_list entries (integration-level)

**Verification:** `cargo test -p emukc_battle && cargo test --test gameplay_tests` all green; no warnings from clippy.

---

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Non-carrier CI types (artillery spotting, night CI) may not need strings | Low | Low (extra strings don't break rendering — client uses `==` loose equality in JS) | Applied based on main.js analysis showing uniform server serialization; can be reverted per-type |
| Night CI string requirement is unverified | Low | Low | main.js shows `CutinKuboNight` shares the same code path as `CutinKuboDay`; applied proactively |
| Golden transcript changes | Low | Low | Transcript rendering handles both types; assertions focus on damage/attack type, not si_list values |
| Serde untagged enum edge cases | Low | Medium | Unit tests on serialization output format |

---

## System-Wide Impact

- **Battle simulation output:** JSON format change in `api_si_list` for CI entries (integer → string)
- **Client compatibility:** Fixes carrier CI animation; no regression for normal attacks
- **No database changes:** ID values (master IDs) remain unchanged
- **No config changes:** Behavior is determined by attack type at simulation time

---

## Deferred / Open Questions

### From 2026-06-24 review

- **[P1] Broad-scope string policy is unverified beyond `at_type=7` (KTD-3 / Scope).** The captured battle data (`battle(2).txt`, `battle(3).txt`) only contains `at_type` 7 and 0. Stringifying `at_type` 2/3-6 and all night CI rests on a "uniform PHP `json_encode`" inference, not direct observation. Before relying on it in production, decide whether to (a) keep the proactive broad bet with the documented per-type revert path, or (b) narrow to the verified carrier-CI case until more captures exist. *(adversarial, confidence 100 — root of the chain below)*
- **[P2] Night CI strings applied proactively despite zero night captures (Deferred / KTD-3 / U3).** The plan defers night-CI confirmation to follow-up yet implements it now, betting `CutinKuboNight` shares `CutinKuboDay`'s serialization. Resolves together with the broad-scope item above. *(adversarial, confidence 75 — depends on the broad-scope decision)*
- **[P2] Test scenarios for `at_type` 2 and 3-6 cannot be validated against current captures (U2 / U4).** Any fixture for these types only re-asserts the KTD-3 assumption; it does not validate against real server output. Mark such tests as explicitly synthetic-pending-capture. Resolves together with the broad-scope item above. *(adversarial, confidence 75 — depends on the broad-scope decision)*
- **[P1] Special-attack si_list stringification is unverified (`at_type` 100-106) (U2 / KTD-3).** The broad-scope item above covers `at_type` 2/3-6 + night CI; special attacks (Nelson Touch, Nagato, Colorado, Richelieu, Queen Elizabeth) use a *separate* `at_type` range that appears in no capture, yet U2 prescribes `SiListId::Text` for them and KTD-3's "uniform PHP" rationale only argues the 0-7 range. Treat as part of the same broad-scope bet: keep the proactive string posture with a per-type revert path, or hold special attacks at `Num` until a capture exists. *(scope-guardian + adversarial, confidence 100 — round-2 finding)*
