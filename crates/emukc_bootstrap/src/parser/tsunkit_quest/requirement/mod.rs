mod conversion;
mod exchange;
mod exercise;
mod expedition;
mod modernization;
mod simple;
mod sortie;

use emukc_model::prelude::*;

use super::{ClassId, ConsumeCategory, Requirements, RequirementsCategory};
use crate::parser::error::ParseError;

impl Requirements {
    pub(super) fn to_requirements(
        &self,
        mst: &ApiManifest,
    ) -> Result<Kc3rdQuestRequirement, ParseError> {
        let conditions = self.extract_conditions(mst)?;
        Ok(match self.category {
            RequirementsCategory::Or => Kc3rdQuestRequirement::OneOf(conditions),
            RequirementsCategory::Then => Kc3rdQuestRequirement::Sequential(conditions),
            _ => Kc3rdQuestRequirement::And(conditions),
        })
    }

    fn extract_conditions(
        &self,
        mst: &ApiManifest,
    ) -> Result<Vec<Kc3rdQuestCondition>, ParseError> {
        match self.category {
            RequirementsCategory::And | RequirementsCategory::Or | RequirementsCategory::Then => {
                self.extract_list(mst)
            }
            RequirementsCategory::Conversion => Ok(self.extract_requirements_conversion(mst)),
            RequirementsCategory::Equipexchange => {
                Ok(self.extract_requirements_equip_exchange(mst))
            }
            RequirementsCategory::Exercise => Ok(self.extract_requirements_exercise(mst)),
            RequirementsCategory::Expedition => Ok(self.extract_requirements_expedition()),
            RequirementsCategory::Fleet => self.extract_requirements_fleet(mst),
            RequirementsCategory::Modernization => self.extract_requirements_modernization(mst),
            RequirementsCategory::Scrapequipment => self.extract_requirements_scrap_equipment(mst),
            RequirementsCategory::Simple => self.extract_requirements_simple(),
            RequirementsCategory::Sink => self.extract_requirements_sink(),
            RequirementsCategory::Sortie => self.extract_requirements_sortie(mst),
            RequirementsCategory::Unknown => Err(ParseError::UnknownCategory),
        }
    }

    fn extract_list(&self, mst: &ApiManifest) -> Result<Vec<Kc3rdQuestCondition>, ParseError> {
        let Some(list) = &self.list else {
            return Ok(vec![]);
        };
        // An `or` branch that cannot be resolved only removes a way to complete
        // the quest, so the remaining branches stay usable. In `and` / `then` a
        // dropped item would silently relax the requirement, so it propagates.
        let tolerate_failed_branch = matches!(self.category, RequirementsCategory::Or);
        let mut result = Vec::new();
        for item in list {
            match Requirements::from(item.clone()).extract_conditions(mst) {
                Ok(conditions) => result.extend(conditions),
                Err(ParseError::EmptyRequirement {
                    reason,
                }) if tolerate_failed_branch => {
                    warn!(reason = %reason, "dropping unresolvable branch of an 'or' requirement");
                }
                Err(e) => return Err(e),
            }
        }
        if result.is_empty() {
            return Err(ParseError::EmptyRequirement {
                reason: format!(
                    "no branch of the {} requirement resolved to a condition",
                    list.len()
                ),
            });
        }
        Ok(result)
    }

    pub(super) fn extract_resource_consumption(&self) -> Option<Kc3rdQuestCondition> {
        self.resources.as_ref().map(|resources| {
            Kc3rdQuestCondition::Consumption(Kc3rdQuestConditionConsumption::Resources(
                Kc3rdQuestConditionMaterialConsumption {
                    fuel: resources[0],
                    ammo: resources[1],
                    steel: resources[2],
                    bauxite: resources[3],
                },
            ))
        })
    }

