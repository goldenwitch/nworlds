# Semantic Contract

This proposal maps [spec/initial.md](../spec/initial.md) to the current Rust
boundary at [crates/engine-core/src/lib.rs](../crates/engine-core/src/lib.rs).
It records observed implementation facts and the provisional journal/worldline
decisions below; it does not add rendering, gameplay, or a second evaluator.

## Ruling ledger (2026-08-01)

The initial specification stays abstract: its logical-time domain is
continuous in the mathematical model. This document is the practical runtime
contract, where the representation must have a finite, exact cutoff.

### Confirmed semantics

- An event whose timestamp equals the evaluation target is included.
- Events sharing a timestamp are applied in journal append order.
- What the draft calls an autonomous rule is a time-indexed definition. The
   evaluator asks that definition for the value at a logical time; it does not
   mutate a previously evaluated state or replay an interval from a cursor.
   For example, crop position at `t` is a pure indexed query; planting is an
   authoritative journal fact that the query can resolve at and after its
   timestamp.
- Reverse playback and arbitrary scrubbing are legal. Playback has no
   monotonicity requirement, and sampling a new `Tau` does not modify history.
- Exact deterministic equality is the equality relation. There is no epsilon
   or approximate-equality rule in the authoritative model.
- Context versioning and identity are deferred. They are not part of the
   canonical contract unless persistence, compatibility, or networking makes
   ## Mutation audit of the prototype

   The current code has two different kinds of mutation:

   - `Journal::append`, `Journal::prefix`, `Worldline::append`, and
     `Worldline::fork` mutate only newly allocated local construction values and
     return new immutable-looking values. They do not mutate the parent journal or
     worldline for ordinary owned data.
   - `evaluate` mutates a local cloned state, and the callback traits receive
     `&mut S`. This is the discarded integrator implementation, not the intended
     indexed evaluator.

   The persistent-looking API is not deeply immutable for arbitrary generic Rust
   types. A state, rule, or event can contain `Cell`, `RefCell`, `Mutex`, shared
   reference-counted state, I/O handles, clocks, or global access. `Clone` and
   shared references do not prevent those values from changing during evaluation
   or from being shared between a parent and a branch. The same limitation applies
   to the current `Playback`, `Renderer`, and `Animation` traits.

   Therefore the current tests establish parent preservation for the plain value
   fixture only. They do not establish the initial specification's stronger
   requirement that mutation and side effects are impossible. The canonical DSL
   must remove arbitrary callbacks from the authoritative and presentation data
   boundaries. Until that DSL exists, no optimization, cache, or additional
   snapshot test should be described as closing this mutation gap.

   ## Invariant coverage

   | Specification invariant | Current status |
   | --- | --- |
   | A state is determined by a worldline and logical time | The current evaluator has this shape for side-effect-free rule and event implementations; the purity condition is not enforced by the type boundary. |
   | Distinct logical times identify distinct states | `GameState` stores the requested time, but the evaluator does not itself prove payloads differ. |
   | Continuous logical-time domain with event discontinuities | The abstract domain is continuous; the runtime cutoff is proposed as exact microsecond ticks. Direct indexed definition semantics remain to be built. |
   | Playback may select past or future logical time | Reverse playback and arbitrary scrubbing are confirmed. The current affine implementation selects finite past/future targets; fixed-point arithmetic remains pending. |
   | Rendering is pure from `GameState` and `Tau` | `Renderer::render` and `present` use shared inputs and the presentation tests repeat frames and compare unchanged inputs; arbitrary renderer side effects are not structurally prevented. |
   | Presentation does not mutate authoritative inputs | `present` borrows the worldline, evaluates a new state, and passes that state by shared reference; parent preservation is demonstrated for ordinary value fixtures only. |
   | Animation is reconstructible from state, definitions, and `Tau` | The current boundary samples from those inputs and tests deterministic repetition; the final data-only presentation DSL remains to be built. |
   | Lookahead holds the journal fixed | The intended evaluator performs the same indexed query at a future time against the unchanged journal; the DSL query/index contract remains to be built. |
   | Counterfactuals are immutable forks | `Worldline::fork` constructs a value from an inclusive journal prefix plus validated alternate events, shares the immutable context, and leaves the parent unchanged for ordinary owned data. |

   ## User-review gates

   The following choices remain open. Current behavior must not be treated as their
   default resolution:

   1. Confirm or replace the proposed seconds-plus-microsecond-tick cutoff,
      including the exact overflow/error behavior and fixed-point playback rate.
   2. Choose the closed declarative or generated representation that makes
      authoritative transitions and presentation side-effect free by construction.
   3. Define the closed temporal DSL/index representation, including typed results
      for definitions that have no value at a requested time.
   4. Define how multiple indexed definitions compose into a `GameState` and how
      journal facts are visible to those definitions. Context versioning remains
      deferred unless a later product requirement activates it.
   5. Define the exact journal-writer and corrected-branch construction API while
      preserving the settled immutable-branch semantics.
   6. Review the minimum domain-neutral indexed reference fixture before the
      canonical engine contract is closed.

   Playback mapping and presentation-host decisions remain owned by
   `playback-presentation`; validated journal and branch operations remain owned by
   `journal-worldline`; the reference evaluator remains owned by
   `deterministic-kernel`. Those later tasks must consume reviewed decisions rather
   than infer them from the current implementation.

