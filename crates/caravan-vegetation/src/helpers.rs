use caravan_domain::{Actor, ActorId, ActorKind, Effect, TileId, TileLayers};

use crate::{VegetationQueryInput, VegetationSnapshot};

pub(crate) fn actor_of_kind<I>(input: &I, kind: ActorKind) -> Option<Actor>
where
    I: VegetationQueryInput,
{
    input
        .current_snapshot()
        .actors()
        .iter()
        .filter(|actor| actor.kind() == kind)
        .min_by_key(|actor| actor.id())
        .copied()
}

pub(crate) fn layers_at<I>(input: &I, tile: TileId) -> TileLayers
where
    I: VegetationQueryInput,
{
    input.current_snapshot().layers_at(tile).unwrap_or_default()
}

pub(crate) fn occupied_by<I>(input: &I, tile: TileId, ignored: Option<ActorId>) -> bool
where
    I: VegetationQueryInput,
{
    let layers = layers_at(input, tile);
    if let Some(actor_id) = layers.actor() {
        if Some(actor_id) != ignored {
            return true;
        }
    }

    input
        .current_snapshot()
        .actors()
        .iter()
        .any(|actor| actor.tile() == tile && Some(actor.id()) != ignored)
}

pub(crate) fn is_open_void<I>(input: &I, tile: TileId, ignored: Option<ActorId>) -> bool
where
    I: VegetationQueryInput,
{
    let layers = layers_at(input, tile);

    layers.terrain() == caravan_domain::Terrain::Void
        && layers.effect() == Effect::None
        && !occupied_by(input, tile, ignored)
}
