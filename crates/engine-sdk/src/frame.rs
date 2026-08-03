use engine_time::Tau;

/// An owned presentation result for one exact presentation-time sample.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Frame<P> {
    tau: Tau,
    payload: P,
}

impl<P> Frame<P> {
    /// Creates a frame result for one exact presentation-time sample.
    pub fn new(tau: Tau, payload: P) -> Self {
        Self { tau, payload }
    }

    /// Returns the exact presentation time owned by this frame result.
    pub fn tau(&self) -> Tau {
        self.tau
    }

    /// Borrows the opaque frame payload.
    pub fn payload(&self) -> &P {
        &self.payload
    }

    /// Consumes the frame envelope and returns its payload.
    pub fn into_payload(self) -> P {
        self.payload
    }

    /// Consumes the frame envelope and returns its time and payload.
    pub fn into_parts(self) -> (Tau, P) {
        (self.tau, self.payload)
    }
}
