# Semantic Contract

This document records the implementation-facing decisions for
[spec/initial.md](../spec/initial.md). The indexed anchor implementation and
reference projection path now exist. The remaining work is the later closed
temporal definition language and broader domain-composition decisions.

## Settled

- `LogicalTime` and `Tau` are distinct types. Runtime time uses signed fixed
  point milliseconds: 1,000 representable ticks per logical second. Scale and
  checked overflow behavior belong in one time module. The automaton game tick
  remains exactly one logical second.
- `t_` is continuously sampleable. The cellular automaton has a separate
  discrete game-tick index.
- `state(worldline, t_)` is a direct indexed query. It does not accept a prior
  state, mutate a current board, rewind a frame, or advance a cursor.
- A fixed worldline means the context and journal are immutable query inputs.
  Journal entries may still cause exact-time discontinuities at their recorded
  timestamps.
- The context supplies interpretation definitions and configuration, not the
  instance's populated geometry or actors.
- `ReferenceContext` carries the anchor radius definition only. A visible
  `CreateSaucer` journal entry adds the instance saucer; an empty journal is
  evaluated through the same zero-fact/default path and returns the ordinary
  empty set.
- Journal entries populate the default empty result. Empty-journal evaluation
  uses the same evaluation path as every other visible-entry prefix.
- Journal timestamps are assigned by a monotonic writer. A late entry creates an
  explicit corrected branch; it never rewrites the actual journal.
- The writer may postdate entries by advancing its cursor into the future before
  evaluation. Queries ignore entries after their requested `t_`; evaluation
  never generates those entries.
- Actual, counterfactual, and corrected branches are immutable values evaluated
  through the same query path.
- Target-time entries are included, and equal-time entries use append order.
- At one activation tick on one tile, authored `SetTerrain` entries are
  ordered by journal append order before derived terrain events. Derived
  events use the fixed rule order and stable actor/tile order already defined
  by the anchor.
- Forester movement uses one shared pre-tick actor occupancy map. Foresters
  propose simultaneously, lower actor identifiers win destination conflicts,
  and the resulting positions become the next tick's occupancy inputs.
- Forester wood totals are sampled from those same collective actor positions
  and the terrain value at each indexed tick; resources do not use a separate
  static occupancy calculation.
- Reverse playback and arbitrary scrubbing are legal.
- Seeded randomness constructs a concrete journal before evaluation. The
  evaluator never owns or advances an RNG.
- Engine SDK objects (`Worldline`, `Journal`, `GameState`, `Playback`, and
  related envelopes) remain distinct from game objects such as saucers, tiles,
  terrain, actors, effects, and resources.
- A discontinuity is a value-level breakpoint in an indexed result. It is not an
  imperative action and does not authorize mutation of a current state.
- Piecewise projection selects immutable visible-entry inputs and a projection
  regime from a discontinuity index. Rule results remain disposable
  query-local calculations over those selected inputs and the requested `t_`.

## Cellular Automata Anchor

The concrete fixture is defined in
[cellular-automata-anchor.md](cellular-automata-anchor.md):

- radius-5 axial saucer with 91 tiles;
- terrain, actor, and effects layers;
- empty journal as the empty set;
- `CreateSaucer` as a journal entry that establishes the tiles;
- continuous `t_` sampling decoupled from discrete game ticks;
- deterministic journal entries and indexed actor/resource results.

## Query Shape

The intended boundary is:

```text
state(context_definitions, journal, t_) -> GameState
```

A returned `GameState` owns the exact sampled `t_` and contains a game-specific
snapshot of derived layers and resources for the corresponding automaton tick
and visible journal entries. It is an SDK result envelope, not a replacement
for the game object's domain model. All intermediate values are disposable
query results. No authoritative value is updated in place.

Presentation remains:

```text
present(worldline, playback, tau) =
    render(state(worldline, playback(tau)), tau)
```

## Current implementation boundary

The public query is direct:

```text
state(worldline, t_) -> GameState
```

The Caravan reference oracle satisfies that boundary through an immutable
discontinuity index and piecewise projection. It may allocate disposable local
calculation values while answering one query, but it must not accept a previous
`GameState`, current board, actor object, cursor, or frame history.

## Historical callback prototype

The removed prototype used:

```rust
trait AutonomousRule<S> {
    fn advance(&self, state: &mut S, from: LogicalTime, to: LogicalTime);
}

trait Event<S> {
    fn apply(&self, state: &mut S, at: LogicalTime);
}
```

That API modeled interval folding and permitted hidden side effects through
arbitrary Rust callbacks. It is retained only in Git history as contrast and
is not an implementation target.

## Open

1. Define the closed temporal DSL for indexed definitions and typed out-of-domain
  results where a definition genuinely has no value.
2. Define composition between terrain, actor, effects, and resource queries,
  including fire, movement, collision, and conversion dependencies.
3. Decide which compact journal entries expand into indexed domain elements and
  how their deterministic identity is represented.

Compiler-checked purity hardening remains a later stage. The discontinuity
projection work is a semantic implementation refactor, not a purity-linting
exercise.
