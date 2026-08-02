use engine_core::{
    evaluate, fork_counterfactual, AutonomousRule, Context, Event, GameState, LogicalTime, Tau,
    Worldline,
};
use presentation::{present, Animation, LinearPlayback, Playback, Renderer};

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

#[derive(Clone, Debug, PartialEq)]
struct TestFrame {
    logical_time: LogicalTime,
    value: f64,
    tau: Tau,
}

struct TestRenderer;

impl Renderer<Scalar> for TestRenderer {
    type Frame = TestFrame;

    fn render(&self, state: &GameState<Scalar>, tau: Tau) -> Self::Frame {
        TestFrame {
            logical_time: state.logical_time(),
            value: state.state().value,
            tau,
        }
    }
}

struct TestAnimation;

impl Animation<Scalar> for TestAnimation {
    type Sample = f64;

    fn sample(&self, state: &GameState<Scalar>, tau: Tau) -> Option<Self::Sample> {
        Some(state.state().value + tau.value())
    }
}

struct AbsentAnimation;

impl Animation<Scalar> for AbsentAnimation {
    type Sample = f64;

    fn sample(&self, _state: &GameState<Scalar>, _tau: Tau) -> Option<Self::Sample> {
        None
    }
}

fn worldline() -> Worldline<Scalar, ConstantRate, Add> {
    Worldline::from_context(Context::new(Scalar { value: 0.0 }, [ConstantRate(1.0)]))
        .append(LogicalTime::new(4.0), Add(10.0))
        .unwrap()
}

#[test]
fn linear_playback_selects_scrubbed_reversed_past_and_future_times() {
    let forward = LinearPlayback::new(LogicalTime::zero(), 1.0);
    assert_eq!(forward.logical_time(Tau::new(-2.0)), LogicalTime::new(-2.0));
    assert_eq!(forward.logical_time(Tau::new(6.0)), LogicalTime::new(6.0));

    let reverse = LinearPlayback::new(LogicalTime::new(6.0), -1.0);
    assert_eq!(reverse.logical_time(Tau::new(1.5)), LogicalTime::new(4.5));
}

#[test]
fn present_is_deterministic_and_does_not_mutate_the_worldline() {
    let worldline = worldline();
    let before = worldline.clone();
    let playback = LinearPlayback::new(LogicalTime::zero(), 1.0);
    let renderer = TestRenderer;
    let tau = Tau::new(6.0);

    let first = present(&worldline, &playback, &renderer, tau);
    let second = present(&worldline, &playback, &renderer, tau);

    assert_eq!(first, second);
    assert_eq!(first.logical_time, LogicalTime::new(6.0));
    assert_eq!(first.value, 16.0);
    assert_eq!(worldline, before);
}

#[test]
fn present_renders_an_immutable_counterfactual_worldline_through_the_renderer() {
    let parent = worldline();
    let parent_before = parent.clone();
    let counterfactual = fork_counterfactual(
        &parent,
        LogicalTime::new(3.0),
        [(LogicalTime::new(4.0), Add(3.0))],
    )
    .unwrap();
    let counterfactual_before = counterfactual.clone();
    let playback = LinearPlayback::new(LogicalTime::zero(), 1.0);
    let renderer = TestRenderer;
    let tau = Tau::new(4.0);

    let parent_frame = present(&parent, &playback, &renderer, tau);
    let first_counterfactual_frame = present(&counterfactual, &playback, &renderer, tau);
    let second_counterfactual_frame = present(&counterfactual, &playback, &renderer, tau);

    assert_eq!(parent_frame.value, 14.0);
    assert_eq!(
        first_counterfactual_frame.logical_time,
        LogicalTime::new(4.0)
    );
    assert_eq!(first_counterfactual_frame.value, 7.0);
    assert_eq!(first_counterfactual_frame.tau, tau);
    assert_eq!(first_counterfactual_frame, second_counterfactual_frame);
    assert_eq!(parent, parent_before);
    assert_eq!(counterfactual, counterfactual_before);
}

#[test]
fn renderer_and_animation_read_a_selected_state_without_mutating_it() {
    let worldline = worldline();
    let state = evaluate(&worldline, LogicalTime::new(2.0));
    let before = state.clone();
    let renderer = TestRenderer;
    let animation = TestAnimation;
    let tau = Tau::new(2.0);

    let frame = renderer.render(&state, tau);
    let first_sample = animation.sample(&state, tau);
    let second_sample = animation.sample(&state, tau);

    assert_eq!(frame.value, 2.0);
    assert_eq!(first_sample, Some(4.0));
    assert_eq!(first_sample, second_sample);
    assert_eq!(state, before);
}

#[test]
fn animation_can_return_none_deterministically() {
    let worldline = worldline();
    let state = evaluate(&worldline, LogicalTime::new(2.0));
    let animation = AbsentAnimation;
    let tau = Tau::new(2.0);

    let first_sample = animation.sample(&state, tau);
    let second_sample = animation.sample(&state, tau);

    assert_eq!(first_sample, None);
    assert_eq!(first_sample, second_sample);
}
