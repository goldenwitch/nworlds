# Semantic Contract

This document records implementation-facing decisions that cross the more
focused documents. The [initial specification](../spec/initial.md) owns generic
vocabulary and invariants; the [cellular automata anchor](cellular-automata-anchor.md)
owns the concrete Caravan fixture; and the
[discontinuity projection proposal](discontinuity-projection.md) owns the
breakpoint and piece contract.

## Settled

### Query and time

- `LogicalTime` and `Tau` are distinct signed `i64` fixed-point millisecond
  types. The anchor's game tick remains one logical second.
- `state(worldline, t_)` is a direct indexed query. It returns an owned
  `GameState` containing the exact sampled time and does not accept or mutate a
  prior state, current board, cursor, RNG, or frame history.
- Reverse playback, arbitrary scrubbing, and non-monotonic query order are
  valid. Presentation composes the same query with playback and rendering.

### Journals and branches

- `JournalWriter` owns game-facing timestamps. `advance_to(t_)` followed by
  `record(event)` may postdate entries; queries include entries at the target
  time and ignore later entries.
- A late fact creates an immutable corrected branch from a prefix. Actual,
  counterfactual, and corrected branches use the same query path and never
  mutate their parent.
- The game-facing `engine-api` exposes `JournalWriter` rather than caller-
  assigned timestamps. Explicit assigned-time SDK constructors remain named
  interoperability imports for low-level consumers.

### Projection and composition

- Discontinuities are immutable value breakpoints. The generic index preserves
  journal, game-tick, and domain-specific sources; Caravan owns its thresholds
  and rule meanings.
- A selected piece carries immutable visible journal inputs and a projection
  regime. Actor trajectories are indexed value functions, and every relevant
  game-tick boundary is covered. Reusable indexes report an explicit horizon
  error rather than silently dropping values.
- Anchor composition is value-level and ordered: authored terrain, Farmer
  wheat, Arborist forest conversion, Fire ignition and aging/spread, then Fire
  destruction. Actor proposals use pre-tick live occupancy; resources sample
  the resulting terrain and actor layers.

### API boundaries

- SDK envelopes remain distinct from Caravan objects. The authoritative API
  admits immutable data inputs and owned results; it does not expose interval
  transition callbacks.
- Seeded randomness constructs a concrete journal before evaluation. The
  evaluator never owns or advances an RNG.
- `QueryAdapter`, `Renderer`, and `Animation` are trusted presentation
  extensions. They receive immutable values and return owned values, but Rust
  cannot prove arbitrary extension bodies have no side effects.

## Current Implementation

The public query is `state(worldline, t_) -> GameState`. The Caravan reference
oracle implements it with an immutable discontinuity index and piecewise
projection. The retired interval-fold evaluator is test-only historical parity
code; production queries do not depend on it.

## Open

1. Define the closed temporal DSL for indexed definitions and typed
   out-of-domain results.
2. Decide which compact journal entries expand into indexed domain elements and
   how their deterministic identity is represented.
