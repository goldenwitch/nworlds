# Caravan Orchestrator Anchor

This proposal records the remapping of the completed Caravan anchor onto the
Stage and Orchestrator boundaries. Its initial application prototype is now
implemented; the proposal remains the design boundary for future extraction
and refinement, not a replacement for the canonical anchor specifications.

The purpose is to exercise the new composition with ordinary developer-written
Rust while preserving the existing indexed-engine invariants. The first
Orchestrator is an experiment in orchestration shape; it is not a generic
engine abstraction or a claim that a universal game DSL has been found.

## Vocabulary Convention

Conceptual terms in this proposal are **bold** when defined or emphasized.
Exact Rust/API spellings remain in backticks. The Stage and host proposals own
the meanings of **Stage**, **Orchestrator**, and **presentation host**; this
document owns their Caravan composition. The initial specification owns the
formal meanings of **worldline**, **game state**, **Tau**, **LogicalTime**, and
**frame**.

## Existing Anchor

The current anchor already provides the authoritative machinery:

```text
ReferenceWorldline
    immutable context + immutable Journal branch

state(worldline, LogicalTime)
    -> GameState<Snapshot>

Renderer<Snapshot>::render(GameState<Snapshot>, Tau)
    -> renderer output
    -> Frame
```

The reference query, journal writer, branch constructors, presentation
adapter, game-facing persistence codec, and evidence remain the engine
boundary. Host byte transport remains a separate presentation-host concern.
This proposal composes them; it does not replace them with mutable game-owned
state. The low-level SDK already has generic journal envelopes; the current
`engine-journal` and `engine-branches` facades remain Caravan-bound and are the
surface this first experiment uses.

## Target Composition

The first concrete composition is:

```text
CaravanStage
    owns CaravanOrchestrator
    owns the Stage rendering composition

CaravanOrchestrator
    selected ReferenceWorldline
    JournalWriter / immutable publication path
    selected LogicalTime and Tau
    SemanticInputBatch orchestration
    Caravan InteractionDefinition
    query, branch, save, and presentation decisions
```

The initial Rust structs live in `caravan-demo`; their shape is intentionally
still application-owned. The first target-specific entrypoint and independent
presentation-host ports are implemented in the Windows `winit`/`wgpu`
composition recorded by [host.vine](../host.vine). No engine-wide
`Orchestrator` trait is required until multiple concrete orchestrators reveal a
stable variation boundary.

Rust's static composition remains preferred: the concrete Caravan query,
interaction definition, renderer, and persistence components are composed as
early as practical, with traits introduced only at genuine seams.

## Typed Journal Boundary

Journal heterogeneity is a closed game-owned discriminated union, not a
collection of runtime objects. The low-level SDK supplies generic journal
entry and journal envelopes for a payload type chosen by the game. The current
`engine-journal` and `engine-branches` facades bind that payload to Caravan's
`GameJournalEntry`:

```rust
struct JournalEntry<E> {
  logical_time: LogicalTime,
  payload: E,
}

struct Journal<E> {
  entries: Vec<JournalEntry<E>>,
}
```

Caravan supplies its own closed payload type, currently represented by
`GameJournalEntry`:

```rust
enum CaravanJournalEntry {
  CreateSaucer { radius: u8 },
  SpawnActor { /* domain values */ },
  SetTerrain { /* domain values */ },
}
```

The existing `GameJournalEntry` name remains the current implementation name;
`CaravanJournalEntry` describes its role, not a required rename. Developers
reason over heterogeneous journal facts with ordinary Rust `match`,
`filter_map`, and iterator expressions. The first Orchestrator does not use
`Box<dyn Trait>`, `Any`, or untyped payloads for journal facts.

The same rule applies to the application `Transformation`: it is a closed data
type whose variants may describe journal authoring, branch operations,
Stage-local changes, or rejection. A future
convenience filter or wrapper is syntactic sugar over these typed values, not a
second dynamic journal model.

The generic engine may later be generalized from the current Caravan-bound
journal plumbing to `Journal<E>` and `Worldline<C, E>`, but this proposal does
not require that refactor before the Orchestrator experiment. The semantic
boundary is already fixed: one statically known heterogeneous payload type per
concrete game.

## Three Immutable Seams

The Orchestrator is imperative control code around three value-producing
operations:

### State

```text
state(worldline, logical_time) -> game_state
```

The selected `ReferenceWorldline` is immutable. Each `GameState<Snapshot>` is a
new direct query result containing the requested `LogicalTime`. No prior state,
current board, actor object, resource counter, frame history, or Orchestrator
continuation is an input to the state query.

### Render

```text
render(game_state, tau) -> frame
```

The renderer receives the selected state and explicit `Tau`. It returns an
owned presentation result through the existing `Renderer` and `Frame` boundary.
It does not mutate the worldline, journal, game state, or prior frame. The first
Caravan render projection composes owned backend-neutral rendering objects and
does not decide camera, HUD, coordinates, assets, GPU, or device behavior.

### Interact

```text
interaction_query(
    interaction_definition,
    game_state,
  semantic_input_batch,
    tau,
) -> transformation
```

The application prototype's `InteractionDefinition` is the current pure seam
where the developer writes interaction reasoning. It receives abstract packets
and the selected read-only `GameState` together with `Tau`; the state's exact
logical time is already carried by `GameState`. It
does not receive host events, host time, a journal writer, or a mutable board.
It returns closed `Transformation` data rather than a timestamped journal entry
or an arbitrary mutation callback. These are application-level types, not
generic engine APIs.

