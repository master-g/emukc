//! Deterministic, diff-stable text renderer for battle simulations.
//!
//! Pure formatting over `emukc_battle` output types — no game logic, no input
//! mutation (R5), and byte-identical output for identical input (R6). Both the
//! `battle sim` CLI and golden-transcript tests render through this one path.
//!
//! Phases are emitted in fixed battle order; absent phases (`None`) are skipped
//! cleanly. Ships are referenced by fleet side + 1-based index and master id so
//! a reader can follow the play-by-play without the raw API arrays.

use std::fmt::Write as _;

use crate::types::{
    BattleHougeki, BattleKouku, BattleNightHougeki, BattleOpeningAttack, BattleOutcome,
    BattleRaigeki, BattleRuntimeShip, BattleSimulation, DamageCell, NightBattleSimulation,
};

/// Render a day-battle simulation as deterministic, human-readable text.
pub fn render_day_battle(sim: &BattleSimulation) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "== Day Battle ==");
    render_formation(&sim.packet.formation, &mut out);

    if let Some(kouku) = &sim.packet.kouku {
        render_kouku(kouku, &mut out);
    }
    if let Some(h) = &sim.packet.opening_taisen {
        render_hougeki("opening ASW", h, &mut out);
    }
    if let Some(op) = &sim.packet.opening_attack {
        render_opening_torpedo(op, &mut out);
    }
    if let Some(h) = &sim.packet.hougeki1 {
        render_hougeki("shelling 1", h, &mut out);
    }
    if let Some(h) = &sim.packet.hougeki2 {
        render_hougeki("shelling 2", h, &mut out);
    }
    if let Some(h) = &sim.packet.hougeki3 {
        render_hougeki("shelling 3", h, &mut out);
    }
    if let Some(r) = &sim.packet.raigeki {
        render_raigeki("closing torpedo", r, &mut out);
    }

    render_outcome(&sim.outcome, &mut out);

    let f_before: Vec<i64> = sim.friendly.iter().map(|s| s.entry_hp).collect();
    let f_max: Vec<i64> = sim.friendly.iter().map(|s| s.ship.api_maxhp).collect();
    let e_before: Vec<i64> = sim.enemy.iter().map(|s| s.entry_hp).collect();
    let e_max: Vec<i64> = sim.enemy.iter().map(|s| s.ship.api_maxhp).collect();
    render_fleet('F', "friendly", &sim.friendly, &f_before, &f_max, &mut out);
    render_fleet('E', "enemy", &sim.enemy, &e_before, &e_max, &mut out);

    out
}

/// Render a night-battle simulation as deterministic, human-readable text.
pub fn render_night_battle(sim: &NightBattleSimulation) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "== Night Battle ==");
    render_formation(&sim.packet.formation, &mut out);

    if let Some(h) = &sim.packet.hougeki {
        render_night_hougeki("midnight", h, &mut out);
    }

    render_outcome(&sim.outcome, &mut out);

    // Night entry HP comes from the packet (the ships' `entry_hp` reflects the
    // day-battle node, not the night node).
    render_fleet(
        'F',
        "friendly",
        &sim.friendly,
        &sim.packet.friendly_nowhps,
        &sim.packet.friendly_maxhps,
        &mut out,
    );
    render_fleet(
        'E',
        "enemy",
        &sim.enemy,
        &sim.packet.enemy_nowhps,
        &sim.packet.enemy_maxhps,
        &mut out,
    );

    out
}

// -- helpers --

/// `eflag == 0` is the friendly side, anything else is the enemy side.
fn attacker_side(eflag: i64) -> char {
    if eflag == 0 {
        'F'
    } else {
        'E'
    }
}

/// The defending side is the opposite of the attacking side.
fn defender_side(eflag: i64) -> char {
    if eflag == 0 {
        'E'
    } else {
        'F'
    }
}

/// `api_cl_list` convention: 0 = miss, 2 = critical, anything else = plain hit.
fn hit_word(cl: i64) -> &'static str {
    match cl {
        0 => "miss",
        2 => "crit",
        _ => "hit",
    }
}

fn render_formation(formation: &[i64; 3], out: &mut String) {
    let _ = writeln!(
        out,
        "formation: friend={} enemy={} engagement={}",
        formation[0], formation[1], formation[2]
    );
}

