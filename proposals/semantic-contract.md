# Semantic Contract

This proposal maps [spec/initial.md](../spec/initial.md) to the current Rust
boundary at [crates/engine-core/src/lib.rs](../crates/engine-core/src/lib.rs).
It records observed implementation facts and the provisional journal/worldline
decisions below; it does not add rendering, gameplay, or a second evaluator.

## Current boundary

| Specification term | Current Rust boundary | Implemented fact |
| --- | --- | --- |
| `LogicalTime` | `LogicalTime(f64)` | A distinct, finite-valued type. `new` rejects non-finite values and normalizes negative zero. Ordering uses `total_cmp`; there is no arithmetic API or approved precision/range policy. |
| `Tau` | `Tau(f64)` | A distinct type with the same local representation rules as `LogicalTime`. No playback mapping exists. |
| `Context` | `Context<S, R>` | Owns one initial state and an ordered `Vec<R>`. Equality is derived structural equality; there is no identity or version field. |
| `Journal` | `Journal<E>` and private `JournalEntry<E>` | Owns an ordered vector. Fallible `append` and `append_all` return new values by cloning the vector and physically appending entries. They reject timestamps earlier than the current horizon. `horizon` returns `None` for an empty journal, and `prefix` extracts an inclusive prefix. |
| `Worldline` | `Worldline<S, R, E>` | Holds an `Arc<Context<S, R>>` and a `Journal<E>`. Fallible `append` returns a new value and shares the context. `fork` keeps the parent prefix through its boundary, rejects alternate events at or before that boundary, and excludes the parent suffix. There is no branch identity or mutable accessor. |
| `GameState` | `GameState<S>` | Stores the evaluated `LogicalTime` beside the payload. Its constructor is private; `evaluate` returns the requested target time. |
| `Playback` | `presentation::Playback` and `LinearPlayback` | The trait maps `Tau` to `LogicalTime`; the provisional implementation uses a signed affine rate. |
| `Frame` | `presentation::Renderer::Frame` | The renderer's associated frame is produced from only `&GameState` and `Tau`. |
| `Animation` | `presentation::Animation` | Sampling receives only `&GameState` and `Tau` and may return `None`. |

The direct evaluator is `evaluate(worldline, target)`. It clones the context's
initial state, selects journal entries with `entry.time() <= target`, sorts the
selected entries by time, advances every rule to each event, applies the event,
then advances to `target`. It reads the `Worldline` through shared references
and returns a new `GameState`. Journal validation makes physical order
nondecreasing, while the evaluator's stable sort preserves insertion order for
equal-time entries. The current tests cover repeatability, state-owned logical
time, unchanged inputs, equal results for equal inputs, and equal-time ordering.

These are observations, not semantic approvals. In particular, the current
traits permit rules and events to mutate the local state, but Rust does not make
their implementations free of interior or external side effects. The evaluator
also currently observes insertion order for equal-time entries through the
journal order and stable time sorting; that behavior remains a review gate.

## Provisional journal/worldline policy

These choices are the smallest domain-neutral behavior needed by the current
branch API. They remain reviewable at the semantic-contract gate:

1. Append-only means physical append. An event must have a timestamp greater
   than or equal to the journal horizon; a late event is rejected with
   `JournalError::LateEvent` rather than inserted, corrected, or made into a
   new branch. Equal-time events are valid and retain append order.
2. An empty journal has no horizon (`None`). A non-empty journal's horizon is
   its final event timestamp.
3. `prefix(fork_t)` is inclusive (`event.time() <= fork_t`). A fork therefore
   agrees with its parent at the fork boundary. Alternate events must have
   timestamps strictly greater than `fork_t` and must themselves be
   nondecreasing; invalid input returns `JournalError` and no child value.
   The parent suffix is not copied.
4. A fork reuses the parent's immutable `Arc<Context>`. There is no branch
   identity, context version, merge, correction, or networking behavior. A
   separately constructed `Worldline` is compatible when its Rust types match;
   the current generic boundary has no domain-neutral context identity to
   validate.
5. `LogicalTime::new` continues to reject non-finite values by assertion. A
   journal therefore receives only finite `LogicalTime` values; adding a
   separate fallible time-construction policy remains outside this slice.

## Provisional lookahead and counterfactual policy

`evaluate_future(&worldline, future_time)` is the named lookahead entry point.
It delegates to the existing pure `evaluate` function with the same immutable
worldline, so a target beyond the journal horizon advances autonomous rules
without inventing an event or recording input. Any finite target remains
valid, including a target before the current horizon; no horizon clamp or
future-input provider is introduced. A `GameState` returned by lookahead is a
value and is not changed when a later journal value is constructed from the
original worldline.

