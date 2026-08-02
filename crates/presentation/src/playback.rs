use engine_core::{LogicalTime, Tau};

pub trait Playback {
    fn logical_time(&self, tau: Tau) -> LogicalTime;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinearPlayback {
    logical_origin: LogicalTime,
    rate: f64,
}

impl LinearPlayback {
    pub fn new(logical_origin: LogicalTime, rate: f64) -> Self {
        assert!(rate.is_finite(), "playback rate must be finite");
        Self {
            logical_origin,
            rate,
        }
    }

    pub const fn logical_origin(self) -> LogicalTime {
        self.logical_origin
    }

    pub const fn rate(self) -> f64 {
        self.rate
    }
}

impl Playback for LinearPlayback {
    fn logical_time(&self, tau: Tau) -> LogicalTime {
        LogicalTime::new(self.logical_origin.value() + self.rate * tau.value())
    }
}