fn render_kouku(k: &BattleKouku, out: &mut String) {
    let s1 = &k.api_stage1;
    let _ = writeln!(out, "\n[aerial]");
    let _ = writeln!(out, "  air superiority: {}", s1.api_disp_seiku);
    let _ = writeln!(
        out,
        "  friendly planes: {} -> {} (lost {})",
        s1.api_f_count,
        s1.api_f_count - s1.api_f_lostcount,
        s1.api_f_lostcount
    );
    let _ = writeln!(
        out,
        "  enemy planes: {} -> {} (lost {})",
        s1.api_e_count,
        s1.api_e_count - s1.api_e_lostcount,
        s1.api_e_lostcount
    );
    let _ = writeln!(out, "  bombing dmg to enemy: {:?}", k.api_stage3.api_edam);
    let _ = writeln!(out, "  bombing dmg to friendly: {:?}", k.api_stage3.api_fdam);
}

fn render_hougeki(label: &str, h: &BattleHougeki, out: &mut String) {
    let _ = writeln!(out, "\n[{label}]");
    if h.api_at_list.is_empty() {
        let _ = writeln!(out, "  (no attacks)");
        return;
    }
    for i in 0..h.api_at_list.len() {
        let eflag = h.api_at_eflag.get(i).copied().unwrap_or(0);
        let attacker = h.api_at_list.get(i).copied().unwrap_or(0);
        let at_type = h.api_at_type.get(i).copied().unwrap_or(0);
        let targets = h.api_df_list.get(i).map(Vec::as_slice).unwrap_or(&[]);
        let cls = h.api_cl_list.get(i).map(Vec::as_slice).unwrap_or(&[]);
        let dmgs = h.api_damage.get(i).map(Vec::as_slice).unwrap_or(&[]);
        render_attack(eflag, attacker, at_type, targets, cls, dmgs, out);
    }
}

fn render_night_hougeki(label: &str, h: &BattleNightHougeki, out: &mut String) {
    let _ = writeln!(out, "\n[{label}]");
    if h.api_at_list.is_empty() {
        let _ = writeln!(out, "  (no attacks)");
        return;
    }
    for i in 0..h.api_at_list.len() {
        let eflag = h.api_at_eflag.get(i).copied().unwrap_or(0);
        let attacker = h.api_at_list.get(i).copied().unwrap_or(0);
        let sp = h.api_sp_list.get(i).copied().unwrap_or(0);
        let targets = h.api_df_list.get(i).map(Vec::as_slice).unwrap_or(&[]);
        let cls = h.api_cl_list.get(i).map(Vec::as_slice).unwrap_or(&[]);
        let dmgs = h.api_damage.get(i).map(Vec::as_slice).unwrap_or(&[]);
        // Night uses `api_sp_list` for the cut-in / special-attack marker.
        render_attack(eflag, attacker, sp, targets, cls, dmgs, out);
    }
}

/// Render one attacker's hits. `cutin` is the attack type / special marker; a
/// non-zero value is surfaced as a ` cutin=N` suffix.
fn render_attack(
    eflag: i64,
    attacker: i64,
    cutin: i64,
    targets: &[i64],
    cls: &[i64],
    dmgs: &[DamageCell],
    out: &mut String,
) {
    let aside = attacker_side(eflag);
    let dside = defender_side(eflag);
    let cutin = if cutin != 0 {
        format!(" cutin={cutin}")
    } else {
        String::new()
    };
    if targets.is_empty() {
        let _ = writeln!(out, "  {aside}{} -> (no target){cutin}", attacker + 1);
        return;
    }
    for (j, &tgt) in targets.iter().enumerate() {
        let cl = cls.get(j).copied().unwrap_or(1);
        let d = dmgs.get(j).map(|c| c.amount()).unwrap_or(0);
        let _ = writeln!(
            out,
            "  {aside}{} -> {dside}{}: dmg {d} [{}]{cutin}",
            attacker + 1,
            tgt + 1,
            hit_word(cl)
        );
    }
}

