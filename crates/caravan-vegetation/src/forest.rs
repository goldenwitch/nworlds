use caravan_domain::{Terrain, TileId};

use crate::{VegetationQueryInput, VegetationSnapshot};

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Forest;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ForestResult {
    forest_tiles: Vec<TileId>,
}

impl Forest {
    pub fn query<I>(&self, input: &I) -> ForestResult
    where
        I: VegetationQueryInput,
    {
        let mut forest_tiles = input
            .current_snapshot()
            .tiles()
            .iter()
            .filter(|indexed_tile| indexed_tile.layers().terrain() == Terrain::Forest)
            .map(|indexed_tile| indexed_tile.tile())
            .collect::<Vec<_>>();
        forest_tiles.sort_unstable();

        ForestResult { forest_tiles }
    }
}

impl ForestResult {
    pub fn tiles(&self) -> &[TileId] {
        &self.forest_tiles
    }

    pub const fn tile_count(&self) -> u64 {
        self.forest_tiles.len() as u64
    }
}
