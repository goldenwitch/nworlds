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

The first executable target is selected for local Windows development. The
native composition uses `winit` for window and input events and `wgpu` for
render execution; those choices remain below the Stage and game-facing
renderer boundaries.

| Target profile | Lifecycle/resource | Input | Ingress | Render/`wgpu` | Persistence codec | Storage transport | Evidence | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| First in-memory host proof | In-memory lifecycle | Abstract packets | `MemoryInputIngress` | `CollectingRenderSink`; no GPU | Anchor codec | `MemoryStorage` | Root workspace integration tests | complete |
| Windows desktop (`x86_64-pc-windows-msvc`) | `winit` window/event loop; `wgpu` surface, device, queue, resize, and shutdown | `winit` native events -> `PlatformInputAdapter` -> `InputPacket` | `MemoryInputIngress`, fed by the native adapter | `WgpuRenderSink`; `wgpu` platform-default adapter selection | Anchor codec | `MemoryStorage` for the first executable slice | Local `cargo run` opens a window, renders a frame, accepts input, and survives resize; workspace tests remain green | complete |
| Additional target | TBD | TBD | TBD | TBD | TBD | TBD | TBD | unselected |

The in-memory row proves independent port ownership and composition without
pretending to be a platform. Its persistence codec is game-facing; its
`MemoryStorage` cell is only byte transport. The Windows row is the first
native execution slice and reuses the same Stage, Orchestrator, semantic input
batch, worldline, and backend-neutral `CaravanRenderer`.

The Windows row is complete for the first local execution slice. Additional
rows remain unselected until their target profile is chosen.