fn render_opening_torpedo(op: &BattleOpeningAttack, out: &mut String) {
    let _ = writeln!(out, "\n[opening torpedo]");
    let mut any = false;
    any |= render_torpedo_side(
        'F',
        'E',
        &op.api_frai_list_items,
        &op.api_fcl_list_items,
        &op.api_fydam_list_items,
        out,
    );
    any |= render_torpedo_side(
        'E',
        'F',
        &op.api_erai_list_items,
        &op.api_ecl_list_items,
        &op.api_eydam_list_items,
        out,
    );
    if !any {
        let _ = writeln!(out, "  (no torpedoes)");
    }
}

fn render_torpedo_side(
    aside: char,
    dside: char,
    rai: &[Option<Vec<i64>>],
    cl: &[Option<Vec<i64>>],
    ydam: &[Option<Vec<DamageCell>>],
    out: &mut String,
) -> bool {
    let mut any = false;
    for (i, item) in rai.iter().enumerate() {
        let Some(targets) = item else {
            continue;
        };
        let dmgs = ydam.get(i).and_then(Option::as_deref).unwrap_or(&[]);
        let cls = cl.get(i).and_then(Option::as_deref).unwrap_or(&[]);
        for (j, &tgt) in targets.iter().enumerate() {
            let d = dmgs.get(j).map(|c| c.amount()).unwrap_or(0);
            let c = cls.get(j).copied().unwrap_or(1);
            let _ = writeln!(
                out,
                "  {aside}{} -> {dside}{}: dmg {d} [{}]",
                i + 1,
                tgt + 1,
                hit_word(c)
            );
            any = true;
        }
    }
    any
}

fn render_raigeki(label: &str, r: &BattleRaigeki, out: &mut String) {
    let _ = writeln!(out, "\n[{label}]");
    let mut any = false;
    for (i, &tgt) in r.api_frai.iter().enumerate() {
        if tgt < 0 {
            continue;
        }
        let d = r.api_fydam.get(i).map(|c| c.amount()).unwrap_or(0);
        let c = r.api_fcl.get(i).copied().unwrap_or(1);
        let _ = writeln!(out, "  F{} -> E{}: dmg {d} [{}]", i + 1, tgt + 1, hit_word(c));
        any = true;
    }
    for (i, &tgt) in r.api_erai.iter().enumerate() {
        if tgt < 0 {
            continue;
        }
        let d = r.api_eydam.get(i).map(|c| c.amount()).unwrap_or(0);
        let c = r.api_ecl.get(i).copied().unwrap_or(1);
        let _ = writeln!(out, "  E{} -> F{}: dmg {d} [{}]", i + 1, tgt + 1, hit_word(c));
        any = true;
    }
    if !any {
        let _ = writeln!(out, "  (no torpedoes)");
    }
}

fn render_outcome(o: &BattleOutcome, out: &mut String) {
    let mvp = if o.mvp >= 1 {
        format!("F{}", o.mvp)
    } else {
        "none".to_string()
    };
    let midnight = if o.can_midnight {
        "yes"
    } else {
        "no"
    };
    let _ = writeln!(out, "\nresult: rank {:?}, mvp {mvp}, midnight {midnight}", o.win_rank);
}

