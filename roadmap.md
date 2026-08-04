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

The indexed anchor packet is complete. `build.vine` is complete, and its
artifacts cover the demo, conformance report, persistence, benchmarks,
compiler-boundary tests, and independent frozen projection corpus. The public
query is direct, the retired tick-fold calculation is absent, and the reference
oracle selects immutable discontinuity pieces before doing query-local rule
calculation.

This is a deterministic reference/demo/evidence package, not yet a finished
game or distribution platform.

## Completed Anchor

The completed packet provides:

- typed logical and presentation time;
- immutable SDK envelopes, journals, branches, and direct indexed queries;
- the radius-5 Caravan domain and vegetation, hazard, and seeded fixtures;
- discontinuity indexing and piecewise reference projection;
- lookahead, playback, rendering, persistence, and deterministic replay; and
- executable conformance, benchmark, demo, parity, and purity evidence.

The packet-level delegation plan and its path ownership remain in
[build.vine](build.vine); this roadmap records the higher-level direction.

## Remaining Work

### Closed temporal definition language

Define closed data values for temporal definitions, identifiers, journal facts,
and typed out-of-domain results where a definition genuinely has no value.
Decide which compact journal entries expand into indexed domain elements and how
their deterministic identity is represented.

### Caravan vertical slice

Turn the fixed anchor into the smallest recognizable game loop: one indexed
quantity, one player event, journal writing, past/present/future lookup,
lookahead, one branch choice, and presentation. Its trace must remain
reproducible without hidden frame state.

### Broader domain composition

Extend the domain only when a concrete requirement identifies the additional
definitions, composition rules, and evidence needed beyond the current anchor.

## Deferred Until Activated

- Networking and synchronization
- Branch merging
- Looping and bounds
- Final graphics/GPU architecture
- Packaging, release targets, content tooling, audio, and device input

Activate one of these only when a concrete requirement supplies its target
regime, owner, contract, dependency, and reproducible acceptance evidence.
