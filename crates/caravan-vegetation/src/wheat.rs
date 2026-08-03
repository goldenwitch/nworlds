use caravan_domain::{Resources, Terrain, TileId};

use crate::{VegetationQueryInput, VegetationSnapshot};

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Wheat;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct WheatResult {
    current_tiles: Vec<TileId>,
    indexed_total: u64,
}

impl Wheat {
    pub fn query<I>(&self, input: &I) -> WheatResult
    where
        I: VegetationQueryInput,
    {
        let mut current_tiles = wheat_tiles(input.current_snapshot());
        current_tiles.sort_unstable();

        let mut indexed_total: u64 = 0;
        for snapshot in input.indexed_snapshots() {
            let tick_index = snapshot.tick_index();
            if tick_index >= 0 && tick_index <= input.game_tick_index() {
                indexed_total = indexed_total
                    .checked_add(wheat_tiles(snapshot).len() as u64)
                    .expect("indexed wheat total overflowed u64");
            }
        }

        WheatResult {
            current_tiles,
            indexed_total,
        }
    }
}

impl WheatResult {
    pub fn current_tiles(&self) -> &[TileId] {
        &self.current_tiles
    }

    pub const fn current_tile_count(&self) -> u64 {
        self.current_tiles.len() as u64
    }

    pub const fn indexed_total(&self) -> u64 {
        self.indexed_total
    }

    pub const fn resources(&self) -> Resources {
        Resources::new(self.indexed_total, 0)
    }
}

fn wheat_tiles<S>(snapshot: &S) -> Vec<TileId>
where
    S: VegetationSnapshot,
{
    snapshot
        .tiles()
        .iter()
        .filter(|indexed_tile| indexed_tile.layers().terrain() == Terrain::Wheat)
        .map(|indexed_tile| indexed_tile.tile())
        .collect()
}
