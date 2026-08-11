# Stage Layer

This proposal defines the boundary between the canonical logical game
experience and the platform plumbing that hosts it. It introduces a
developer-authored **Orchestrator** as ordinary mutable control code inside the
**Stage** composition; it does not change the authoritative **worldline**,
**game state**, or rendering contract in [spec/initial.md](../spec/initial.md).

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
environment. The presentation host is plumbing, not a second game-facing
layer.

The **Stage** is canonical within presentation. It does not replace the domain
model or the reference oracle: those define game meaning and authoritative
state. Stage composes those values into the logical experience a user is
viewing.

## Vocabulary

This proposal uses **bold** for conceptual vocabulary and backticks for exact
Rust/API spellings. The owning concepts are:

> **Stage**: The canonical logical game experience for a selected view. It
> owns the selected worldline, logical and presentation times, Orchestrator,
> and presentation composition, but it is not a generic engine type yet.
>
> **Orchestrator**: Developer-authored ordinary mutable control code inside a
> Stage. It owns orchestration state and decisions, but it cannot own a second
> authoritative game-state model.
>

The target-specific entrypoint is an OS- or runtime-specific executable
composition root. `ApplicationHost` may name a target-local bundle of concrete
ports, but it is not a generic engine layer or a required abstraction. The
presentation-host vocabulary and port roles are owned by
[presentation-host.md](presentation-host.md).
ports, but it is not a generic engine layer or a required abstraction.

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
state, but it may not mutate a published `Worldline`, `Journal`, or
`GameState`. It may replace the selected worldline with a new immutable value
after journal publication or branch construction, while each query remains a
direct evaluation of immutable inputs.

## Orchestrator

The `Orchestrator` is where the developer writes ordinary game control code.
It may use a literal `while (true)` loop, a pull loop over independent
presentation-host ports, a replay driver, or another application-specific
control shape. The engine does not impose a universal loop API at this stage.

The Orchestrator owns decisions that are not yet reusable abstractions:

- which `Tau` to sample and whether presentation time advances;
- which `LogicalTime` and `Tau` values are selected for a sample;
- when to perform lookahead or select another branch;
- how input packets are retained and assembled;
- when to run `InteractionDefinition`;
- whether to accept a closed transformation result;
- when to publish a new journal/worldline value;
- which values to save; and
- which samples to present.

The Orchestrator may mutate its own control state. It may not own an imperative
board, actor set, resource counter, effect layer, or other parallel source of
authoritative game state. The only authoritative ingress remains publication of
new immutable journal/worldline values.

In the current Caravan prototype, these decisions are exercised through
application methods on `CaravanStage` and `CaravanOrchestrator`. In a future
target composition, the Orchestrator remains the caller: it pulls from the
independent presentation-host ports when it needs input, render submission,
storage transport, or lifecycle/resource information.

The pure interaction query and journal-publication path are owned by
[input-and-interaction.md](input-and-interaction.md). Stage composes that path
with the selected worldline and the Orchestrator's admission decisions;
`InteractionDefinition` remains unable to construct timestamped journal
entries directly.

The current Caravan prototype exposes these operations on different concrete
types:

```text
CaravanOrchestrator.sample()
CaravanOrchestrator.lookahead_at(logical_time)
    -> GameState
```

A future generic Stage may provide a direct `sample(logical_time)` convenience
operation; that is not part of the current application API.

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

## Presentation host responsibilities

The presentation host owns platform and execution concerns that do not define the
logical game experience. The detailed host boundary is maintained in the
[Presentation Host proposal](presentation-host.md).

The host must not decide which worldline or branch is canonical for a Stage,
which independent `LogicalTime` and `Tau` samples the Stage selects, or what a
Caravan domain value means.

The target-specific entrypoint composes concrete Stage dependencies and narrow
host ports at compile time. It remains plumbing around Stage rather than the
owner of the game experience.

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

These are boundary sketches, not an instruction to introduce these exact
structs or to make every helper a trait. The useful constraint is that a
concrete game composition is visible in types and invalid combinations are
rejected before runtime where practical. Target-entrypoint composition and
independent presentation-host ports are defined in
[presentation-host.md](presentation-host.md).

The Orchestrator invokes the engine's state operation for an already-selected
worldline and logical time. That operation owns indexed evaluation semantics;
the Orchestrator owns selection and control flow, not a second authoritative
state model.

`Renderer` belongs to Stage's logical presentation composition. The eventual
backend that turns renderer output into device or surface work may remain host
plumbing. The boundary keeps owned rendering objects downstream: game reasoning
never consumes them as an alternate state model. Game-facing persistence
similarly operates on semantic immutable values before a host storage port
transports encoded bytes.

## Reserved Levers

The following concerns are intentionally reserved and are not settled by this
proposal:

### Input

The Orchestrator requests abstract `InputPacket` values from the input ingress
port.
The Stage's Orchestrator owns the `InteractionDefinition` that reasons over an
`InputPacketSet`, as well as
the input orchestration that constructs that set. Packets may be delivered
directly or retained across calls. The canonical query takes the selected
read-only `GameState`, packet set, and `Tau`; its boundary is recorded in
[input-and-interaction.md](input-and-interaction.md).

### Camera and HUD

Camera and HUD are not currently assigned to Stage, Host, or a third view layer.
They remain reserved extension points until their state, time, and rendering
relationships are discussed explicitly.

### Rendering backend

Stage owns the logical renderer abstraction and composition. The current
generic boundary is `Renderer<S>::render(GameState<S>, Tau) -> Output` followed
by `Frame<Output>`. The planned rendering contract will name the concrete
owned rendering-object output and its division from host-owned device
execution; that output remains below game reasoning.

### Host time

Host clock is outside this model. No host-time or clock port is part of the
Stage, Orchestrator, or presentation-host contract.

## Non-Goals

This proposal does not:

- add `Stage` or `Orchestrator` types to the generic engine; the experimental
    `CaravanStage` and `CaravanOrchestrator` live in the application layer;
- change `spec/initial.md`;
- redefine `Worldline`, `LogicalTime`, `Tau`, `GameState`, or
  `Frame`;
- settle input commands or input timestamps;
- settle camera or HUD ownership;
- choose a GPU, windowing, asset, or device architecture; or
- introduce runtime dependency injection.

## Open Questions

1. Which Stage operations are view-local changes and which author journal facts
    or create branches?
2. Which simple persistence format should the first game-facing storage
    composition use?
3. Which repeated Stage, Orchestrator, and port-composition patterns are strong
    enough to extract after concrete traces exist?
