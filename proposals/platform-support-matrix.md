# Platform Support Matrix

This document records host-owned platform adaptation as a composition matrix.
Each target row records the target profile that the nworlds target factory may
resolve, its host adapters, owner, evidence, build/run condition, and explicit
support gap. It is not a game-package developer interface.

## Ownership

The target factory selects or mints one target-specific entrypoint and bundle
of independent ports at composition time. The game and generic engine do not
branch on operating system, windowing library, device, or render backend. A local
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

## Scope and Status

The current support commitment is a closed desktop universe:

- Windows x86_64;
- Linux x86_64 on an Ubuntu-compatible distribution;
- Linux x86_64 on an Arch/SteamOS-compatible distribution;
- SteamOS 3 on physical Steam Deck hardware; and
- macOS x86_64 and Apple Silicon.

`complete` means the target has a native entrypoint, native runtime evidence,
and appropriate CI. `gap` means the target is in the support commitment but
does not yet have all of those things. `out-of-scope` is an explicit gap with
no active implementation plan in this support cycle; it is not an accidental
omission.

The planned work for every committed `gap` row is owned by
[support.vine](../support.vine). Build-only evidence never upgrades a row to
`complete`; runtime/device acceptance is required.

## Target Rows

The Windows composition uses `winit` for window and input events and `wgpu`
for render execution. Those choices remain below the Stage and game-facing
renderer boundaries on every desktop target.

| Target profile | Lifecycle/resource | Input | Ingress | Render/`wgpu` | Persistence codec | Storage transport | Evidence / CI | Status / gap |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| First in-memory host proof | In-memory lifecycle | Abstract packets | `MemoryInputIngress` | `CollectingRenderSink`; no GPU | Anchor codec | `MemoryStorage` | Root workspace integration tests | complete |
| Windows desktop (`x86_64-pc-windows-msvc`) | `winit` window/event loop; `wgpu` surface, device, queue, resize, and shutdown | `winit` native events -> `PlatformInputAdapter` -> `InputPacket` | `MemoryInputIngress`, fed by the native adapter | `WgpuRenderSink`; platform-default adapter selection | Anchor codec | `MemoryStorage` for the first executable slice | Native Windows launch; manual frame, input, resize, and shutdown evidence; Windows host CI build | complete |
| Linux desktop (`x86_64-unknown-linux-gnu`, Ubuntu-compatible) | `winit` X11/Wayland lifecycle; `wgpu` surface, device, queue, resize, and shutdown | Native events -> `PlatformInputAdapter` -> `InputPacket` | Target ingress adapter | `WgpuRenderSink`; Vulkan/GL backend selection | Anchor codec | Target transport TBD | `ubuntu-latest` build/test CI plus native X11 and Wayland runtime evidence | gap: no Linux entrypoint or runtime evidence |
| Arch Linux / SteamOS-family desktop (`x86_64-unknown-linux-gnu`) | `winit` Wayland/X11 lifecycle; `wgpu` surface, device, queue, resize, and shutdown | Native events -> `PlatformInputAdapter` -> `InputPacket` | Target ingress adapter | `WgpuRenderSink`; Vulkan backend preferred, fallback measured | Anchor codec | Target transport TBD | Arch-compatible build CI plus native Wayland runtime evidence | gap: Arch build lane added; runtime evidence incomplete |
| SteamOS 3 / Steam Deck (`x86_64-unknown-linux-gnu`) | SteamOS/gamescope Wayland lifecycle; `wgpu` surface, device, queue, resize, and shutdown | Deck keyboard/controller events -> `PlatformInputAdapter` -> `InputPacket` | Target ingress adapter | `WgpuRenderSink`; Deck-supported Vulkan path | Anchor codec | Target transport TBD | Physical Steam Deck run and a self-hosted Deck or SteamOS runner where practical | gap: no Steam Deck hardware or self-hosted CI evidence |
| macOS Intel (`x86_64-apple-darwin`) | `winit` lifecycle; `wgpu` surface, device, queue, resize, and shutdown | Native events -> `PlatformInputAdapter` -> `InputPacket` | Target ingress adapter | `WgpuRenderSink`; Metal backend | Anchor codec | Target transport TBD | Native Intel macOS runner build plus local runtime evidence | gap: no macOS entrypoint or runtime evidence |
| macOS Apple Silicon (`aarch64-apple-darwin`) | `winit` lifecycle; `wgpu` surface, device, queue, resize, and shutdown | Native events -> `PlatformInputAdapter` -> `InputPacket` | Target ingress adapter | `WgpuRenderSink`; Metal backend | Anchor codec | Target transport TBD | Native Apple Silicon macOS runner build plus local runtime evidence | gap: no arm64 entrypoint or runtime evidence |
| Web (`wasm32-unknown-unknown`) | Browser lifecycle and canvas surface | Browser events -> `PlatformInputAdapter` -> `InputPacket` | Browser ingress adapter | `WgpuRenderSink`; WebGPU/WebGL path | Anchor codec | Browser storage TBD | No browser build or runtime contract in this cycle | out-of-scope: no web product requirement |
| Android (`aarch64-linux-android`) | Mobile lifecycle and surface | Android events -> `PlatformInputAdapter` -> `InputPacket` | Mobile ingress adapter | `WgpuRenderSink`; Vulkan/GLES path | Anchor codec | Mobile storage TBD | No mobile entrypoint or device lab in this cycle | out-of-scope: no Android requirement |
| iOS (`aarch64-apple-ios`) | Mobile lifecycle and surface | iOS events -> `PlatformInputAdapter` -> `InputPacket` | Mobile ingress adapter | `WgpuRenderSink`; Metal path | Anchor codec | Mobile storage TBD | No mobile entrypoint or device lab in this cycle | out-of-scope: no iOS requirement |
| Other architectures and consoles | Target-specific | Target-specific | Target-specific | Target-specific | Anchor codec or target requirement | Target-specific | No target regime or CI owner | out-of-scope: no target requirement |

The in-memory row proves independent port ownership without pretending to be a
platform. Its persistence codec is game-facing; its `MemoryStorage` cell is
only byte transport. The Windows row is the first native execution slice and
reuses the same Stage, Orchestrator, semantic input batch, worldline, and
backend-neutral `CaravanRenderer` that the planned desktop rows must preserve.