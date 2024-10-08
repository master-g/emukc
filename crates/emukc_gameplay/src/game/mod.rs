//! Gameplay logic.

use furniture::FurnitureOps;
use incentive::IncentiveOps;
use material::MaterialOps;
use ship::ShipOps;

use crate::prelude::HasContext;

// modules

mod furniture;
mod incentive;
mod material;
mod ship;
mod slot_item;
mod use_item;

/// A trait for gameplay logic.
#[async_trait::async_trait]
pub trait GameOps: FurnitureOps + IncentiveOps + MaterialOps + ShipOps {}

#[async_trait::async_trait]
impl<T: HasContext + ?Sized> GameOps for T {}
