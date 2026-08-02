use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

mod counterfactual;
mod lookahead;

pub use counterfactual::fork_counterfactual;
pub use lookahead::evaluate_future;

macro_rules! define_time {
    ($name:ident, $description:literal) => {
        #[doc = $description]
        #[derive(Copy, Clone, Debug, Default)]
        pub struct $name(f64);

        impl $name {
            pub fn new(value: f64) -> Self {
                assert!(
                    value.is_finite(),
                    concat!(stringify!($name), " must be finite")
                );
                Self(if value == 0.0 { 0.0 } else { value })
            }

            pub const fn zero() -> Self {
                Self(0.0)
            }

            pub const fn value(self) -> f64 {
                self.0
            }
        }

        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                self.0 == other.0
            }
        }

        impl Eq for $name {}

        impl PartialOrd for $name {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        impl Ord for $name {
            fn cmp(&self, other: &Self) -> Ordering {
                self.0.total_cmp(&other.0)
            }
        }

        impl Hash for $name {
            fn hash<H: Hasher>(&self, state: &mut H) {
                self.0.to_bits().hash(state);
            }
        }
    };
}

define_time!(
    LogicalTime,
    "Continuous authoritative time used to evaluate a worldline."
);
define_time!(
    Tau,
    "Presentation time used to select or render a game state."
);

pub trait AutonomousRule<S> {
    fn advance(&self, state: &mut S, from: LogicalTime, to: LogicalTime);
}

pub trait Event<S> {
    fn apply(&self, state: &mut S, at: LogicalTime);
}

#[derive(Clone, Debug, PartialEq)]
pub struct Context<S, R> {
    initial_state: S,
    initial_logical_time: LogicalTime,
    rules: Vec<R>,
}

impl<S, R> Context<S, R> {
    pub fn new<I>(initial_state: S, rules: I) -> Self
    where
        I: IntoIterator<Item = R>,
    {
        Self::with_initial_logical_time(initial_state, rules, LogicalTime::zero())
    }

    pub fn with_initial_logical_time<I>(
        initial_state: S,
        rules: I,
        initial_logical_time: LogicalTime,
    ) -> Self
    where
        I: IntoIterator<Item = R>,
    {
        Self {
            initial_state,
            initial_logical_time,
            rules: rules.into_iter().collect(),
        }
    }

    pub fn initial_state(&self) -> &S {
        &self.initial_state
    }

    pub fn initial_logical_time(&self) -> LogicalTime {
        self.initial_logical_time
    }

    pub fn rules(&self) -> &[R] {
        &self.rules
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct JournalEntry<E> {
    time: LogicalTime,
    event: E,
}

impl<E> JournalEntry<E> {
    pub fn time(&self) -> LogicalTime {
        self.time
    }

    pub fn event(&self) -> &E {
        &self.event
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JournalError {
    LateEvent {
        previous: LogicalTime,
        attempted: LogicalTime,
    },
    EventAtOrBeforeFork {
        fork: LogicalTime,
        attempted: LogicalTime,
    },
}

impl fmt::Display for JournalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LateEvent {
                previous,
                attempted,
            } => write!(
                formatter,
                "journal event at {} follows event at {}",
                attempted.value(),
                previous.value()
            ),
            Self::EventAtOrBeforeFork { fork, attempted } => write!(
                formatter,
                "alternate event at {} is not after fork at {}",
                attempted.value(),
                fork.value()
            ),
        }
    }
}

impl std::error::Error for JournalError {}

#[derive(Clone, Debug, PartialEq)]
pub struct Journal<E> {
    entries: Vec<JournalEntry<E>>,
}

impl<E> Default for Journal<E> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl<E> Journal<E> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append(&self, time: LogicalTime, event: E) -> Result<Self, JournalError>
    where
        E: Clone,
    {
        if let Some(previous) = self.horizon() {
            if time < previous {
                return Err(JournalError::LateEvent {
                    previous,
                    attempted: time,
                });
            }
        }

        let mut entries = self.entries.clone();
        entries.push(JournalEntry { time, event });
        Ok(Self { entries })
    }

    pub fn append_all<I>(&self, events: I) -> Result<Self, JournalError>
    where
        E: Clone,
        I: IntoIterator<Item = (LogicalTime, E)>,
    {
        events
            .into_iter()
            .try_fold(self.clone(), |journal, (time, event)| {
                journal.append(time, event)
            })
    }

    pub fn horizon(&self) -> Option<LogicalTime> {
        self.entries.last().map(JournalEntry::time)
    }

