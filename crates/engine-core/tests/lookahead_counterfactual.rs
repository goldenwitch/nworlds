use engine_core::{
    evaluate, evaluate_future, fork_counterfactual, AutonomousRule, Context, Event, JournalError,
    LogicalTime, Worldline,
};

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
struct Add(f64);

impl Event<Scalar> for Add {
    fn apply(&self, state: &mut Scalar, _at: LogicalTime) {
        state.value += self.0;
    }
}

fn parent_worldline() -> Worldline<Scalar, ConstantRate, Add> {
    Worldline::from_context(Context::new(Scalar { value: 0.0 }, [ConstantRate(1.0)]))
        .append(LogicalTime::new(2.0), Add(2.0))
        .unwrap()
        .append(LogicalTime::new(5.0), Add(10.0))
        .unwrap()
}

#[test]
fn context_new_uses_a_zero_time_origin() {
    let context = Context::new(Scalar { value: 0.0 }, std::iter::empty::<ConstantRate>());

    assert_eq!(context.initial_logical_time(), LogicalTime::zero());
}

#[test]
fn evaluation_starts_at_the_configured_origin_in_either_direction() {
    let origin = LogicalTime::new(5.0);
    let context =
        Context::with_initial_logical_time(Scalar { value: 10.0 }, [ConstantRate(2.0)], origin);
    assert_eq!(context.initial_logical_time(), origin);
    let worldline = Worldline::<Scalar, ConstantRate, Add>::from_context(context);

    assert_eq!(
        evaluate(&worldline, LogicalTime::new(7.0)).state().value,
        14.0
    );
    assert_eq!(
        evaluate(&worldline, LogicalTime::new(3.0)).state().value,
        6.0
    );
}

#[test]
fn future_evaluation_keeps_the_journal_fixed() {
    let parent = parent_worldline();
    let future_time = LogicalTime::new(8.0);
    let future = evaluate_future(&parent, future_time);
    let parent_with_later_event = parent.append(LogicalTime::new(7.0), Add(100.0)).unwrap();
    let reevaluated_future = evaluate_future(&parent_with_later_event, future_time);

    assert_eq!(future, evaluate(&parent, future_time));
    assert_eq!(future.state().value, 20.0);
    assert_eq!(future, evaluate_future(&parent, future_time));
    assert_ne!(future, reevaluated_future);
    assert_eq!(reevaluated_future.state().value, 120.0);
}

#[test]
fn counterfactual_agrees_before_the_fork() {
    let parent = parent_worldline();
    let branch = fork_counterfactual(
        &parent,
        LogicalTime::new(3.0),
        [(LogicalTime::new(4.0), Add(3.0))],
    )
    .unwrap();

    assert_eq!(
        evaluate_future(&parent, LogicalTime::new(2.5)),
        evaluate_future(&branch, LogicalTime::new(2.5))
    );
}

#[test]
fn empty_counterfactual_rejects_events_at_or_before_the_fork() {
    let parent = parent_worldline();
    let fork_time = LogicalTime::new(1.0);
    let branch =
        fork_counterfactual(&parent, fork_time, std::iter::empty::<(LogicalTime, Add)>()).unwrap();

    assert!(branch.journal().is_empty());
    for attempted in [LogicalTime::new(0.5), fork_time] {
        assert_eq!(
            branch.append(attempted, Add(1.0)),
            Err(JournalError::EventAtOrBeforeFork {
                fork: fork_time,
                attempted,
            })
        );
    }
}

#[test]
fn counterfactual_diverges_after_the_fork() {
    let parent = parent_worldline();
    let branch = fork_counterfactual(
        &parent,
        LogicalTime::new(3.0),
        [(LogicalTime::new(4.0), Add(3.0))],
    )
    .unwrap();

    assert_ne!(
        evaluate_future(&parent, LogicalTime::new(4.0)),
        evaluate_future(&branch, LogicalTime::new(4.0))
    );
    assert_eq!(
        evaluate_future(&parent, LogicalTime::new(4.0))
            .state()
            .value,
        6.0
    );
    assert_eq!(
        evaluate_future(&branch, LogicalTime::new(4.0))
            .state()
            .value,
        9.0
    );
}

#[test]
fn counterfactual_excludes_parent_suffix_at_a_later_target() {
    let parent = parent_worldline();
    let branch = fork_counterfactual(
        &parent,
        LogicalTime::new(3.0),
        [(LogicalTime::new(4.0), Add(3.0))],
    )
    .unwrap();
    let target = LogicalTime::new(6.0);

    assert_eq!(evaluate_future(&parent, target).state().value, 18.0);
    assert_eq!(evaluate_future(&branch, target).state().value, 11.0);
}

#[test]
fn counterfactual_branch_is_isolated_from_parent() {
    let parent = parent_worldline();
    let parent_before = parent.clone();
    let branch = fork_counterfactual(
        &parent,
        LogicalTime::new(3.0),
        [(LogicalTime::new(4.0), Add(3.0))],
    )
    .unwrap();
    let parent_with_new_event = parent.append(LogicalTime::new(6.0), Add(6.0)).unwrap();
    let branch_with_new_event = branch.append(LogicalTime::new(7.0), Add(7.0)).unwrap();

    assert_eq!(parent, parent_before);
    assert_eq!(parent.journal().len(), 2);
    assert_eq!(branch.journal().len(), 2);
    assert_eq!(parent_with_new_event.journal().len(), 3);
    assert_eq!(branch_with_new_event.journal().len(), 3);
    assert_eq!(branch.journal().horizon(), Some(LogicalTime::new(4.0)));
    assert_eq!(parent.journal().horizon(), Some(LogicalTime::new(5.0)));
}

#[test]
fn counterfactuals_are_replayable_from_the_same_inputs() {
    let parent = parent_worldline();
    let fork_time = LogicalTime::new(3.0);
    let alternate_events = [(LogicalTime::new(4.0), Add(3.0))];
    let first_branch = fork_counterfactual(&parent, fork_time, alternate_events.clone()).unwrap();
    let second_branch = fork_counterfactual(&parent, fork_time, alternate_events).unwrap();
    let target = LogicalTime::new(6.0);

    assert_eq!(first_branch, second_branch);
    assert_eq!(
        evaluate_future(&first_branch, target),
        evaluate_future(&second_branch, target)
    );
    assert_eq!(
        evaluate_future(&first_branch, target),
        evaluate(&first_branch, target)
    );
}
