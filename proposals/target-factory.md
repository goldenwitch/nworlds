# Target Factory Proposal

This proposal defines the nworlds host that turns a target-neutral game package
into a runnable artifact. Game developers do not select operating systems,
architectures, window systems, GPU backends, or recipient hardware as part of
game composition.

This proposal owns target minting. The [presentation host](presentation-host.md)
owns runtime ports and adapters. The [platform support matrix](platform-support-matrix.md)
owns declared target regimes and evidence. The game owns its world and
meaning. The reusable temporal and host-library boundary is recorded in
[library-contract.md](library-contract.md); this proposal consumes that
boundary and does not define it.

## Boundary

```text
GamePackage
  -> nworlds TargetFactory
    -> internal TargetProfile resolution
    -> HostContract
      -> input abstraction
      -> storage abstraction
      -> lifecycle/resource abstraction
      -> renderer-agnostic render abstraction
    -> generated or selected target composition
    -> TargetArtifact
      -> GamePackage + HostContract
      -> renderer-agnostic render batch
        -> target RenderSink
          -> wgpu/backend instructions
```

The game package is target-neutral. The host supplies every environmental
capability the game needs through `HostContract`; the game does not select or
construct those capabilities. A target profile is host-owned metadata, not a
game API. A target artifact may be native, packaged, or launched through a
host-managed runtime; the recipient should not need to know its architecture
or backend.

## Developer Experience

The public game-development path is target-neutral:

```text
nworlds test
nworlds run
nworlds package
```

`nworlds test` validates the game package and its semantic evidence. `nworlds
run` resolves the local environment, mints or reuses a compatible artifact,
and launches it. `nworlds package` mints artifacts for supported environments.
Normal use does not require a target flag, architecture name, windowing
library, or GPU backend.

These commands are the desired host contract. The current repository contains
the isolated temporal library, the target-neutral host library, and the Caravan
reference/demo proof client while the target factory is being designed;
repository maintenance commands are not the public game-development path.

The authoritative game boundaries remain unchanged:

```text
Worldline + LogicalTime -> GameState
GameState + Tau -> minimal fire-and-forget renderer-agnostic render batch
  -> Frame -> target RenderSink -> backend instructions
```

The target factory must not add journals, host state, device state, or platform
metadata to game-state production or render-batch production. The render sink
receives only the minimal renderer-agnostic batch and translates it into the
appropriate target instructions.

## Recommended Shape

Use static host composition behind a target-neutral command surface. The first
implementation should not require a plugin ABI, dynamic game loading, or a
runtime object registry:

```text
nworlds test
  -> GamePackage semantic tests and evidence

nworlds run
  -> discover local RuntimeCapabilities
  -> resolve TargetProfile
  -> mint or reuse TargetArtifact
  -> launch generated host composition

nworlds package
  -> select requested supported profiles internally
  -> mint named TargetArtifacts
```

The factory may generate a small target composition crate internally. That
generated detail is host machinery and is not part of the game package.

`HostContract` is a family of narrow typed abstractions supplied by the host,
not a broad mutable capabilities object. It covers the environmental things a
game needs: input, byte storage, lifecycle/resource access, and a minimal
renderer-agnostic draw vocabulary. Concrete target adapters implement those
abstractions; game code does not select their implementations.

## Consumer Inventory

The contract is discovered from concrete consumers. A row records the current
proof boundary; its candidate abstraction is not a settled API merely because
the proof uses a particular Rust type.

| Consumer need | Current consumer | Direction | Current owner | Lifetime | Target-neutrality requirement | Candidate abstraction | Not yet settled |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Semantic input delivery | `ApplicationHost::step`, `InputIngress`, and the Windows `PlatformInputAdapter` | Host -> game | Target adapter translates native events; ingress transports packets; Orchestrator normalizes and retains semantic observations | Ingress lives for one host composition; ordered batches and input windows are owned values per interaction attempt | `InputPacket`, `SemanticInputBatch`, and interaction logic contain no native event, window, device, or target identity | Package-defined neutral observations delivered through an ingress | How a package declares its input vocabulary and how a host maps native events |
| Encoded persistence transport | `ApplicationHost::save_selected`/`load_selected`, `StorageTransport`, and `MemoryStorage` | Game <-> host | Game persistence codec owns meaning; storage adapter owns bytes | Encoded records are owned across one save/load operation; storage lifetime is host-selected | The game sees owned bytes or a typed codec result, never a file handle, path policy, or platform storage object | Owned bytes in and out | File/package lifetime and user-data location |
| Render intent | `CaravanRenderer`, `RenderSinkAdapter`, and the desktop `WgpuRenderSink` | Game -> host | Renderer projects `GameState + Tau`; sink executes or collects the owned frame | `Frame<RenderOutput>` is owned by one submission and may be copied, queued, or discarded | Render output contains no journal, worldline, Orchestrator, input, branch selector, device, or host-clock state | Host-defined owned `RenderBatch` from `GameState + Tau` | Smallest geometry/appearance vocabulary, coordinate semantics, and whether the first batch is game-defined or host-defined |
| Lifecycle and resource execution | Windows `NativeApplication` and `WgpuRenderSink` | Host -> composition | Target entrypoint owns process/window lifecycle; target sink owns device/surface resources | Process, window, surface, device, and queue lifetimes are target-local | No lifecycle, window, device, backend, or architecture type crosses into game meaning unless a named consumer later requires it | Host-owned launch and execution flow, outside the first game contract | Which lifecycle observations, if any, a game actually consumes |

