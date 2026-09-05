use crate::engine_integration::{CaravanFrame, RenderBatch};

pub use nworlds_host::RenderSink as RenderSinkAdapter;
pub type CollectingRenderSink = nworlds_host::CollectingRenderSink<CaravanFrame<RenderBatch>>;

#[cfg(test)]
mod tests {
    use super::{CollectingRenderSink, RenderSinkAdapter};
    use crate::engine_integration::{present, CaravanJournalWriter, LogicalTime, Tau};
    use crate::render::CaravanRenderer;
    use caravan_domain::GameJournalEntry;
    use caravan_reference::{actual, state, Snapshot};

    #[test]
    fn collecting_sink_owns_repeated_submissions_in_order() {
        let mut writer = CaravanJournalWriter::new();
        writer.record(GameJournalEntry::create_saucer());
        let worldline = actual(writer.finish());
        let state = state(&worldline, LogicalTime::zero());
        let first = present::<Snapshot, CaravanRenderer>(&state, Tau::from_ticks(1));
        let second = present::<Snapshot, CaravanRenderer>(&state, Tau::from_ticks(2));
        let mut sink = CollectingRenderSink::new();

        sink.submit(first.clone());
        sink.submit(second.clone());

        assert_eq!(sink.frames()[0], first);
        assert_eq!(sink.frames()[1], second);
        assert_eq!(sink.last(), Some(&second));
    }
}
