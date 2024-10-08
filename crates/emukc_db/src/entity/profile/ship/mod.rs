//! Ship related entities
use emukc_model::kc2::KcApiShip;
use sea_orm::{entity::prelude::*, ActiveValue};

pub mod picturebook;

#[allow(missing_docs)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "ship")]
pub struct Model {
	/// Instance ID
	#[sea_orm(primary_key, auto_increment = true)]
	pub id: i64,

	/// Profile ID
	pub profile_id: i64,

	/// sort number
	pub sort_num: i64,

	/// Manifest ID
	pub mst_id: i64,

	/// ship level
	pub level: i64,

	/// current experience
	pub exp_now: i64,

	/// experience needed for next level
	pub exp_next: i64,

	/// experience progress bar percentage
	pub exp_progress: i64,

	/// Married
	pub married: bool,

	/// Locked
	pub locked: bool,

	/// background, for rarity
	pub backs: i64,

	/// current hp
	pub hp_now: i64,

	/// maximum hp
	pub hp_max: i64,

	/// speed, soku
	pub speed: i64,

	/// range, leng
	pub range: i64,

	/// slots, first
	pub slot_1: i64,

	/// slots, second
	pub slot_2: i64,

	/// slots, third
	pub slot_3: i64,

	/// slots, fourth
	pub slot_4: i64,

	/// slots, fifth
	pub slot_5: i64,

	/// extra slots
	pub slot_ex: i64,

	/// aircraft capacity left
	pub onslot_1: i64,

	/// aircraft capacity left
	pub onslot_2: i64,

	/// aircraft capacity left
	pub onslot_3: i64,

	/// aircraft capacity left
	pub onslot_4: i64,

	/// aircraft capacity left
	pub onslot_5: i64,

	/// modrenization, firepower
	pub mod_firepower: i64,

	/// modrenization, torpedo
	pub mod_torpedo: i64,

	/// modrenization, AA
	pub mod_aa: i64,

	/// modrenization, armor
	pub mod_armor: i64,

	/// modrenization, luck
	pub mod_luck: i64,

	/// modrenization, HP
	pub mod_hp: i64,

	/// modrenization, ASW
	pub mod_asw: i64,

	/// fuel left
	pub fuel: i64,

	/// ammo left
	pub ammo: i64,

	/// slot number
	pub slot_num: i64,

	/// repair time, in milliseconds
	pub ndock_time: i64,

	/// repair fuel consumption
	pub ndock_fuel: i64,

	/// repair steel consumption
	pub ndock_steel: i64,

	/// modernization rate, number of stars
	pub srate: i64,

	/// morale
	pub condition: i64,

	/// firepower now, including equipment
	pub firepower_now: i64,

	/// firepower max
	pub firepower_max: i64,

	/// torpedo now
	pub torpedo_now: i64,

	/// torpedo max
	pub torpedo_max: i64,

	/// AA now
	pub aa_now: i64,

	/// AA max
	pub aa_max: i64,

	/// armor now
	pub armor_now: i64,

	/// armor max
	pub armor_max: i64,

	/// evasion now
	pub evasion_now: i64,

	/// evasion max
	pub evasion_max: i64,

	/// ASW now
	pub asw_now: i64,

	/// ASW max
	pub asw_max: i64,

	/// LOS now
	pub los_now: i64,

	/// LOS max
	pub los_max: i64,

	/// luck now
	pub luck_now: i64,

	/// luck max
	pub luck_max: i64,

	/// has locked equipment
	pub has_locked_euqip: bool,

	/// Sally area
	pub sally_area: i64,
}

/// Relation
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	/// Relation to `Profile`
	#[sea_orm(
		belongs_to = "crate::entity::profile::Entity",
		from = "Column::ProfileId",
		to = "crate::entity::profile::Column::Id"
	)]
	Profile,
}

impl Related<crate::entity::profile::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Profile.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}