    pub fn prefix(&self, through: LogicalTime) -> Self
    where
        E: Clone,
    {
        Self {
            entries: self
                .entries
                .iter()
                .take_while(|entry| entry.time() <= through)
                .cloned()
                .collect(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &JournalEntry<E>> {
        self.entries.iter()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Worldline<S, R, E> {
    context: Arc<Context<S, R>>,
    journal: Journal<E>,
    fork_time: Option<LogicalTime>,
}

impl<S, R, E> Worldline<S, R, E> {
    pub fn new(context: Context<S, R>, journal: Journal<E>) -> Self {
        Self {
            context: Arc::new(context),
            journal,
            fork_time: None,
        }
    }

    pub fn from_context(context: Context<S, R>) -> Self {
        Self::new(context, Journal::new())
    }

    pub fn context(&self) -> &Context<S, R> {
        &self.context
    }

    pub fn journal(&self) -> &Journal<E> {
        &self.journal
    }

    pub fn append(&self, time: LogicalTime, event: E) -> Result<Self, JournalError>
    where
        E: Clone,
    {
        if let Some(fork_time) = self.fork_time {
            if time <= fork_time {
                return Err(JournalError::EventAtOrBeforeFork {
                    fork: fork_time,
                    attempted: time,
                });
            }
        }

        Ok(Self {
            context: Arc::clone(&self.context),
            journal: self.journal.append(time, event)?,
            fork_time: self.fork_time,
        })
    }

    pub fn fork<I>(&self, fork_time: LogicalTime, alternate_events: I) -> Result<Self, JournalError>
    where
        E: Clone,
        I: IntoIterator<Item = (LogicalTime, E)>,
    {
        let mut journal = self.journal.prefix(fork_time);
        for (time, event) in alternate_events {
            if time <= fork_time {
                return Err(JournalError::EventAtOrBeforeFork {
                    fork: fork_time,
                    attempted: time,
                });
            }
            journal = journal.append(time, event)?;
        }

        Ok(Self {
            context: Arc::clone(&self.context),
            journal,
            fork_time: Some(fork_time),
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GameState<S> {
    logical_time: LogicalTime,
    state: S,
}

impl<S> GameState<S> {
    fn new(logical_time: LogicalTime, state: S) -> Self {
        Self {
            logical_time,
            state,
        }
    }

    pub fn logical_time(&self) -> LogicalTime {
        self.logical_time
    }

    pub fn state(&self) -> &S {
        &self.state
    }

    pub fn into_state(self) -> S {
        self.state
    }
}

pub fn evaluate<S, R, E>(worldline: &Worldline<S, R, E>, target: LogicalTime) -> GameState<S>
where
    S: Clone,
    R: AutonomousRule<S>,
    E: Event<S>,
{
    let mut state = worldline.context().initial_state().clone();
    let mut cursor = worldline.context().initial_logical_time();
    let mut entries: Vec<_> = worldline
        .journal()
        .iter()
        .filter(|entry| entry.time() <= target)
        .collect();
    entries.sort_by_key(|entry| entry.time());

    for entry in entries {
        advance_rules(
            worldline.context().rules(),
            &mut state,
            cursor,
            entry.time(),
        );
        entry.event().apply(&mut state, entry.time());
        cursor = entry.time();
    }

    advance_rules(worldline.context().rules(), &mut state, cursor, target);
    GameState::new(target, state)
}

fn advance_rules<S, R>(rules: &[R], state: &mut S, from: LogicalTime, to: LogicalTime)
where
    R: AutonomousRule<S>,
{
    for rule in rules {
        rule.advance(state, from, to);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct Scalar {
        value: f64,
    }

    #[derive(Clone, Debug, PartialEq)]
    struct ConstantRate(f64);

    impl AutonomousRule<Scalar> for ConstantRate {
        fn advance(&self, state: &mut Scalar, from: LogicalTime, to: LogicalTime) {
            state.value += self.0 * (to.value() - from.value());
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    enum Adjustment {
        Add(f64),
        Multiply(f64),
    }

    impl Event<Scalar> for Adjustment {
        fn apply(&self, state: &mut Scalar, _at: LogicalTime) {
            match self {
                Self::Add(amount) => state.value += amount,
                Self::Multiply(factor) => state.value *= factor,
            }
        }
    }

    fn worldline() -> Worldline<Scalar, ConstantRate, Adjustment> {
        let context = Context::new(Scalar { value: 1.0 }, [ConstantRate(2.0)]);
        Worldline::from_context(context)
            .append(LogicalTime::new(3.0), Adjustment::Add(5.0))
            .unwrap()
    }

    #[test]
    fn evaluation_is_repeatable_and_does_not_mutate_the_worldline() {
        let worldline = worldline();
        let before = worldline.clone();
        let target = LogicalTime::new(4.5);

        let first = evaluate(&worldline, target);
        let second = evaluate(&worldline, target);

        assert_eq!(first, second);
        assert_eq!(worldline, before);
    }

    #[test]
    fn logical_time_is_owned_by_the_game_state() {
        let target = LogicalTime::new(4.5);
        let result = evaluate(&worldline(), target);

        assert_eq!(result.logical_time(), target);
        assert_eq!(result.state().value, 15.0);
    }

    #[test]
    fn identical_inputs_produce_equal_worldlines_and_states() {
        let left = worldline();
        let right = worldline();
        let target = LogicalTime::new(4.5);

        assert_eq!(left, right);
        assert_eq!(evaluate(&left, target), evaluate(&right, target));
    }

    #[test]
    fn journal_tracks_horizon_and_extracts_an_inclusive_prefix() {
        let first_time = LogicalTime::new(1.0);
        let boundary = LogicalTime::new(2.0);
        let journal = Journal::new()
            .append(first_time, Adjustment::Add(1.0))
            .unwrap()
            .append(boundary, Adjustment::Add(2.0))
            .unwrap()
            .append(LogicalTime::new(3.0), Adjustment::Add(3.0))
            .unwrap();

        assert_eq!(Journal::<Adjustment>::new().horizon(), None);
        assert_eq!(journal.horizon(), Some(LogicalTime::new(3.0)));

        let prefix = journal.prefix(boundary);
        assert_eq!(prefix.len(), 2);
        assert_eq!(prefix.horizon(), Some(boundary));
        assert_eq!(
            prefix.iter().map(JournalEntry::time).collect::<Vec<_>>(),
            vec![first_time, boundary,]
        );
    }

    #[test]
    fn late_events_are_rejected_without_changing_the_journal() {
        let journal = Journal::new()
            .append(LogicalTime::new(2.0), Adjustment::Add(2.0))
            .unwrap();

        let result = journal.append(LogicalTime::new(1.0), Adjustment::Add(1.0));

        assert_eq!(
            result,
            Err(JournalError::LateEvent {
                previous: LogicalTime::new(2.0),
                attempted: LogicalTime::new(1.0),
            })
        );
        assert_eq!(journal.len(), 1);
        assert_eq!(journal.horizon(), Some(LogicalTime::new(2.0)));
    }

    #[test]
    fn equal_time_events_keep_append_order() {
        let time = LogicalTime::new(1.0);
        let context = Context::new(Scalar { value: 1.0 }, std::iter::empty::<ConstantRate>());
        let worldline = Worldline::from_context(context)
            .append(time, Adjustment::Add(1.0))
            .unwrap()
            .append(time, Adjustment::Multiply(2.0))
            .unwrap();

        assert_eq!(evaluate(&worldline, time).state().value, 4.0);
    }

    #[test]
    fn fork_keeps_the_boundary_and_replaces_the_parent_suffix() {
        let fork_time = LogicalTime::new(2.0);
        let parent = Worldline::from_context(Context::new(
            Scalar { value: 0.0 },
            std::iter::empty::<ConstantRate>(),
        ))
        .append(LogicalTime::new(1.0), Adjustment::Add(1.0))
        .unwrap()
        .append(fork_time, Adjustment::Add(2.0))
        .unwrap()
        .append(LogicalTime::new(4.0), Adjustment::Add(40.0))
        .unwrap();

        let child = parent
            .fork(fork_time, [(LogicalTime::new(3.0), Adjustment::Add(3.0))])
            .unwrap();

        assert_eq!(child.journal().len(), 3);
        assert_eq!(child.journal().horizon(), Some(LogicalTime::new(3.0)));
        assert_eq!(evaluate(&parent, fork_time), evaluate(&child, fork_time));
        assert_eq!(evaluate(&child, LogicalTime::new(3.0)).state().value, 6.0);
        assert_eq!(evaluate(&parent, LogicalTime::new(3.0)).state().value, 3.0);
    }

    #[test]
    fn fork_rejects_boundary_events_and_invalid_sequences() {
        let fork_time = LogicalTime::new(2.0);
        let parent = Worldline::from_context(Context::new(
            Scalar { value: 0.0 },
            std::iter::empty::<ConstantRate>(),
        ))
        .append(LogicalTime::new(1.0), Adjustment::Add(1.0))
        .unwrap();

        assert_eq!(
            parent.fork(fork_time, [(fork_time, Adjustment::Add(2.0))]),
            Err(JournalError::EventAtOrBeforeFork {
                fork: fork_time,
                attempted: fork_time,
            })
        );
        assert_eq!(
            parent.fork(
                fork_time,
                [
                    (LogicalTime::new(4.0), Adjustment::Add(4.0)),
                    (LogicalTime::new(3.0), Adjustment::Add(3.0)),
                ],
            ),
            Err(JournalError::LateEvent {
                previous: LogicalTime::new(4.0),
                attempted: LogicalTime::new(3.0),
            })
        );
    }

    #[test]
    fn parent_and_child_are_isolated_value_producing_worldlines() {
        let parent = Worldline::from_context(Context::new(
            Scalar { value: 0.0 },
            std::iter::empty::<ConstantRate>(),
        ))
        .append(LogicalTime::new(1.0), Adjustment::Add(1.0))
        .unwrap();
        let parent_before = parent.clone();
        let child = parent
            .fork(
                LogicalTime::new(1.0),
                [(LogicalTime::new(2.0), Adjustment::Add(2.0))],
            )
            .unwrap();
        let parent_after = parent
            .append(LogicalTime::new(3.0), Adjustment::Add(3.0))
            .unwrap();

        assert_eq!(parent, parent_before);
        assert_eq!(child.journal().len(), 2);
        assert_eq!(parent_after.journal().len(), 2);
        assert_eq!(child.journal().horizon(), Some(LogicalTime::new(2.0)));
        assert_eq!(
            parent_after.journal().horizon(),
            Some(LogicalTime::new(3.0))
        );
    }
}
