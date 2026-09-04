use caravan_domain::{ActorId, ActorKind, Effect, Resources, Terrain, TileId};
use caravan_reference::Snapshot;

use crate::engine_integration::{GameState, LogicalTime, RenderBatch, RenderVertex, Renderer, Tau};

/// The first owned backend-neutral rendering projection for Caravan.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct CaravanRenderer;

/// Owned rendering data for one sampled Caravan state.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RenderOutput {
    logical_time: LogicalTime,
    tiles: Vec<RenderTile>,
    actors: Vec<RenderActor>,
    resources: Resources,
}

impl RenderOutput {
    /// Returns the exact logical time carried by the projected state.
    pub const fn logical_time(&self) -> LogicalTime {
        self.logical_time
    }

    /// Returns tiles in the source snapshot's stable order.
    pub fn tiles(&self) -> &[RenderTile] {
        &self.tiles
    }

    /// Returns one rendered tile by identity.
    pub fn tile(&self, tile: TileId) -> Option<&RenderTile> {
        self.tiles
            .iter()
            .find(|render_tile| render_tile.tile == tile)
    }

    /// Returns actors in the source snapshot's stable order.
    pub fn actors(&self) -> &[RenderActor] {
        &self.actors
    }

    /// Returns the projected global resources.
    pub const fn resources(&self) -> Resources {
        self.resources
    }
}

/// Owned rendering data for one Caravan tile.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RenderTile {
    tile: TileId,
    terrain: Terrain,
    actor: Option<ActorId>,
    effect: Effect,
}

impl RenderTile {
    /// Returns the tile identity.
    pub const fn tile(self) -> TileId {
        self.tile
    }

    /// Returns the independent terrain layer.
    pub const fn terrain(self) -> Terrain {
        self.terrain
    }

    /// Returns the actor occupying the tile, if any.
    pub const fn actor(self) -> Option<ActorId> {
        self.actor
    }

    /// Returns the independent effect layer.
    pub const fn effect(self) -> Effect {
        self.effect
    }
}

/// Owned rendering data for one Caravan actor.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RenderActor {
    id: ActorId,
    kind: ActorKind,
    tile: TileId,
}

impl RenderActor {
    /// Returns the actor identity.
    pub const fn id(self) -> ActorId {
        self.id
    }

    /// Returns the actor kind.
    pub const fn kind(self) -> ActorKind {
        self.kind
    }

    /// Returns the actor's tile.
    pub const fn tile(self) -> TileId {
        self.tile
    }
}

impl Renderer<Snapshot> for CaravanRenderer {
    type Output = RenderBatch;

    fn render(state: &GameState<Snapshot>, _tau: Tau) -> Self::Output {
        render_batch(&project_output(state))
    }
}

/// Projects one Caravan state into the sample's inspectable semantic render model.
pub fn project_output(state: &GameState<Snapshot>) -> RenderOutput {
    let snapshot = state.payload();
    let tiles = snapshot
        .tiles()
        .iter()
        .map(|tile| {
            let layers = tile.layers();
            RenderTile {
                tile: tile.tile(),
                terrain: layers.terrain(),
                actor: layers.actor(),
                effect: layers.effect(),
            }
        })
        .collect();
    let actors = snapshot
        .actors()
        .iter()
        .map(|actor| RenderActor {
            id: actor.id(),
            kind: actor.kind(),
            tile: actor.tile(),
        })
        .collect();

    RenderOutput {
        logical_time: state.logical_time(),
        tiles,
        actors,
        resources: snapshot.resources(),
    }
}

fn render_batch(output: &RenderOutput) -> RenderBatch {
    let mut vertices = Vec::with_capacity(output.tiles().len() * 18);
    for tile in output.tiles() {
        let center = tile_center(tile);
        let color = tile_color(tile);
        for corner in 0..6 {
            let first = hex_corner(center, corner);
            let second = hex_corner(center, corner + 1);
            vertices.extend([
                RenderVertex::new(
                    [center[0], center[1], 0.0],
                    [color[0], color[1], color[2], 1.0],
                ),
                RenderVertex::new(
                    [first[0], first[1], 0.0],
                    [color[0], color[1], color[2], 1.0],
                ),
                RenderVertex::new(
                    [second[0], second[1], 0.0],
                    [color[0], color[1], color[2], 1.0],
                ),
            ]);
        }
    }
    RenderBatch::new(vertices)
}

fn tile_center(tile: &RenderTile) -> [f32; 2] {
    const HEX_RADIUS: f32 = 0.095;
    let q = tile.tile().q() as f32;
    let r = tile.tile().r() as f32;
    [HEX_RADIUS * 1.732 * (q + r * 0.5), HEX_RADIUS * 1.5 * r]
}

