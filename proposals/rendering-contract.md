# Rendering Contract

This proposal defines the first concrete Caravan rendering boundary on top of
the generic `Renderer<S>` and `Frame<P>` APIs.

It is a game-facing projection contract. Backend, scene-graph, GPU, and
platform-window responsibilities belong to the host and target layers.

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

The generic `Renderer<S>` boundary receives `GameState<S>` and `Tau`. A
game-owned client projection may also receive an explicit presentation view
value, such as a camera or viewport, before it constructs the target
`RenderBatch`. The view value is an explicit input to a pure function and
remains presentation data.

The target crossing is a separate client projection:

```text
GameState<Snapshot> + Tau
  -> Caravan semantic `RenderOutput` inspection view
  -> `RenderBatch`
  -> Frame<RenderBatch>
  -> target RenderSink
```

The output is downstream, fire-and-forget rendering data. A render sink may
copy, queue, submit, or discard it. Interaction logic, hit testing, collision
reasoning, and game rules query the selected authoritative `GameState`; the
renderer projects that state into presentation data.

## RenderOutput

The first Caravan `RenderOutput` is a minimal, owned, backend-neutral packet
of values sufficient to draw one frame. It carries the current frame's draw
data:

- the exact sampled `LogicalTime` from `GameState`;
- the enclosing `Tau` through `Frame`;
- stable saucer tile order;
- independent terrain, actor, and effect values for each tile;
- actor identity and actor kind;
- global wheat and wood resources; and
- deterministic equality for equal `GameState` and `Tau` inputs.

The output carries current-frame draw data. Authoritative state, interaction
state, view selection, frame history, device handles, host scheduling, journal
history, and worldline selection remain in their owning layers.

The output vocabulary stays deliberately small. Add a render field when a
selected gameplay loop requires a value already present in `GameState` and the
field is needed to draw that frame. View state enters through explicitly named
presentation inputs.

## Ownership

- `GameState<Snapshot>` owns the authoritative Caravan values being projected.
- `Tau` selects presentation sampling and remains visible on `Frame`.
- `GameState<Snapshot> + Tau` are the semantic inputs to generic `Renderer`
  production; a client projection may add explicitly named presentation view
  state.
- `Renderer<Snapshot>` owns the pure projection from immutable state to owned
  rendering data.
- `RenderOutput` is a Caravan-owned semantic inspection value; `RenderBatch`
  carries the shared target-facing draw vocabulary.
- `Frame<RenderBatch>` owns the target-facing presentation envelope and shared
  draw value.
- Stage owns renderer composition and chooses the selected state and `Tau`.
- The presentation host owns render-sink transport and target/backend
  execution after the frame crosses the host boundary.
- Interaction and transport/journal logic remain independent of render output.

If a player-visible fact is absent from `GameState`, state production or the
authoritative domain model supplies it. The renderer receives the selected
state and explicit presentation values; the Stage chooses the worldline,
branch, and input context before projection.

A renderer implementation is a trusted extension boundary: the trait receives
immutable values and returns an owned value. Implementations follow the
project's deterministic presentation discipline.

## Determinism

For equal `GameState<Snapshot>` and equal `Tau`, rendering returns equal owned
output. Output ordering is explicit and follows the selected state and
presentation inputs, independent of collection order, prior frames, device
state, or host scheduling.

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

After the client projection, `Frame<RenderBatch>` crosses the render sink
defined by the [presentation-host proposal](presentation-host.md). The sink
may copy, queue, submit, or discard the frame while the Stage remains the
source of authoritative state and renderer selection.

## Current Boundary

This first contract currently establishes:

- `GameState<S> + Tau -> owned render output`;
- explicit client view values such as camera and viewport;
- `Frame<RenderBatch>` as the target-facing presentation envelope; and
- host-owned backend, device, window, asset, audio, and interaction adapters.

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
- generic `Renderer` production receives no input besides the selected
  `GameState` and `Tau`; any client view input is explicit and presentation-only.

The contract leaves Rust struct layout open while the existing
`Renderer<Snapshot>` and `Frame` boundaries carry the behavior.

## Presentation Driver Extension

The first rendering slice above is the primitive projection boundary. The next
developer-facing layer encodes presentation with redraw as a presentation
demand around the selected state.

### Complete-state sampling

Read-ahead is a sampling operation over complete immutable worlds:

```text
S0 = state(worldline, t0)
S1 = state(worldline, t1)
S2 = state(worldline, t2)
```

Each `S` is a complete `GameState` at its exact `LogicalTime`. Sample plans
operate on complete authoritative states and may return one or many values for
scrubbing, preview, comparison, or presentation.

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

Changing `Tau` changes presentation/animation output for the fixed selected
state. `Tau` is a visual phase/coordinate; `LogicalTime` selects game state and
the Orchestrator selects branches and publishes journal facts.

The exact selected sample is the anchor identity. A change of selected
`GameState` value, including its sampled `LogicalTime`, resets `Tau`; the driver
anchors visual time to the newly selected sample even when two samples render
identically.

### Redraw independence

The target may redraw at any rate. Redraw requests presentation of the current
selected state at the driver's current visual `Tau`; authoritative state
advancement remains an input/publication decision:

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
times.

### Reserved Extensions

Keyframes, skeletal animation, retained scene graphs, GPU resource lifetimes,
automatic transition interpolation, and animation-rate policy remain later
consumers of the complete-state plus visual-time boundary. Visual comparison
or diff between two complete sampled worlds is an explicit presentation
operation.
