use crate::{
    kc2::start2::ApiManifest,
    prelude::{
        Kc3rdQuestConditionConsumption, Kc3rdQuestConditionFactory, Kc3rdQuestConditionScrap,
    },
    thirdparty::{Kc3rdQuestCondition, Kc3rdQuestConditionComposition},
};

use super::Kc3rdQuestDebugJson;

impl Kc3rdQuestDebugJson for Kc3rdQuestCondition {
    fn to_json(&self, mst: &ApiManifest) -> serde_json::Value {
        match self {
            Kc3rdQuestCondition::Composition(comp) => serde_json::json!({
                "type": "COMPOSITION",
                "composition": comp.to_json(mst),
            }),
            Kc3rdQuestCondition::Exercise(info) => serde_json::json!({
                "type": "EXERCISE",
                "info": info.to_json(mst),
            }),
            Kc3rdQuestCondition::Expedition(info) => serde_json::json!({
                "type": "EXPEDITION",
                "info": info.iter().map(|i| i.to_json(mst)).collect::<Vec<serde_json::Value>>(),
            }),
            Kc3rdQuestCondition::ModelConversion(info) => serde_json::json!({
                "type": "MODEL_CONVERSION",
                "info": info.to_json(mst),
            }),
            Kc3rdQuestCondition::Modernization(info) => serde_json::json!({
                "type": "MODERNIZATION",
                "info": info.to_json(mst),
            }),
            Kc3rdQuestCondition::Repair(n) => serde_json::json!({
                "type": "REPAIR",
                "times": n,
            }),
            Kc3rdQuestCondition::Resupply(n) => serde_json::json!({
                "type": "RESUPPLY",
                "times": n,
            }),
            Kc3rdQuestCondition::Sink(ship, amount) => serde_json::json!({
                "type": "SINK",
                "ships": ship.to_json(mst),
                "amount": amount,
            }),
            Kc3rdQuestCondition::Sortie(info) => serde_json::json!({
                "type": "SORTIE",
                "info": info.to_json(mst),
            }),
            Kc3rdQuestCondition::Factory(factory) => match factory {
                Kc3rdQuestConditionFactory::ShipConstruction(n) => {
                    serde_json::json!({
                        "type": "FACTORY_SHIP_CONSTRUCTION",
                        "times": n,
                    })
                }
                Kc3rdQuestConditionFactory::SlotItemConstruction(n) => {
                    serde_json::json!({
                        "type": "FACTORY_SLOT_ITEM_CONSTRUCTION",
                        "times": n,
                    })
                }
                Kc3rdQuestConditionFactory::SlotItemImprovement(n) => {
                    serde_json::json!({
                        "type": "FACTORY_SLOT_ITEM_IMPROVEMENT",
                        "times": n,
                    })
                }
            },
            Kc3rdQuestCondition::Scrap(scrap) => match scrap {
                Kc3rdQuestConditionScrap::AnyEquipment(n) => {
                    serde_json::json!({
                        "type": "SCRAP_ANY_EQUIPMENT",
                        "times": n,
                    })
                }
                Kc3rdQuestConditionScrap::AnyShip(n) => {
                    serde_json::json!({
                        "type": "SCRAP_ANY_SHIP",
                        "times": n,
                    })
                }
                Kc3rdQuestConditionScrap::SpecificItems(items) => {
                    serde_json::json!({
                        "type": "SCRAP_SPECIFIC_ITEMS",
                        "items": items.iter().map(|i| i.to_json(mst)).collect::<Vec<serde_json::Value>>(),
                    })
                }
            },
            Kc3rdQuestCondition::Consumption(consumption) => match consumption {
                Kc3rdQuestConditionConsumption::Resources(res) => {
                    serde_json::json!({
                        "type": "CONSUMPTION_RESOURCES",
                        "resources": {
                            "fuel": res.fuel,
                            "ammo": res.ammo,
                            "steel": res.steel,
                            "bauxite": res.bauxite,
                        },
                    })
                }
                Kc3rdQuestConditionConsumption::SlotItemConsumption(items) => {
                    serde_json::json!({
                        "type": "CONSUMPTION_SLOT_ITEM",
                        "items": items.iter().map(|i| i.to_json(mst)).collect::<Vec<serde_json::Value>>(),
                    })
                }
                Kc3rdQuestConditionConsumption::UseItemConsumption(items) => {
                    serde_json::json!({
                        "type": "CONSUMPTION_USE_ITEM",
                        "items": items.iter().map(|i| i.to_json(mst)).collect::<Vec<serde_json::Value>>(),
                    })
                }
            },
        }
    }
}

impl Kc3rdQuestDebugJson for Kc3rdQuestConditionComposition {
    fn to_json(&self, mst: &ApiManifest) -> serde_json::Value {
        let disallow = self.disallowed.as_ref().map(|disallowed| {
            disallowed.iter().map(|item| item.to_json(mst)).collect::<Vec<serde_json::Value>>()
        });

        let groups = self.groups.iter().map(|g| g.to_json(mst)).collect::<Vec<serde_json::Value>>();

        serde_json::json!({
            "fleet_id": self.fleet_id,
            "groups": groups,
            "disallowed": disallow,
        })
    }
}
