# Productization Review

## Decision

Productization remains conditional. The current packet supports a deterministic,
terminal-oriented developer/demo artifact and an inspectable evidence bundle. It
does not yet justify platform features or production abstractions.

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
- The benchmark report measures the unreplaced reference path on fixed small
  workloads in a release build, with no cache or optimization. It records
  direct queries, non-monotonic scrubbing, and frame production, but supplies
  no scale target or GPU requirement.

## Scope Review

| Concern | Outcome | Reason to defer or bind |
| --- | --- | --- |
| Demo, proof, and benchmark reproducibility | **Binds now** | Existing artifacts and commands are the current product surface for developers and reviewers. |
| Persistence and deterministic replay | **Baseline only** | `engine-persistence` is present, but no shipped workflow or release target activates additional product work. |
| Packaging and release targets | **Deferred** | No target platform, installer/package format, versioning policy, or distribution requirement is recorded. |
| Content tooling and large-scale authoring | **Deferred** | The evidence uses a fixed anchor, hand-authored traces, and a finite seeded fixture; no authoring volume or content format requirement binds. |
| Audio and device/input integration | **Deferred** | The demo exercises authored journal entries and presentation, not a device-backed playable loop. |
| Networking and synchronization | **Deferred** | Current worldlines and branches are local immutable values; no multiplayer, authority, latency, or reconciliation requirement is evidenced. |
| Merge semantics | **Deferred** | Counterfactual and corrected branches are selected and evaluated, but no requirement joins two branch histories. |
| Final GPU architecture | **Deferred** | Measurements are reference CPU observations on fixed workloads and do not establish a failing scale regime or GPU contract. |

## Activation Gate

Activate a deferred concern only when a concrete requirement supplies its target
regime. The resulting packet should name its owner and contract, depend on the
relevant anchor/persistence/proof evidence, and add a reproducible acceptance
test or report. Until then, keep the current terminal demo, evidence reports,
and reference measurements as the productization boundary.

This review does not change source, manifests, specifications, the semantic
contract, the roadmap, or `build.vine`.