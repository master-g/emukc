//! API packet types with Serialize derivations.
//! These directly serialize to JSON for the `KanColle` API response.

use serde::Serialize;

use super::domain::TorpedoAttackerSide;
use super::domain::TorpedoHit;

/// A single equipment ID in `api_si_list`.
///
/// The KC API uses JSON value type to distinguish attack rendering paths:
/// cut-in / special-attack entries are **strings** (e.g. `"22"`),
/// while normal-attack entries are **integers** (e.g. `161`).
/// The `-1` sentinel (no equipment) is always an integer.
///
/// `#[serde(untagged)]` ensures clean JSON output without type tags.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(untagged)]
pub enum SiListId {
    /// Integer-valued ID (normal attacks, sentinels).
    Num(i64),
    /// String-valued ID (cut-in / special attacks).
    Text(String),
}

impl SiListId {
    /// Convert a slice of `i64` IDs into `Text` variants for CI / special-attack entries.
    /// The `-1` sentinel is kept as `Num(-1)` per the official server format.
    pub(crate) fn text_from_i64(ids: &[i64]) -> Vec<Self> {
        ids.iter()
            .map(|&id| {
                if id < 0 {
                    Self::Num(id)
                } else {
                    Self::Text(id.to_string())
                }
            })
            .collect()
    }

    /// Convert a slice of `i64` IDs into `Num` variants for normal-attack entries.
    pub(crate) fn num_from_i64(ids: &[i64]) -> Vec<Self> {
        ids.iter().map(|&id| Self::Num(id)).collect()
    }
}

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
    pub api_si_list: Vec<Vec<SiListId>>,
    pub api_cl_list: Vec<Vec<i64>>,
    pub api_damage: Vec<Vec<i64>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BattleNightHougeki {
    pub api_at_eflag: Vec<i64>,
    pub api_at_list: Vec<i64>,
    pub api_n_mother_list: Vec<i64>,
    pub api_df_list: Vec<Vec<i64>>,
    pub api_si_list: Vec<Vec<SiListId>>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn num_serializes_as_json_integer() {
        let json = serde_json::to_string(&SiListId::Num(161)).unwrap();
        assert_eq!(json, "161");
    }

    #[test]
    fn text_serializes_as_json_string() {
        let json = serde_json::to_string(&SiListId::Text("291".into())).unwrap();
        assert_eq!(json, "\"291\"");
    }

    #[test]
    fn num_negative_one_serializes_as_integer_not_string() {
        let json = serde_json::to_string(&SiListId::Num(-1)).unwrap();
        assert_eq!(json, "-1");
    }

    #[test]
    fn text_from_i64_stringifies_positive_ids() {
        let ids = SiListId::text_from_i64(&[22, 291, 112]);
        let json = serde_json::to_string(&ids).unwrap();
        assert_eq!(json, "[\"22\",\"291\",\"112\"]");
    }

    #[test]
    fn text_from_i64_keeps_negative_as_num() {
        let ids = SiListId::text_from_i64(&[-1, 22]);
        assert_eq!(ids[0], SiListId::Num(-1));
        assert_eq!(ids[1], SiListId::Text("22".into()));
    }

    #[test]
    fn num_from_i64_all_num_variants() {
        let ids = SiListId::num_from_i64(&[161, -1]);
        assert_eq!(ids[0], SiListId::Num(161));
        assert_eq!(ids[1], SiListId::Num(-1));
    }

    #[test]
    fn hougeki_si_list_mixed_types_serialize_correctly() {
        let hougeki = BattleHougeki {
            api_at_eflag: vec![0],
            api_at_list: vec![0],
            api_at_type: vec![7],
            api_df_list: vec![vec![0]],
            // Carrier CI: FBA pattern with string IDs
            api_si_list: vec![SiListId::text_from_i64(&[22, 291, 112])],
            api_cl_list: vec![vec![1]],
            api_damage: vec![vec![150]],
        };
        let json = serde_json::to_string(&hougeki).unwrap();
        assert!(
            json.contains("\"22\",\"291\",\"112\""),
            "CI entries must serialize as strings: {json}"
        );
    }

    #[test]
    fn hougeki_si_list_normal_attack_serializes_as_integers() {
        let hougeki = BattleHougeki {
            api_at_eflag: vec![0],
            api_at_list: vec![0],
            api_at_type: vec![0],
            api_df_list: vec![vec![0]],
            api_si_list: vec![SiListId::num_from_i64(&[161])],
            api_cl_list: vec![vec![1]],
            api_damage: vec![vec![50]],
        };
        let json = serde_json::to_string(&hougeki).unwrap();
        assert!(
            !json.contains("\"161\""),
            "normal attack must serialize as integer, not string: {json}"
        );
        assert!(json.contains("161"), "161 must appear in JSON: {json}");
    }
}
