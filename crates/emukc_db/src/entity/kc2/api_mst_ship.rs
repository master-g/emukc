use emukc_model::start2::ApiMstShip;
use sea_orm::{entity::prelude::*, ActiveValue};

#[allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "api_mst_ship")]
pub struct Model {
	/// Primary key, `api_id`
	#[sea_orm(primary_key, auto_increment = false)]
	pub id: i64,
	pub afterbull: Option<i64>,
	pub afterfuel: Option<i64>,
	pub afterlv: Option<i64>,
	pub aftershipid: Option<String>,
	pub backs: Option<i64>,
	pub broken_fuel: Option<i64>,
	pub broken_ammo: Option<i64>,
	pub broken_steel: Option<i64>,
	pub broken_baux: Option<i64>,
	pub buildtime: Option<i64>,
	pub bull_max: Option<i64>,
	pub ctype: i64,
	pub fuel_max: Option<i64>,
	pub getmes: Option<String>,
	pub houg_min: Option<i64>,
	pub houg_max: Option<i64>,
	pub leng: Option<i64>,
	pub luck_min: Option<i64>,
	pub luck_max: Option<i64>,
	pub maxeq_slot_1: Option<i64>,
	pub maxeq_slot_2: Option<i64>,
	pub maxeq_slot_3: Option<i64>,
	pub maxeq_slot_4: Option<i64>,
	pub maxeq_slot_5: Option<i64>,
	pub name: String,
	pub powerup_firepower: Option<i64>,
	pub powerup_torpedo: Option<i64>,
	pub powerup_antiair: Option<i64>,
	pub powerup_armor: Option<i64>,
	pub raig_max: Option<i64>,
	pub raig_min: Option<i64>,
	pub slot_num: i64,
	pub soku: i64,
	pub sort_id: i64,
	pub sortno: Option<i64>,
	pub souk_max: Option<i64>,
	pub souk_min: Option<i64>,
	pub stype: i64,
	pub taik_max: Option<i64>,
	pub taik_min: Option<i64>,
	pub tais_max: Option<i64>,
	pub tais_min: Option<i64>,
	pub tyku_max: Option<i64>,
	pub tyku_min: Option<i64>,
	pub voicef: Option<i64>,
	pub yomi: String,
}

/// See <https://www.sea-ql.org/SeaORM/docs/generate-entity/entity-structure>
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<ApiMstShip> for ActiveModel {
	fn from(value: ApiMstShip) -> Self {
		Self {
			id: ActiveValue::Set(value.api_id),
			afterbull: ActiveValue::Set(value.api_afterbull),
			afterfuel: ActiveValue::Set(value.api_afterfuel),
			afterlv: ActiveValue::Set(value.api_afterlv),
			aftershipid: ActiveValue::Set(value.api_aftershipid),
			backs: ActiveValue::Set(value.api_backs),
			broken_fuel: ActiveValue::Set(
				value.api_broken.as_ref().and_then(|v| v.first().cloned()),
			),
			broken_ammo: ActiveValue::Set(
				value.api_broken.as_ref().and_then(|v| v.get(1).cloned()),
			),
			broken_steel: ActiveValue::Set(
				value.api_broken.as_ref().and_then(|v| v.get(2).cloned()),
			),
			broken_baux: ActiveValue::Set(
				value.api_broken.as_ref().and_then(|v| v.get(3).cloned()),
			),
			buildtime: ActiveValue::Set(value.api_buildtime),
			bull_max: ActiveValue::Set(value.api_bull_max),
			ctype: ActiveValue::Set(value.api_ctype),
			fuel_max: ActiveValue::Set(value.api_fuel_max),
			getmes: ActiveValue::Set(value.api_getmes),
			houg_min: ActiveValue::Set(value.api_houg.as_ref().and_then(|v| v.first().cloned())),
			houg_max: ActiveValue::Set(value.api_houg.as_ref().and_then(|v| v.get(1).cloned())),
			leng: ActiveValue::Set(value.api_leng),
			luck_min: ActiveValue::Set(value.api_luck.as_ref().and_then(|v| v.first().cloned())),
			luck_max: ActiveValue::Set(value.api_luck.as_ref().and_then(|v| v.get(1).cloned())),
			maxeq_slot_1: ActiveValue::Set(
				value.api_maxeq.as_ref().and_then(|v| v.first().cloned()),
			),
			maxeq_slot_2: ActiveValue::Set(
				value.api_maxeq.as_ref().and_then(|v| v.get(1).cloned()),
			),
			maxeq_slot_3: ActiveValue::Set(
				value.api_maxeq.as_ref().and_then(|v| v.get(2).cloned()),
			),
			maxeq_slot_4: ActiveValue::Set(
				value.api_maxeq.as_ref().and_then(|v| v.get(3).cloned()),
			),
			maxeq_slot_5: ActiveValue::Set(
				value.api_maxeq.as_ref().and_then(|v| v.get(4).cloned()),
			),
			name: ActiveValue::Set(value.api_name),
			powerup_firepower: ActiveValue::Set(
				value.api_powup.as_ref().and_then(|v| v.first().cloned()),
			),
			powerup_torpedo: ActiveValue::Set(
				value.api_powup.as_ref().and_then(|v| v.get(1).cloned()),
			),
			powerup_antiair: ActiveValue::Set(
				value.api_powup.as_ref().and_then(|v| v.get(2).cloned()),
			),
			powerup_armor: ActiveValue::Set(
				value.api_powup.as_ref().and_then(|v| v.get(3).cloned()),
			),
			raig_min: ActiveValue::Set(value.api_raig.as_ref().and_then(|v| v.first().cloned())),
			raig_max: ActiveValue::Set(value.api_raig.as_ref().and_then(|v| v.get(1).cloned())),
			slot_num: ActiveValue::Set(value.api_slot_num),
			soku: ActiveValue::Set(value.api_soku),
			sort_id: ActiveValue::Set(value.api_sort_id),
			sortno: ActiveValue::Set(value.api_sortno),
			souk_min: ActiveValue::Set(value.api_souk.as_ref().and_then(|v| v.first().cloned())),
			souk_max: ActiveValue::Set(value.api_souk.as_ref().and_then(|v| v.get(1).cloned())),
			stype: ActiveValue::Set(value.api_stype),
			taik_min: ActiveValue::Set(value.api_taik.as_ref().and_then(|v| v.first().cloned())),
			taik_max: ActiveValue::Set(value.api_taik.as_ref().and_then(|v| v.get(1).cloned())),
			tais_min: ActiveValue::Set(value.api_tais.as_ref().and_then(|v| v.first().cloned())),
			tais_max: ActiveValue::Set(value.api_tais.as_ref().and_then(|v| v.get(1).cloned())),
			tyku_min: ActiveValue::Set(value.api_tyku.as_ref().and_then(|v| v.first().cloned())),
			tyku_max: ActiveValue::Set(value.api_tyku.as_ref().and_then(|v| v.get(1).cloned())),
			voicef: ActiveValue::Set(value.api_voicef),
			yomi: ActiveValue::Set(value.api_yomi),
		}
	}
}

