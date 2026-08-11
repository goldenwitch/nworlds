# Platform Support Matrix

This document records platform adaptation as a composition matrix. Each target
row records its target-specific entrypoint, independent adapter choices, owner,
evidence, and build/run condition.

## Ownership

The target-specific entrypoint selects one bundle of independent ports at
composition time. The game and generic engine do not branch on operating
system, windowing library, device, or render backend. A local
`ApplicationHost` value may group the selected ports for convenience; it is not
the support-matrix abstraction.

```text
target-specific entrypoint
    -> lifecycle/resource adapter
    -> platform input adapter
    -> input ingress
    -> render sink adapter
    -> storage transport
    -> Stage/Orchestrator
```

The adapters translate or execute platform work around the Stage composition.

## Matrix Axes

Every target row records these independent decisions:

| Axis | Records |
| --- | --- |
| Target profile | OS, runtime, architecture, entrypoint, and build condition |
| Lifecycle/resource | Process, window, surface, display, device, and resource lifecycle |
| Input translation | Native event source and `PlatformInputAdapter` |
| Input ingress | Packet transport and queue behavior |
| Render execution | `RenderSinkAdapter`, surface path, and backend |
| `wgpu` backend | Selected `wgpu` instance/backend requirements where applicable |
| Persistence codec | Game-facing worldline/save encoding and version policy |
| Storage transport | Encoded-byte destination and I/O condition |
| Evidence | Build, test, launch, and observable acceptance condition |

## Target Rows

No target platform is selected by this checkpoint. Rows remain explicit work
items until the user chooses a target regime.

| Target profile | Lifecycle/resource | Input | Ingress | Render/`wgpu` | Persistence codec | Storage transport | Evidence | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| First in-memory host proof | In-memory lifecycle | Abstract packets | `MemoryInputIngress` | `CollectingRenderSink`; no GPU | Anchor codec | `MemoryStorage` | Root workspace integration tests | selected, notstarted |
| First desktop target | TBD | TBD | TBD | TBD, including `wgpu` backend | Anchor codec or target requirement | TBD | TBD | unselected |
| Additional target | TBD | TBD | TBD | TBD | TBD | TBD | TBD | unselected |

The in-memory row proves independent port ownership and composition without
pretending to be a platform. Its persistence codec is game-facing; its
`MemoryStorage` cell is only byte transport. The desktop and additional rows
require a concrete target decision before implementation.

The in-memory row is selected as the first target composition but remains
notstarted. Other rows remain unselected until their target profile is chosen.