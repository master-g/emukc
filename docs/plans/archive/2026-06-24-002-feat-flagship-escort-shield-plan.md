---
title: "feat: Flagship escort shield (旗艦援護/かばう)"
status: completed
created: 2026-06-24
type: feat
depth: deep
origin: docs/brainstorms/2026-06-24-flagship-escort-shield-requirements.md
---

# feat: Flagship Escort Shield (旗艦援護/かばう)

When an attack targets a fleet's flagship, an escort ship may intercept and take
the damage instead. The server decides interception via a formation-based
probability roll, swaps the target to a healthy same-category escort, and encodes
the interception as a `.1` decimal suffix on the reported damage so the client
plays its shield animation. Bidirectional (friendly and enemy fleets both
intercept). Combined-fleet restrictions are out of scope.

Coverage decided at planning: **all phases the brainstorm names** — day/night
shelling (`api_damage`) and opening torpedo (`api_fydam`/`api_eydam`). Enablement:
**always on**; the new RNG draws shift the deterministic battle sequence, so all
golden baselines are deliberately re-frozen (see U6).

---

## Problem Frame

Official server battle data (`~/Downloads/kcsapi/battle*.txt`) carries fractional
damage values (e.g. `198.1`, `6.1`) in `api_hougeki.api_damage`. The client's
`HougekiData.isShield(i)` checks `damage % 1 > 0` to detect them and renders a
shield banner on the flagship; `getDamage(i)` returns `Math.floor(damage)` for the
actual HP change. The same pattern drives `RaigekiData.isShield_f/_e` over
`api_fydam`/`api_eydam` (there it is `% 1 != 0`).

Our emulator has zero support for this:

- `api_damage` is `Vec<Vec<i64>>` (and raigeki ydam fields are `Vec<i64>` /
  `Vec<Option<Vec<i64>>>`), so the `.1` flag cannot be encoded.
- Target selection (`targeting::select_random_target_index`) never attempts
  interception — the flagship takes damage that escorts should have absorbed.

Result: the client never plays the shield animation, and flagship HP diverges
from the official server.

---

## Requirements Trace

Carried from origin (`see origin` for full text). All requirements are in scope
**except** the combined-fleet items.

| ID | Requirement | Unit(s) |
|----|-------------|---------|
| R1 | Roll interception when an attack targets the flagship (defender index 0) | U2, U3, U4 |
| R2 | Formation rates: 単縦 45%, 複縦/梯形/単横 60%, 輪形/警戒 75% | U2 |
| R3 | Requires ≥1 non-flagship escort in green health (小破未満, HP > 75%) | U2 |
| R4 | Interceptor chosen **uniformly at random** among eligible escorts (resolved — see KTD-1) | U2 |
| R5 | Type match: surface protects surface flagship, submarine protects submarine | U2 |
| R6 | `.1` decimal suffix encodes the shield flag; integer part is real damage | U1 |
| R7 | `api_df_list` shows the interceptor's index, not the flagship's | U3, U4 |
| R8 | Applies to day/night hougeki (incl. day-ASW and special-attack) **and** opening torpedo | U3, U4 |
| R9 | Bidirectional — friendly and enemy fleets both intercept | U2, U3, U4 |
| R10 | Combined-fleet escort flagship not protected in day battle | **Out of scope** (combined fleet unimplemented) |

Acceptance Examples AE1–AE5 map to test scenarios in U2/U3/U4 (see each unit).

---

## Key Technical Decisions

**KTD-1 — Interceptor selection is uniform random among eligible escorts.**
Resolved from the authoritative wiki (`wikiwiki.jp/kancolle/攻撃対象の選択`):
*"発動判定に成功した場合にかばう艦の判定を行う(旗艦以外からランダムで1隻選択)"*,
candidate set *"小破未満の僚艦"*. So selection is **not** slot-order — it is uniform
over eligible escorts, implemented with the existing `BattleRng::choose_index`.

**KTD-2 — Damage carries a shield flag via a wire-encoding type, internal math
stays `i64`.** Mirror the just-landed `SiListId` precedent in
`crates/emukc_battle/src/types/packet.rs`: an enum with a hand-written `Serialize`
that emits a plain integer normally and a `.1`-suffixed float when shielded. All
damage calculation and HP application stay integer — only the wire value becomes
fractional. **Rejected:** `f64` everywhere (precision risk threading through the
whole damage pipeline, and `apply_damage`/HP are integer by contract — see
`docs/solutions/architecture-patterns/battle-damage-foundation.md`).

