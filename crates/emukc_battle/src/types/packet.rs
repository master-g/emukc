//! API packet types with Serialize derivations.
//! These directly serialize to JSON for the `KanColle` API response.

use serde::Serialize;

use super::domain::TorpedoAttackerSide;
use super::domain::TorpedoHit;

#[derive(Debug, Clone, Serialize)]
pub struct BattleKouku {
    pub api_plane_from: [Vec<i64>; 2],
    pub api_stage1: BattleKoukuStage1,
    pub api_stage2: BattleKoukuStage2,
    pub api_stage3: BattleKoukuStage3,
}

#[derive(Debug, Clone, Serialize)]
pub struct BattleKoukuStage1 {
    pub api_f_count: i64,
    pub api_f_lostcount: i64,
    pub api_e_count: i64,
    pub api_e_lostcount: i64,
    pub api_disp_seiku: i64,
    pub api_touch_plane: [i64; 2],
}

#[derive(Debug, Clone, Serialize)]
pub struct BattleKoukuStage2 {
    pub api_f_count: i64,
    pub api_f_lostcount: i64,
    pub api_e_count: i64,
    pub api_e_lostcount: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BattleKoukuStage3 {
    pub api_frai: Vec<i64>,
    pub api_erai: Vec<i64>,
    pub api_fbak: Vec<i64>,
    pub api_ebak: Vec<i64>,
    pub api_frai_flag: Vec<i64>,
    pub api_erai_flag: Vec<i64>,
    pub api_fbak_flag: Vec<i64>,
    pub api_ebak_flag: Vec<i64>,
    pub api_fcl_flag: Vec<i64>,
    pub api_ecl_flag: Vec<i64>,
    pub api_fdam: Vec<i64>,
    pub api_edam: Vec<i64>,
    pub api_f_sp_list: Vec<Option<i64>>,
    pub api_e_sp_list: Vec<Option<i64>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BattleOpeningAttack {
    pub api_frai_list_items: Vec<Option<Vec<i64>>>,
    pub api_fcl_list_items: Vec<Option<Vec<i64>>>,
    pub api_fdam: Vec<i64>,
    pub api_fydam_list_items: Vec<Option<Vec<i64>>>,
    pub api_erai_list_items: Vec<Option<Vec<i64>>>,
    pub api_ecl_list_items: Vec<Option<Vec<i64>>>,
    pub api_edam: Vec<i64>,
    pub api_eydam_list_items: Vec<Option<Vec<i64>>>,
}

impl BattleOpeningAttack {
    pub(crate) fn blank(len: usize) -> Self {
        Self {
            api_frai_list_items: vec![None; len],
            api_fcl_list_items: vec![None; len],
            api_fdam: vec![0; len],
            api_fydam_list_items: vec![None; len],
            api_erai_list_items: vec![None; len],
            api_ecl_list_items: vec![None; len],
            api_edam: vec![0; len],
            api_eydam_list_items: vec![None; len],
        }
    }

    pub(crate) fn record_torpedo_hit(
        &mut self,
        attacker_side: TorpedoAttackerSide,
        hit: TorpedoHit,
    ) {
        match attacker_side {
            TorpedoAttackerSide::Friendly => {
                self.api_frai_list_items[hit.attacker_index] =
                    Some(vec![hit.defender_index as i64]);
                self.api_fcl_list_items[hit.attacker_index] = Some(vec![1]);
                self.api_fydam_list_items[hit.attacker_index] = Some(vec![hit.damage]);
                self.api_edam[hit.defender_index] += hit.damage;
            }
            TorpedoAttackerSide::Enemy => {
                self.api_erai_list_items[hit.attacker_index] =
                    Some(vec![hit.defender_index as i64]);
                self.api_ecl_list_items[hit.attacker_index] = Some(vec![1]);
                self.api_eydam_list_items[hit.attacker_index] = Some(vec![hit.damage]);
                self.api_fdam[hit.defender_index] += hit.damage;
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BattleHougeki {
    pub api_at_eflag: Vec<i64>,
    pub api_at_list: Vec<i64>,
    pub api_at_type: Vec<i64>,
    pub api_df_list: Vec<Vec<i64>>,
    pub api_si_list: Vec<Vec<i64>>,
    pub api_cl_list: Vec<Vec<i64>>,
    pub api_damage: Vec<Vec<i64>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BattleNightHougeki {
    pub api_at_eflag: Vec<i64>,
    pub api_at_list: Vec<i64>,
    pub api_n_mother_list: Vec<i64>,
    pub api_df_list: Vec<Vec<i64>>,
    pub api_si_list: Vec<Vec<i64>>,
    pub api_cl_list: Vec<Vec<i64>>,
    pub api_sp_list: Vec<i64>,
    pub api_damage: Vec<Vec<i64>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BattleRaigeki {
    pub api_frai: Vec<i64>,
    pub api_fcl: Vec<i64>,
    pub api_fdam: Vec<i64>,
    pub api_fydam: Vec<i64>,
    pub api_erai: Vec<i64>,
    pub api_ecl: Vec<i64>,
    pub api_edam: Vec<i64>,
    pub api_eydam: Vec<i64>,
}

impl BattleRaigeki {
    pub(crate) fn blank(len: usize) -> Self {
        Self {
            api_frai: vec![-1; len],
            api_fcl: vec![0; len],
            api_fdam: vec![0; len],
            api_fydam: vec![0; len],
            api_erai: vec![-1; len],
            api_ecl: vec![0; len],
            api_edam: vec![0; len],
            api_eydam: vec![0; len],
        }
    }

    pub(crate) fn record_torpedo_hit(
        &mut self,
        attacker_side: TorpedoAttackerSide,
        hit: TorpedoHit,
    ) {
        match attacker_side {
            TorpedoAttackerSide::Friendly => {
                self.api_frai[hit.attacker_index] = hit.defender_index as i64;
                self.api_fcl[hit.attacker_index] = 1;
                self.api_fydam[hit.attacker_index] = hit.damage;
                self.api_edam[hit.defender_index] += hit.damage;
            }
            TorpedoAttackerSide::Enemy => {
                self.api_erai[hit.attacker_index] = hit.defender_index as i64;
                self.api_ecl[hit.attacker_index] = 1;
                self.api_eydam[hit.attacker_index] = hit.damage;
                self.api_fdam[hit.defender_index] += hit.damage;
            }
        }
    }
}
