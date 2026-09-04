# Library Contract and Ownership Register

This document is the working register for the library-first boundary tracked by
`library-boundary.vine`. It records the current repository facts and the
contract decisions that constrain remediation. The core-contract task will
settle any remaining public type or crate-name choices; this register does not
promote current implementation convenience into a library obligation.

## Product Boundary

The reusable library is the product. A reference game, sample application,
target adapter, test, or benchmark may consume the library, but the library
must not acquire a dependency on those consumers to make the sample work.

The temporal library owns the following conceptual path:

```text
immutable context + immutable journal/worldline + LogicalTime
    -> owned GameState
GameState + Tau
    -> owned presentation output
```

The host library is adjacent to that path. It transports already-owned input,
bytes, and frames; it does not define game meaning, authoritative time, or
target selection.

## Ownership Classes

| Class | Owns | Does not own |
| --- | --- | --- |
| Temporal library | Fixed-point time, opaque SDK envelopes, immutable journals and branches, direct query mechanics, lookahead, persistence mechanics, and state-first presentation contracts | Caravan rules, target identity, native events, device state, or application loop policy |
| Host library | Target-neutral package and passive input, storage, lifecycle/resource, and render-sink ports | Game semantics, journal timestamps, worldline selection, target resolution, or backend types |
| Caravan reference implementation | Caravan geometry, journal vocabulary, rules, projection, fixtures, and game-specific persistence/reference APIs | Generic engine contract or target lifecycle |
| Sample application | Demonstrates a complete public-library composition and user-visible behavior | Requirements that exist only to make the sample convenient |
| Target adapter | Native lifecycle, event translation, backend/device execution, and target-local resources | Game meaning, authoritative state, or generic library semantics |
| Evidence | Tests, snapshots, benchmarks, reports, and dependency guards that consume the other classes | A production API or ownership decision hidden in a fixture |
| Planning/tooling | Specifications, proposals, VINE graphs, manifests, and CI orchestration | Runtime game or library state |

## Current Component Map

| Component | Owner class | Current role and boundary condition |
| --- | --- | --- |
| `engine-time` | Temporal library | Generic `LogicalTime`, `Tau`, checked arithmetic, and tick conversion. |
| `engine-sdk` | Temporal library | Generic immutable envelopes with opaque payloads. |
| `engine-journal` | Temporal library | Generic journal mechanics over opaque payloads; Caravan payloads are dev/test consumer values only. |
| `engine-branches` | Temporal library | Generic immutable branch mechanics over opaque context and journal payloads; Caravan specialization is owned by the reference layer. |
| `engine-index` | Temporal library | Generic direct query/index kernel over opaque journal sources and query results. |
| `engine-presentation` | Temporal library | Production renderer contract is generic; current Caravan dependencies are dev-dependencies used by tests. |
| `engine-api` | Temporal library | Generic facade for time, SDK envelopes, journal/branch/index mechanics, and state-first presentation. |
| `nworlds-host` | Host library | Dependency-free target-neutral ports and `ApplicationHost` composition. |
| `caravan-domain` | Caravan reference implementation | Caravan geometry, values, and closed journal payloads. |
| `caravan-vegetation` | Caravan reference implementation | Farmer, wheat, forester, forest, and wood rules. |
| `caravan-hazards` | Caravan reference implementation | Arsonist, fire, fighter, and arborist rules. |
| `caravan-seeded` | Caravan reference implementation | Deterministic Caravan journal fixtures. |
| `caravan-reference` | Caravan reference implementation | Caravan projection/oracle, discontinuity meanings, snapshots, and parity baseline. |
| `caravan-persistence` | Caravan reference implementation | Versioned Caravan worldline codec, branch lineage, save/load, and replay. |
| `caravan-demo` | Sample application | Developer-authored Stage/Orchestrator composition and terminal/native sample package. |
| `nworlds-desktop` | Target adapter | Windows `winit`/`wgpu` proof client using the target-neutral host and Caravan sample. |
| `engine-benchmarks` | Evidence | Release measurements for the current reference implementation and presentation path. |
| `purity-tests` | Evidence | Compiler and runtime checks for immutable/data-only boundaries. |
| `tests/conformance` | Evidence | Separate executable catalog for the Caravan anchor and library behavior. |
| `README.md`, `index.md`, `roadmap.md`, `spec/**`, `proposals/**`, `*.vine` | Planning/tooling | Design truth, ownership records, execution graphs, and repository navigation. |
| `target/**` | Generated output | Build artifacts; not a library or consumer boundary. |

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

Current production contamination register: **empty**. The remaining Caravan
references in engine manifests are dev-dependencies for reference fixtures and
are guarded separately.

The following references are dev-dependencies rather than production edges:

| Source test surface | Dev-dependencies | Treatment |
| --- | --- | --- |
| `engine-presentation` tests | `caravan-domain`, `caravan-reference`, `engine-journal` | Allowed only as reference fixtures; must not shape the production renderer contract. |
| `engine-benchmarks` and `tests/conformance` | Engine and Caravan crates | Evidence consumers; not library production dependencies. |
| `caravan-demo` | Engine, Caravan, and `nworlds-host` crates | Sample consumer; its dependency direction is expected. |
| `nworlds-desktop` | Caravan sample, host, and backend crates | Target-client consumer; its target and sample dependencies must not flow upward. |