    fn extract_requirements_scrap_equipment(
        &self,
        mst: &ApiManifest,
    ) -> Result<Vec<Kc3rdQuestCondition>, ParseError> {
        let Some(list) = &self.list else {
            return Err(ParseError::EmptyRequirement {
                reason: "scrap equipment requirement must have a list".to_string(),
            });
        };

        let slotitems: Vec<Kc3rdQuestConditionSlotItem> = list
            .iter()
            .filter_map(|item| {
                let amount = item.amount.unwrap_or(1);
                let id = item.id.unwrap_or(0).abs(); // there are some negative ids in tsunkit db
                match item.category.as_str() {
                    "equipment" => match mst.find_slotitem(id) {
                        Some(mst) => {
                            debug!("slot item found: {}, {}", mst.api_id, mst.api_name);
                            Some(Kc3rdQuestConditionSlotItem {
                                item_type: Kc3rdQuestConditionSlotItemType::single_equipment(id),
                                amount,
                                stars: 0,
                                fully_skilled: false,
                            })
                        }
                        None => None,
                    },
                    "equipgroup" => match mst.find_slotitem_type(id) {
                        Some(mst) => {
                            debug!("slot item type found: {}, {}", mst.api_id, mst.api_name);
                            Some(Kc3rdQuestConditionSlotItem {
                                item_type: Kc3rdQuestConditionSlotItemType::single_type(id),
                                amount,
                                stars: 0,
                                fully_skilled: false,
                            })
                        }
                        None => None,
                    },
                    _ => None,
                }
            })
            .collect();

        if slotitems.is_empty() {
            return Err(ParseError::EmptyRequirement {
                reason: "scrap equipment requirement resolved to no known items".to_string(),
            });
        }

        let mut all: Vec<Kc3rdQuestCondition> = Vec::new();
        if let Some(res) = self.extract_resource_consumption() {
            all.push(res);
        }
        all.push(Kc3rdQuestCondition::Scrap(Kc3rdQuestConditionScrap::SpecificItems(slotitems)));

        Ok(all)
    }

    pub(super) fn extract_useitem_consume(
        &self,
        mst: &ApiManifest,
    ) -> Option<Vec<Kc3rdQuestConditionUseItemConsumption>> {
        if let Some(consume) = &self.consume {
            let consumptions: Vec<Kc3rdQuestConditionUseItemConsumption> = consume
                .iter()
                .filter_map(|c| {
                    let api_id = if let Some(id) = c.id {
                        id.abs()
                    } else {
                        error!("consume requirement must have an id");
                        return None;
                    };

                    if let Some(category) = &c.category {
                        match category {
                            ConsumeCategory::Inventory => {
                                if let Some(useitem_mst) = mst.find_useitem(api_id) {
                                    debug!(
                                        "use item found: {}, {}",
                                        useitem_mst.api_id, useitem_mst.api_name
                                    );
                                } else {
                                    debug!("use item not found: {}", api_id);
                                    return None;
                                }
                                Some(Kc3rdQuestConditionUseItemConsumption {
                                    api_id,
                                    amount: c.amount,
                                })
                            }
                            _ => None,
                        }
                    } else {
                        None
                    }
                })
                .collect();
            if consumptions.is_empty() {
                None
            } else {
                Some(consumptions)
            }
        } else {
            None
        }
    }

    pub(super) fn extract_slotitem_consume(
        &self,
        mst: &ApiManifest,
    ) -> Option<Vec<Kc3rdQuestConditionSlotItem>> {
        if let Some(consume) = &self.consume {
            let consumptions: Vec<Kc3rdQuestConditionSlotItem> = consume
                .iter()
                .filter_map(|c| {
                    let api_id = if let Some(id) = c.id {
                        id.abs()
                    } else {
                        error!("consume requirement must have an id");
                        return None;
                    };

                    let stars = c.stars.unwrap_or(0);

                    if let Some(category) = &c.category {
                        match category {
                            ConsumeCategory::Equipgroup => {
                                if let Some(equipgroup_mst) = mst.find_slotitem_type(api_id) {
                                    debug!(
                                        "slot item type found: {}, {}",
                                        equipgroup_mst.api_id, equipgroup_mst.api_name
                                    );
                                } else {
                                    error!("slot item type not found: {}", api_id);
                                    return None;
                                }
                                Some(Kc3rdQuestConditionSlotItem {
                                    item_type: Kc3rdQuestConditionSlotItemType::single_type(api_id),
                                    amount: c.amount,
                                    stars,
                                    fully_skilled: false,
                                })
                            }
                            ConsumeCategory::Equipment => {
                                if let Some(equipment_mst) = mst.find_slotitem(api_id) {
                                    debug!(
                                        "slot item found: {}, {}",
                                        equipment_mst.api_id, equipment_mst.api_name
                                    );
                                } else {
                                    error!("slot item not found: {}", api_id);
                                    return None;
                                }
                                Some(Kc3rdQuestConditionSlotItem {
                                    item_type: Kc3rdQuestConditionSlotItemType::single_equipment(
                                        api_id,
                                    ),
                                    amount: c.amount,
                                    stars,
                                    fully_skilled: false,
                                })
                            }
                            _ => None,
                        }
                    } else {
                        None
                    }
                })
                .collect();
            if consumptions.is_empty() {
                None
            } else {
                Some(consumptions)
            }
        } else {
            None
        }
    }

