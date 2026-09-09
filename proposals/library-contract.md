# Library Contract and Ownership Register

This document is the working register for the library-first boundary. It records
the current repository facts and the
contract decisions that constrain remediation. The core-contract task will
settle remaining public type or crate-name choices; this register distinguishes durable boundaries from implementation convenience.

The all-up cross-boundary redundancy and canonical-owner map is
[redundancy-register.md](redundancy-register.md); this document owns the
library boundary and its dependency direction.

## Product Boundary

The reusable library is the product. Reference games, sample applications,
target adapters, tests, and benchmarks consume it through a one-way dependency
direction:

```text
reusable library -> game consumer -> target adapter
```

The temporal library owns the following conceptual path:

```text
immutable context + immutable journal/worldline + LogicalTime
    -> owned GameState
GameState + Tau
    -> owned presentation output
```

The host library is adjacent to that path. It transports already-owned input,
bytes, and frames. Stage owns game meaning and authoritative time; the target
factory owns target selection.

## Ownership Classes

| Class | Primary responsibility | Adjacent responsibilities |
| --- | --- | --- |
| Temporal library | Fixed-point time, opaque SDK envelopes, immutable journals and branches, direct query mechanics, lookahead, persistence mechanics, and state-first presentation contracts | Caravan meaning is authored by the reference game; target identity and loop policy are authored by host/application layers. |
| Host library | Target-neutral package and passive input, storage, lifecycle/resource, and render-sink ports | Stage supplies game semantics and journal time; target adapters supply concrete execution. |
| Caravan reference implementation | Caravan geometry, journal vocabulary, rules, projection, fixtures, and game-specific persistence/reference APIs | The generic engine supplies reusable contracts; target compositions supply lifecycle. |
| Sample application | Complete public-library composition and user-visible behavior | Reusable boundaries are promoted from repeated consumer evidence. |
| Target adapter | Native lifecycle, event translation, backend/device execution, and target-local resources | Stage supplies game meaning and authoritative state. |
| Evidence | Tests, snapshots, benchmarks, reports, and dependency guards that consume the other classes | Production contracts remain in their owning implementation or design record. |
| Planning/tooling | Specifications, proposals, VINE graphs, manifests, and CI orchestration | Runtime values remain in game and library implementations. |

## Current Component Map

| Component | Owner class | Current role and boundary condition |
| --- | --- | --- |
| `engine-time` | Temporal library | Generic `LogicalTime`, `Tau`, checked arithmetic, and tick conversion. |
| `engine-sdk` | Temporal library | Generic immutable envelopes with opaque payloads. |
| `engine-journal` | Temporal library | Generic journal mechanics over opaque payloads; Caravan payloads are dev/test consumer values only. |
| `engine-branches` | Temporal library | Generic immutable branch mechanics over opaque context and journal payloads; Caravan specialization is owned by the reference layer. |
| `engine-index` | Temporal library | Generic direct query/index kernel over opaque journal sources and query results. |
| `engine-controls` | Temporal library | Target-neutral `Pixels`/`Viewport` units, timeline slider/step mapping, typed logical/Tau deltas, fixed-focus parabolic time reprojection, viewport layout scaling, and automatic/manual presentation control values; no game or host types. |
| `engine-presentation` | Temporal library | Production renderer contract is generic; current Caravan dependencies are dev-dependencies used by tests. |
| `engine-api` | Temporal library | Generic facade for time, SDK envelopes, journal/branch/index mechanics, controls, and state-first presentation. |
| `nworlds-host` | Host library | Dependency-free target-neutral ports and `ApplicationHost` composition. |
| `caravan-domain` | Caravan reference implementation | Caravan geometry, values, and closed journal payloads. |
| `caravan-vegetation` | Caravan reference implementation | Farmer, wheat, forester, forest, and wood rules. |
| `caravan-hazards` | Caravan reference implementation | Arsonist, fire, fighter, and arborist rules. |
| `caravan-seeded` | Caravan reference implementation | Deterministic Caravan journal fixtures. |
| `caravan-reference` | Caravan reference implementation | Caravan projection/oracle, discontinuity meanings, snapshots, and parity baseline. |
| `caravan-persistence` | Caravan reference implementation | Versioned Caravan worldline codec, branch lineage, save/load, and replay. |
| `caravan-sample` | Sample application | Developer-authored Stage/Orchestrator composition and native sample package, with a separate trace binary for evidence. |
| `nworlds-desktop` | Target composition | Generic Windows `winit`/`wgpu` composition over the target-neutral host; game packages connect through static client composition. |
| `engine-benchmarks` | Evidence | Release measurements for the current reference implementation and presentation path. |
| `purity-tests` | Evidence | Compiler and runtime checks for immutable/data-only boundaries. |
| `tests/conformance` | Evidence | Separate executable catalog for the Caravan anchor and library behavior. |
| `README.md`, `index.md`, `roadmap.md`, `spec/**`, `proposals/**`, `*.vine` | Planning/tooling | Design truth, ownership records, execution graphs, and repository navigation. |
| `target/**` | Generated output | Build artifacts held in the generated-output boundary. |