## Forbidden Production Directions

These rules are the remediation test, independent of crate names:

```text
temporal library  -X-> Caravan reference implementation
temporal library  -X-> sample application
temporal library  -X-> host library or target adapter
temporal library  -X-> operating-system, window, device, or backend crate
host library      -X-> Caravan reference implementation or target adapter
host library      -X-> operating-system, window, device, or backend crate
sample/target     -->  approved library and host surfaces
evidence          -->  the surfaces it measures, without defining them
```

Dev-dependencies from an evidence or consumer crate may point at a reference
implementation when the test names that reference scope. They do not create a
public library contract and may not be copied into `[dependencies]` to avoid
designing a generic API.

## Contract Items To Settle Next

The following are constraints for the next graph tasks, not alternate designs:

- Generic payloads remain opaque at the SDK boundary; Caravan journal payloads
  remain owned by the Caravan reference implementation.
- `JournalWriter` owns authoritative timestamp assignment; query and
  presentation never mutate a worldline.
- Actual, counterfactual, and corrected histories remain immutable values.
- Direct state queries accept arbitrary logical times and remain independent of
  query order.
- Presentation accepts only `GameState` and `Tau`, then returns owned output.
- Host ports transport values and bytes but do not add host clock or device
  state to game-state or render production.
- The public facade must be consumable without importing `caravan-demo`,
  `nworlds-desktop`, or private implementation modules.

The core-contract task must turn these constraints into one public crate/type
map. The remediation tasks must then move or parameterize production code
according to that map rather than inventing a second generic engine.

## Settled Temporal Library Surface

The first isolated library surface is deliberately small:

| Crate | Public responsibility | Payload policy |
| --- | --- | --- |
| `engine-time` | `LogicalTime`, `Tau`, checked arithmetic, and game-tick conversion | No game or host types. |
| `engine-sdk` | `Context<C>`, `JournalEntry<P>`, `Journal<P>`, `Worldline<C, P>`, `GameState<S>`, `Frame<F>`, and query result envelopes | `C`, `P`, `S`, and `F` are opaque caller-owned values. |
| `engine-journal` | `Journal<P>` and `JournalWriter<P>` with monotonic authoring and immutable publication | `P` is generic; timestamp assignment is library-owned. Branching requires a cloneable payload. |
| `engine-branches` | `Branch<C, P>`, `Worldline<C, P>`, branch kind, immutable inclusive-prefix construction, and branch errors | `C` and `P` are generic; no game entry conversion. |
| `engine-index` | `JournalSource`, `QueryInput<C, P>`, `IndexedQuery<C, P>`, direct indexed state evaluation, and generic discontinuity pieces | Breakpoint payloads and query results are opaque. |
| `engine-presentation` | `Renderer<S>` and `present(GameState<S>, Tau) -> Frame<F>` | Renderer output is owned; only state and `Tau` enter production. |
| `engine-api` | A generic re-export facade for the supported temporal surface, if it removes real consumer friction | It may not re-export Caravan types. |

The public contract does not require a separate generic lookahead crate. A
future observation is the same direct query against an unchanged immutable
worldline. The former Caravan-only lookahead aliases now live in
`caravan-reference::lookahead`; no generic lookahead crate is retained.

The public contract does not require a generic binary persistence format for
opaque values. Persistence is split into two boundaries: a package-owned codec
that understands the package's context and journal payloads, and a host-owned
transport of encoded bytes. The current Caravan codec is therefore rehomed as
Caravan persistence rather than retained as a misleading generic engine crate.

The generic library must be usable by a consumer that supplies its own context,
journal payload, indexed query, and renderer. A Caravan reference alias may
make the same surface convenient for the sample, but it must be defined
outside the generic crates.

## Reference-Game Boundary

The Caravan layer owns the closed `GameJournalEntry` vocabulary, Caravan
geometry and values, actor/vegetation/hazard rules, discontinuity meanings,
reference projection, seeded fixtures, and any codec that serializes those
values. `caravan-reference` may expose convenience aliases such as
`ReferenceWorldline` and `State`, but those aliases are game-owned and must be
built from the generic library types.

`caravan-demo` owns the developer-authored `Stage`, `Orchestrator`, input
interpretation, Caravan transformations, and the sample renderer. It is a
consumer of the library and host contracts. Its existence is justified by the
complete public-library path it demonstrates, not by any type that the generic
crates need to import.

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

`ApplicationHost` is a composition convenience around those ports. It may
drain input, delegate one package step, submit one owned frame, and transport
bytes, but it may not assign game time, interpret packets, select branches,
construct game state, or inspect target metadata. A target adapter constructs
the host; a game package supplies the meaning.

## Facade Boundary

`engine-api` is retained only if it reduces friction for external temporal
library consumers. Its supported exports are generic time, SDK envelopes,
journal/branch/index mechanics, and state-first presentation. It must not
re-export `GameJournalEntry`, `ReferenceWorldline`, `Snapshot`, Caravan rules,
or target/host types. If a generic facade cannot provide that value without
duplicating the crate map, it is removed rather than kept as a Caravan facade.