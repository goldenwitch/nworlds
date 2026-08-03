use engine_sdk::Playback;
use engine_time::{LogicalTime, Tau};

/// A checked affine mapping from presentation ticks to logical ticks.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LinearPlayback {
    origin: LogicalTime,
    rate: i64,
}

impl LinearPlayback {
    /// Creates a playback mapping with `origin + tau * rate` semantics.
    pub const fn new(origin: LogicalTime, rate: i64) -> Self {
        Self { origin, rate }
    }

    /// Creates one-to-one forward playback from logical time zero.
    pub const fn one_to_one() -> Self {
        Self::new(LogicalTime::zero(), 1)
    }

    /// Creates one-to-one reverse playback from the supplied logical time.
    pub const fn reverse_from(origin: LogicalTime) -> Self {
        Self::new(origin, -1)
    }

    /// Returns the logical-time origin used when `tau` is zero.
    pub const fn origin(self) -> LogicalTime {
        self.origin
    }

    /// Returns the signed logical ticks selected for each presentation tick.
    pub const fn rate(self) -> i64 {
        self.rate
    }

    /// Maps `tau` without hiding fixed-point overflow.
    pub fn try_logical_time_at(self, tau: Tau) -> Option<LogicalTime> {
        let offset = tau.checked_mul(self.rate)?;
        self.origin.checked_add_ticks(offset.ticks())
    }
}

impl Playback for LinearPlayback {
    fn logical_time_at(&self, tau: Tau) -> LogicalTime {
        self.try_logical_time_at(tau)
            .expect("linear playback overflowed LogicalTime")
    }
}
