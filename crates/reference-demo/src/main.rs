use std::fmt::Write as _;

use engine_core::{
    evaluate, evaluate_future, fork_counterfactual, AutonomousRule, Context, Event, GameState,
    LogicalTime, Tau, Worldline,
};
use presentation::{present, LinearPlayback, Playback, Renderer};

#[derive(Clone, Debug, PartialEq)]
struct FixtureState {
    quantity: f64,
}

#[derive(Clone, Debug, PartialEq)]
struct ConstantRate {
    per_time: f64,
}

impl AutonomousRule<FixtureState> for ConstantRate {
    fn advance(&self, state: &mut FixtureState, from: LogicalTime, to: LogicalTime) {
        state.quantity += self.per_time * (to.value() - from.value());
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Pulse {
    amount: f64,
}

impl Event<FixtureState> for Pulse {
    fn apply(&self, state: &mut FixtureState, _at: LogicalTime) {
        state.quantity += self.amount;
    }
}

type FixtureWorldline = Worldline<FixtureState, ConstantRate, Pulse>;

#[derive(Clone, Debug, PartialEq)]
struct TerminalFrame {
    logical_time: LogicalTime,
    quantity: f64,
    tau: Tau,
}

impl TerminalFrame {
    fn describe(&self) -> String {
        format!(
            "tau={:.1} -> t={:.1}, quantity={:.1}",
            self.tau.value(),
            self.logical_time.value(),
            self.quantity
        )
    }
}

struct TerminalRenderer;

impl Renderer<FixtureState> for TerminalRenderer {
    type Frame = TerminalFrame;

    fn render(&self, state: &GameState<FixtureState>, tau: Tau) -> Self::Frame {
        TerminalFrame {
            logical_time: state.logical_time(),
            quantity: state.state().quantity,
            tau,
        }
    }
}

fn parent_worldline() -> FixtureWorldline {
    Worldline::from_context(Context::new(
        FixtureState { quantity: 0.0 },
        [ConstantRate { per_time: 1.0 }],
    ))
    .append(LogicalTime::new(2.0), Pulse { amount: 5.0 })
    .expect("the fixed fixture event is in chronological order")
}

fn trace() -> String {
    let mut output = String::new();
    let parent = parent_worldline();
    let parent_before = parent.clone();
    let event_time = LogicalTime::new(2.0);
    let playback = LinearPlayback::new(LogicalTime::zero(), 1.0);
    let renderer = TerminalRenderer;

    writeln!(output, "reference-demo").unwrap();
    writeln!(
        output,
        "fixture: quantity=0.0, autonomous rate=+1.0 per logical time"
    )
    .unwrap();
    writeln!(
        output,
        "journal: Pulse(+5.0) at t={:.1}",
        event_time.value()
    )
    .unwrap();

    writeln!(output, "state evaluation:").unwrap();
    for target in [1.0, 2.0, 3.5] {
        let state = evaluate(&parent, LogicalTime::new(target));
        writeln!(
            output,
            "  t={:.1} -> quantity={:.1}{}",
            target,
            state.state().quantity,
            if target == event_time.value() {
                " (event included)"
            } else {
                ""
            }
        )
        .unwrap();
    }

    writeln!(output, "playback / scrub:").unwrap();
    for tau_value in [1.0, 3.0, 1.0] {
        let tau = Tau::new(tau_value);
        let selected_time = playback.logical_time(tau);
        let frame = present(&parent, &playback, &renderer, tau);
        assert_eq!(frame.logical_time, selected_time);
        writeln!(output, "  {}", frame.describe()).unwrap();
    }

    let future_time = LogicalTime::new(5.0);
    let lookahead = evaluate_future(&parent, future_time);
    let later_worldline = parent
        .append(LogicalTime::new(6.0), Pulse { amount: 100.0 })
        .expect("the later fixture event is in chronological order");
    assert_eq!(lookahead, evaluate_future(&parent, future_time));
    assert_eq!(lookahead, evaluate_future(&later_worldline, future_time));
    assert_eq!(parent, parent_before);

    writeln!(output, "lookahead / fixed worldline:").unwrap();
    writeln!(
        output,
        "  t=5.0 -> quantity={:.1} (journal horizon=2.0)",
        lookahead.state().quantity
    )
    .unwrap();
    writeln!(
        output,
        "  later branch journal length={} (original={})",
        later_worldline.journal().len(),
        parent.journal().len()
    )
    .unwrap();
    writeln!(output, "  original lookahead remains unchanged: yes").unwrap();

    let fork_time = LogicalTime::new(2.0);
    let counterfactual = fork_counterfactual(
        &parent,
        fork_time,
        [(LogicalTime::new(3.0), Pulse { amount: -2.0 })],
    )
    .expect("the alternate event is after the fork boundary");
    assert_eq!(
        evaluate(&parent, fork_time),
        evaluate(&counterfactual, fork_time)
    );
    assert_eq!(parent, parent_before);

    let parent_frame = present(&parent, &playback, &renderer, Tau::new(4.0));
    let branch_frame = present(&counterfactual, &playback, &renderer, Tau::new(4.0));

    writeln!(output, "counterfactual / immutable fork:").unwrap();
    writeln!(output, "  fork at t=2.0: branch adds Pulse(-2.0) at t=3.0").unwrap();
    writeln!(output, "  agreement at fork: yes").unwrap();
    writeln!(output, "  parent @ t=4.0: {}", parent_frame.describe()).unwrap();
    writeln!(output, "  branch @ t=4.0: {}", branch_frame.describe()).unwrap();
    writeln!(
        output,
        "  parent journal unchanged: yes (len={}, horizon=2.0)",
        parent.journal().len()
    )
    .unwrap();
    writeln!(
        output,
        "  branch journal: len={}, horizon=3.0",
        counterfactual.journal().len()
    )
    .unwrap();

    output
}

fn main() {
    print!("{}", trace());
}

#[cfg(test)]
mod tests {
    use super::trace;

    #[test]
    fn terminal_trace_is_stable() {
        assert_eq!(
            trace(),
            concat!(
                "reference-demo\n",
                "fixture: quantity=0.0, autonomous rate=+1.0 per logical time\n",
                "journal: Pulse(+5.0) at t=2.0\n",
                "state evaluation:\n",
                "  t=1.0 -> quantity=1.0\n",
                "  t=2.0 -> quantity=7.0 (event included)\n",
                "  t=3.5 -> quantity=8.5\n",
                "playback / scrub:\n",
                "  tau=1.0 -> t=1.0, quantity=1.0\n",
                "  tau=3.0 -> t=3.0, quantity=8.0\n",
                "  tau=1.0 -> t=1.0, quantity=1.0\n",
                "lookahead / fixed worldline:\n",
                "  t=5.0 -> quantity=10.0 (journal horizon=2.0)\n",
                "  later branch journal length=2 (original=1)\n",
                "  original lookahead remains unchanged: yes\n",
                "counterfactual / immutable fork:\n",
                "  fork at t=2.0: branch adds Pulse(-2.0) at t=3.0\n",
                "  agreement at fork: yes\n",
                "  parent @ t=4.0: tau=4.0 -> t=4.0, quantity=9.0\n",
                "  branch @ t=4.0: tau=4.0 -> t=4.0, quantity=7.0\n",
                "  parent journal unchanged: yes (len=1, horizon=2.0)\n",
                "  branch journal: len=2, horizon=3.0\n",
            )
        );
    }
}