### Purity ruling and API status

The current API is:

```rust
trait AutonomousRule<S> {
      fn advance(&self, state: &mut S, from: LogicalTime, to: LogicalTime);
}

trait Event<S> {
      fn apply(&self, state: &mut S, at: LogicalTime);
}
```

The presentation boundary similarly calls `Renderer::render` and
`Animation::sample` on shared inputs. These signatures prevent the engine
from handing out mutable access to the `Worldline`, but they do not make
side-effects impossible: an implementation can still use interior mutability,
global state, I/O, or a clock. The current fixture therefore demonstrates
purity by behavior, not by type-level enforcement.

The desired canonical contract is stronger: temporal definitions, journal
events, and presentation must be data-only and side-effect free by
construction. Ordinary Rust has no purity or effect type that can make
arbitrary function bodies obey that rule. To satisfy the requirement literally,
the engine must expose a closed declarative temporal representation or another
restricted generated language, then interpret that data inside the engine.
Events should be values in that DSL, not executable Rust callbacks. Replacing
`&mut S` with an owned return value would improve the boundary but would not,
by itself, make external side-effects impossible. Choosing the closed
representation is an open design gate; the current callback traits are not the
final contract.

## Current boundary

| Specification term | Current Rust boundary | Implemented fact |
| --- | --- | --- |
| `LogicalTime` | `LogicalTime(f64)` | Current prototype type only. The proposed canonical runtime is a distinct fixed-point tick wrapper; the tick cutoff is not yet ratified. |
| `Tau` | `Tau(f64)` | Current prototype type only. The proposed canonical runtime uses the same tick resolution in a distinct wrapper. |
| `Context` | `Context<S, R>` | Current prototype owns an initial state, an explicit initial `LogicalTime`, and an ordered `Vec<R>`. Those fields belong to the discarded integrator shape; the canonical context should own closed temporal definitions and immutable configuration instead. |
| `Journal` | `Journal<E>` and private `JournalEntry<E>` | Owns an ordered vector. Fallible `append` and `append_all` return new values by cloning the vector and physically appending entries. They reject timestamps earlier than the current horizon. `horizon` returns `None` for an empty journal, and `prefix` extracts an inclusive prefix. |
| `Worldline` | `Worldline<S, R, E>` | Holds an `Arc<Context<S, R>>`, a `Journal<E>`, and an optional fork boundary. Fallible `append` returns a new value and shares the context. `fork` keeps the parent prefix through its boundary, rejects alternate events at or before that boundary, retains the boundary for later child appends, and excludes the parent suffix. There is no branch identity or mutable accessor. |
| `GameState` | `GameState<S>` | Stores the evaluated `LogicalTime` beside the payload. Its constructor is private; `evaluate` returns the requested target time. |
| `Playback` | `presentation::Playback` and `LinearPlayback` | The trait maps `Tau` to `LogicalTime`; the provisional implementation uses a signed affine rate. |
| `Frame` | `presentation::Renderer::Frame` | The renderer's associated frame is produced from only `&GameState` and `Tau`. |
| `Animation` | `presentation::Animation` | Sampling receives only `&GameState` and `Tau` and may return `None`. |

The current direct evaluator is still the prototype integrator: it clones the
context's initial state, starts a cursor at the context's initial `LogicalTime`,
selects journal entries with `entry.time() <= target`, and applies callbacks
across intervals. That implementation is not the canonical model described by
this ruling ledger. The replacement evaluator must resolve closed temporal
definitions directly at `target`; journal order remains relevant to equal-time
facts, but no mutable state is carried from an earlier query.

These are observations, not semantic approvals. In particular, the current
traits permit rules and events to mutate the local state, but Rust does not make
their implementations free of interior or external side effects. The evaluator
observes the confirmed insertion order for equal-time entries through the
journal order and stable time sorting.