There is no current game consumer for a generic asset loader, audio port,
camera service, widget system, device handle, or runtime-diagnostics port.
Those concerns remain factory or host responsibilities until a named consumer
requires a narrower crossing. `engine-api` exposes generic query, journal,
branch, time, and presentation values; Caravan domain values and reference
aliases live in the Caravan consumer layer. It exposes no target or host
capability.

The current proof therefore demonstrates three game-facing host crossings:
neutral input transport, encoded byte transport, and owned render submission.
`ApplicationHost` is a proof-local convenience bundle around those crossings;
the generic composition now lives in `nworlds-host` and is not a Caravan-owned

## Implemented Static Composition

The first target-neutral host composition is implemented in the `nworlds-host`
crate. It keeps the host contract as independent typed ports and lets the
package retain semantic control:

```text
GamePackage
  ingest_batch(package-defined batch)
  step() -> (accepted, owned frame)
  save_selected() -> encoded bytes
  load_selected(encoded bytes)

InputIngress -> package-defined batch
GamePackage -> Frame
StorageTransport <-> encoded bytes
RenderSink<Frame> <- owned frame
```

`nworlds-host::ApplicationHost<P, I, S, R>` drains `I`, delegates input and
control to `P`, submits the returned frame to `R`, and transports persistence
bytes through `S`. It does not know Caravan state, journal facts, logical time,
render objects, native events, windows, devices, or backends. The package owns
the order of interaction, publication, state selection, and presentation
inside its `step` implementation.

The Caravan mapping is `caravan-demo::CaravanPackage`, a target-neutral alias
for its existing Stage/Orchestrator/renderer composition. The desktop proof
now requests `caravan_demo::demo_package()` and uses the generic host; its
remaining code is limited to native event translation and `wgpu` frame
execution. This is a host-client mapping, not a second Caravan game model.

The host-owned render vocabulary is the crossing:

```text
GameState + Tau
  -> host-defined minimal renderer-agnostic render batch
  -> Frame<RenderBatch>
  -> target RenderSink
  -> wgpu/backend instructions
```

The game supplies draw intent using the host-defined vocabulary. The target
sink translates that intent into backend instructions. The render batch is
owned, fire-and-forget, and has no journal, worldline, Orchestrator, input,
branch-selection, device, or host-clock state.

## Ownership

- **GamePackage** supplies game meaning, immutable world/query behavior, and
  target-neutral composition against `HostContract` abstractions.
- **TargetFactory** resolves profiles, creates target compositions, and reports
  unsupported environments.
- **HostContract** supplies the environmental capabilities consumed by a game;
  its abstractions do not expose target implementation details.
- **TargetProfile** records host-internal OS, architecture, runtime, and backend
  choices; it is not visible to game logic.
- **TargetArtifact** is the build or distribution result for one resolved
  environment.
- **RenderSink** receives the minimal renderer-agnostic render batch and
  translates it into target/backend instructions.
- **Presentation host** supplies runtime lifecycle, input, storage, and render
  capabilities selected by the factory.
- **Platform matrix** records which profiles are supported and what evidence
  exists; it does not require each game to assemble those profiles.

The target-factory concepts are distinct:

```text
TargetProfile        static build/deployment recipe
RuntimeCapabilities  actual local machine/device facts
TargetResolution     profile + capabilities -> adapters or explicit failure
TargetArtifact       minted runnable or distributable result
```

## Decisions To Settle

1. **Game contract**: Specify the minimal target-neutral package boundary and
  the narrow `HostContract` abstractions it may consume.
2. **Package declaration**: Specify how identity, assets, save/schema version,
  host version, and render-vocabulary requirements are declared without
  declaring platforms.
3. **Render vocabulary**: Specify the smallest host-owned draw vocabulary and
  its `Frame<RenderBatch>` ownership/lifetime rules.
4. **Composition**: Specify the generated static composition and keep it out
  of the game package source tree.
5. **Selection**: Specify build-time profile selection, distribution-time
  artifact selection, and runtime capability resolution as separate stages.
6. **Unsupported environments**: Specify a stable host-owned resolution error
  containing available/required capabilities and remediation.
7. **CI and artifacts**: Specify how CI mints, names, tests, publishes, and
  retains one artifact per supported profile, with device evidence separate
  from compilation evidence.

## Constraints

- Game code contains no target-selection branch and does not depend on a
  platform crate.
- Game code consumes host capabilities through target-neutral abstractions; it
  does not construct platform resources or target adapters.
- Target support is a host/distribution capability, not a per-game design task.
- Runtime adapter ownership remains separate from target artifact minting.
- The target RenderSink translates renderer-agnostic render abstractions into
  backend instructions; the game never emits `wgpu` or platform commands.
- A compile result is not runtime or device support evidence.
- Unsupported environments fail at the host boundary with an explicit result.
- Existing Caravan composition remains a client of this contract; it is not
  the contract itself.

## Deferred Implementation

The dependency-ordered design and implementation work is recorded in
[target-factory.vine](../target-factory.vine). No target-factory crate,
generated entrypoint, or package manifest is implemented by this proposal;
those are downstream artifacts of the decisions and acceptance evidence.