**KTD-3 — Interception is resolved at targeting time; the integer part follows the
existing `display_damage` policy.** The swap happens between target selection and
damage calculation, so the reported integer is whatever `display_damage` already
produces for the *escort* defender — **dealt** (effective, post-protection) for a
friendly escort, **raw** (overkill) for an enemy escort (see
`targeting::display_damage`). The `.1` shield flag is layered orthogonally on top of
that value and does not change the raw-vs-dealt choice. Consequence: the
"interceptor HP drops by the reported integer" identity holds only for **friendly**
escorts; enemy escorts report raw overkill, so U3/U4 scope the HP assertion to the
friendly side.

**KTD-4 — Interception rolls always fire; golden baselines are re-frozen.**
The probability + selection draws insert into the deterministic RNG stream, so
every battle golden transcript changes. This is a deliberate, PR-documented
re-freeze (U6), not a per-assertion hand-patch. Draw order is fixed (KTD-5) so the
re-freeze is reproducible.

**KTD-5 — Fixed RNG draw order for reproducibility.** At a flagship-targeted hit:
(1) compute eligibility with **no** RNG draw; (2) if no eligible escort, draw
nothing and proceed normally; (3) otherwise draw **one** `roll_range` for the
formation probability; (4) on success, draw **one** `choose_index(eligible.len())`
for the interceptor. This mirrors the existing `choose_index(0)` "no draw when
empty" invariant in `random.rs`.

---

## High-Level Technical Design

Per-hit flow at every target site (day shelling, day-ASW, special attack, night
shelling, opening torpedo). The single choke point is
`targeting::select_random_target_index`, which all of these call — including the
day-shelling ASW branch in `shelling.rs`. (Opening ASW in `asw.rs` is **not** one of
these; it selects via `select_submarine_target` — see Scope Boundaries.)

```mermaid
flowchart TD
    A[select_random_target_index → target_idx] --> B{target_idx == 0\n(flagship)?}
    B -->|no| Z[normal: damage on target_idx]
    B -->|yes| C[eligible escorts =\nidx≠0 ∧ alive ∧ green(hp*4>maxhp*3)\n∧ class matches flagship]
    C --> D{any eligible?}
    D -->|no| Z
    D -->|yes| E[draw roll_range vs\nformation rate 45/60/75]
    E --> F{hit?}
    F -->|no| Z
    F -->|yes| G[draw choose_index → interceptor]
    G --> H[target_idx = interceptor\nshield_flag = true]
    H --> I[damage calc + apply_damage\non interceptor]
    I --> J[df_list = interceptor idx (R7)\ndamage = Shielded(display_damage) → x.1 (R6)]
```

Wire-encoding type (KTD-2), directional shape — not final code:

```text
enum DamageCell { Plain(i64), Shielded(i64) }
impl Serialize:
    Plain(n)    -> serialize n as integer
    Shielded(n) -> serialize (n as f64 + 0.1)   // client floors for value, % 1 > 0 for shield
impl From<i64> for DamageCell { Plain }          // existing builders keep pushing i64
```

Formation → rate (defender's formation; surface fleet IDs only — combined IDs
deferred with R10):

| formation_id | name | rate |
|--------------|------|------|
| 1 | 単縦陣 | 45% |
| 2 / 4 / 5 | 複縦 / 梯形 / 単横 | 60% |
| 3 / 6 | 輪形 / 警戒 | 75% |

---

## Implementation Units

### U1. Shield-capable damage wire-encoding type

**Goal:** Add a damage cell type that serializes as integer normally and `x.1`
when shielded, and thread it through every damage-array packet field.

**Requirements:** R6.

**Dependencies:** none.

**Files:**
- `crates/emukc_battle/src/types/packet.rs` — define the type next to `SiListId`;
  change `BattleHougeki.api_damage` and `BattleNightHougeki.api_damage` from
  `Vec<Vec<i64>>` to `Vec<Vec<DamageCell>>`; make `BattleRaigeki.api_fydam` /
  `api_eydam` and `BattleOpeningAttack.api_fydam_list_items` /
  `api_eydam_list_items` shield-capable.
- `crates/emukc_battle/src/transcript.rs`, `debug_overlay.rs` — adjust the
  damage reads/writes (`api_damage[i]`, `.extend`, zeroing) to the new type.

