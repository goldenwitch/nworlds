use caravan_demo::host::application::ApplicationHost;
use caravan_demo::host::input::{InputIngress, MemoryInputIngress};
use caravan_demo::host::render::CollectingRenderSink;
use caravan_demo::host::storage::{MemoryStorage, StorageTransport};
use caravan_demo::input::{Button, InputPacket};
use caravan_demo::{CaravanInteraction, CaravanOrchestrator, CaravanStage};
use caravan_domain::GameJournalEntry;
use caravan_reference::actual;
use engine_journal::JournalWriter;
use engine_time::{LogicalTime, Tau};

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
    assert_eq!(
        application
            .render()
            .last()
            .expect("render sink should contain a frame")
            .payload()
            .logical_time(),
        LogicalTime::zero()
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
