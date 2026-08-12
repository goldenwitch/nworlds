use crate::render::RenderOutput;
use engine_sdk::Frame;

pub use nworlds_host::RenderSink as RenderSinkAdapter;
pub type CollectingRenderSink = nworlds_host::CollectingRenderSink<Frame<RenderOutput>>;

#[cfg(test)]
mod tests {
    use super::{CollectingRenderSink, RenderSinkAdapter};
    use crate::render::CaravanRenderer;
    use caravan_domain::GameJournalEntry;
    use caravan_reference::{actual, state, Snapshot};
    use engine_journal::JournalWriter;
    use engine_presentation::present;
    use engine_time::{LogicalTime, Tau};

    #[test]
    fn collecting_sink_owns_repeated_submissions_in_order() {
        let mut writer = JournalWriter::new();
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
