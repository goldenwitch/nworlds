# Stage Layer

This proposal defines the boundary between the canonical logical game
experience and the platform plumbing that hosts it. It introduces a
developer-authored **Orchestrator** as ordinary mutable control code inside the
**Stage** composition; it works alongside the authoritative **worldline**,
**game state**, and rendering contract in [spec/initial.md](../spec/initial.md).

## Boundary

The presentation system separates the game-facing description from the
plumbing that satisfies it:

```text
Target-specific entrypoint
    composes Stage with independent presentation-host ports
    (an ApplicationHost bundle is optional target-local convenience)

    is composed with and called by

Stage
    canonical logical game experience
```

**Stage** defines what the game experience is for the selected view. The
target-specific entrypoint supplies the independent presentation-host ports
needed by a particular operating system, device, window, surface, or rendering
environment. The presentation host supplies environmental plumbing while Stage
supplies the game-facing experience.

The **Stage** is canonical within presentation. The domain model and reference
oracle define game meaning and authoritative state; Stage composes those values
into the logical experience a user is viewing.

## Vocabulary

This proposal uses **bold** for conceptual vocabulary and backticks for exact
Rust/API spellings. The owning concepts are:

> **Stage**: The canonical logical game experience for a selected view. It
> owns the selected worldline, logical and presentation times, Orchestrator,
> and presentation composition. The current Stage is an application-layer
> composition.
>
> **Orchestrator**: Developer-authored ordinary mutable control code inside a
> Stage. It owns orchestration state and decisions while the selected
> worldline remains the authoritative game-state source.
>

Target entrypoints and host port roles are defined by the
[presentation-host proposal](presentation-host.md). This document defines
Stage ownership after those ports are composed around it.

## Stage Responsibilities

Stage owns the selected game view and its temporal policy:

- the immutable `Worldline` being viewed;
- the selected actual, counterfactual, or corrected branch;
- the selected `LogicalTime` and `Tau` values;
- sample policy and explicit temporal queries;
- lookahead and future-state views through its Orchestrator;
- branch selection and branch-view operations through its Orchestrator;
- the developer-authored `Orchestrator`, including abstract interaction
    definitions and input orchestration; and
- the logical query, rendering, and game-facing persistence composition over
    those values.

Stage ownership means that these values and policies belong to the game-facing
composition. The `Orchestrator` is the single mutable owner of Stage control
state. Authoritative history remains an immutable `Worldline`, `Journal`, or
`GameState`; publication and branch construction produce replacement values,
and each query evaluates immutable inputs directly.

## Orchestrator

The `Orchestrator` is where the developer writes ordinary game control code.
It may use a literal `while (true)` loop, a pull loop over independent
presentation-host ports, a replay driver, or another application-specific
control shape. The engine supplies temporal and presentation primitives while
the application chooses its loop shape.

The Orchestrator owns these application-specific decisions:

- which `Tau` to sample and whether presentation time advances;
- which `LogicalTime` and `Tau` values are selected for a sample;
- when to perform lookahead or select another branch;
- how input packets are retained and assembled;
- when to run `InteractionDefinition`;
- whether to accept a closed transformation result;
- when to publish a new journal/worldline value;
- which values to save; and
- which samples to present.

The Orchestrator may mutate its own control state. Authoritative game state
comes from publication of new immutable journal/worldline values; boards,
actor sets, resource counters, and effect layers are derived through the game
query.

In the current Caravan prototype, these decisions are exercised through
application methods on `CaravanStage` and `CaravanOrchestrator`. The target
factory composes the target entrypoint around that Stage; the Orchestrator
pulls from the independent presentation-host ports when it needs input, render
submission, storage transport, or lifecycle/resource information.

The pure interaction query and journal-publication path are owned by
[input-and-interaction.md](input-and-interaction.md). Stage composes that path
with the selected worldline and the Orchestrator's admission decisions;
`InteractionDefinition` returns untimestamped transformations, while the
Orchestrator and journal machinery supply admission and timestamp authority.

The current Caravan prototype exposes these operations on different concrete
types:

