use emukc_model::prelude::*;

use crate::parser::{error::ParseError, tsunkit_quest::Requirements};

impl Requirements {
    pub(super) fn extract_requirements_modernization(
        &self,
        mst: &ApiManifest,
    ) -> Result<Vec<Kc3rdQuestCondition>, ParseError> {
        let mut all: Vec<Kc3rdQuestCondition> = Vec::new();

        let times = self.times.unwrap_or(0);
        let target_ship = match &self.class_id {
            Some(class_id) => class_id.to_kc3rd_ship_class(mst),
            None => match &self.family_id {
                Some(family_id) => Some(Kc3rdQuestConditionShip::single_class(*family_id)),
                None => {
                    return Err(ParseError::EmptyRequirement {
                        reason: "modernization requirement must have a class_id or family_id"
                            .to_string(),
                    });
                }
            },
        };

        let Some(consumes) = &self.consume else {
            return Err(ParseError::EmptyRequirement {
                reason: "modernization requirement must have a consume".to_string(),
            });
        };

        if consumes.is_empty() || consumes.len() > 1 {
            return Err(ParseError::EmptyRequirement {
                reason: format!(
                    "modernization requirement must have exactly one consume, got {}",
                    consumes.len()
                ),
            });
        }

        let consume = &consumes[0];
        let material_ship = match &consume.class_id {
            Some(class_id) => class_id.to_kc3rd_ship_types(mst),
            None => {
                return Err(ParseError::EmptyRequirement {
                    reason: "modernization requirement consume must have a class_id".to_string(),
                });
            }
        };

        let Some(target_ship) = target_ship else {
            return Err(ParseError::EmptyRequirement {
                reason: "modernization requirement target_ship not found".to_string(),
            });
        };

        let Some(material_ship) = material_ship else {
            return Err(ParseError::EmptyRequirement {
                reason: "modernization requirement material_ship not found".to_string(),
            });
        };

        all.push(Kc3rdQuestCondition::Modernization(Kc3rdQuestConditionModernization {
            target_ship,
            material_ship,
            batch_size: consume.amount,
            times,
        }));

        if let Some(res) = self.extract_resource_consumption() {
            all.push(res);
        }

        Ok(all)
    }
}
