# Rendering Contract

This proposal defines the first concrete Caravan rendering boundary on top of
the generic `Renderer<S>` and `Frame<P>` APIs.

It is a game-facing projection contract, not a backend, scene-graph, GPU, or
platform-window contract.

Cross-boundary ownership is indexed in
[redundancy-register.md](redundancy-register.md). This proposal owns the
current Caravan `RenderOutput` semantic inspection projection; the shared
target vocabulary is owned by [target-factory.md](target-factory.md) as
`RenderBatch`.

## Boundary

The semantic inspection composition is:

```text
GameState<Snapshot> + Tau
  -> `project_output`
  -> RenderOutput
```

The selected `GameState` is authoritative input. `Tau` is the independent
presentation sample. The Caravan inspection projection returns owned
`RenderOutput` data; the target-facing renderer separately returns the shared
`Frame<RenderBatch>` value.

The target crossing is a separate client projection:

```text
GameState<Snapshot> + Tau
  -> Caravan semantic `RenderOutput` inspection view
  -> `RenderBatch`
  -> Frame<RenderBatch>
  -> target RenderSink
```

The output is downstream, fire-and-forget rendering data. A render sink may
copy, queue, submit, or discard it, but no later game decision or frame
production may depend on it. Interaction logic, hit testing, collision
reasoning, and game rules never consume `RenderOutput`; they query the selected
authoritative `GameState` instead.

## RenderOutput

The first Caravan `RenderOutput` is a minimal, owned, backend-neutral packet
of values sufficient to draw one frame. It preserves only the current frame's
draw data:

- the exact sampled `LogicalTime` from `GameState`;
- the enclosing `Tau` through `Frame`;
- stable saucer tile order;
- independent terrain, actor, and effect values for each tile;
- actor identity and actor kind;
- global wheat and wood resources; and
- deterministic equality for equal `GameState` and `Tau` inputs.

The output contains no authoritative state, interaction state, view-selection
state, continuation state, frame history, device handle, host clock, journal,
or worldline. It is not a cache or a second game-state representation.

The output vocabulary stays deliberately small. Add a render field only when a
selected gameplay loop requires a value already present in `GameState` and the
field is needed to draw that frame. Do not add a second renderer input to carry
missing facts.

## Ownership

- `GameState<Snapshot>` owns the authoritative Caravan values being projected.
- `Tau` selects presentation sampling and remains visible on `Frame`.
- `GameState<Snapshot> + Tau` are the only inputs to render production.
- `Renderer<Snapshot>` owns the pure projection from immutable state to owned
  rendering data.
- `RenderOutput` is a Caravan-owned semantic inspection value; it does not
  cross into generic target execution.
- `Frame<RenderBatch>` owns the target-facing presentation envelope and shared
  draw value.
- Stage owns renderer composition and chooses the selected state and `Tau`.
- The presentation host owns render-sink transport and target/backend
  execution after the frame crosses the host boundary.
- Interaction and transport/journal logic remain independent of render output.

If a player-visible fact is absent from `GameState`, the missing work belongs
in state production or the authoritative domain model. The renderer does not
receive the journal, worldline, Orchestrator, input buffer, branch selector, or
an auxiliary view context to recover it.

A renderer implementation is a trusted extension boundary: the trait receives
immutable values and returns an owned value, while arbitrary implementation
body purity is not compiler-proven.

## Determinism

For equal `GameState<Snapshot>` and equal `Tau`, rendering returns equal owned
output. Output ordering is explicit and does not depend on hash-map iteration,
query order, prior frames, device state, or host scheduling.

Forward, reverse, repeated, arbitrary, and branch samples use the same semantic
path:

```text
selected Worldline + LogicalTime
    -> GameState<Snapshot>
    -> Renderer<Snapshot> + Tau
    -> semantic `RenderOutput` inspection view
    -> Frame<RenderBatch>
```

## Host Crossing

  The semantic rendering contract hands its client projection to the shared host
  boundary:

```text
  Frame<RenderBatch>
    -> RenderSink port
    -> target backend, surface, or device
```

The render sink may copy, queue, submit, or discard the frame. It does not
reinterpret authoritative game state or become a second renderer authority.

## Non-Goals

This first contract does not define:

- a GPU or `wgpu` architecture;
- an operating-system window or surface API;
- a camera, HUD, coordinate projection, or widget model;
- asset, audio, or device-resource ownership;
- a persistent GPU simulation or frame-history model;
- a target-specific render command stream; or
- interaction or hit testing over rendering objects.

## Acceptance Evidence

The first implementation is sufficient when focused evidence proves:

- an empty snapshot produces a deterministic empty output;
- a radius-5 snapshot preserves all 91 tiles in stable order;
- terrain, actors, and effects remain independent in the output;
- actor identity and kind survive projection;
- global resources survive projection;
- exact sampled `LogicalTime` and `Tau` remain observable;
- equal inputs produce equal output across repeated samples;
- actual, counterfactual, and corrected states use one rendering path; and
- no render output is supplied to interaction reasoning or authoritative state
  evaluation; and
- render production receives no input besides the selected `GameState` and
  `Tau`.

The contract does not require a particular Rust struct layout beyond the
existing `Renderer<Snapshot>` and `Frame` boundaries.

## Presentation Driver Extension

The first rendering slice above is the primitive projection boundary. The next
developer-facing layer encodes presentation without making a redraw callback
the game loop.

### Complete-state sampling

Read-ahead is a sampling operation over complete immutable worlds:

```text
S0 = state(worldline, t0)
S1 = state(worldline, t1)
S2 = state(worldline, t2)
```

Each `S` is a complete `GameState` at its exact `LogicalTime`. `S1` is not an
animation endpoint or a partially advanced `S0`; the engine never requires
interpolation between authoritative states. A sample plan may return one or
many complete states for scrubbing, preview, comparison, or presentation.

### Visual-time anchoring

`Tau` is visual time relative to the currently selected complete `GameState`.
When a different exact `GameState` sample is selected, the presentation driver
resets its visual-time anchor:

```text
select(S0)       -> Tau = 0
advance_visual   -> Tau = Tau + delta
present(S0,Tau)  -> Frame<RenderBatch>
select(S1)       -> Tau = 0
```

Changing `Tau` alone may change only presentation/animation output for the
fixed selected state. It may not query another logical state, select a branch,
publish a journal fact, mutate authoritative values, or turn host time into
logical game time. `Tau` is a visual phase/coordinate, not a logical-time
advance, a state-transition instruction, or a host-clock API.

The exact selected sample is the anchor identity. A change of selected
`GameState` value, including its sampled `LogicalTime`, resets `Tau`; equal
visual output does not authorize the driver to retain a stale anchor across a
new selection.

### Redraw independence

The target may redraw at any rate. Redraw is a request to present the current
selected state at the driver's current visual `Tau`; it is not a request to
advance authoritative game state:

```text
native redraw
  -> present(selected GameState, current Tau)
  -> Frame<RenderBatch>
  -> RenderSink
```

Input/publication, complete-state sampling, visual-time advancement, and target
submission are separate responsibilities. A package may publish a new
immutable worldline in response to semantic input, select a complete state
from that worldline, and render that state repeatedly at different visual
times without a traditional mutable update loop.

### Explicit non-goals

This extension does not yet define keyframes, skeletal animation, retained
scene graphs, GPU resource lifetimes, automatic transition interpolation, or a
universal animation-rate policy. Those are later consumers of the complete
state plus visual-time boundary. Any visual comparison or diff between two
complete sampled worlds must be an explicit presentation operation, not an
implicit change to authoritative state semantics.