`fork_counterfactual(&worldline, fork_time, alternate_events)` is the named
branch-construction entry point and delegates to `Worldline::fork`. The
resulting immutable `Worldline` is the branch selection: callers pass that
value to `evaluate`, `evaluate_future`, or the existing presentation path.
There is no branch registry, branch identity, merge, correction, or implicit
future input. The existing inclusive prefix, strictly-later alternate-event,
physical-append, and parent-suffix exclusion rules remain the provisional
fork policy; invalid input returns `JournalError` without a child value.

## Provisional playback and presentation policy

The `crates/presentation` boundary is now implemented, but its playback policy
is provisional and does not resolve the open semantic gate. `Playback` exposes
the pure mapping `Tau -> LogicalTime`. `LinearPlayback::new(origin, rate)` uses

```text
logical_time = origin + rate * tau
```

with a finite `rate`. A positive rate plays forward, a negative rate plays in
reverse, and any finite `Tau` may select a past or future logical time. There is
no journal-horizon clamp, pause policy, looping policy, or bounds policy. An
affine result that is not finite is rejected by the existing `LogicalTime`
constructor. Scrubbing is direct selection of another `Tau`; it does not write
to the journal or retain frame history.

`Renderer::render` and `Animation::sample` receive only the selected
`GameState` and `Tau` (plus immutable renderer/animation definitions held by
the implementation). `present` computes the selected time, calls
`engine_core::evaluate`, and passes the resulting state to the renderer. The
presentation crate contains no graphics API or game-specific content. Its
deterministic integration tests cover repeated frame and animation sampling,
past/future and reverse selection, and unchanged worldline/state inputs.

The playback mapping, rate, bounds, extrapolation, and journal-horizon
behavior remain review gates. The presentation host and graphics API remain
deferred until renderer-boundary evidence and a concrete visual workload exist.

## Invariant coverage

| Specification invariant | Current status |
| --- | --- |
| A state is determined by a worldline and logical time | The current evaluator has this shape for side-effect-free rule and event implementations; the purity condition is not enforced by the type boundary. |
| Distinct logical times identify distinct states | `GameState` stores the requested time, but the evaluator does not itself prove payloads differ. |
| Continuous logical-time domain with event discontinuities | Finite `f64` values and rule advancement are present; domain, arithmetic, and event-boundary policy are unresolved. |
| Playback may select past or future logical time | The provisional affine playback selects any finite mapped target, including before and beyond the journal horizon; policy remains reviewable. |
| Rendering is pure from `GameState` and `Tau` | `Renderer::render` and `present` use shared inputs and the presentation tests repeat frames and compare unchanged inputs. |
| Presentation does not mutate authoritative inputs | `present` borrows the worldline, evaluates a new state, and passes that state by shared reference; the presentation tests compare worldline and state snapshots before and after sampling. |
| Animation is reconstructible from state, definitions, and `Tau` | `Animation::sample` is a pure boundary over shared state, `Tau`, and immutable implementation definitions; deterministic sampling is covered in presentation tests. |
| Lookahead holds the journal fixed | Direct evaluation can advance beyond the last journal entry, but horizon and autonomous-rule policy are not yet contracted. |
| Counterfactuals are immutable forks | `Worldline::fork` constructs a value from an inclusive journal prefix plus validated alternate events, shares the immutable context, and leaves the parent unchanged. Branch identity and merge semantics remain deferred. |

## User-review gates

The following choices remain open. Current behavior must not be treated as their
default resolution:

1. Choose `LogicalTime` representation, arithmetic, precision, allowed range,
   and the corresponding `Tau` relationship.
2. Define event inclusion at a target time, ordering for shared timestamps, and
   autonomous evolution before, between, and after journal events.
3. Define `Context` identity, version compatibility, canonical rule ordering,
   and deterministic equality.
4. Review the provisional journal horizon, append, prefix, fork, and
   invalid-input behavior recorded above, including late events.
5. Review the minimum domain-neutral reference fixture before kernel work
   proceeds.

Playback mapping and presentation-host decisions remain owned by
`playback-presentation`; validated journal and branch operations remain owned by
`journal-worldline`; the reference evaluator remains owned by
`deterministic-kernel`. Those later tasks must consume reviewed decisions rather
than infer them from the current implementation.

## Conformance scope

The inline tests in `lib.rs` exercise repeatable evaluation, equal-time
ordering, horizon and inclusive prefix behavior, late-event rejection,
fork-boundary behavior, invalid alternate sequences, and parent/child
isolation. The focused integration tests in
`crates/presentation/tests/presentation.rs` exercise provisional playback,
state-plus-`Tau` rendering, reconstructible animation sampling, and unchanged
presentation inputs. These tests demonstrate the current boundary; they do
not close the playback or host review gates above.