**Approach:** Hand-written `Serialize` per KTD-2; `#[derive(PartialEq, Debug, Clone)]`
plus `From<i64>` (→ `Plain`) so existing builders that push `i64` need only a
`.into()` (or a constructor) rather than a rewrite. Add a `shielded(i64)`
constructor and an accessor for the integer value (debug_overlay zeroing, transcript
reads). Keep the `#[serde(untagged)]`-style clean output — no type tags.

**Patterns to follow:** `SiListId` enum + `text_from_i64`/`num_from_i64` helpers in
the same file; its `#[cfg(test)]` serialization assertions.

**Test scenarios** (`crates/emukc_battle/src/types/packet.rs` test mod):
- `Plain(55)` serializes to JSON integer `55` (no decimal). Covers R6.
- `Shielded(55)` serializes to JSON `55.1`; `serde_json` round-trip yields a value
  with `% 1 > 0` and floor `55`. Covers R6.
- `Shielded(0)` serializes to `0.1` (zero-damage shield still flagged).
- `From<i64>` produces `Plain`.

**Verification:** Crate builds with the new field types; serialization tests green;
no behavioral change yet (all cells `Plain`).

### U2. かばう eligibility + interceptor selection helper

**Goal:** Pure targeting helper that, given the defending fleet, the resolved
target index, the defender's formation, and the RNG, returns the interceptor index
(or `None`) per the official rules.

**Requirements:** R1, R2, R3, R4, R5, R9.

**Dependencies:** none (independent of U1).

**Files:**
- `crates/emukc_battle/src/targeting.rs` — new `pub(crate)` function (e.g.
  `select_escort_shield`) + a private formation→rate map.

**Approach:** Per KTD-5. Eligibility (no RNG): `target_idx == 0`; candidate escorts
are `idx != 0`, `is_alive()`, green health `hp() * 4 > api_maxhp * 3` (小破未満,
strictly above 75%), and `target_class` category matches the flagship's
(`is_surface_like()` ↔ surface flagship, `is_submarine()` ↔ submarine flagship via
the existing `TargetClass`). **Category match uses the two existing buckets, no
special-casing:** `is_surface_like()` deliberately folds `PtBoat` and `Installation`
into the surface bucket, so a PT/Installation flagship is protected by (and protects
with) any surface-like escort, and submarine matches submarine — relevant because R9
makes enemy fleets (which often have Installation/PT flagships) in scope. Flagship's
own HP state does **not** gate (大破 OK — per wiki *"旗艦の大破時でも発動可能"*). Then
probability `roll_range(0,100) < rate`, then
`choose_index(eligible.len())`. Combined-fleet formation IDs (11–14) are not mapped
(R10 out of scope) — treat unknown IDs as no interception with a code comment.

**Patterns to follow:** `select_random_target_index` / `select_submarine_target`
shape (alive filter → candidate vec → `choose_index`); the chūha boundary math
`hp() * 2 <= api_maxhp` already in `can_closing_torpedo_ship` (green is the 75%
analog).

**Test scenarios** (`crates/emukc_battle/src/targeting.rs` test mod, `SeededRng`):
- Covers AE2. 単縦陣 (id 1) flagship-targeted: over a fixed seeded sequence of N
  rolls, interception fires at the deterministic 45% positions.
- Covers AE3. All escorts at 小破 or worse (HP ≤ 75%) → returns `None` even on a
  seed that would otherwise hit.
- Covers AE4. Surface flagship, only healthy escort is a submarine → `None` (type
  mismatch).
- Green-health boundary: escort at exactly 75% maxHP is **ineligible**; at 76% is
  eligible (assert the `hp*4 > maxhp*3` cutoff on odd maxHP too).
- Rate table: ids 2/4/5 → 60%, ids 3/6 → 75%, id 1 → 45% (assert the threshold per
  id); unknown/combined id → `None`, no panic.
- `target_idx != 0` → `None`, **no RNG draw consumed** (assert next roll unchanged).
- Submarine flagship protected only by a submarine escort → eligible.

**Verification:** Helper is deterministic under a fixed seed; all eligibility and
rate scenarios green.

### U3. Wire interception into hougeki phases (api_damage)

**Goal:** Apply U2 at the day-shelling, night-shelling, ASW, and special-attack
target sites; on interception, redirect the target, set `df_list` to the
interceptor, and emit `Shielded` damage.

**Requirements:** R1, R7, R8 (hougeki), R9.