    fn extract_requirements_sink(&self) -> Result<Vec<Kc3rdQuestCondition>, ParseError> {
        let Some(group_id) = self.group_id else {
            return Err(ParseError::EmptyRequirement {
                reason: "sink requirement must have a group_id".to_string(),
            });
        };

        let Some(ship) = ClassId::find_ship_group(group_id) else {
            return Err(ParseError::EmptyRequirement {
                reason: format!("sink requirement ship group not found: {group_id}"),
            });
        };

        let amount = self.amount.unwrap_or(1);

        Ok(vec![Kc3rdQuestCondition::Sink(ship, amount)])
    }

    fn extract_requirements_fleet(
        &self,
        mst: &ApiManifest,
    ) -> Result<Vec<Kc3rdQuestCondition>, ParseError> {
        let Some(comp) = self.extract_fleet(mst) else {
            return Err(ParseError::EmptyRequirement {
                reason: "fleet requirement must have a comp".to_string(),
            });
        };

        Ok(vec![Kc3rdQuestCondition::Composition(comp)])
    }

    fn extract_fleet(&self, mst: &ApiManifest) -> Option<Kc3rdQuestConditionComposition> {
        let fleet_id = self.fleet_id.unwrap_or(0);

        let groups: Vec<Kc3rdQuestConditionShipGroup> = if let Some(comp) = &self.comp {
            comp.iter().filter_map(|c| c.to_kc3rd_ship_group(mst)).collect()
        } else {
            return None;
        };

        let disallowed = if let Some(comp_banned) = &self.comp_banned {
            let mut banned: Vec<Kc3rdQuestConditionShip> = comp_banned
                .iter()
                .filter_map(|ban| {
                    if let Some(class_id) = &ban.class_id {
                        class_id.to_kc3rd_ship_class(mst)
                    } else if let Some(ship_id) = &ban.ship_id {
                        ship_id.to_kc3rd_ship_ids(mst)
                    } else {
                        None
                    }
                })
                .collect();

            if let Some(extra_banned) = &self.disallowed {
                let ship = match extra_banned {
                    super::Disallowed::Aviation => Some(Kc3rdQuestConditionShip::Aviation),
                    super::Disallowed::Carriers => Some(Kc3rdQuestConditionShip::Carrier),
                    _ => None,
                };
                if let Some(ship) = ship {
                    banned.push(ship);
                }
            }

            Some(banned)
        } else {
            None
        };

        Some(Kc3rdQuestConditionComposition {
            groups,
            disallowed,
            fleet_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use emukc_model::prelude::*;

    use super::super::{Id, List, Requirements, RequirementsCategory, Sortie};
    use super::ParseError;

    fn empty_manifest() -> ApiManifest {
        ApiManifest::default()
    }

    fn empty_requirements() -> Requirements {
        Requirements {
            category: RequirementsCategory::And,
            comp: None,
            fleet_id: None,
            disallowed: None,
            comp_banned: None,
            sortie: None,
            subcategory: None,
            times: None,
            group_id: None,
            amount: None,
            list: None,
            result: None,
            daily: None,
            expeds: None,
            resources: None,
            secretary: None,
            slots: None,
            scrap: None,
            consume: None,
            batch: None,
            secretary_banned: None,
            class_id: None,
            family_id: None,
        }
    }

    fn empty_list() -> List {
        List {
            category: String::new(),
            sortie: None,
            comp: None,
            disallowed: None,
            result: None,
            daily: None,
            times: None,
            slots: None,
            id: None,
            amount: None,
            scrap: None,
            resources: None,
            consume: None,
        }
    }

    #[test]
    fn unknown_category_returns_error() {
        let req = Requirements {
            category: RequirementsCategory::Unknown,
            ..empty_requirements()
        };

        let result = req.to_requirements(&empty_manifest());
        assert!(matches!(result, Err(ParseError::UnknownCategory)));
    }

    #[test]
    fn and_category_with_empty_list_succeeds() {
        let req = Requirements {
            category: RequirementsCategory::And,
            ..empty_requirements()
        };

        let result = req.to_requirements(&empty_manifest());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Kc3rdQuestRequirement::And(vec![]));
    }

    #[test]
    fn nested_unknown_category_propagates_error() {
        let list_item = List {
            category: "unknown_xyz".to_string(),
            ..empty_list()
        };

        let req = Requirements {
            list: Some(vec![list_item]),
            ..empty_requirements()
        };

        let result = req.to_requirements(&empty_manifest());
        assert!(matches!(result, Err(ParseError::UnknownCategory)));
    }

    /// The shape of api_no 1033: `category: sortie` with no `sortie` block.
    /// Before plan 006 this degraded to `And([])`, which the progress
    /// calculator reads as already completed.
    #[test]
    fn sortie_category_without_sortie_field_is_rejected() {
        let req = Requirements {
            category: RequirementsCategory::Sortie,
            ..empty_requirements()
        };

        let result = req.to_requirements(&empty_manifest());
        assert!(matches!(result, Err(ParseError::EmptyRequirement { .. })));
    }

    #[test]
    fn sortie_category_with_sortie_field_succeeds() {
        let req = Requirements {
            category: RequirementsCategory::Sortie,
            sortie: Some(vec![Sortie {
                map: Some(Id::String("1-1".to_string())),
                boss: Some(true),
                result: None,
                times: Some(2),
                node: None,
                any: None,
            }]),
            ..empty_requirements()
        };

        let result = req.to_requirements(&empty_manifest());
        let Ok(Kc3rdQuestRequirement::And(conditions)) = result else {
            panic!("expected And conditions, got {result:?}");
        };
        assert!(matches!(
            conditions.as_slice(),
            [Kc3rdQuestCondition::Sortie(sortie)] if sortie.times == 2 && sortie.defeat_boss
        ));
    }

    /// The shape of api_no 1019: an `or` whose first branch has no `sortie`
    /// block and whose second one is complete. Dropping a branch only removes
    /// a way to finish the quest, so the quest itself must survive.
    #[test]
    fn or_keeps_the_branches_that_resolve() {
        let broken = List {
            category: "sortie".to_string(),
            ..empty_list()
        };
        let usable = List {
            category: "sortie".to_string(),
            sortie: Some(vec![Sortie {
                map: Some(Id::String("4-5".to_string())),
                boss: Some(true),
                result: None,
                times: Some(2),
                node: None,
                any: None,
            }]),
            ..empty_list()
        };

        let req = Requirements {
            category: RequirementsCategory::Or,
            list: Some(vec![broken, usable]),
            ..empty_requirements()
        };

        let result = req.to_requirements(&empty_manifest());
        let Ok(Kc3rdQuestRequirement::OneOf(conditions)) = result else {
            panic!("expected OneOf conditions, got {result:?}");
        };
        assert!(matches!(conditions.as_slice(), [Kc3rdQuestCondition::Sortie(_)]));
    }

    #[test]
    fn or_with_every_branch_unresolvable_is_rejected() {
        let broken = List {
            category: "sortie".to_string(),
            ..empty_list()
        };

        let req = Requirements {
            category: RequirementsCategory::Or,
            list: Some(vec![broken]),
            ..empty_requirements()
        };

        let result = req.to_requirements(&empty_manifest());
        assert!(matches!(result, Err(ParseError::EmptyRequirement { .. })));
    }

    /// Semantics flipped by plan 006: a `simple` requirement with no
    /// subcategory used to degrade to `And([])`; it is now rejected.
    #[test]
    fn simple_category_without_subcategory_is_rejected() {
        let req = Requirements {
            category: RequirementsCategory::Simple,
            ..empty_requirements()
        };

        let result = req.to_requirements(&empty_manifest());
        assert!(matches!(result, Err(ParseError::EmptyRequirement { .. })));
    }
}