## Journal/worldline policy

These choices are the smallest domain-neutral behavior needed by the current
branch API. The inclusion, shared-timestamp, and corrected-branch rules below
are confirmed.

1. Physical append means that normal journal writing places a new event at the
   tail. The canonical authoring API will have a journal-owned monotonic time
   cursor: `advance_to(t)` moves it forward and `record(event)` assigns the
   cursor's canonical timestamp to the event. Event payloads do not carry their
   own timestamp. Equal-time events are valid and retain append order. The
   current timestamp-taking `append` API is prototype scaffolding for this
   writer.
2. A normal writer cannot move its cursor backward. A late fact is not inserted
   into the actual journal. An explicit corrected branch is constructed from a
   prefix before the late fact, with a replacement suffix containing the late
   fact and any later facts that are still intended to exist. The parent actual
   journal remains unchanged. This is the same immutable branch family as a
   counterfactual; the difference is why the branch was created, not how it is
   evaluated. Network delivery, reconciliation, and merge are not required.
3. An empty journal has no horizon (`None`). A non-empty journal's horizon is
   its final event timestamp.
4. `prefix(fork_t)` is inclusive (`event.time() <= fork_t`). A fork therefore
   agrees with its parent at the fork boundary. Alternate events must have
   timestamps strictly greater than `fork_t` and must themselves be
   nondecreasing; invalid input returns `JournalError` and no child value.
   The parent suffix is not copied. A forked child retains `fork_t` as a lower
   bound for later `Worldline::append` calls: events at or before `fork_t`
   return `JournalError::EventAtOrBeforeFork`, even when the child journal has
   no alternate events or no event near the boundary. Actual worldlines created
   directly retain their normal journal-horizon append behavior.
5. A fork reuses the parent's immutable `Arc<Context>`. There is no branch
   identity, context version, merge, or networking behavior. A
   separately constructed `Worldline` is compatible when its Rust types match;
   the current generic boundary has no domain-neutral context identity to
   validate.
6. `LogicalTime::new` continues to reject non-finite values by assertion. A
   journal therefore receives only finite `LogicalTime` values; adding a
   separate fallible time-construction policy remains outside this slice.

## Lookahead and counterfactual policy

`evaluate_future(&worldline, future_time)` is the named lookahead entry point.
It is the same pure indexed lookup as `evaluate`, using the same immutable
worldline and a later query time. A target beyond the journal horizon asks the
temporal definitions for their values at that time; it does not invent an event
or record input. A `GameState` returned by lookahead is a value and is not
changed when a later journal value is constructed from the original worldline.

`fork_counterfactual(&worldline, fork_time, alternate_events)` is the named
branch-construction entry point and delegates to `Worldline::fork`. The
resulting immutable `Worldline` is the branch selection: callers pass that
value to `evaluate`, `evaluate_future`, or the existing presentation path.
There is no branch registry, branch identity, merge, or implicit future input.
The same operation can represent either a counterfactual or a
corrected branch; both use an inclusive prefix, an explicit replacement
suffix, and parent-suffix exclusion. Invalid input returns `JournalError`
without a child value.

## Playback and presentation policy

The `crates/presentation` boundary is now implemented. `Playback` exposes
the pure mapping `Tau -> LogicalTime`. `LinearPlayback::new(origin, rate)` uses

```text
logical_time = origin + rate * tau
```

with a finite `rate`. A positive rate plays forward, a negative rate plays in
reverse, and any finite `Tau` may select a past or future logical time. There
is no monotonicity requirement. Scrubbing is direct selection of another
`Tau`; it does not write to the journal or retain frame history.

Looping means wrapping playback around an explicitly chosen interval; bounds
mean rejecting or clamping a requested time range; neither is required by the
initial model. They should remain product-level controls rather than hidden
engine behavior. A zero rate is simply a constant playback mapping if a caller
wants a paused view. A target beyond the journal horizon is legal: indexed
temporal definitions answer at that time, but no unrecorded authoritative
event is invented.

`Renderer::render` and `Animation::sample` receive only the selected
`GameState` and `Tau` (plus immutable renderer/animation definitions held by
the implementation). `present` computes the selected time, calls
`engine_core::evaluate`, and passes the resulting state to the renderer. The
presentation crate contains no graphics API or game-specific content. Its
deterministic integration tests cover repeated frame and animation sampling,
past/future and reverse selection, and unchanged worldline/state inputs.

