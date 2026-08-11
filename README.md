# nworlds

nworlds is a Rust research/toy workspace for deterministic, directly indexed
temporal game worlds. **Caravan of Seasons** is the contained demo world used
to exercise immutable worldlines, indexed state queries, semantic input,
presentation, persistence, and a native Windows `wgpu` host.

This repository is deliberately a pure demo/toy for the engine. It is an
active proof-of-life and research implementation, not a production game, a
stable engine SDK, or a release distribution.

## Core Model

Authoritative state is queried from an immutable worldline:

```text
state(worldline, logical_time) -> game_state
```

Presentation is downstream:

```text
GameState + Tau -> Frame<RenderOutput>
```

The game-facing path is independent of transport and device choices. Native
input is normalized into ordered observations and then into a payload-only
`SemanticInputBatch`. Rendering consumes owned output; it does not become an
authoritative interaction surface.

## Run It

From the repository root:

```text
cargo test --workspace
cargo test --manifest-path tests/conformance/Cargo.toml
cargo run --manifest-path crates/caravan-demo/Cargo.toml
```

On Windows, the first native host slice uses `winit` and `wgpu`:

```text
cargo run --manifest-path crates/caravan-demo/Cargo.toml --bin caravan-windows
```

The native window renders the Caravan demo scene. Resize is handled by the
host; Space sends the first semantic interaction and changes the center tile.

## Repository Map

- [Roadmap](roadmap.md) - current position, settled constraints, and deferred work.
- [Initial specification](spec/initial.md) - temporal and immutable-state rules.
- [Presentation host](proposals/presentation-host.md) - target and adapter ownership.
- [Platform matrix](proposals/platform-support-matrix.md) - selected target profile.
- [Productization review](proposals/productization-review.md) - what is and is not production-bound.
- [Evidence matrix](evidence/clause-to-test.md) - executable and manual acceptance evidence.
- [Build graph](build.vine) - dependency-ordered anchor work.
- [Host graph](host.vine) - completed host and Windows `wgpu` composition.

## Development Checks

Before opening a pull request, run:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test --manifest-path tests/conformance/Cargo.toml --locked
cargo check --manifest-path crates/caravan-demo/Cargo.toml --bin caravan-windows --locked
```

The native window/device acceptance is currently manual local evidence. CI
checks the Windows binary compilation but does not claim automated pixel or
GPU-device behavior.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for the branch, review, testing, and
design-record practices. Security reports belong in [SECURITY.md](SECURITY.md),
not in public issues.

## License

The current source and documentation are released under the [MIT License](LICENSE).