**Dependencies:** U1, U2.

**Files:**
- `crates/emukc_battle/src/simulation/shelling.rs` — after
  `select_random_target_index` returns `target_idx`, apply the shield helper before
  damage calc; build `damage`/`df_list` accordingly. Covers the single and
  double-attack hit loops **and the day-shelling ASW (`is_asw_attack`) branch** —
  this is the only ASW path that routes through the choke point.
- `crates/emukc_battle/src/simulation/night.rs`, `special_attack.rs` — same wiring
  at each `api_damage`-producing target site. (`asw.rs` / opening ASW is **not**
  wired — see Scope Boundaries.)
- `crates/emukc_battle/src/types/runtime.rs` — `ShellingParams` gains an explicit
  `defender_formation_id`. Its existing `formation_id` is the *attacker's*, resolved
  at the `mod.rs` call site, and the phase never receives `BattleContext`, so the
  defender's formation cannot be derived from `attacker_is_enemy` inside the phase —
  it must be passed in. Populate it at the `mod.rs` call sites (where both
  `friendly_formation_id` / `enemy_formation_id` are in scope). `NightBattleParams`
  already carries both formation IDs.

**Approach:** Single shared call site pattern — a small wrapper that takes
`(defenders, target_idx, defender_formation_id, rng)` and returns
`(effective_target_idx, shield: bool)`. `push_attack` (and the night/special
equivalents) take the shield flag and wrap the per-hit `display_damage` value as
`Shielded` when set (per KTD-3 the wrapped integer keeps `display_damage`'s
raw-vs-dealt choice). For multi-hit attacks (double attack, CI), the shield decision
is per the redirected target; resolve interception once per attack on the original
flagship target, then all hits of that attack land on the same interceptor (matches
client `isShield(i)` per-hit semantics; intra-attack health-state change between
hits is deferred — see Scope Boundaries).

**Patterns to follow:** existing `push_attack` plumbing in `shelling.rs`;
`display_damage(defender, raw, dealt)` already chooses raw vs dealt — `Shielded`
wraps its result.

**Test scenarios** (per-phase test mods + a gameplay integration test):
- Covers AE1. Friendly fleet 単縦陣, healthy escort, enemy attacks flagship, seed
  forces interception: the hit's `api_damage` cell is `Shielded(X)` (serializes
  `X.1`), and `api_df_list` for that attack is the escort index, not 0.
- Enemy flagship interception (R9): friendly attacks enemy flagship, enemy fleet has
  a healthy escort → same redirection on the enemy side.
- No interception (seed forces miss): `api_damage` cell stays `Plain`, `df_list` is
  flagship 0, escort HP unchanged, flagship HP reduced.
- Double-attack / CI on flagship with interception: both hits land on the
  interceptor, both cells `Shielded`.
- HP bookkeeping (**friendly** escort): the interceptor's HP drops by the reported
  integer; flagship HP unchanged. For an **enemy** escort, assert redirect + `.1` +
  `df_list` only — the reported integer is raw/overkill per `display_damage`, not the
  HP delta.

**Execution note:** Add the AE1 contract test first (failing) before wiring, so the
redirect + encoding behavior is pinned.

**Verification:** Targeted phase tests green; the AE1 contract test passes; flagship
HP no longer changes on intercepted hits.

### U4. Wire interception into opening torpedo (api_fydam/api_eydam)

**Goal:** Same mechanic for the opening-torpedo phase, where the shield flag lives
on `api_fydam`/`api_eydam` rather than `api_damage`.

**Requirements:** R1, R7, R8 (opening torpedo), R9.

**Dependencies:** U1, U2.

**Files:**
- `crates/emukc_battle/src/simulation/torpedo.rs` — both `simulate_opening_torpedo`
  (friendly→enemy and enemy→friendly) sites that call
  `select_random_target_index`; redirect target, record `defender_index` =
  interceptor on the `TorpedoHit`, and emit the shielded ydam value.
- `crates/emukc_battle/src/types/packet.rs` (consumed from U1) —
  `record_torpedo_hit` on `BattleOpeningAttack` / `BattleRaigeki` writes the
  shield-capable ydam value.

