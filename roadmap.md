# Caravan of Seasons Roadmap

The abstract model is defined in [spec/initial.md](spec/initial.md). The
implementation graph is [build.vine](build.vine); semantic decisions belong in
[proposals/semantic-contract.md](proposals/semantic-contract.md).

## Target

The engine directly answers:

```text
state(worldline, t_) -> game_state
```

A temporal definition is indexed at `t_`; the engine does not evolve a prior
state or depend on frame history. Presentation composes the query with a
reusable playback function:

```text
present(worldline, playback, tau) =
    render(state(worldline, playback(tau)), tau)
```

## Settled Constraints

- `LogicalTime` and `Tau` are distinct types backed by signed `i64` fixed-point
  milliseconds. Tick scale and checked overflow behavior are centralized; one
  automaton game tick remains one logical second.
- Target-time events are included; equal-time events use journal append order.
- Journal writing owns timestamps through a monotonic cursor:
  `advance_to(t_)`, then `record(event)`.
- A late fact creates an explicit corrected branch from a prefix. The actual
  journal is never rewritten.
- Actual, counterfactual, and corrected branches are immutable values.
- Engine SDK objects remain separate from game-domain objects; `GameState` and
  journal entries carry game-specific values without defining them as engine
  primitives.
- Reverse playback and arbitrary scrubbing are legal.
- Networking, reconciliation, merging, looping, bounds, and final graphics
  architecture are deferred until a concrete requirement activates them.

## Current Position

The indexed anchor implementation exists and is covered by the demo,
conformance report, persistence, benchmarks, and compiler-boundary tests. Its
public query is direct, but the reference oracle still contains a private
tick-fold calculation. The next step is to replace that calculation with an
explicit discontinuity index and piecewise projection.

The packet-level delegation plan is maintained in `build.vine`: each packet
owns a disjoint path set and can be assigned to one Luna agent. The graph makes
the parallel frontier explicit; this roadmap keeps the higher-level sequence.

## Stages

### 1. Contract and time

Canonicalize the `i64` time wrappers, tick scale, checked arithmetic,
out-of-domain results, and the way journal facts are visible to indexed
queries.

**Exit:** one contract, one time module, and a small domain-neutral fixture.

### 2. Temporal DSL and index

Define closed data values for temporal definitions, journal facts, identifiers,
and `GameState`. Implement the smallest direct query, for example:

```text
crop_position(crop_id, t_) -> Position | Absent | Error
```

**Exit:** repeated queries at arbitrary times are equal and do not consume or
mutate prior results.

### 3. Journal writer and branches

Replace caller-assigned timestamps with a journal-owned monotonic writer. Add
explicit corrected-branch construction from an inclusive prefix and replacement
suffix. Keep counterfactuals on the same branch machinery.

**Exit:** late facts cannot enter the actual journal; parent and branch values
remain independently evaluable.

### 4. Direct evaluator and presentation

Make `evaluate` and lookahead use the same direct indexed query. Move playback,
rendering, and animation onto the new values. Prove actual, counterfactual, and
corrected branches use one presentation path.

**Exit:** query order, scrub direction, repeated samples, and branch selection do
not change states or frames.

### 5. Discontinuity index and piecewise projection

Extract journal timestamps, game-tick boundaries, and actor/rule thresholds as
an immutable ordered discontinuity index. Project terrain, actors, effects, and
resources from the selected piece and requested `t_`; do not fold a current
board through ticks. Compare the new path with the existing oracle, then delete
the private fold after parity is proven.

See [discontinuity-projection.md](proposals/discontinuity-projection.md).

**Exit:** the reference oracle has no `WorkingState`/`transition` tick loop,
and all existing anchor, conformance, demo, persistence, benchmark, and
presentation evidence still passes.

### 6. Caravan vertical slice

Choose the smallest recognizable game loop that exercises one indexed quantity,
one player event, journal writing, past/present/future lookup, lookahead, one
branch choice, and presentation.

**Exit:** a fixed playable trace is reproducible without engine exceptions or
hidden frame state.

### 7. Proof, persistence, and performance

Build the skeptic-facing proof package: clause-to-test matrix, property tests,
deterministic replay, and a conformance report. Then add save/load and measure
query cost, journal length, branch count, scrub latency, and frame production.

**Exit:** every claim is executable evidence or an explicit limitation; caches
and indexes have measured identity/invalidation rules.

### 8. Compiler-checked purity hardening

This is deliberately last, after behavior, the game slice, proof, persistence,
and workload are known. Narrow the public boundary with closed DSL types,
private constructors, `#![forbid(unsafe_code)]`, and `trybuild` compile-fail
tests for forbidden mutation, callback injection, caller-assigned timestamps,
and mutation of published branches.

Rust cannot prove arbitrary function bodies are pure. The hardening stage makes
that question irrelevant at the authoritative boundary by admitting data-only
DSL values instead of arbitrary callbacks. A custom lint may supplement this,
but is not the guarantee.

**Exit:** prohibited operations are ill-typed and earlier conformance evidence
still passes.

## Gates

A stage advances when its decisions are recorded, its boundary has executable
evidence, its demo or trace is reproducible, and its outputs do not mutate an
earlier branch or depend on frame history. Compiler evidence is required only
at Stage 8; earlier stages document and test their limitations.
