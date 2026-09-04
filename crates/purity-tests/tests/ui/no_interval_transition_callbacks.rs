#![forbid(unsafe_code)]

use engine_api::{state, Context, IndexedQuery, Journal, LogicalTime, QueryInput};

struct Query;

impl IndexedQuery<(), ()> for Query {
    type Result = ();

    fn query(&self, _input: QueryInput<'_, (), ()>) -> Self::Result {
        ()
    }
}

fn interval_transition(_: &mut (), _: LogicalTime, _: LogicalTime) {}

fn main() {
    let context = Context::new(());
    let journal = Journal::<()>::empty();
    state(
        &context,
        &journal,
        LogicalTime::zero(),
        Query,
        interval_transition,
    );
}
