#![forbid(unsafe_code)]

mod arborist;
mod arsonist;
mod fighter;
mod fire;
mod input;

pub use arborist::{ArboristDefinition, ArboristResult};
pub use arsonist::{ArsonistDefinition, ArsonistResult};
pub use fighter::{FighterDefinition, FighterResult};
pub use fire::{FireDefinition, FireOutcome, FireResult, FireStart};
pub use input::HazardCell;