**Approach:** Opening torpedo already passes `friendly_formation_id` /
`enemy_formation_id` into `torpedo.rs`. Apply the U2 helper at the
`select_random_target_index` site with the defender's formation. The encoding
target differs: `isShield_f/_e` read `api_fydam`/`api_eydam` and use `% 1 != 0`, so
the shielded value goes on the attacker's ydam entry for that hit. `api_frai`/
`api_erai` (the target index list) points at the interceptor (R7 analog). Closing
torpedo (`simulate_raigeki`) is **out of scope** for shielding (the brainstorm names
opening torpedo for R8; flag closing as deferred).

**Patterns to follow:** `BattleOpeningAttack::record_torpedo_hit` and
`BattleRaigeki::record_torpedo_hit` in `packet.rs`; `TorpedoHit` struct in
`types/domain.rs`.

**Test scenarios** (`crates/emukc_battle/src/simulation/torpedo.rs` test mod):
- Covers AE5. Enemy opening torpedo targets friendly flagship, seed forces
  interception → the attacker's `api_fydam`/`api_eydam` entry carries `.1`
  (`isShield_f`/`_e` would return true), `api_frai`/`api_erai` points at the
  interceptor.
- No-interception seed: ydam stays integer, target is flagship 0.
- Type match enforced (submarine opening torpedo vs surface flagship escort set):
  reuses U2 eligibility — assert redirection only to a type-matching escort.

**Verification:** Torpedo phase tests green; AE5 passes.

### U5. Confirm fractional api_damage passes the battle validator / incident analyzer

**Goal:** Guarantee the `battle` CLI diagnostics (`validate`, `analyze-incident`)
accept `.1`-encoded damage rather than silently breaking once U1 ships.

**Requirements:** supports R6 (diagnostics parity).

**Dependencies:** U1.

**Approach (re-ground before editing):** The premise is *verify, then change only if
a damage-value read exists*. Reading `battle_rules.rs`: the hougeki `as_i64()` reads
are over `api_si_list` (already string-aware) and flag fields, **not** over
`api_damage` / ydam *element* values, and the day-payload protocol check only
verifies field *presence* by `access_kind`. If that holds, no production change is
needed and this unit is a regression test only. A production change is warranted
**only** if a damage *magnitude* check is found — then it floors via `as_f64()`
instead of rejecting. Asset JSONs stay untouched (Do-Not-Modify); any change is
confined to Rust validator logic.

**Files:**
- `crates/emukc_bootstrap/src/battle_rules.rs` — read first to confirm the above;
  add the regression test; touch validator logic only if a damage-magnitude read
  exists.

**Test scenarios** (`crates/emukc_bootstrap/src/battle_rules.rs` test mod):
- A battle payload with `api_damage: [[55.1],[0]]` and an opening-torpedo payload
  with a fractional `api_fydam` both validate without a type/format error.
- Integer-only payloads still validate identically (no regression to the existing
  `[[11],[13]]` / `[[11,11,11]]` fixtures).

**Test expectation:** likely a fixture-only regression test (no production change),
contingent on the read-grounding above showing the validator never inspects damage
element values.

**Verification:** `cargo test -p emukc_bootstrap battle_rules` green; existing
fixtures unchanged in outcome.

### U6. Re-freeze golden battle transcripts

**Goal:** Regenerate the deterministic battle baselines that the new RNG draws
(KTD-4/KTD-5) necessarily shift, and document the diff.

**Requirements:** supports KTD-4.

**Dependencies:** U3, U4 (behavior must be final first).

**Files:**
- `crates/emukc_battle/tests/golden/*.txt` — regenerate via the established golden
  workflow (not hand-edited).
- `tests/gameplay_tests/battle_golden.rs` — the frozen full-sortie transcript;
  re-freeze deliberately per CLAUDE.md (Do-Not-Modify governance).

**Approach:** Regenerate through the same harness that produced the baselines.
Inspect the diff to confirm changes are explained by interception (shield `.1`
cells, redirected `df_list`/`frai` indices, and downstream RNG-sequence shifts) and
not by an unintended logic change. Capture the rationale for the PR description.

**Test expectation:** the regenerated baselines are the test — no new assertions.
The PR description must explain the re-freeze (required by CLAUDE.md).

**Verification:** `cargo test -p emukc_battle` and `cargo test --test gameplay_tests`
green against the regenerated baselines; diff reviewed and explained.

---

## Scope Boundaries

**In scope:** day/night shelling + day-ASW + special-attack hougeki shielding,
opening torpedo shielding, bidirectional interception, the wire-encoding type,
validator compatibility, golden re-freeze.

### Out of Scope