## Remediated Production Dependency Register

The following direct production edges were present at the start of this
execution and were removed by the core-remediation task:

| Source production crate | Direct consumer dependency | Classification |
| --- | --- | --- |
| `engine-journal` | `caravan-domain` | Removed: journal payloads are generic. |
| `engine-branches` | `caravan-domain` | Removed: branch payloads are generic. |
| `engine-index` | `caravan-domain` | Removed: journal sources are generic. |
| `engine-lookahead` | `caravan-reference` | Removed: the misleading crate was deleted; convenience views live in `caravan-reference`. |
| `engine-persistence` | `caravan-domain`, `caravan-reference` | Removed: the Caravan codec moved to `caravan-persistence`. |
| `engine-api` | `caravan-domain`, `caravan-reference` | Removed: the facade now re-exports only generic surfaces. |

Current production dependency set: **generic**. Caravan references in engine
manifests serve reference fixtures as dev-dependencies and are guarded
separately.

The following references are dev-dependencies rather than production edges:

| Source test surface | Dev-dependencies | Treatment |
| --- | --- | --- |
| `engine-presentation` tests | `caravan-domain`, `caravan-reference`, `engine-journal` | Reference fixtures that exercise the production renderer contract. |
| `engine-benchmarks` and `tests/conformance` | Engine and Caravan crates | Evidence consumers with their own executable scope. |
| `caravan-sample` | Engine, Caravan, and `nworlds-host` crates | Sample consumer; its dependency direction is expected. |
| `nworlds-desktop` | Host, engine, and backend crates | Generic target consumer; no Caravan or voxel production dependency is permitted. |

## Production Dependency Direction

The remediation test follows this dependency direction, independent of crate
names:

```text
temporal library  --> generic temporal consumers
generic consumer  --> target-neutral host and target adapter
sample/target     -->  approved library and host surfaces
evidence          -->  the surfaces it measures
```

Evidence and consumer crates may use reference implementations as
dev-dependencies when a test names that reference scope. Those fixtures remain
test consumers while the public library stays generic.

## Contract Items To Settle Next

The next graph tasks use this contract map:

- Generic payloads remain opaque at the SDK boundary; Caravan journal payloads
  remain owned by the Caravan reference implementation.
- `JournalWriter` owns authoritative timestamp assignment; query and
  presentation operate on immutable worldlines.
- Actual, counterfactual, and corrected histories remain immutable values.
- Direct state queries accept arbitrary logical times and remain independent of
  query order.
- Presentation accepts `GameState` and `Tau`, then returns owned output.
- Screen-control state remains explicit presentation/control state; authoritative
  game state carries game meaning and renderer inputs remain explicit.
- Host ports transport values and bytes; Stage owns game time and target
  adapters own device state.