The query is identical for samples in the past, present, or future. Whether the
packet set was delivered directly or retained by Orchestrator input
orchestration is invisible at this seam.

## Orchestrator Control State

The Orchestrator may mutate its own control state because it is the place where
we are learning the shape of a game loop. Its mutable state may include:

- the selected immutable `ReferenceWorldline` value;
- a private `JournalWriter` or equivalent unpublished authoring mechanism;
- the currently selected `LogicalTime` and `Tau`;
- retained input packets and packet-set construction state;
- branch, lookahead, save, and presentation choices; and
- ordinary developer control variables needed to coordinate the loop.

These are orchestration values, not a second game-state model. The Orchestrator
must not own or mutate an imperative board, actor set, terrain layer, effect
layer, resource counter, or current `GameState`.

The Orchestrator owns semantic control flow and may run a literal loop or pull
loop over independent presentation-host ports when composed with a target
entrypoint. It requests platform input, storage transport, or backend work
when needed. It decides which `Tau` to sample, whether
presentation time advances, whether to query, which branch to view, which
values to save, and which frame to present. The host does not become the owner
of logical time or journal semantics.

A literal `while (true)` loop, a replay driver, or a test harness are all valid
first orchestration shapes. The engine does not impose a static clock or a
universal loop frequency at this stage.

## Transformation and Journal Publication

An interaction result changes authoritative game state only through journal or
branch publication:

```text
GameState + SemanticInputBatch + Tau
    -> InteractionDefinition
    -> Transformation
    -> Orchestrator accepts or rejects
    -> GameJournalEntry payload, if authoritative
    -> JournalWriter
    -> immutable Journal snapshot
    -> new immutable ReferenceWorldline
```

`JournalWriter` remains the timestamp authority for game-facing authoring. The
low-level SDK retains explicitly named assigned-time interoperability
constructors, but the Orchestrator uses `JournalWriter`. It may apply an
accepted transformation during the current control step, but that means
publishing a new immutable journal/worldline value; it does not mutate the
previously selected value in place.

The writer's current authoring cursor supplies the ordinary append timestamp.
No `InputTime` or packet timestamp is introduced by this proposal. If a
transformation concerns a past sample and must alter history at that point, the
Orchestrator uses the existing corrected-branch machinery to retain the
inclusive prefix and replace or append a suffix strictly after the fork
boundary; it does not silently insert into the actual journal. Counterfactual
and corrected results remain new immutable branch values.

The publication rule is:

> Journal publication is the only authoritative ingress for a change in the
> result of `state(worldline, logical_time)`.

A rejected transformation, a Stage-local decision, a discarded lookahead, or a
render choice does not change authoritative game state.

## First Caravan Flow

A first implementation may follow this developer-authored control flow:

```text
Orchestrator control-flow iteration
  -> receive abstract input packets through the Stage boundary
  -> normalize transport observations into a SemanticInputBatch
    -> choose LogicalTime, Tau, and the viewed ReferenceWorldline
    -> query ReferenceWorldline -> GameState<Snapshot>
    -> run InteractionDefinition
    -> accept/reject/apply Transformation
    -> publish a new Journal/ReferenceWorldline when required
    -> choose values to save
    -> render selected GameState<Snapshot> at selected Tau
    -> present or discard the Frame
```

The Orchestrator may choose a different order when the game requires it. The
three seams and journal-only authoritative ingress do not change with that
choice.

## Acceptance Evidence

The remapping is successful when a concrete Caravan Orchestrator demonstrates:

- it owns a selected `ReferenceWorldline` without mutating a published parent;
- its journal facts and transformations use closed, statically typed game
  variants rather than runtime object collections;
- it selects `LogicalTime` and `Tau` explicitly;
- it queries arbitrary past, present, and future times through the existing
  direct reference oracle;
- the application `InteractionDefinition` seam returns closed
  `Transformation` data from the selected read-only `GameState`,
  `SemanticInputBatch`, and `Tau`;
- accepted authoritative transformations publish new immutable journal or
  branch values;
- rejected transformations do not alter the selected worldline;
- journal timestamps remain owned by `JournalWriter`;
- state results contain no imperative Orchestrator state;
- render output is derived only from the selected `GameState` and `Tau`;
- persistence and presentation choices are Orchestrator decisions without
  changing query semantics; and
- the existing anchor, branch, lookahead, presentation, persistence,
  conformance, purity, and demo evidence remains green.

## Non-Goals

This proposal does not:

- add the Orchestrator or Stage types to the engine yet;
- define a universal Orchestrator trait or loop API;
- introduce imperative game-owned state;
- add `InputTime` or packet timestamps;
- generalize the initial application `InputPacket` or `Transformation` enums
  into engine-wide APIs;
- choose camera, HUD, coordinate projection, assets, GPU, windowing, or device
  architecture; or
- extract a reusable DSL before concrete Orchestrator behavior exists.

## Open Questions

1. Which additional Caravan transformations should be added after the initial
  `SetTerrain`/`Noop` prototype without inventing unnecessary domain rules?
2. Which Orchestrator control state must survive persistence or replay, and
   which is disposable execution state?
3. Which additional target profiles or production host policies should follow
  the completed first composition?
4. Which repeated Orchestrator patterns are strong enough to extract after the
   first working traces?
