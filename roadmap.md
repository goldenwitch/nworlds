# Caravan of Seasons Roadmap

The abstract model is defined in [spec/initial.md](spec/initial.md). The
implementation graph is [build.vine](build.vine); canonical behavior lives in
the specifications and active proposals that own each boundary.

## Target

The engine directly answers:

```text
state(worldline, t_) -> game_state
```

A temporal definition is indexed at `t_`; the engine does not evolve a prior
state or depend on frame history. Presentation composes the query with the
selected logical and presentation times:

```text
present(worldline, logical_time, tau) =
  render(state(worldline, logical_time), tau)
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
- Sampling backward and arbitrary logical times are legal.
- Networking, reconciliation, merging, looping, bounds, and final graphics
  architecture are deferred until a concrete requirement activates them.

## Current Position

The indexed anchor packet is complete. `build.vine` is complete, and its
artifacts cover the demo, conformance report, persistence, benchmarks,
compiler-boundary tests, and independent frozen projection corpus. The public
query is direct, the retired tick-fold calculation is absent, and the reference
oracle selects immutable discontinuity pieces before doing query-local rule
calculation. The first application-level Caravan Orchestrator slice is also
implemented and tested; it does not add a generic engine Orchestrator API.

This repository is a deterministic reference/demo/evidence package and a pure
demo/toy for the engine. It is not a finished game, production host, or
distribution platform.

## Completed Anchor

The completed packet provides:

- typed logical and presentation time;
- immutable SDK envelopes, journals, branches, and direct indexed queries;
- the radius-5 Caravan domain and vegetation, hazard, and seeded fixtures;
- discontinuity indexing and piecewise reference projection;
- lookahead, the generic state-first presentation boundary, game-facing
  persistence, and deterministic replay; and
- executable conformance, benchmark, demo, parity, and purity evidence.

The packet-level delegation plan and its path ownership remain in
[build.vine](build.vine); this roadmap records the higher-level direction.

The Orchestrator experiment provides a mutable control layer over immutable
worldline, journal, and game-state values. Its input, transformation,
publication, Stage, persistence, and presentation evidence is in
[orchestrator.vine](orchestrator.vine).

The concrete Caravan rendering-object projection, target presentation-host
ports, and selected Windows `winit`/`wgpu` entrypoint are implemented and
manually verified. This is a local proof-of-life slice, not yet a production
host or distribution platform.

## Remaining Work

### Broader domain composition

Extend the domain only when a concrete requirement identifies the additional
definitions, composition rules, and evidence needed beyond the current anchor.

The reusable transport and journal layer is implemented in
[proposals/transport-and-journal.md](proposals/transport-and-journal.md) and
[transport.vine](transport.vine). `OrderedInputBatch`, `InputBuffer`,
`InputWindow`, and `SemanticInputBatch` are the canonical transport-to-game
crossing; `InputPacketSet` remains only as a compatibility membership view
until deliberate cleanup.

Reusable DSL extraction and generic compact journal-entry expansion are
deferred until Orchestrator prototypes reveal repeated patterns worth making
canonical. They are not current implementation obligations.

## Deferred Until Activated

- Networking and synchronization
- Branch merging
- Looping and bounds
- Production host hardening beyond the first local Windows `winit`/`wgpu` slice
- Final graphics/GPU architecture beyond the selected first target
- Packaging, release targets, content tooling, and audio

Activate one of these only when a concrete requirement supplies its target
regime, owner, contract, dependency, and reproducible acceptance evidence.
