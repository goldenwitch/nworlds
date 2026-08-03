use caravan_domain::{Actor, TileId, TileLayers};
use engine_index::game_tick_index;
use engine_time::LogicalTime;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct IndexedTile {
    tile: TileId,
    layers: TileLayers,
}

impl IndexedTile {
    pub const fn new(tile: TileId, layers: TileLayers) -> Self {
        Self { tile, layers }
    }

    pub const fn tile(self) -> TileId {
        self.tile
    }

    pub const fn layers(self) -> TileLayers {
        self.layers
    }
}

pub trait VegetationSnapshot {
    fn tick_index(&self) -> i64;

    fn tiles(&self) -> &[IndexedTile];

    fn actors(&self) -> &[Actor];

    fn layers_at(&self, tile: TileId) -> Option<TileLayers> {
        self.tiles()
            .iter()
            .find(|indexed_tile| indexed_tile.tile() == tile)
            .map(|indexed_tile| indexed_tile.layers())
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Snapshot {
    tick_index: i64,
    tiles: Vec<IndexedTile>,
    actors: Vec<Actor>,
}

impl Snapshot {
    pub fn new(
        tick_index: i64,
        tiles: impl IntoIterator<Item = IndexedTile>,
        actors: impl IntoIterator<Item = Actor>,
    ) -> Self {
        Self {
            tick_index,
            tiles: tiles.into_iter().collect(),
            actors: actors.into_iter().collect(),
        }
    }
}

impl VegetationSnapshot for Snapshot {
    fn tick_index(&self) -> i64 {
        self.tick_index
    }

    fn tiles(&self) -> &[IndexedTile] {
        &self.tiles
    }

    fn actors(&self) -> &[Actor] {
        &self.actors
    }
}

pub trait VegetationQueryInput {
    type Snapshot: VegetationSnapshot;

    fn game_tick_index(&self) -> i64;

    fn current_snapshot(&self) -> &Self::Snapshot;

    fn indexed_snapshots(&self) -> &[Self::Snapshot];

    fn snapshot_at(&self, tick_index: i64) -> Option<&Self::Snapshot> {
        self.indexed_snapshots()
            .iter()
            .find(|snapshot| snapshot.tick_index() == tick_index)
    }
}

#[derive(Clone, Copy)]
pub struct IndexedInput<'a, S>
where
    S: VegetationSnapshot,
{
    game_tick_index: i64,
    current_snapshot: &'a S,
    indexed_snapshots: &'a [S],
}

impl<'a, S> IndexedInput<'a, S>
where
    S: VegetationSnapshot,
{
    pub fn new(current_snapshot: &'a S, indexed_snapshots: &'a [S]) -> Self {
        Self::at_tick(
            current_snapshot,
            indexed_snapshots,
            current_snapshot.tick_index(),
        )
    }

    pub fn at_tick(
        current_snapshot: &'a S,
        indexed_snapshots: &'a [S],
        game_tick_index: i64,
    ) -> Self {
        Self {
            game_tick_index,
            current_snapshot,
            indexed_snapshots,
        }
    }

    pub fn at_time(
        current_snapshot: &'a S,
        indexed_snapshots: &'a [S],
        logical_time: LogicalTime,
    ) -> Self {
        Self::at_tick(
            current_snapshot,
            indexed_snapshots,
            game_tick_index(logical_time),
        )
    }
}

impl<S> VegetationQueryInput for IndexedInput<'_, S>
where
    S: VegetationSnapshot,
{
    type Snapshot = S;

    fn game_tick_index(&self) -> i64 {
        self.game_tick_index
    }

    fn current_snapshot(&self) -> &Self::Snapshot {
        self.current_snapshot
    }

    fn indexed_snapshots(&self) -> &[Self::Snapshot] {
        self.indexed_snapshots
    }
}
