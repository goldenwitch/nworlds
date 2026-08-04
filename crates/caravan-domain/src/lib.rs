#![forbid(unsafe_code)]

mod entries;
mod geometry;
mod values;

pub use entries::GameJournalEntry;
pub use geometry::{
    Axial, Saucer, TileId, NEIGHBOR_OFFSETS, RADIUS_5_TILES, SAUCER_RADIUS, SAUCER_TILE_COUNT,
};
pub use values::{Actor, ActorId, ActorKind, Effect, Resources, Terrain, TileLayers};
