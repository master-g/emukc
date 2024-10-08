//! Gameplay logic.

pub use furniture::FurnitureOps;
pub use incentive::IncentiveOps;
pub use material::MaterialOps;
pub use picturebook::PictureBookOps;
pub use ship::ShipOps;

use crate::prelude::HasContext;

// modules

mod furniture;
mod incentive;
mod material;
mod picturebook;
mod ship;
mod slot_item;
mod use_item;

/// A trait for gameplay logic.
#[async_trait::async_trait]
pub trait GameOps: FurnitureOps + IncentiveOps + MaterialOps + ShipOps + PictureBookOps {}

#[async_trait::async_trait]
impl<T: HasContext + ?Sized> GameOps for T {}
