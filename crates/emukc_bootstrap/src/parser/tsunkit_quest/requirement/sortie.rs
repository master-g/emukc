use emukc_model::prelude::*;

use crate::parser::{error::ParseError, tsunkit_quest::Requirements};

impl Requirements {
    pub(super) fn extract_requirements_sortie(
        &self,
        mst: &ApiManifest,
    ) -> Result<Vec<Kc3rdQuestCondition>, ParseError> {
        let mut result: Vec<Kc3rdQuestCondition> = Vec::new();
        let fleet = self.extract_fleet(mst);
        if let Some(fleet) = fleet {
            result.push(Kc3rdQuestCondition::Composition(fleet));
        }
        let Some(sorties) = &self.sortie else {
            return Err(ParseError::EmptyRequirement {
                reason: "sortie requirement must have a 'sortie' field".to_string(),
            });
        };
        sorties.iter().for_each(|sortie| {
            let kc3_sortie = sortie.to_kc3rd_sortie();
            result.push(Kc3rdQuestCondition::Sortie(kc3_sortie));
        });
        Ok(result)
    }
}
