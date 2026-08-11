# Demo Gameplay Plan

This proposal turns the existing design corpus into a plan for the remaining
player-facing Caravan demo. It is a planning boundary, not a claim that the
engine should become a finished game or that the current demo needs a generic
engine abstraction.

The repository currently proves the temporal engine and one native host slice.
The next gap is experiential: the underlying world already has actors, terrain,
effects, resources, time, branches, persistence, and deterministic fixtures, but
the player-facing demo exposes only a small `Noop`/`SetTerrain` interaction.

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

These are engine and world substrates. They are not, by themselves, a complete
player-facing feature set.

## Design-Derived Feature Inventory

| Area | Existing design commitment | Current player-facing gap |
| --- | --- | --- |
| World creation | `CreateSaucer { radius: 5 }` creates the 91-tile world | The demo has one fixed initial world and no user-facing world setup flow |
| Terrain | `Void`, `Wheat`, and `Forest` are authoritative terrain values | Only one center-tile terrain action is exposed |
| Actors | Farmer, Forester, Arsonist, Fighter, and Arborist have indexed rules | The demo does not let a player author, select, or observe an actor-driven situation deliberately |
| Effects | Fire is an independent effect with age, spread, and destruction behavior | Fire is present in fixtures and output but has no player-facing trigger or readable progression |
| Resources | Wheat and wood are indexed totals derived from state | Resources are not presented as a useful player-facing readout |
| Logical time | Queries may sample arbitrary past, present, and future logical times | The native demo does not expose a time-selection or progression control |
| Presentation time | `Tau` is independent of logical time and supports deterministic presentation | The native demo does not expose scrubbing or an explicit presentation policy |
| Branches | Actual, counterfactual, and corrected branches are immutable values | The demo does not expose branch creation, selection, comparison, or return to parent |
| Persistence | Worldline/save encoding and deterministic replay exist below host byte transport | The native demo has no user-facing save/load/replay workflow |
| Seeded worlds | Fixed-seed journal generation is deterministic and reproducible | Seed selection and world setup are not part of the demo experience |
| Input | Transport identity/order becomes a payload-only semantic batch | Only the first primary-button action is mapped in the native host |
| Rendering | Owned output preserves tiles, actors, effects, resources, logical time, and `Tau` | The native sink currently renders a minimal colored tile field without readable world/status presentation |

The source commitments for this inventory are
[spec/initial.md](../spec/initial.md),
[spec/cellular-automata-anchor.md](../spec/cellular-automata-anchor.md),
[proposals/input-and-interaction.md](input-and-interaction.md),
[proposals/stage-layer.md](stage-layer.md),
[proposals/rendering-contract.md](rendering-contract.md), and
[proposals/caravan-orchestrator-anchor.md](caravan-orchestrator-anchor.md).

## Planning Boundary

The first implementation task is to select one coherent player-facing loop
from this inventory. The selected loop must use existing indexed world behavior
and make at least one of its temporal, actor, effect, resource, branch, or
persistence properties observable. It must not be a second authoritative state
model hidden inside the demo.

After that ruling, the work proceeds in this order:

1. Define the closed input commands and transformations required by the loop.
2. Define the logical-time, branch, replay, or persistence controls the loop
   actually needs.
3. Compose the loop through the existing Stage, Orchestrator, journal, query,
   renderer, and host boundaries.
4. Add deterministic and manual evidence for the player-facing behavior.
5. Polish only the engine boundary that the selected loop makes concrete.

The demo remains a toy and evidence vehicle. A feature is complete when its
player-facing behavior is observable and its authoritative consequences remain
represented by immutable journal/worldline values; a lower-level API existing
in isolation is not enough.

## Reserved Design Questions

These questions require a design ruling before implementation begins:

- Which existing world behavior is the first player-facing loop?
- Which actions are authored journal facts, and which are view or control
  operations?
- Which time controls belong in the first experience: progression, explicit
  sampling, scrubbing, or replay?
- Which branch operation is visible first, if any?
- Which state values must be readable in the first presentation: actors,
  effects, resources, logical time, branch identity, or save status?
- What constitutes a complete demo interaction cycle from input to visible
  consequence?

Camera, HUD, coordinate projection, richer input devices, networking, merge
semantics, packaging, and final graphics architecture remain separate decisions
unless the selected gameplay loop binds one of them.
