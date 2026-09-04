use caravan_demo::engine_integration::{
    actual_worldline as actual, CaravanJournalWriter as JournalWriter, LogicalTime, Tau,
};
use caravan_demo::host::application::ApplicationHost;
use caravan_demo::host::input::{InputIngress, MemoryInputIngress};
use caravan_demo::host::render::CollectingRenderSink;
use caravan_demo::host::storage::{MemoryStorage, StorageTransport};
use caravan_demo::input::{Button, InputPacket};
use caravan_demo::{CaravanInteraction, CaravanOrchestrator, CaravanStage};
use caravan_domain::GameJournalEntry;
use nworlds_host::SemanticVersion;

fn application() -> ApplicationHost {
    let mut writer = JournalWriter::new();
    writer.record(GameJournalEntry::create_saucer());
    let orchestrator = CaravanOrchestrator::new(
        actual(writer.finish()),
        LogicalTime::zero(),
        Tau::zero(),
        CaravanInteraction,
    )
    .expect("host fixture should initialize");
    ApplicationHost::new(
        CaravanStage::new(orchestrator),
        MemoryInputIngress::new(),
        MemoryStorage::new(),
        CollectingRenderSink::new(),
    )
}

#[test]
fn host_crossing_delivers_input_publishes_immutable_state_and_collects_frame() {
    let mut application = application();
    application
        .input_mut()
        .push(InputPacket::ButtonPressed(Button::Primary));

    assert!(application.step().expect("host step should succeed"));
    assert_eq!(application.render().frames().len(), 1);
    assert_eq!(
        application
            .package()
            .orchestrator()
            .worldline()
            .journal()
            .len(),
        2
    );
    assert!(!application
        .render()
        .last()
        .expect("render sink should contain a frame")
        .payload()
        .is_empty());
}

#[test]
fn host_exposes_caravan_semantic_package_declaration() {
    let declaration = application().package_declaration();

    assert_eq!(declaration.identity(), "caravan-demo");
    assert_eq!(declaration.version(), SemanticVersion::new(0, 1, 0));
    assert!(declaration.assets().is_empty());
    assert_eq!(declaration.persistence().format(), "caravan-worldline");
    assert_eq!(declaration.persistence().schema().value(), 1);
    assert_eq!(declaration.host().minimum(), SemanticVersion::new(0, 1, 0));
    assert_eq!(declaration.render_vocabulary().name(), "triangle-list-rgba");
    assert_eq!(
        declaration.render_vocabulary().version(),
        SemanticVersion::new(1, 0, 0)
    );
}

#[test]
fn input_transport_order_is_preserved_before_semantic_batch_conversion() {
    let mut ingress = MemoryInputIngress::new();
    ingress.push(InputPacket::ButtonPressed(Button::Primary));
    ingress.push(InputPacket::ButtonReleased(Button::Primary));

    let batch = ingress
        .drain()
        .expect("ingress should normalize its stream");
    assert_eq!(
        batch.packets().collect::<Vec<_>>(),
        vec![
            InputPacket::ButtonPressed(Button::Primary),
            InputPacket::ButtonReleased(Button::Primary),
        ]
    );
}

#[test]
fn host_storage_round_trip_preserves_selected_worldline_and_scrubbing() {
    let mut original = application();
    original
        .input_mut()
        .push(InputPacket::ButtonPressed(Button::Primary));
    original.step().expect("host step should publish");
    original.save_selected().expect("host save should succeed");
    let bytes = original
        .storage()
        .load()
        .expect("host storage should contain bytes");

    let mut restored = application();
    restored.storage_mut().store(bytes);
    restored
        .load_selected()
        .expect("host load should decode the selected worldline");

    assert_eq!(
        restored.package().orchestrator().worldline(),
        original.package().orchestrator().worldline()
    );
    let later = restored
        .package()
        .present_at(LogicalTime::from_game_ticks(3).unwrap(), Tau::from_ticks(3))
        .expect("later sample should render");
    let earlier = restored
        .package()
        .present_at(LogicalTime::zero(), Tau::zero())
        .expect("earlier sample should render");
    let later_again = restored
        .package()
        .present_at(LogicalTime::from_game_ticks(3).unwrap(), Tau::from_ticks(3))
        .expect("repeated later sample should render");

    assert_eq!(later, later_again);
    assert_eq!(earlier.tau(), Tau::zero());
}
