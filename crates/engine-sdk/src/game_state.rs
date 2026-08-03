use engine_time::LogicalTime;

/// An owned indexed-query result with the exact sampled logical time.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct GameState<P> {
    logical_time: LogicalTime,
    payload: P,
}

impl<P> GameState<P> {
    /// Creates a state result for one exact logical-time sample.
    pub fn new(logical_time: LogicalTime, payload: P) -> Self {
        Self {
            logical_time,
            payload,
        }
    }

    /// Returns the exact logical time owned by this state result.
    pub fn logical_time(&self) -> LogicalTime {
        self.logical_time
    }

    /// Borrows the opaque state payload.
    pub fn payload(&self) -> &P {
        &self.payload
    }

    /// Consumes the state envelope and returns its payload.
    pub fn into_payload(self) -> P {
        self.payload
    }

    /// Consumes the state envelope and returns its time and payload.
    pub fn into_parts(self) -> (LogicalTime, P) {
        (self.logical_time, self.payload)
    }
}
