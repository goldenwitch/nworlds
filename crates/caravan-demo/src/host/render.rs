use crate::render::RenderOutput;
use engine_sdk::Frame;

/// A target-facing execution port for owned Caravan render frames.
pub trait RenderSinkAdapter {
    /// Submits one owned frame for target execution or collection.
    fn submit(&mut self, frame: Frame<RenderOutput>);
}

/// In-memory render sink for tests and the first target composition.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CollectingRenderSink {
    frames: Vec<Frame<RenderOutput>>,
}

impl CollectingRenderSink {
    /// Creates an empty collecting sink.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns submitted frames in submission order.
    pub fn frames(&self) -> &[Frame<RenderOutput>] {
        &self.frames
    }

    /// Returns the most recently submitted frame, if any.
    pub fn last(&self) -> Option<&Frame<RenderOutput>> {
        self.frames.last()
    }
}

impl RenderSinkAdapter for CollectingRenderSink {
    fn submit(&mut self, frame: Frame<RenderOutput>) {
        self.frames.push(frame);
    }
}

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
