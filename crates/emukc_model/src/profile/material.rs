use serde::{Deserialize, Serialize};

use crate::{KcApiMaterialElement, MaterialCategory};

/// In game materials
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Material {
	/// Profile ID
	pub id: i64,

	/// Fuel
	pub fuel: i64,

	/// Ammo
	pub ammo: i64,

	/// Steel
	pub steel: i64,

	/// Bauxite
	pub bauxite: i64,

	/// Torch, fast construction item
	pub torch: i64,

	/// Bucket, fast repair item
	pub bucket: i64,

	/// Development material
	pub devmat: i64,

	/// Screw, improvement material
	pub screw: i64,
}

impl From<Material> for Vec<KcApiMaterialElement> {
	fn from(value: Material) -> Self {
		vec![
			KcApiMaterialElement {
				api_member_id: value.id,
				api_id: MaterialCategory::Fuel as i64,
				api_value: value.fuel,
			},
			KcApiMaterialElement {
				api_member_id: value.id,
				api_id: MaterialCategory::Ammo as i64,
				api_value: value.ammo,
			},
			KcApiMaterialElement {
				api_member_id: value.id,
				api_id: MaterialCategory::Steel as i64,
				api_value: value.steel,
			},
			KcApiMaterialElement {
				api_member_id: value.id,
				api_id: MaterialCategory::Bauxite as i64,
				api_value: value.bauxite,
			},
			KcApiMaterialElement {
				api_member_id: value.id,
				api_id: MaterialCategory::Torch as i64,
				api_value: value.torch,
			},
			KcApiMaterialElement {
				api_member_id: value.id,
				api_id: MaterialCategory::Bucket as i64,
				api_value: value.bucket,
			},
			KcApiMaterialElement {
				api_member_id: value.id,
				api_id: MaterialCategory::DevMat as i64,
				api_value: value.devmat,
			},
			KcApiMaterialElement {
				api_member_id: value.id,
				api_id: MaterialCategory::Screw as i64,
				api_value: value.screw,
			},
		]
	}
}

/// Primary resource hard cap
const PRIMARY_RESOURCE_HARD_CAP: i64 = 35000;

/// Special resource cap
const SPECIAL_RESOURCE_CAP: i64 = 3000;

/// Material config
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct MaterialConfig {
	/// Initial fuel
	pub init_fuel: i64,

	/// Initial ammo
	pub init_ammo: i64,

	/// Initial steel
	pub init_steel: i64,

	/// Initial bauxite
	pub init_bauxite: i64,

	/// Initial torch
	pub init_torch: i64,

	/// Initial bucket
	pub init_bucket: i64,

	/// Initial development material
	pub init_devmat: i64,

	/// Initial screw
	pub init_screw: i64,

	/// Primary resource hard cap
	pub primary_resource_hard_cap: i64,

	/// Special resource cap
	pub special_resource_cap: i64,

	/// Primary resource regenerate rate, per milliseconds
	pub primary_resource_regenerate_rate: i64,

	/// Bauxite regenerate rate, per milliseconds
	pub bauxite_regenerate_rate: i64,
}

impl Default for MaterialConfig {
	fn default() -> Self {
		Self {
			init_fuel: 1000,
			init_ammo: 1000,
			init_steel: 1000,
			init_bauxite: 1000,
			init_torch: 3,
			init_bucket: 3,
			init_devmat: 5,
			init_screw: 0,
			primary_resource_hard_cap: PRIMARY_RESOURCE_HARD_CAP,
			special_resource_cap: SPECIAL_RESOURCE_CAP,
			primary_resource_regenerate_rate: 60_000,
			bauxite_regenerate_rate: 180_000,
		}
	}
}

impl MaterialConfig {
	/// Create a new material record
	///
	/// # Arguments
	///
	/// * `id` - The profile ID
	pub fn new_material(&self, id: i64) -> Material {
		Material {
			id,
			fuel: self.init_fuel,
			ammo: self.init_ammo,
			steel: self.init_steel,
			bauxite: self.init_bauxite,
			torch: self.init_torch,
			bucket: self.init_bucket,
			devmat: self.init_devmat,
			screw: self.init_screw,
		}
	}

	/// Apply hard cap to the material
	///
	/// # Arguments
	///
	/// * `material` - The material to apply hard cap
	pub fn apply_hard_cap(&self, material: &mut Material) {
		if material.fuel > self.primary_resource_hard_cap {
			material.fuel = self.primary_resource_hard_cap;
		}
		if material.ammo > self.primary_resource_hard_cap {
			material.ammo = self.primary_resource_hard_cap;
		}
		if material.steel > self.primary_resource_hard_cap {
			material.steel = self.primary_resource_hard_cap;
		}
		if material.bauxite > self.primary_resource_hard_cap {
			material.bauxite = self.primary_resource_hard_cap;
		}
		if material.torch > self.special_resource_cap {
			material.torch = self.special_resource_cap;
		}
		if material.bucket > self.special_resource_cap {
			material.bucket = self.special_resource_cap;
		}
		if material.devmat > self.special_resource_cap {
			material.devmat = self.special_resource_cap;
		}
		if material.screw > self.special_resource_cap {
			material.screw = self.special_resource_cap;
		}
	}
}