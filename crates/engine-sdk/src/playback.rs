use engine_time::{LogicalTime, Tau};

/// A value-producing mapping from presentation time to logical time.
pub trait Playback {
    /// Selects the logical time displayed at the supplied presentation time.
    fn logical_time_at(&self, tau: Tau) -> LogicalTime;
}
