# Anchor Demonstration Plan

This proposal turns the existing Caravan anchor specification into a plan for
the remaining demo experience. The anchor is the design source for this work:
we are exposing and composing its specified behavior, not inventing a separate
player game or promoting fixture rules into new gameplay semantics.

The repository currently proves the temporal engine and one native host slice.
The next gap is demonstrative: the anchor already specifies actors, terrain,
effects, resources, time, branches, seeded journals, and persistence, but the
native demo exposes only a small `Noop`/`SetTerrain` interaction and does not
walk through the rest of the anchor behavior.

## Current Baseline

The implemented demo currently provides:

- immutable `ReferenceWorldline` selection and journal publication;
- direct arbitrary-time queries with exact `LogicalTime`;
- indexed Farmer, Forester, Arsonist, Fighter, and Arborist behavior;
- independent terrain, actor, effect, wheat, and wood state layers;
- actual, counterfactual, and corrected branch values;
- game-facing persistence and deterministic replay primitives;
- backend-neutral `RenderOutput` and a native Windows `winit`/`wgpu` host;
- ordered transport observations, input windows, and `SemanticInputBatch`; and
- a native Space input that publishes `SetTerrain` at the center tile.

These are engine and world substrates. The remaining work is to make the
anchor's required queries and acceptance cases understandable and observable in
the demo without changing their semantics.

## Anchor-Derived Demonstration Inventory

| Area | Existing design commitment | Current player-facing gap |
| --- | --- | --- |
| Empty and created worlds | Empty journal is valid; `CreateSaucer { radius: 5 }` creates 91 tiles | The native demo does not walk through both states as an anchor case |
| Journal visibility and time boundaries | Spawn visibility, inside-tick stability, exact journal discontinuities, and tick-boundary changes are specified | The native demo does not expose these anchor queries as a deliberate walkthrough |
| Terrain and vegetation | Farmer movement, wheat placement, wheat totals, forester movement, forest, and wood totals are specified | The native demo does not expose the hand-authored vegetation trace as an interaction sequence |
| Hazards and conflicts | Arsonist ignition, fire aging/spread/destruction, fighter pursuit/collision, and arborist conversion are specified | The native demo does not expose the hand-authored hazard traces as a deliberate walkthrough |
| Lookahead and branches | Future queries, counterfactual branches, corrected branches, and parent isolation are specified | The native demo prints branch evidence but does not let a user inspect the branch relationship interactively |
| Seeded determinism | Fixed-seed journal construction and repeatability are specified | Seeded construction is test/demo infrastructure, not a player world-creation feature yet |
| Persistence and replay | Journals and branches round-trip without changing query semantics | Persistence exists below the host, but the demo has no guided save/load/replay walkthrough |
| Input | Transport identity/order becomes a payload-only semantic batch | Only the first primary-button action is mapped in the native host |
| Rendering | `GameState + Tau` produces minimal fire-and-forget owned output | The native sink renders a minimal colored tile field; the remaining work is to make the selected anchor case legible without adding render inputs |

The source commitments for this inventory are
[spec/initial.md](../spec/initial.md),
[spec/cellular-automata-anchor.md](../spec/cellular-automata-anchor.md),
[proposals/input-and-interaction.md](input-and-interaction.md),
[proposals/stage-layer.md](stage-layer.md),
[proposals/rendering-contract.md](rendering-contract.md), and
[proposals/caravan-orchestrator-anchor.md](caravan-orchestrator-anchor.md).

## Planning Boundary

The first implementation task is to select the next coherent anchor walkthrough
slice from the required queries in
[cellular-automata-anchor.md](../spec/cellular-automata-anchor.md). The slice
must expose existing indexed behavior and its acceptance evidence; it must not
invent a new gameplay loop or create a second authoritative state model inside
the demo.

The anchor-derived order is:

1. Empty journal and saucer creation.
2. Journal visibility, sub-tick sampling, tick boundaries, and discontinuities.
3. Farmer, wheat, forester, forest, and resource traces.
4. Arsonist, fire, fighter, and arborist traces.
5. Lookahead, counterfactual/corrected branches, seeded determinism, and
  persistence/replay.
6. Native presentation of the selected anchor observations.

After the next slice is selected, define only the controls needed to navigate
that slice, compose them through the existing Stage/Orchestrator/journal/query/
renderer/host boundaries, add deterministic and manual evidence, and polish
only the boundary that the evidence makes concrete.

The demo remains a toy and evidence vehicle. An anchor slice is complete when
its specified behavior is observable and its authoritative consequences remain
represented by immutable journal/worldline values; a lower-level API existing
in isolation is not enough. Render production remains exactly
`GameState + Tau -> minimal fire-and-forget RenderOutput`; a feature does not
introduce an auxiliary renderer input for selection, focus, pending work,
branch identity, or presentation mode.

## Reserved Design Questions

These questions require a design ruling before implementation begins:

- Which anchor-required query slice should be made navigable first?
- Which actions are authored journal facts, and which are view or control
  operations?
- Which time controls belong in the first experience: progression, explicit
  sampling, scrubbing, or replay?
- Which branch operation is visible first, if any?
- Which state values must be readable in the first presentation: actors,
  effects, resources, logical time, branch identity, or save status?
- What constitutes a complete demo interaction cycle from input to visible
  consequence?

For each selected visible value, the plan must identify its `GameState` source.
If no such source exists, the state-production gap must be resolved before
rendering work is planned.

Camera, HUD, coordinate projection, richer input devices, networking, merge
semantics, packaging, and final graphics architecture remain separate decisions
unless the selected gameplay loop binds one of them.
