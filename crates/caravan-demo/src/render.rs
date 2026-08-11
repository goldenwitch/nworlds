use caravan_domain::{ActorId, ActorKind, Effect, Resources, Terrain, TileId};
use caravan_reference::Snapshot;
use engine_presentation::Renderer;
use engine_sdk::GameState;
use engine_time::{LogicalTime, Tau};

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
    type Output = RenderOutput;

    fn render(state: &GameState<Snapshot>, _tau: Tau) -> Self::Output {
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
}

#[cfg(test)]
mod tests {
    use super::{CaravanRenderer, RenderOutput};
    use caravan_domain::{ActorId, ActorKind, Effect, GameJournalEntry, Terrain, TileId};
    use caravan_reference::{actual, state, Snapshot};
    use caravan_seeded::hand_authored_behavior_fixture;
    use engine_journal::{Journal, JournalWriter};
    use engine_presentation::{present, Renderer};
    use engine_sdk::Frame;
    use engine_time::{LogicalTime, Tau};

    fn frame(
        worldline: &caravan_reference::ReferenceWorldline,
        time: LogicalTime,
        tau: Tau,
    ) -> Frame<RenderOutput> {
        let state = state(worldline, time);
        present::<Snapshot, CaravanRenderer>(&state, tau)
    }

    fn saucer_worldline() -> caravan_reference::ReferenceWorldline {
        let mut writer = JournalWriter::new();
        writer.record(GameJournalEntry::create_saucer());
        actual(writer.finish())
    }

    #[test]
    fn empty_state_projects_to_owned_empty_output() {
        let worldline = actual(Journal::empty());
        let tau = Tau::from_ticks(7);
        let frame = frame(&worldline, LogicalTime::from_ticks(-1), tau);

        assert_eq!(frame.tau(), tau);
        assert_eq!(frame.payload().logical_time(), LogicalTime::from_ticks(-1));
        assert!(frame.payload().tiles().is_empty());
        assert!(frame.payload().actors().is_empty());
        assert_eq!(frame.payload().resources().wheat(), 0);
        assert_eq!(frame.payload().resources().wood(), 0);
    }

    #[test]
    fn saucer_projection_preserves_stable_tile_order() {
        let worldline = saucer_worldline();
        let state = state(&worldline, LogicalTime::zero());
        let output = CaravanRenderer::render(&state, Tau::zero());

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
        let rendered = frame(
            &worldline,
            LogicalTime::from_game_ticks(1).expect("test time is representable"),
            Tau::from_ticks(11),
        );
        let output = rendered.payload();
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
        assert_eq!(rendered.tau(), Tau::from_ticks(11));
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
    }
}