- **Combined fleet (連合艦隊):** R10's "escort flagship not protected in day battle"
  restriction, and cross-fleet submarine かばう in opening ASW — deferred until
  combined-fleet sortie exists. Combined formation IDs (11–14) are intentionally
  unmapped.
- **Closing torpedo (`api_raigeki` / `simulate_raigeki`) shielding** — the
  brainstorm names opening torpedo for R8; closing torpedo is deferred.
- **Opening ASW (`asw.rs` / `simulate_opening_taisen`) shielding** — opening ASW
  selects via `select_submarine_target` (submarine-only) and does not route through
  `select_random_target_index`, so a submarine-flagship-vs-opening-ASW shield is an
  edge case deferred with the combined-fleet cross-fleet submarine かばう above.
  Day-shelling ASW (the `shelling.rs` `is_asw_attack` branch) **is** in scope — it
  routes through the choke point.

### Deferred to Follow-Up Work

- **Intra-attack health-state change** (escort goes green→小破 between hit 1 and hit
  2 of a multi-hit attack, e.g. 瑞雲 CI) — accuracy tuning; this plan resolves
  interception once per attack.
- **Escort-selection refinement** — should additional official data show selection
  is not uniform-random, revisit KTD-1.

---

## Risks & Dependencies

- **Golden churn (high-touch, expected).** Every battle golden changes (KTD-4). Risk
  is masking an unrelated regression inside the re-freeze. Mitigation: U6 reviews the
  diff explicitly for interception-only causes; the fixed draw order (KTD-5) makes
  the shift reproducible.
- **Defender-formation threading.** Phases currently carry the *attacker's*
  `formation_id` for damage modifiers; interception needs the *defender's*. The
  shelling phase cannot derive it from `attacker_is_enemy` alone — it never receives
  `BattleContext`. Mitigation: `ShellingParams` gains an explicit
  `defender_formation_id` populated at the `mod.rs` call site (where both formation
  IDs are in scope); `NightBattleParams` already carries both. A test asserts the
  defender's (not attacker's) formation drives the rate.
- **Type-change blast radius.** Changing `api_damage`/ydam element types touches
  builders, transcript, debug_overlay, and the validator. Mitigation: `From<i64>` on
  the new type keeps most call sites a `.into()`; U1 lands the type before any
  behavior.
- **Sequencing dependency (resolved).** The si_list fix (`docs/plans/2026-06-24-001`,
  completed) also touched `BattleHougeki`; that ordering conflict is cleared.

---

## Open Questions (resolve at implementation)

These are execution-time unknowns, not planning blockers — resolve them while
implementing the named unit.

- **[U4] Scalar damage totals stay unencoded.** `record_torpedo_hit` accumulates
  per-ship totals into `api_fdam` / `api_edam` (kept `i64`). The shield `.1` flag
  goes only on the per-attacker `api_fydam` / `api_eydam` *list* entry the client's
  `isShield_f/_e` reads — confirm the scalar `fdam`/`edam` totals are **not**
  shield-encoded (they would corrupt the cumulative HP sum the client derives).
- **[U1] god_mode zeroing drops the shield flag.** When `debug_overlay` zeroes an
  intercepted friendly-directed hit, it should become `Plain(0)`, not `Shielded(0)` —
  a fully negated hit must not render a shield banner. Confirm the integer accessor /
  zeroing path produces `Plain`.

---

## Sources & Research

- Origin: `docs/brainstorms/2026-06-24-flagship-escort-shield-requirements.md`.
- Escort-selection rule (KTD-1) and formation rates (R2): wikiwiki.jp かばう page
  `攻撃対象の選択` — *"旗艦以外からランダムで1隻選択"*, candidate *"小破未満の僚艦"*,
  rates 単縦 0.45 / 複縦・梯形・単横 0.6 / 輪形・警戒 0.75, and *"旗艦の大破時でも発動可能"*.
- Client decode (R6 encoding): `main-decoder/out/main.decoded.js` —
  `HougekiData.isShield` (`% 1 > 0`) / `getDamage` (`Math.floor`),
  `RaigekiData.isShield_f/_e` (`% 1 != 0`) over `api_fydam`/`api_eydam`.
- Damage-reporting invariant (KTD-3): `docs/solutions/architecture-patterns/battle-damage-foundation.md`.
- Encoding-type precedent (KTD-2): `SiListId` in `crates/emukc_battle/src/types/packet.rs`.