```text
CaravanOrchestrator.sample()
CaravanOrchestrator.lookahead_at(logical_time)
    -> GameState
```

The current application API exposes sampling through the concrete Orchestrator;
a future generic Stage may provide a direct `sample(logical_time)` convenience
operation.

Rendering composes with the selected state through the existing presentation
boundary:

```text
Stage.present_at(logical_time, tau)
    -> GameState
    -> Renderer.render(game_state, tau)
    -> Frame
```

The Orchestrator chooses `Tau` samples and owns their meaning. Explicit
`LogicalTime` and `Tau` values remain valid for scrubbing, replay, testing, and
deterministic presentation.

## Presentation host boundary

The [presentation-host proposal](presentation-host.md) owns host port roles,
target entrypoints, and platform execution. Stage consumes those ports while
retaining worldline selection, time sampling, and domain meaning.

## Static Composition

The preferred implementation is Rust's static composition model:

- traits define genuine variation boundaries;
- generic parameters carry those abstractions through the pipeline; and
- the application composes concrete implementations as early as practical.

A conceptual shape is:

```rust
struct Stage<W, I, R> {
    orchestrator: Orchestrator<W, I>,
    renderer: R,
}

struct Orchestrator<W, I> {
    worldline: W,
    interaction: I,
    logical_time: LogicalTime,
    tau: Tau,
}
```

These sketches show the boundary shape; concrete games can choose the types
and helpers that express their own composition. The useful constraint is that a
concrete game composition is visible in types and invalid combinations are
rejected before runtime where practical. Target-entrypoint composition and
host ports are defined in [presentation-host.md](presentation-host.md).

The Orchestrator invokes the engine's state operation for an already-selected
worldline and logical time. The engine owns indexed evaluation semantics; the
Orchestrator owns selection and control flow around that operation.

`Renderer` belongs to Stage's logical presentation composition. The rendering
contract and host crossing are defined in
[rendering-contract.md](rendering-contract.md) and
[presentation-host.md](presentation-host.md). Game-facing persistence likewise
operates on immutable values before a host storage port transports bytes.

## Reserved Levers

The following concerns remain reserved extension points for later concrete
consumers:

### Input

The Orchestrator requests abstract `InputPacket` values from the input ingress
port.
The Stage's Orchestrator owns the `InteractionDefinition` that reasons over a
`SemanticInputBatch`, as well as the input orchestration that constructs that
batch. `InputPacketSet` remains a derived membership compatibility view.
Packets may be delivered directly or retained across calls. The canonical
query takes the selected read-only `GameState`, semantic batch, and `Tau`; its boundary is recorded in
[input-and-interaction.md](input-and-interaction.md).

### Camera and HUD

Camera and HUD remain reserved extension points until their state, time, and
rendering relationships are discussed explicitly.

### Rendering backend

Stage owns the logical renderer abstraction and composition. The current
generic boundary is `Renderer<S>::render(GameState<S>, Tau) -> Output` followed
by `Frame<Output>`. The completed rendering contract defines the concrete
owned rendering-object output and its division from host-owned device
execution. Gameplay-specific presentation adds minimal fire-and-forget render
data projected from `GameState`; client view state enters as an explicit
presentation value.

### Host time

Stage and Orchestrator use `LogicalTime` and `Tau`. Host scheduling remains a
presentation-host concern and supplies redraw opportunities around those
explicit values.

## Current Boundaries

This proposal currently establishes:

- `CaravanStage` and `CaravanOrchestrator` as application-layer compositions;
- the existing `Worldline`, `LogicalTime`, `Tau`, `GameState`, and `Frame`
    contracts from `spec/initial.md`;
- input ownership through the semantic input proposal;
- camera and HUD as reserved presentation extension points;
- target execution through the presentation-host boundary; and
- static Rust composition at genuine variation boundaries.

## Open Questions

1. Which Stage operations are view-local changes and which author journal facts
    or create branches?
2. Which simple persistence format should the first game-facing storage
    composition use?
3. Which repeated Stage, Orchestrator, and port-composition patterns are strong
    enough to extract after concrete traces exist?
