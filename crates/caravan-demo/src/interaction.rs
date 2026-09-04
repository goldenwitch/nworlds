use caravan_domain::{Terrain, TileId};

use crate::engine_integration::{CaravanState, Tau};
use crate::input::{Button, InputPacket, InteractionDefinition, SemanticInputBatch};
use crate::transformation::Transformation;

/// Developer-authored meaning for the first Caravan interaction.
#[derive(Clone, Copy, Debug, Default)]
pub struct CaravanInteraction;

impl InteractionDefinition for CaravanInteraction {
    type Transformation = Transformation;

    fn query(
        &self,
        _state: &CaravanState,
        input: &SemanticInputBatch,
        _tau: Tau,
    ) -> Self::Transformation {
        if input.contains(&InputPacket::ButtonPressed(Button::Primary)) {
            Transformation::SetTerrain {
                tile: TileId::origin(),
                terrain: Terrain::Wheat,
            }
        } else {
            Transformation::Noop
        }
    }
}
