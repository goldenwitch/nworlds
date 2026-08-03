use caravan_domain::{ActorId, Effect, Terrain, TileId, TileLayers};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct HazardCell {
    tile: TileId,
    layers: TileLayers,
}

impl HazardCell {
    pub const fn new(tile: TileId, layers: TileLayers) -> Self {
        Self { tile, layers }
    }

    pub const fn tile(self) -> TileId {
        self.tile
    }

    pub const fn layers(self) -> TileLayers {
        self.layers
    }

    pub const fn terrain(self) -> Terrain {
        self.layers.terrain()
    }

    pub const fn actor(self) -> Option<ActorId> {
        self.layers.actor()
    }

    pub const fn effect(self) -> Effect {
        self.layers.effect()
    }
}
