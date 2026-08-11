# Target Factory Proposal

This is a design draft for the nworlds host that turns a target-neutral game
package into a runnable artifact. It exists because game developers should not
select operating systems, architectures, window systems, GPU backends, or
recipient hardware as part of game composition.

This proposal owns target minting. The [presentation host](presentation-host.md)
owns runtime ports and adapters. The [platform support matrix](platform-support-matrix.md)
owns declared target regimes and evidence. The game owns its world and
meaning.

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

These commands are the desired host contract. The current repository remains
the engine/demo proof substrate while the target factory is being designed;
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

## Questions To Settle

1. **Game contract**: What minimal target-neutral value/trait does a game package
  expose to the factory, and which `HostContract` abstractions does it consume?
2. **Package declaration**: How does a game declare its package, executable
   identity, assets, and persistence requirements without declaring platforms?
3. **Composition**: Does the factory generate a target entrypoint, select a
   generic host executable, or use another static composition mechanism?
4. **Selection**: Which decisions happen at build time, distribution time, and
   runtime capability detection?
5. **Host capability and render crossing**: What minimal `HostContract` does a
  game consume, and what renderer-agnostic render batch does the host provide
  for the game to produce from `GameState + Tau` before a target `RenderSink`
  translates it into backend instructions?
6. **Unsupported environments**: What stable host-level result reports that no
   compatible artifact, adapter, or backend exists?
7. **CI and artifacts**: How does CI mint, name, test, publish, and retain one
   artifact per supported target profile, including manual or self-hosted
   device evidence where hosted runners are insufficient?

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

No target-factory crate, generated entrypoint, generic host trait, or package
manifest is selected by this draft. Those are consequences of the seven
questions above and should be added only after their ownership and acceptance
evidence are settled.
