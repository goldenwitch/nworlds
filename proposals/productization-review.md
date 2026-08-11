# Productization Review

## Decision

Productization remains conditional. This repository is a pure demo/toy for the
engine: it supports a deterministic, terminal-oriented developer/demo artifact,
an inspectable evidence bundle, and one manually verified Windows desktop
target using `winit` and `wgpu`. It does not yet justify release packaging,
scale claims, or production abstractions.

The only product concern that binds now is reproducibility of that reference
packet: the demo trace, conformance run/report, and benchmark conditions must
remain runnable and discoverable. This is stewardship of the current evidence,
not a new distribution or runtime layer.

## Current Basis

- `build.vine` makes productization depend on benchmarks, persistence, and the
  proof package. Its acceptance condition requires an owner, contract,
  dependency, and reproducible evidence for every activated concern.
- The root workspace contains the engine/domain crates, a terminal demo,
  persistence, and a non-published benchmark package. Conformance is a separate
  workspace. There are no `product/`, `crates/tools/`, or `crates/release/`
  surfaces.
- The anchor and demo establish one fixed radius-5, 91-tile fixture with
  deterministic journal entries, branch views, and presentation over the
  reference query. The checked-in snapshot is a usable developer-facing
  observable, not a platform commitment.
- The conformance report records all listed cases as passing. The authoritative
  boundary has compiler-checked purity tests; renderer bodies remain an
  explicit trusted extension boundary.
- The selected Windows target opens a native window, initializes `wgpu`, renders
  the owned Caravan output, routes Space through the existing input path, and
  survives resize and shutdown. These observations are local manual evidence,
  not automated GPU or pixel evidence.
- The benchmark report measures the unreplaced reference path on fixed small
  workloads in a release build, with no cache or optimization. It records
  direct queries, non-monotonic scrubbing, and frame production, but supplies
  no scale target or GPU requirement.

## Scope Review

| Concern | Outcome | Reason to defer or bind |
| --- | --- | --- |
| Demo, proof, and benchmark reproducibility | **Binds now** | Existing artifacts and commands are the current product surface for developers and reviewers. |
| Windows desktop host/render proof | **Baseline now** | The selected `x86_64-pc-windows-msvc` target has a working local entrypoint and independent `winit`/`wgpu` adapters; no production host policy is implied. |
| Persistence and deterministic replay | **Baseline only** | `engine-persistence` is present, but no shipped workflow or release target activates additional product work. |
| Packaging and release targets | **Deferred** | No target platform, installer/package format, versioning policy, or distribution requirement is recorded. |
| Content tooling and large-scale authoring | **Deferred** | The evidence uses a fixed anchor, hand-authored traces, and a finite seeded fixture; no authoring volume or content format requirement binds. |
| Audio and broader device/input integration | **Deferred** | The first target proves keyboard input only; audio, additional devices, and a production input policy remain unspecified. |
| Networking and synchronization | **Deferred** | Current worldlines and branches are local immutable values; no multiplayer, authority, latency, or reconciliation requirement is evidenced. |
| Merge semantics | **Deferred** | Counterfactual and corrected branches are selected and evaluated, but no requirement joins two branch histories. |
| Final GPU architecture | **Deferred** | `wgpu` is selected for the first local target, but measurements establish no failing scale regime, resource policy, device-loss policy, or production GPU contract. |

## Activation Gate

Activate a deferred concern only when a concrete requirement supplies its target
regime. The resulting packet should name its owner and contract, depend on the
relevant anchor/persistence/proof evidence, and add a reproducible acceptance
test or report. Until then, keep the terminal demo, local Windows proof,
evidence reports, and reference measurements as the productization boundary.

This review does not change source, manifests, specifications, the semantic
contract, the roadmap, or `build.vine`.