The exact fixed-point playback arithmetic remains coupled to the time-cutoff
ruling. The presentation host and graphics API remain deferred until
renderer-boundary evidence and a concrete visual workload exist.

## Mutation audit of the prototype

The current code has two different kinds of mutation:

- `Journal::append`, `Journal::prefix`, `Worldline::append`, and
   `Worldline::fork` mutate only newly allocated local construction values and
   return new immutable-looking values. They do not mutate the parent journal or
   worldline for ordinary owned data.
- `evaluate` mutates a local cloned state, and the callback traits receive
   `&mut S`. This is the discarded integrator implementation, not the intended
   indexed evaluator.

The persistent-looking API is not deeply immutable for arbitrary generic Rust
types. A state, rule, or event can contain `Cell`, `RefCell`, `Mutex`, shared
reference-counted state, I/O handles, clocks, or global access. `Clone` and
shared references do not prevent those values from changing during evaluation
or from being shared between a parent and a branch. The same limitation applies
to the current `Playback`, `Renderer`, and `Animation` traits.

Therefore the current tests establish parent preservation for the plain value
fixture only. They do not establish the initial specification's stronger
requirement that mutation and side effects are impossible. The canonical DSL
must remove arbitrary callbacks from the authoritative and presentation data
boundaries. Until that DSL exists, no optimization, cache, or additional
snapshot test should be described as closing this mutation gap.

## Invariant coverage

| Specification invariant | Current status |
| --- | --- |
| A state is determined by a worldline and logical time | The current evaluator has this shape for side-effect-free rule and event implementations; the purity condition is not enforced by the type boundary. |
| Distinct logical times identify distinct states | `GameState` stores the requested time, but the evaluator does not itself prove payloads differ. |
| Continuous logical-time domain with event discontinuities | The abstract domain is continuous; the runtime cutoff is proposed as exact microsecond ticks. Direct indexed definition semantics remain to be built. |
| Playback may select past or future logical time | Reverse playback and arbitrary scrubbing are confirmed. The current affine implementation selects finite past/future targets; fixed-point arithmetic remains pending. |
| Rendering is pure from `GameState` and `Tau` | `Renderer::render` and `present` use shared inputs and the presentation tests repeat frames and compare unchanged inputs; arbitrary renderer side effects are not structurally prevented. |
| Presentation does not mutate authoritative inputs | `present` borrows the worldline, evaluates a new state, and passes that state by shared reference; parent preservation is demonstrated for ordinary value fixtures only. |
| Animation is reconstructible from state, definitions, and `Tau` | The current boundary samples from those inputs and tests deterministic repetition; the final data-only presentation DSL remains to be built. |
| Lookahead holds the journal fixed | The intended evaluator performs the same indexed query at a future time against the unchanged journal; the DSL query/index contract remains to be built. |
| Counterfactuals are immutable forks | `Worldline::fork` constructs a value from an inclusive journal prefix plus validated alternate events, shares the immutable context, and leaves the parent unchanged for ordinary owned data. |

The following choices remain open. Current behavior must not be treated as their
default resolution:

1. Confirm or replace the proposed seconds-plus-microsecond-tick cutoff,
   including the exact overflow/error behavior and fixed-point playback rate.
2. Choose the closed declarative or generated representation that makes
   authoritative transitions and presentation side-effect free by construction.
3. Define the closed temporal DSL/index representation, including typed results
   for definitions that have no value at a requested time.
4. Define how multiple indexed definitions compose into a `GameState` and how
   journal facts are visible to those definitions. Context versioning remains
   deferred unless a later product requirement activates it.
5. Define the exact journal-writer and corrected-branch construction API while
   preserving the settled immutable-branch semantics.
6. Review the minimum domain-neutral indexed reference fixture before the canonical
   engine contract is closed.

Playback mapping and presentation-host decisions remain owned by
`playback-presentation`; validated journal and branch operations remain owned by
`journal-worldline`; the reference evaluator remains owned by
`deterministic-kernel`. Those later tasks must consume reviewed decisions rather
than infer them from the current implementation.

## Conformance scope

The inline tests in `lib.rs` exercise repeatable evaluation, equal-time
ordering, horizon and inclusive prefix behavior, prototype late-event
rejection, fork-boundary behavior, invalid alternate sequences, and
parent/child isolation. The canonical journal-writer and corrected-branch
tests are not implemented yet. The focused integration tests in
`crates/presentation/tests/presentation.rs` exercise playback,
state-plus-`Tau` rendering, reconstructible animation sampling, and unchanged
presentation inputs. These tests demonstrate the current boundary; they do
not close the playback or host review gates above.