fn hex_corner(center: [f32; 2], index: usize) -> [f32; 2] {
    const HEX_RADIUS: f32 = 0.095;
    let angle = (30.0 + 60.0 * (index % 6) as f32).to_radians();
    [
        center[0] + HEX_RADIUS * angle.cos(),
        center[1] + HEX_RADIUS * angle.sin(),
    ]
}

fn tile_color(tile: &RenderTile) -> [f32; 3] {
    if tile.effect().fire_age().is_some() {
        return [0.9, 0.16, 0.04];
    }
    if tile.actor().is_some() {
        return [0.12, 0.72, 0.88];
    }
    match tile.terrain() {
        Terrain::Void => [0.18, 0.23, 0.3],
        Terrain::Wheat => [0.94, 0.68, 0.08],
        Terrain::Forest => [0.12, 0.56, 0.25],
    }
}

#[cfg(test)]
mod tests {
    use super::{project_output, CaravanRenderer, RenderOutput};
    use crate::engine_integration::{
        present, CaravanFrame, CaravanJournal, CaravanJournalWriter, LogicalTime, RenderBatch, Tau,
    };
    use caravan_domain::{ActorId, ActorKind, Effect, GameJournalEntry, Terrain, TileId};
    use caravan_reference::{actual, state, Snapshot};
    use caravan_seeded::hand_authored_behavior_fixture;

    fn frame(
        worldline: &caravan_reference::ReferenceWorldline,
        time: LogicalTime,
        tau: Tau,
    ) -> CaravanFrame<RenderBatch> {
        let state = state(worldline, time);
        present::<Snapshot, CaravanRenderer>(&state, tau)
    }

    fn saucer_worldline() -> caravan_reference::ReferenceWorldline {
        let mut writer = CaravanJournalWriter::new();
        writer.record(GameJournalEntry::create_saucer());
        actual(writer.finish())
    }

    #[test]
    fn empty_state_projects_to_owned_empty_output() {
        let worldline = actual(CaravanJournal::empty());
        let tau = Tau::from_ticks(7);
        let state = state(&worldline, LogicalTime::from_ticks(-1));
        let output = project_output(&state);
        let rendered = frame(&worldline, LogicalTime::from_ticks(-1), tau);

        assert_eq!(rendered.tau(), tau);
        assert_eq!(output.logical_time(), LogicalTime::from_ticks(-1));
        assert!(output.tiles().is_empty());
        assert!(output.actors().is_empty());
        assert_eq!(output.resources().wheat(), 0);
        assert_eq!(output.resources().wood(), 0);
    }

    #[test]
    fn saucer_projection_preserves_stable_tile_order() {
        let worldline = saucer_worldline();
        let state = state(&worldline, LogicalTime::zero());
        let output = project_output(&state);

        assert_eq!(output.tiles().len(), 91);
        assert_eq!(
            output
                .tiles()
                .iter()
                .map(|tile| tile.tile())
                .collect::<Vec<_>>(),
            state
                .payload()
                .tiles()
                .iter()
                .map(|tile| tile.tile())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn projection_preserves_layers_actors_and_resources() {
        let worldline = actual(hand_authored_behavior_fixture());
        let state = state(
            &worldline,
            LogicalTime::from_game_ticks(1).expect("test time is representable"),
        );
        let output = project_output(&state);
        let wheat_tile = output.tile(TileId::new(1, 0).expect("tile is inside the saucer"));
        let fire_tile = output.tile(TileId::new(-1, 0).expect("tile is inside the saucer"));

        assert_eq!(wheat_tile.map(|tile| tile.terrain()), Some(Terrain::Wheat));
        assert_eq!(fire_tile.map(|tile| tile.effect()), Some(Effect::fire(0)));
        assert!(output
            .actors()
            .iter()
            .any(|actor| actor.id() == ActorId::new(2).unwrap()
                && actor.kind() == ActorKind::Forester));
        assert_eq!(output.resources().wheat(), 9);
    }

    #[test]
    fn repeated_equal_state_and_tau_inputs_project_equal_output() {
        let worldline = saucer_worldline();
        let first = frame(&worldline, LogicalTime::zero(), Tau::from_ticks(3));
        let second = frame(&worldline, LogicalTime::zero(), Tau::from_ticks(3));

        assert_eq!(first, second);
    }

    fn assert_fire_and_forget_data<T: Send + Sync + 'static>() {}

    #[test]
    fn render_packets_are_owned_static_send_and_sync_data() {
        assert_fire_and_forget_data::<RenderOutput>();
        assert_fire_and_forget_data::<super::RenderTile>();
        assert_fire_and_forget_data::<super::RenderActor>();
        assert_fire_and_forget_data::<RenderBatch>();
    }
}
