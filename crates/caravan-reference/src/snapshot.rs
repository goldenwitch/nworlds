use caravan_domain::{Actor, Resources, Saucer, TileId, TileLayers};
use caravan_vegetation::{IndexedTile, VegetationSnapshot};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Snapshot {
    saucer: Option<Saucer>,
    tick_index: i64,
    tiles: Vec<IndexedTile>,
    actors: Vec<Actor>,
    resources: Resources,
}

impl Snapshot {
    pub(crate) fn from_parts(
        saucer: Option<Saucer>,
        tick_index: i64,
        tiles: impl IntoIterator<Item = IndexedTile>,
        actors: impl IntoIterator<Item = Actor>,
        resources: Resources,
    ) -> Self {
        Self {
            saucer,
            tick_index,
            tiles: tiles.into_iter().collect(),
            actors: actors.into_iter().collect(),
            resources,
        }
    }

    pub const fn saucer(&self) -> Option<Saucer> {
        self.saucer
    }

    pub const fn has_saucer(&self) -> bool {
        self.saucer.is_some()
    }

    pub const fn tick_index(&self) -> i64 {
        self.tick_index
    }

    pub fn tiles(&self) -> &[IndexedTile] {
        &self.tiles
    }

    pub fn actors(&self) -> &[Actor] {
        &self.actors
    }

    pub const fn resources(&self) -> Resources {
        self.resources
    }

    pub fn layers_at(&self, tile: TileId) -> Option<TileLayers> {
        self.tiles
            .iter()
            .find(|indexed_tile| indexed_tile.tile() == tile)
            .map(|indexed_tile| indexed_tile.layers())
    }

    pub fn tile_at(&self, tile: TileId) -> Option<IndexedTile> {
        self.tiles
            .iter()
            .find(|indexed_tile| indexed_tile.tile() == tile)
            .copied()
    }

    pub fn terrain_at(&self, tile: TileId) -> Option<caravan_domain::Terrain> {
        self.layers_at(tile).map(TileLayers::terrain)
    }

    pub fn actor_at(&self, tile: TileId) -> Option<caravan_domain::ActorId> {
        self.layers_at(tile).and_then(TileLayers::actor)
    }

    pub fn effect_at(&self, tile: TileId) -> Option<caravan_domain::Effect> {
        self.layers_at(tile).map(TileLayers::effect)
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
