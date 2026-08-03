/// Immutable game-definition input carried by a worldline.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Context<P> {
    payload: P,
}

impl<P> Context<P> {
    /// Wraps an opaque context payload.
    pub fn new(payload: P) -> Self {
        Self { payload }
    }

    /// Borrows the opaque context payload.
    pub fn payload(&self) -> &P {
        &self.payload
    }

    /// Consumes the envelope and returns its payload.
    pub fn into_payload(self) -> P {
        self.payload
    }
}