- External game consumers reach the temporal library through generic exports.

The core-contract task turns these constraints into one public crate/type map.
Remediation tasks move or parameterize production code according to that map,
keeping one generic engine.

## Settled Temporal Library Surface

The first isolated library surface is deliberately small:

| Crate | Public responsibility | Payload policy |
| --- | --- | --- |
| `engine-time` | `LogicalTime`, `Tau`, checked arithmetic, and game-tick conversion | Time values remain generic. |
| `engine-sdk` | `Context<C>`, `JournalEntry<P>`, `Journal<P>`, `Worldline<C, P>`, `GameState<S>`, `Frame<F>`, and query result envelopes | `C`, `P`, `S`, and `F` are opaque caller-owned values. |
| `engine-journal` | `Journal<P>` and `JournalWriter<P>` with monotonic authoring and immutable publication | `P` is generic; timestamp assignment is library-owned. Branching uses cloneable payloads. |
| `engine-branches` | `Branch<C, P>`, `Worldline<C, P>`, branch kind, immutable inclusive-prefix construction, and branch errors | `C` and `P` are generic. |
| `engine-index` | `JournalSource`, `QueryInput<C, P>`, `IndexedQuery<C, P>`, direct indexed state evaluation, and generic discontinuity pieces | Breakpoint payloads and query results are opaque. |
| `engine-controls` | Explicit screen units, normalized control geometry, timeline slider/step mapping, typed `LogicalTime`/`Tau` deltas, fixed-focus parabolic time reprojection, viewport layout scaling, and automatic/manual control values | Control values remain independent of game and host types. |
| `engine-presentation` | `Renderer<S>` and `present(GameState<S>, Tau) -> Frame<F>` | Renderer output is owned from state and `Tau`. |
| `engine-api` | Generic re-export facade for the supported temporal surface | Exports remain generic. |

Lookahead uses the same direct query against an unchanged immutable worldline.
The former Caravan-only lookahead aliases live in
`caravan-reference::lookahead`, while the generic surface stays focused on
direct queries.

Persistence is split into two boundaries: a package-owned codec that
understands the package's context and journal payloads, and a host-owned
transport of encoded bytes. The current Caravan codec lives in
`caravan-persistence` as the concrete reference implementation.

The generic library serves consumers that supply their own context, journal
payload, indexed query, and renderer. Caravan reference aliases make the same
surface convenient for the sample while remaining consumer-layer types.

## Reference-Game Boundary

The Caravan layer owns the closed `GameJournalEntry` vocabulary, Caravan
geometry and values, actor/vegetation/hazard rules, discontinuity meanings,
reference projection, seeded fixtures, and any codec that serializes those
values. `caravan-reference` may expose convenience aliases such as
`ReferenceWorldline` and `State`; those aliases are game-owned specializations
of the generic library types.

`caravan-sample` owns the developer-authored `Stage`, `Orchestrator`, input
interpretation, Caravan transformations, and the sample renderer. It is a
consumer of the library and host contracts. Its existence is justified by the
complete public-library path it demonstrates.

## Host-Library Boundary

`nworlds-host` remains dependency-free and target-neutral. Its stable
responsibilities are the narrow ports already present in the proof:

```text
InputIngress<Packet>       transport of translated observations
StorageTransport           transport of package-owned bytes
RenderSink<Frame>           execution or collection of owned frames
PlatformInputAdapter       native event -> package packet translation
GamePackage                package-owned semantic step and save/load hooks
```

`ApplicationHost` is a composition convenience around those ports. It drains
input, delegates one package step, submits one owned frame, and transports
bytes. The target adapter constructs the host; the game package supplies game
time, packet meaning, branch selection, and state semantics.

## Facade Boundary

`engine-api` serves external temporal library consumers through generic time,
SDK envelopes, journal/branch/index mechanics, controls, and state-first
presentation. Caravan values and target/host types remain in their consumer
layers, keeping the facade aligned with the generic crate map.