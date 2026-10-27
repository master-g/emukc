---
title: "Working notes for battle changes: sources, verification, and what was decided not to model"
date: 2026-10-27
category: best-practices
module: emukc_battle
problem_type: workflow
component: battle_simulation
severity: low
applies_when:
  - "Porting a battle rule from kancolle-replay or KC3Kai"
  - "Verifying a battle change against the real client"
  - "Picking up a paused battle issue (#282, #285, #291 and the #272–#281 batch)"
tags: [battle, kancolle-replay, headless-check, seeds, out-of-scope]
related_components: [emukc_gameplay, main-decoder]
---

# Working notes for battle changes

These notes were moved out of `PROJECT_MEMORY.md` on 2026-10-27, when battle work was paused in favour of the account import. They were written over the October battle sessions.

## Sources

- Re-fetch `js/` from `kancolle-replay@69097abc`. `js/simulator-ui/common.js` holds the barrage-balloon and atoll cell tables.
- Constants are written with no leading zero, as in `.39`, so a search for `0.39` alone misses them.
- An equipment's type is the source's `type`, not a guess from its name.
- `en.kancollewiki.net` puts a bot check in front of its pages.
- The improvement table `improvement_coefficients` covers firepower and the ten accuracy and evasion entries (#206, #194).
- `isdivebomber` in the source counts land-based attack aircraft too (`kcEQDATA.js`).

## Verifying

- For anything the client shows, read `main.decoded.js` and the cache first, then run `make headless-check`. Compare portrait positions on screenshots taken before and after the fix.
- After changing a multi-gauge map's rules, run `kcnav normalize`, rebuild the codex, then run `kcnav normalize` again.
- When a seeded battle test stops reaching its case, find another seed instead of loosening the assertion.
- A practice test without a fixed seed can earn 0 base experience. Practice rules are in `practice-rivals.md`.

## Not modelled (decided)

- `isSummerCV` and the other lists covering 19 ships: none of these ships appears on a regular map.
- `bonusSpecialP`: it applies to events only.

## Unresolved

- 瑞雲 cut-in: no evidence yet for what is shown after the third piece, or for whether it counts as a star shell.
