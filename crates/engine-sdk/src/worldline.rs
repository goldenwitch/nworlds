use crate::{Context, Journal};

/// An immutable context and journal branch evaluated together.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Worldline<C, P> {
    context: Context<C>,
    journal: Journal<P>,
}

impl<C, P> Worldline<C, P> {
    /// Creates a worldline from immutable context and journal values.
    pub fn new(context: Context<C>, journal: Journal<P>) -> Self {
        Self { context, journal }
    }

    /// Borrows the immutable context envelope.
    pub fn context(&self) -> &Context<C> {
        &self.context
    }

    /// Borrows the immutable journal branch.
    pub fn journal(&self) -> &Journal<P> {
        &self.journal
    }

    /// Consumes the worldline and returns its two immutable inputs.
    pub fn into_parts(self) -> (Context<C>, Journal<P>) {
        (self.context, self.journal)
    }
}
