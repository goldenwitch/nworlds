# Stage Layer

This proposal defines the boundary between the canonical logical game
experience and the platform plumbing that hosts it. It introduces a
developer-authored `Orchestrator` as ordinary mutable control code inside the
Stage composition; it does not change the authoritative `Worldline`,
`GameState`, or rendering contract in [spec/initial.md](../spec/initial.md).

## Boundary

The presentation system has two conceptual layers:

```text
PresentationHost
    platform and execution plumbing

    composes

Stage
    canonical logical game experience
```

`Stage` defines what the game experience is for the selected view. The
`PresentationHost` makes that experience executable in a particular operating
system, device, window, surface, or rendering environment.

The Stage is canonical within presentation. It does not replace the domain
model or the reference oracle: those define game meaning and authoritative
state. Stage composes those values into the logical experience a user is
viewing.

## Stage Responsibilities

Stage owns the selected game view and its temporal policy:

- the immutable `Worldline` being viewed;
- the selected actual, counterfactual, or corrected branch;
- `Playback`, including the mapping from presentation `Tau` to `LogicalTime`;
- sample policy and explicit temporal queries;
- lookahead and future-state views;
- branch selection and branch-view operations;
- the developer-authored `Orchestrator`, including abstract interaction
    definitions and input orchestration; and
- the logical query and rendering composition over those values.

Stage ownership means that these values and policies belong to the game-facing
composition. The `Orchestrator` is the single mutable owner of Stage control
state, but it may not mutate a published `Worldline`, `Journal`, or
`GameState`. It may replace the selected worldline with a new immutable value
after journal publication or branch construction, while each query remains a
direct evaluation of immutable inputs.

## Orchestrator

The `Orchestrator` is where the developer writes ordinary game control code.
It may use a literal `while (true)` loop, a host callback, a replay driver, or
another application-specific execution shape. The engine does not impose a
static clock or a universal loop API at this stage.

The Orchestrator owns decisions that are not yet reusable abstractions:

- which `Tau` to sample and whether presentation time advances;
- which `LogicalTime` is queried through the Stage's playback policy;
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

The controlled transformation path is:

```text
InputPacketSet + Tau + LogicalTime
        -> InteractionDefinition
        -> closed Transformation
        -> Orchestrator admission
        -> JournalWriter or branch construction
        -> new immutable Journal/Worldline
```

`InteractionDefinition` cannot construct timestamped journal entries directly.
When an accepted transformation becomes authoritative, the Orchestrator uses
the journal and branch APIs; journal machinery retains timestamp legality and
immutable publication semantics.

The conceptual sample path is:

```text
Stage.sample(tau)
    -> Playback.logical_time_at(tau)
    -> QueryAdapter.query(worldline, logical_time)
    -> GameState
```

Rendering composes with the selected state through the existing presentation
boundary:

```text
Stage.present(tau)
    -> GameState
    -> Renderer.render(game_state, tau)
    -> Frame
```

The host may supply successive `Tau` samples from a platform clock, but Stage
owns the meaning and use of those samples. Explicit `Tau` values remain valid
for scrubbing, replay, testing, and deterministic presentation.

## PresentationHost Responsibilities

`PresentationHost` owns platform and execution concerns that do not define the
logical game experience:

- operating-system lifecycle and event-loop integration;
- acquisition of a host or device clock;
- window, surface, and display integration;
- hardware and device configuration;
- platform input acquisition and translation infrastructure; and
- backend, resource, and device lifecycle plumbing.

The host must not decide which worldline or branch is canonical for a Stage,
how Stage time maps to logical time, or what a Caravan domain value means.

The host may compose concrete Stage dependencies at compile time and may
provide the capabilities needed to execute them. It remains an adapter around
Stage rather than the owner of the game experience.

## Static Composition

The preferred implementation is Rust's static composition model:

- traits define genuine variation boundaries;
- generic parameters carry those abstractions through the pipeline; and
- the application composes concrete implementations as early as practical.

A conceptual shape is:

```rust
struct Stage<W, P, Q, I, R> {
    orchestrator: Orchestrator<W, P, Q, I>,
    renderer: R,
}

struct Orchestrator<W, P, Q, I> {
    worldline: W,
    playback: P,
    query: Q,
    interaction: I,
    tau: Tau,
}

struct PresentationHost<S, Clock, Platform, Backend> {
    stage: S,
    clock: Clock,
    platform: Platform,
    backend: Backend,
}
```

These are boundary sketches, not an instruction to introduce these exact
structs or to make every helper a trait. The useful constraint is that a
concrete game composition is visible in types and invalid combinations are
rejected before runtime where practical.

`QueryAdapter` remains a narrow Orchestrator dependency. It answers a query for
an already-selected worldline and logical time; it does not own Stage time,
branch policy, or the Orchestrator itself.

`Renderer` belongs to Stage's logical presentation composition. The eventual
backend that turns renderer output into device or surface work may remain host
plumbing. The boundary between those two rendering concerns is intentionally
left open until the render output shape is designed.

## Reserved Levers

The following concerns are intentionally reserved and are not settled by this
proposal:

### Input

The host supplies abstract `InputPacket` values. The Stage's Orchestrator owns
the `InteractionDefinition` that reasons over an `InputPacketSet`, as well as
the input orchestration that constructs that set. Packets may be delivered
directly or retained across calls. The canonical query takes the packet set,
`Tau`, and `LogicalTime`; its boundary is recorded in
[input-and-interaction.md](input-and-interaction.md).

### Camera and HUD

Camera and HUD are not currently assigned to Stage, Host, or a third view layer.
They remain reserved extension points until their state, time, and rendering
relationships are discussed explicitly.

### Rendering backend

Stage owns the logical renderer abstraction and composition. The division
between backend-neutral renderer output and host-owned device execution is
still open.

### Clock intersection

The host may provide a clock, but the contract for converting host clock samples
into explicit `Tau` values is not decided here. Stage remains the owner of the
presentation-time policy once a `Tau` exists.

## Non-Goals

This proposal does not:

- add `Stage` or `Orchestrator` types to the generic engine; the experimental
    `CaravanStage` and `CaravanOrchestrator` live in the application layer;
- change `spec/initial.md`;
- redefine `Worldline`, `LogicalTime`, `Tau`, `Playback`, `GameState`, or
  `Frame`;
- settle input commands or input timestamps;
- settle camera or HUD ownership;
- choose a GPU, windowing, asset, or device architecture; or
- introduce runtime dependency injection.

## Open Questions

1. What is the smallest concrete Stage composition that exercises worldline
   ownership, playback ownership, lookahead, branch selection, and presentation?
2. What host execution-opportunity shape best drives an Orchestrator without
    making host timing authoritative game time?
3. Which Stage operations are view-local changes and which author journal facts
    or create branches?
4. What renderer output crosses from Stage into host/backend plumbing?
5. How should host clock samples become explicit `Tau` values without making
   host time authoritative game time?
