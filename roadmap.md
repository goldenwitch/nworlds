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
artifacts cover the reference game, conformance report, Caravan persistence,
benchmarks, compiler-boundary tests, and independent frozen projection corpus.
The public query is direct, the retired tick-fold calculation is absent, and
the reference oracle selects immutable discontinuity pieces before doing
query-local rule calculation. The first application-level Caravan
Orchestrator slice is also implemented and tested; it does not add a generic
engine Orchestrator API.

The reusable temporal library boundary is now isolated and evidenced by
`proposals/library-contract.md`, `library-boundary.vine`, the generic
`engine-api` consumer proof, the dependency-direction guard, and the
library-only build check. Caravan is the reference game/sample consumer, and
`nworlds-desktop` now supplies the generic target-host lifecycle with a
synthetic package proof; the historical Caravan desktop run remains client
evidence until migration. The repository remains a research implementation,
not a finished game, production host, released SDK, or distribution platform.

## Completed Anchor

The completed packet provides:

- typed logical and presentation time;
- immutable SDK envelopes, journals, branches, and direct indexed queries;
- the radius-5 Caravan domain and vegetation, hazard, and seeded fixtures;
- discontinuity indexing and piecewise reference projection;
- direct future queries, the generic state-first presentation boundary,
  Caravan-specific persistence, and deterministic replay; and
- executable conformance, benchmark, demo, parity, and purity evidence.

The packet-level delegation plan and its path ownership remain in
[build.vine](build.vine); this roadmap records the higher-level direction.

The Orchestrator experiment provides a mutable control layer over immutable
worldline, journal, and game-state values. Its input, transformation,
publication, Stage, persistence, and presentation evidence is in
[orchestrator.vine](orchestrator.vine).

The concrete Caravan rendering-object projection, first presentation-host proof,
and Windows `winit`/`wgpu` runtime evidence are implemented and manually
verified. That proof is host-internal evidence, not the public developer
workflow or a production distribution platform.

Target breadth is now canonized in
[proposals/platform-support-matrix.md](proposals/platform-support-matrix.md).
The committed desktop support gaps and their native or appropriate CI plan are
owned by [support.vine](support.vine); runtime/device evidence is required
before a target becomes supported.

The desired public developer path is defined by
[proposals/target-factory.md](proposals/target-factory.md):

```text
nworlds test
nworlds run
nworlds package
```

Target resolution, host capabilities, artifact minting, and unsupported-
environment reporting belong to nworlds, not to each game package.
The design work is ordered in [target-factory.vine](target-factory.vine); the
desktop proof client is not the target-factory implementation.

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

### Target support

Close the committed Linux, SteamOS/Steam Deck, and macOS gaps through the
target-specific host and CI tasks in [support.vine](support.vine). Keep Web,
mobile, other architectures, and consoles explicitly out of scope until a
concrete requirement activates them.

### Demo gameplay

The remaining player-facing work is planned from the existing design corpus in
[proposals/demo-gameplay.md](proposals/demo-gameplay.md) and
[gameplay.vine](gameplay.vine). The engine and Caravan rules already provide
actors, effects, resources, time, branches, persistence, and deterministic
fixtures; the planning gap is selecting one coherent loop that makes those
capabilities observable without creating a second authoritative state model.

## Deferred Until Activated

- Networking and synchronization
- Branch merging
- Looping and bounds
- Production host hardening beyond the first local Windows `winit`/`wgpu` slice
- Final graphics/GPU architecture beyond the selected first target
- Packaging, release targets, content tooling, and audio

Activate one of these only when a concrete requirement supplies its target
regime, owner, contract, dependency, and reproducible acceptance evidence.