/// Bootstrap the database with the necessary tables
pub async fn bootstrap(db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::error::DbErr> {
	let schema = sea_orm::Schema::new(db.get_database_backend());
	// ship
	{
		let stmt = schema.create_table_from_entity(Entity).if_not_exists().to_owned();
		db.execute(db.get_database_backend().build(&stmt)).await?;
	}
	// picturebook
	{
		let stmt = schema.create_table_from_entity(picturebook::Entity).if_not_exists().to_owned();
		db.execute(db.get_database_backend().build(&stmt)).await?;
	}

	Ok(())
}

impl From<KcApiShip> for ActiveModel {
	fn from(value: KcApiShip) -> Self {
		Self {
			profile_id: ActiveValue::NotSet,
			id: ActiveValue::Set(value.api_id),
			sort_num: ActiveValue::Set(value.api_sortno),
			mst_id: ActiveValue::Set(value.api_ship_id),
			level: ActiveValue::Set(value.api_lv),
			exp_now: ActiveValue::Set(value.api_exp[0]),
			exp_next: ActiveValue::Set(value.api_exp[1]),
			exp_progress: ActiveValue::Set(value.api_exp[2]),
			married: ActiveValue::Set(value.api_lv > 99),
			locked: ActiveValue::Set(value.api_locked == 1),
			backs: ActiveValue::Set(value.api_backs),
			hp_now: ActiveValue::Set(value.api_nowhp),
			hp_max: ActiveValue::Set(value.api_maxhp),
			speed: ActiveValue::Set(value.api_soku),
			range: ActiveValue::Set(value.api_leng),
			slot_1: ActiveValue::Set(value.api_slot[0]),
			slot_2: ActiveValue::Set(value.api_slot[1]),
			slot_3: ActiveValue::Set(value.api_slot[2]),
			slot_4: ActiveValue::Set(value.api_slot[3]),
			slot_5: ActiveValue::Set(value.api_slot[4]),
			slot_ex: ActiveValue::Set(value.api_slot_ex),
			onslot_1: ActiveValue::Set(value.api_onslot[0]),
			onslot_2: ActiveValue::Set(value.api_onslot[1]),
			onslot_3: ActiveValue::Set(value.api_onslot[2]),
			onslot_4: ActiveValue::Set(value.api_onslot[3]),
			onslot_5: ActiveValue::Set(value.api_onslot[4]),
			mod_firepower: ActiveValue::Set(value.api_kyouka[0]),
			mod_torpedo: ActiveValue::Set(value.api_kyouka[1]),
			mod_aa: ActiveValue::Set(value.api_kyouka[2]),
			mod_armor: ActiveValue::Set(value.api_kyouka[3]),
			mod_luck: ActiveValue::Set(value.api_kyouka[4]),
			mod_hp: ActiveValue::Set(value.api_kyouka[5]),
			mod_asw: ActiveValue::Set(value.api_kyouka[6]),
			fuel: ActiveValue::Set(value.api_fuel),
			ammo: ActiveValue::Set(value.api_bull),
			slot_num: ActiveValue::Set(value.api_slotnum),
			ndock_time: ActiveValue::Set(value.api_ndock_time),
			ndock_fuel: ActiveValue::Set(value.api_ndock_item[0]),
			ndock_steel: ActiveValue::Set(value.api_ndock_item[1]),
			srate: ActiveValue::Set(value.api_srate),
			condition: ActiveValue::Set(value.api_cond),
			firepower_now: ActiveValue::Set(value.api_karyoku[0]),
			firepower_max: ActiveValue::Set(value.api_karyoku[1]),
			torpedo_now: ActiveValue::Set(value.api_raisou[0]),
			torpedo_max: ActiveValue::Set(value.api_raisou[1]),
			aa_now: ActiveValue::Set(value.api_taiku[0]),
			aa_max: ActiveValue::Set(value.api_taiku[1]),
			armor_now: ActiveValue::Set(value.api_soukou[0]),
			armor_max: ActiveValue::Set(value.api_soukou[1]),
			evasion_now: ActiveValue::Set(value.api_kaihi[0]),
			evasion_max: ActiveValue::Set(value.api_kaihi[1]),
			asw_now: ActiveValue::Set(value.api_taisen[0]),
			asw_max: ActiveValue::Set(value.api_taisen[1]),
			los_now: ActiveValue::Set(value.api_sakuteki[0]),
			los_max: ActiveValue::Set(value.api_sakuteki[1]),
			luck_now: ActiveValue::Set(value.api_lucky[0]),
			luck_max: ActiveValue::Set(value.api_lucky[1]),
			has_locked_euqip: ActiveValue::Set(value.api_locked_equip == 1),
			sally_area: ActiveValue::Set(value.api_sally_area),
		}
	}
}

impl From<Model> for KcApiShip {
	fn from(value: Model) -> Self {
		Self {
			api_id: value.id,
			api_sortno: value.sort_num,
			api_ship_id: value.mst_id,
			api_lv: value.level,
			api_exp: [value.exp_now, value.exp_next, value.exp_progress],
			api_nowhp: value.hp_now,
			api_maxhp: value.hp_max,
			api_soku: value.speed,
			api_leng: value.range,
			api_slot: [value.slot_1, value.slot_2, value.slot_3, value.slot_4, value.slot_5],
			api_onslot: [
				value.onslot_1,
				value.onslot_2,
				value.onslot_3,
				value.onslot_4,
				value.onslot_5,
			],
			api_slot_ex: value.slot_ex,
			api_kyouka: [
				value.mod_firepower,
				value.mod_torpedo,
				value.mod_aa,
				value.mod_armor,
				value.mod_luck,
				value.mod_hp,
				value.mod_asw,
			],
			api_backs: value.backs,
			api_fuel: value.fuel,
			api_bull: value.ammo,
			api_slotnum: value.slot_num,
			api_ndock_time: value.ndock_time,
			api_ndock_item: [value.ndock_fuel, value.ndock_steel],
			api_srate: value.srate,
			api_cond: value.condition,
			api_karyoku: [value.firepower_now, value.firepower_max],
			api_raisou: [value.torpedo_now, value.torpedo_max],
			api_taiku: [value.aa_now, value.aa_max],
			api_soukou: [value.armor_now, value.armor_max],
			api_kaihi: [value.evasion_now, value.evasion_max],
			api_taisen: [value.asw_now, value.asw_max],
			api_sakuteki: [value.los_now, value.los_max],
			api_lucky: [value.luck_now, value.luck_max],
			api_locked: if value.locked {
				1
			} else {
				0
			},
			api_locked_equip: if value.has_locked_euqip {
				1
			} else {
				0
			},
			api_sally_area: value.sally_area,
		}
	}
}