impl From<Model> for ApiMstShip {
	fn from(value: Model) -> Self {
		Self {
			api_id: value.id,
			api_afterbull: value.afterbull,
			api_afterfuel: value.afterfuel,
			api_afterlv: value.afterlv,
			api_aftershipid: value.aftershipid,
			api_backs: value.backs,
			api_broken: {
				let broken: Vec<i64> = vec![
					value.broken_fuel,
					value.broken_ammo,
					value.broken_steel,
					value.broken_baux,
				]
				.into_iter()
				.flatten()
				.collect();
				if broken.is_empty() {
					None
				} else {
					Some(broken)
				}
			},
			api_buildtime: value.buildtime,
			api_bull_max: value.bull_max,
			api_ctype: value.ctype,
			api_fuel_max: value.fuel_max,
			api_getmes: value.getmes,
			api_houg: {
				let houg: Vec<i64> =
					vec![value.houg_min, value.houg_max].into_iter().flatten().collect();
				if houg.is_empty() {
					None
				} else {
					Some(houg)
				}
			},
			api_leng: value.leng,
			api_luck: {
				let luck: Vec<i64> =
					vec![value.luck_min, value.luck_max].into_iter().flatten().collect();
				if luck.is_empty() {
					None
				} else {
					Some(luck)
				}
			},
			api_maxeq: {
				let maxeq: Vec<i64> = vec![
					value.maxeq_slot_1,
					value.maxeq_slot_2,
					value.maxeq_slot_3,
					value.maxeq_slot_4,
					value.maxeq_slot_5,
				]
				.into_iter()
				.flatten()
				.collect();
				if maxeq.is_empty() {
					None
				} else {
					Some(maxeq)
				}
			},
			api_name: value.name,
			api_powup: {
				let powup: Vec<i64> = vec![
					value.powerup_firepower,
					value.powerup_torpedo,
					value.powerup_antiair,
					value.powerup_armor,
				]
				.into_iter()
				.flatten()
				.collect();
				if powup.is_empty() {
					None
				} else {
					Some(powup)
				}
			},
			api_raig: {
				let raig: Vec<i64> =
					vec![value.raig_min, value.raig_max].into_iter().flatten().collect();
				if raig.is_empty() {
					None
				} else {
					Some(raig)
				}
			},
			api_slot_num: value.slot_num,
			api_soku: value.soku,
			api_sort_id: value.sort_id,
			api_sortno: value.sortno,
			api_souk: {
				let souk: Vec<i64> =
					vec![value.souk_min, value.souk_max].into_iter().flatten().collect();
				if souk.is_empty() {
					None
				} else {
					Some(souk)
				}
			},
			api_stype: value.stype,
			api_taik: {
				let taik: Vec<i64> =
					vec![value.taik_min, value.taik_max].into_iter().flatten().collect();
				if taik.is_empty() {
					None
				} else {
					Some(taik)
				}
			},
			api_tyku: {
				let tyku: Vec<i64> =
					vec![value.tyku_min, value.tyku_max].into_iter().flatten().collect();
				if tyku.is_empty() {
					None
				} else {
					Some(tyku)
				}
			},
			api_voicef: value.voicef,
			api_yomi: value.yomi,
			api_tais: {
				let tais: Vec<i64> =
					vec![value.tais_min, value.tais_max].into_iter().flatten().collect();
				if tais.is_empty() {
					None
				} else {
					Some(tais)
				}
			},
		}
	}
}