fn render_fleet(
    prefix: char,
    label: &str,
    ships: &[BattleRuntimeShip],
    befores: &[i64],
    maxes: &[i64],
    out: &mut String,
) {
    let _ = writeln!(out, "\n{label}:");
    for (i, ship) in ships.iter().enumerate() {
        let before = befores.get(i).copied().unwrap_or(ship.entry_hp);
        let max = maxes.get(i).copied().unwrap_or(ship.ship.api_maxhp);
        let after = ship.hp().max(0);
        let sunk = if ship.is_sunk() {
            " SUNK"
        } else {
            ""
        };
        let _ = writeln!(
            out,
            "  {prefix}{} ship{}: {before} -> {after} (max {max}){sunk}",
            i + 1,
            ship.ship.api_ship_id
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::make_test_ship_ctx;
    use crate::types::{
        BattleKoukuStage1, BattleKoukuStage2, BattleKoukuStage3, BattleOutcome, BattlePacket,
        NightBattlePacket,
    };
    use emukc_model::kc2::KcSortieResultRank;

    fn ship(
        master_id: i64,
        before: i64,
        after: i64,
        max: i64,
        friendly: bool,
    ) -> BattleRuntimeShip {
        let mut s = make_test_ship_ctx(before, before, after, max, friendly, true);
        s.ship.api_ship_id = master_id;
        s
    }

    /// Build a `BattleHougeki` from `(eflag, attacker, at_type, [(target, cl, dmg)])`.
    fn hougeki(attacks: &[(i64, i64, i64, &[(i64, i64, i64)])]) -> BattleHougeki {
        let mut h = BattleHougeki {
            api_at_eflag: vec![],
            api_at_list: vec![],
            api_at_type: vec![],
            api_df_list: vec![],
            api_si_list: vec![],
            api_cl_list: vec![],
            api_damage: vec![],
        };
        for &(eflag, attacker, at_type, targets) in attacks {
            h.api_at_eflag.push(eflag);
            h.api_at_list.push(attacker);
            h.api_at_type.push(at_type);
            h.api_df_list.push(targets.iter().map(|t| t.0).collect());
            h.api_cl_list.push(targets.iter().map(|t| t.1).collect());
            h.api_damage.push(targets.iter().map(|t| DamageCell::Plain(t.2)).collect());
            h.api_si_list.push(vec![]);
        }
        h
    }

    fn sample_kouku() -> BattleKouku {
        BattleKouku {
            api_plane_from: [vec![1], vec![-1]],
            api_stage1: BattleKoukuStage1 {
                api_f_count: 18,
                api_f_lostcount: 2,
                api_e_count: 12,
                api_e_lostcount: 12,
                api_disp_seiku: 1,
                api_touch_plane: [-1, -1],
            },
            api_stage2: BattleKoukuStage2 {
                api_f_count: 16,
                api_f_lostcount: 0,
                api_e_count: 0,
                api_e_lostcount: 0,
            },
            api_stage3: BattleKoukuStage3 {
                api_frai: vec![],
                api_erai: vec![],
                api_fbak: vec![],
                api_ebak: vec![],
                api_frai_flag: vec![],
                api_erai_flag: vec![],
                api_fbak_flag: vec![],
                api_ebak_flag: vec![],
                api_fcl_flag: vec![],
                api_ecl_flag: vec![],
                api_fdam: vec![0],
                api_edam: vec![45],
                api_f_sp_list: vec![None],
                api_e_sp_list: vec![None],
            },
        }
    }

    fn day_packet(kouku: Option<BattleKouku>) -> BattlePacket {
        BattlePacket {
            formation: [1, 1, 1],
            friendly_nowhps: vec![80],
            enemy_nowhps: vec![0],
            smoke_type: 0,
            balloon_cell: 0,
            atoll_cell: 0,
            midnight_flag: 0,
            search: [1, 1],
            stage_flag: [1, 1, 1],
            kouku,
            opening_taisen_flag: 0,
            opening_taisen: None,
            opening_flag: 0,
            opening_attack: None,
            hourai_flag: [1, 0, 0, 1],
            hougeki1: Some(hougeki(&[(0, 0, 0, &[(0, 1, 53)]), (1, 0, 0, &[(0, 0, 0)])])),
            hougeki2: None,
            hougeki3: None,
            raigeki: Some(BattleRaigeki {
                api_frai: vec![0],
                api_fcl: vec![1],
                api_fdam: vec![0],
                api_fydam: vec![DamageCell::Plain(60)],
                api_erai: vec![-1],
                api_ecl: vec![0],
                api_edam: vec![60],
                api_eydam: vec![DamageCell::Plain(0)],
            }),
        }
    }

    fn day_fixture(kouku: Option<BattleKouku>) -> BattleSimulation {
        BattleSimulation {
            friendly: vec![ship(123, 80, 80, 80, true)],
            enemy: vec![ship(456, 90, 0, 90, false)],
            packet: day_packet(kouku),
            outcome: BattleOutcome {
                win_rank: KcSortieResultRank::S,
                mvp: 1,
                can_midnight: false,
            },
        }
    }

    fn night_fixture() -> NightBattleSimulation {
        let hougeki = BattleNightHougeki {
            api_at_eflag: vec![0, 1],
            api_at_list: vec![0, 0],
            api_n_mother_list: vec![-1, -1],
            api_df_list: vec![vec![0], vec![0]],
            api_si_list: vec![vec![], vec![]],
            api_cl_list: vec![vec![2], vec![0]],
            api_sp_list: vec![5, 0],
            api_damage: vec![vec![DamageCell::Plain(140)], vec![DamageCell::Plain(0)]],
        };
        NightBattleSimulation {
            friendly: vec![ship(123, 80, 80, 80, true)],
            enemy: vec![ship(456, 90, 0, 90, false)],
            packet: NightBattlePacket {
                formation: [1, 1, 1],
                friendly_nowhps: vec![80],
                friendly_maxhps: vec![80],
                enemy_nowhps: vec![90],
                enemy_maxhps: vec![90],
                touch_plane: [-1, -1],
                flare_pos: [-1, -1],
                hougeki: Some(hougeki),
            },
            outcome: BattleOutcome {
                win_rank: KcSortieResultRank::S,
                mvp: 1,
                can_midnight: false,
            },
        }
    }

    #[test]
    fn rendering_is_deterministic() {
        let sim = day_fixture(Some(sample_kouku()));
        assert_eq!(render_day_battle(&sim), render_day_battle(&sim));
    }

    #[test]
    fn day_battle_renders_each_phase() {
        let text = render_day_battle(&day_fixture(Some(sample_kouku())));
        assert!(text.contains("[aerial]"), "aerial phase missing:\n{text}");
        assert!(text.contains("[shelling 1]"), "shelling phase missing:\n{text}");
        assert!(text.contains("F1 -> E1: dmg 53 [hit]"), "attacker line missing:\n{text}");
        assert!(text.contains("E1 -> F1: dmg 0 [miss]"), "miss line missing:\n{text}");
        assert!(text.contains("[closing torpedo]"), "torpedo phase missing:\n{text}");
        assert!(text.contains("result: rank S, mvp F1"), "result line missing:\n{text}");
        assert!(text.contains("E1 ship456: 90 -> 0 (max 90) SUNK"), "enemy hp missing:\n{text}");
    }

    #[test]
    fn absent_aerial_phase_is_skipped_cleanly() {
        let text = render_day_battle(&day_fixture(None));
        assert!(!text.contains("[aerial]"), "aerial section must be absent:\n{text}");
        assert!(!text.contains("air superiority"), "no stray aerial lines:\n{text}");
        // No blank/garbage where the section would have been: shelling follows formation.
        assert!(text.contains("[shelling 1]"));
    }

    #[test]
    fn night_battle_renders_midnight_and_cutin() {
        let text = render_night_battle(&night_fixture());
        assert!(text.contains("== Night Battle =="), "night header missing:\n{text}");
        assert!(text.contains("[midnight]"), "midnight phase missing:\n{text}");
        assert!(text.contains("F1 -> E1: dmg 140 [crit] cutin=5"), "cutin marker missing:\n{text}");
    }

    /// Frozen golden: a fixed fixture renders to this exact string. Regenerate
    /// intentionally — run the test, copy the `left` value from the failure into
    /// `EXPECTED`, and note the behavior change in the commit message (mirrors the
    /// `roll_scratch_damage_golden_vector` convention in `random.rs`).
    #[test]
    fn day_transcript_golden() {
        let got = render_day_battle(&day_fixture(Some(sample_kouku())));
        const EXPECTED: &str = concat!(
            "== Day Battle ==\n",
            "formation: friend=1 enemy=1 engagement=1\n",
            "\n",
            "[aerial]\n",
            "  air superiority: 1\n",
            "  friendly planes: 18 -> 16 (lost 2)\n",
            "  enemy planes: 12 -> 0 (lost 12)\n",
            "  bombing dmg to enemy: [45]\n",
            "  bombing dmg to friendly: [0]\n",
            "\n",
            "[shelling 1]\n",
            "  F1 -> E1: dmg 53 [hit]\n",
            "  E1 -> F1: dmg 0 [miss]\n",
            "\n",
            "[closing torpedo]\n",
            "  F1 -> E1: dmg 60 [hit]\n",
            "\n",
            "result: rank S, mvp F1, midnight no\n",
            "\n",
            "friendly:\n",
            "  F1 ship123: 80 -> 80 (max 80)\n",
            "\n",
            "enemy:\n",
            "  E1 ship456: 90 -> 0 (max 90) SUNK\n",
        );
        assert_eq!(got, EXPECTED, "transcript drifted:\n{got}");
    }
}
