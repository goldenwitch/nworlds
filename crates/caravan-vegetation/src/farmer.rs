use caravan_domain::{ActorKind, TileId};

use crate::helpers::{actor_of_kind, is_open_void};
use crate::VegetationQueryInput;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Farmer;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum FarmerResult {
    NoFarmer,
    Completed(FarmerAction),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FarmerAction {
    origin: TileId,
    destination: TileId,
    moved: bool,
    wheat_tiles: Vec<TileId>,
}

impl Farmer {
    pub fn query<I>(&self, input: &I) -> FarmerResult
    where
        I: VegetationQueryInput,
    {
        let Some(farmer) = actor_of_kind(input, ActorKind::Farmer) else {
            return FarmerResult::NoFarmer;
        };

        let origin = farmer.tile();
        let destination = origin
            .neighbors()
            .into_iter()
            .flatten()
            .find(|tile| is_open_void(input, *tile, Some(farmer.id())))
            .unwrap_or(origin);
        let wheat_tiles = destination
            .neighbors()
            .into_iter()
            .flatten()
            .filter(|tile| is_open_void(input, *tile, Some(farmer.id())))
            .collect();

        FarmerResult::Completed(FarmerAction {
            origin,
            destination,
            moved: destination != origin,
            wheat_tiles,
        })
    }
}

impl FarmerResult {
    pub fn action(&self) -> Option<&FarmerAction> {
        match self {
            Self::NoFarmer => None,
            Self::Completed(action) => Some(action),
        }
    }
}

impl FarmerAction {
    pub const fn origin(&self) -> TileId {
        self.origin
    }

    pub const fn destination(&self) -> TileId {
        self.destination
    }

    pub const fn moved(&self) -> bool {
        self.moved
    }

    pub fn wheat_tiles(&self) -> &[TileId] {
        &self.wheat_tiles
    }
}
