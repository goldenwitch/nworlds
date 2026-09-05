use caravan_domain::{GameJournalEntry, Terrain, TileId};
use caravan_reference::{ProjectionError, ReferenceWorldline, State};
use caravan_sample::engine_integration::{
    actual_worldline as actual, CaravanJournalWriter as JournalWriter, LogicalTime, Tau,
};
use caravan_sample::input::{
    Button, InputObservation, InputPacket, InputPacketSet, InteractionDefinition, ObservationId,
    OrderedInputBatch, SemanticInputBatch,
};
use caravan_sample::transformation::Transformation;
use caravan_sample::{CaravanOrchestrator, OrchestratorError};

struct StateAwareInteraction;

impl InteractionDefinition for StateAwareInteraction {
    type Transformation = Transformation;

    fn query(&self, state: &State, input: &SemanticInputBatch, tau: Tau) -> Self::Transformation {
        assert!(input.contains(&InputPacket::ButtonPressed(Button::Primary)));
        assert_eq!(tau, Tau::from_ticks(7));

        match state.payload().terrain_at(TileId::origin()) {
            Some(Terrain::Forest) => Transformation::SetTerrain {
                tile: TileId::origin(),
                terrain: Terrain::Wheat,
            },
            _ => Transformation::Noop,
        }
    }
}

fn time(game_ticks: i64) -> LogicalTime {
    LogicalTime::from_game_ticks(game_ticks).expect("test time is representable")
}

fn worldline() -> ReferenceWorldline {
    let mut writer = JournalWriter::new();
    writer.record(GameJournalEntry::create_saucer());
    writer
        .advance_to(time(3))
        .expect("test terrain timestamp is forward");
    writer.record(GameJournalEntry::SetTerrain {
        tile: TileId::origin(),
        terrain: Terrain::Forest,
    });
    actual(writer.finish())
}

fn orchestrator(
    worldline: ReferenceWorldline,
    logical_time: LogicalTime,
) -> CaravanOrchestrator<StateAwareInteraction> {
    CaravanOrchestrator::new(
        worldline,
        logical_time,
        Tau::from_ticks(7),
        StateAwareInteraction,
    )
    .expect("state-aware test worldline should initialize")
}

#[test]
fn identical_input_and_tau_use_the_selected_logical_state() {
    let parent = worldline();
    let mut void_orchestrator = orchestrator(parent.clone(), time(0));
    let mut forest_orchestrator = orchestrator(parent, time(3));
    let packet = InputPacket::ButtonPressed(Button::Primary);

    void_orchestrator.receive_packet(packet);
    forest_orchestrator.receive_packet(packet);
    let void_parent = void_orchestrator.worldline().clone();
    let forest_parent = forest_orchestrator.worldline().clone();

    assert_eq!(
        void_orchestrator
            .sample()
            .expect("void state should project")
            .payload()
            .terrain_at(TileId::origin()),
        Some(Terrain::Void)
    );
    assert_eq!(
        forest_orchestrator
            .sample()
            .expect("forest state should project")
            .payload()
            .terrain_at(TileId::origin()),
        Some(Terrain::Forest)
    );

    let void_transformation = void_orchestrator
        .interaction()
        .expect("void interaction should project");
    let forest_transformation = forest_orchestrator
        .interaction()
        .expect("forest interaction should project");

    assert_eq!(void_transformation, Transformation::Noop);
    assert_eq!(
        forest_transformation,
        Transformation::SetTerrain {
            tile: TileId::origin(),
            terrain: Terrain::Wheat,
        }
    );
    assert_ne!(void_transformation, forest_transformation);
    assert_eq!(
        void_orchestrator
            .interaction()
            .expect("repeated void interaction should project"),
        void_transformation
    );
    assert_eq!(
        forest_orchestrator
            .interaction()
            .expect("repeated forest interaction should project"),
        forest_transformation
    );
    assert_eq!(void_orchestrator.worldline(), &void_parent);
    assert_eq!(forest_orchestrator.worldline(), &forest_parent);
    assert_eq!(void_orchestrator.packets().len(), 1);
    assert_eq!(forest_orchestrator.packets().len(), 1);
}

#[test]
fn malformed_selected_state_fails_before_interaction() {
    let mut writer = JournalWriter::new();
    writer.record(GameJournalEntry::CreateSaucer { radius: 4 });
    let mut orchestrator = orchestrator(actual(writer.finish()), time(0));
    orchestrator.receive_packet(InputPacket::ButtonPressed(Button::Primary));
    let parent = orchestrator.worldline().clone();

    assert_eq!(
        orchestrator.interaction(),
        Err(OrchestratorError::Projection(
            ProjectionError::UnsupportedSaucerRadius {
                append_ordinal: 0,
                expected: 5,
                found: 4,
            }
        ))
    );
    assert_eq!(orchestrator.worldline(), &parent);
    assert_eq!(orchestrator.packets().len(), 1);
}

#[test]
fn ordered_transport_batch_derives_the_current_membership_interaction_view() {
    let batch = OrderedInputBatch::from_observations([
        InputObservation::new(
            ObservationId::new(2, 1),
            InputPacket::ButtonReleased(Button::Secondary),
        ),
        InputObservation::new(
            ObservationId::new(1, 2),
            InputPacket::ButtonPressed(Button::Primary),
        ),
        InputObservation::new(
            ObservationId::new(1, 1),
            InputPacket::ButtonPressed(Button::Primary),
        ),
    ])
    .expect("source observations have distinct identities");

    assert_eq!(batch.len(), 3);
    assert_eq!(
        batch
            .observations()
            .iter()
            .map(|observation| observation.id())
            .collect::<Vec<_>>(),
        vec![
            ObservationId::new(1, 1),
            ObservationId::new(1, 2),
            ObservationId::new(2, 1),
        ]
    );

    let membership = InputPacketSet::from_batch(&batch);
    assert_eq!(membership.len(), 2);
    assert!(membership.contains(&InputPacket::ButtonPressed(Button::Primary)));
    assert!(membership.contains(&InputPacket::ButtonReleased(Button::Secondary)));
}
