# Semantic Contract

This document records the implementation-facing decisions for
[spec/initial.md](../spec/initial.md). The former Rust interval prototype has
been removed. No canonical Rust implementation exists yet.

## Settled

- `LogicalTime` and `Tau` are distinct types. Runtime time uses signed fixed
  point ticks; scale and checked overflow behavior belong in one time module.
- `t_` is continuously sampleable. The cellular automaton has a separate
  discrete game-tick index.
- `state(worldline, t_)` is a direct indexed query. It does not accept a prior
  state, mutate a current board, rewind a frame, or advance a cursor.
- A fixed worldline means the context and journal are immutable query inputs.
  Journal entries may still cause exact-time discontinuities at their recorded
  timestamps.
- The context supplies interpretation definitions and configuration, not the
  instance's populated geometry or actors.
- Journal entries populate the void. An empty journal may validly produce the
  empty set. A fixture may use a compact entry such as `CreateSaucer` to create
  a domain and its derived tiles.
- Journal timestamps are assigned by a monotonic writer. A late entry creates an
  explicit corrected branch; it never rewrites the actual journal.
- The writer may postdate entries by advancing its cursor into the future before
  evaluation. Queries ignore entries after their requested `t_`; evaluation
  never generates those entries.
- Actual, counterfactual, and corrected branches are immutable values evaluated
  through the same query path.
- Target-time entries are included, and equal-time entries use append order.
- Reverse playback and arbitrary scrubbing are legal.
- Seeded randomness constructs a concrete journal before evaluation. The
  evaluator never owns or advances an RNG.
- Engine SDK objects (`Worldline`, `Journal`, `GameState`, `Playback`, and
  related envelopes) remain distinct from game objects such as saucers, tiles,
  terrain, actors, effects, and resources.

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

## Historical Prototype

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

1. Choose the exact fixed-point tick scale and overflow error API.
2. Define the closed temporal DSL for indexed definitions and typed out-of-domain
   results where a definition genuinely has no value.
3. Define composition between terrain, actor, effects, and resource queries,
   including fire, movement, collision, and conversion dependencies.
4. Implement the journal writer and corrected-branch construction API.
5. Decide which compact journal entries expand into indexed domain elements and
   how their deterministic identity is represented.
6. Add the reference implementation and proof tests against the anchor proposal.

Compiler-checked purity hardening remains a later stage. Until then, the
canonical design is data-oriented and the absence of an implementation is
intentional, not a missing fallback to the old callback model.
