# Rendering Contract

This proposal defines the first concrete Caravan rendering boundary on top of
the generic `Renderer<S>` and `Frame<P>` APIs.

It is a game-facing projection contract, not a backend, scene-graph, GPU, or
platform-window contract.

## Boundary

The rendering composition is:

```text
GameState<Snapshot> + Tau
    -> Renderer<Snapshot>
    -> Frame<RenderOutput>
```

The selected `GameState` is authoritative input. `Tau` is the independent
presentation sample. The renderer returns owned `RenderOutput` data inside the
existing SDK `Frame` envelope.

The output is downstream rendering data. Interaction logic, hit testing,
collision reasoning, and game rules never consume `RenderOutput`; they query
the selected authoritative `GameState` instead.

## RenderOutput

The first Caravan `RenderOutput` is an owned, backend-neutral composition of
rendering objects sufficient to inspect one frame. It preserves:

- the exact sampled `LogicalTime` from `GameState`;
- the enclosing `Tau` through `Frame`;
- stable saucer tile order;
- independent terrain, actor, and effect values for each tile;
- actor identity and actor kind;
- global wheat and wood resources; and
- deterministic equality for equal `GameState` and `Tau` inputs.

The output may contain rendering-oriented values derived from the snapshot. It
must not contain a mutable authoritative board, a hidden continuation state,
frame history, a device handle, or an implicit host clock.

The first output shape should stay small and value-oriented. It may be composed
into richer target-local objects later; that is not a reason to make the game
engine know about coordinates, cameras, widgets, assets, GPU resources, or
backend commands now.

## Ownership

- `GameState<Snapshot>` owns the authoritative Caravan values being projected.
- `Tau` selects presentation sampling and remains visible on `Frame`.
- `Renderer<Snapshot>` owns the pure projection from immutable state to owned
  rendering data.
- `Frame<RenderOutput>` owns the presentation envelope and output value.
- Stage owns renderer composition and chooses the selected state and `Tau`.
- The presentation host owns render-sink transport and target/backend
  execution after the frame crosses the host boundary.
- Interaction and transport/journal logic remain independent of render output.

A renderer implementation is a trusted extension boundary: the trait receives
immutable values and returns an owned value, while arbitrary implementation
body purity is not compiler-proven.

## Determinism

For equal `GameState<Snapshot>` and equal `Tau`, rendering returns equal owned
output. Output ordering is explicit and does not depend on hash-map iteration,
query order, prior frames, device state, or host scheduling.

Forward, reverse, repeated, arbitrary, and branch samples use the same path:

```text
selected Worldline + LogicalTime
    -> GameState<Snapshot>
    -> Renderer<Snapshot> + Tau
    -> Frame<RenderOutput>
```

## Host Crossing

The rendering contract ends at the owned frame value:

```text
Frame<RenderOutput>
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
  evaluation.

The contract does not require a particular Rust struct layout beyond the
existing `Renderer<Snapshot>` and `Frame` boundaries.
