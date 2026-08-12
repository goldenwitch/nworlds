# nworlds

nworlds is a Rust research/toy workspace for deterministic, directly indexed
temporal game worlds. **Caravan of Seasons** is the contained demo world used
to exercise immutable worldlines, indexed state queries, semantic input,
presentation, and persistence.

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

## Developer Path

The intended target-neutral game-development path is:

```text
nworlds test
nworlds run
nworlds package
```

The host resolves the local environment and mints or reuses the appropriate
artifact. Developers do not select operating systems, architectures, windowing
libraries, or GPU backends. These commands are the desired public workflow;
the target factory is the next host design surface.

## Repository Checks

The current proof repository is maintained with:

```text
cargo test --workspace --locked
cargo test --manifest-path tests/conformance/Cargo.toml --locked
```

Target-specific launch and device evidence belongs to host CI and the target
factory, not to the game package's public workflow.

## Repository Map

- [Roadmap](roadmap.md) - current position, settled constraints, and deferred work.
- [Initial specification](spec/initial.md) - temporal and immutable-state rules.
- [Presentation host](proposals/presentation-host.md) - target and adapter ownership.
- [Platform matrix](proposals/platform-support-matrix.md) - host-owned target profiles.
- [Productization review](proposals/productization-review.md) - what is and is not production-bound.
- [Evidence matrix](evidence/clause-to-test.md) - executable and manual acceptance evidence.
- [Build graph](build.vine) - dependency-ordered anchor work.
- [Host graph](host.vine) - first host proof composition; target minting is
  owned by the target factory.
- [Desktop host client](crates/nworlds-desktop/Cargo.toml) - target-local
  `winit`/`wgpu` composition mapped over the target-neutral host contract.
- [Host runtime](crates/nworlds-host) - target-neutral package contract and
  independent host-port composition.
- [Target factory proposal](proposals/target-factory.md) - the desired
  target-neutral developer path and host-owned artifact minting.
- [Target factory plan](target-factory.vine) - the dependency-ordered contract
  and migration decisions for host-owned target minting.
- [Support graph](support.vine) - committed desktop targets, explicit gaps, and
  native or appropriate CI work.
- [Demo gameplay plan](gameplay.vine) - the design-derived inventory and
  ordered plan for the remaining player-facing demo loop.

## Development Checks

Before opening a pull request, run:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test --manifest-path tests/conformance/Cargo.toml --locked
cargo check --workspace --all-targets --locked
```

Target build and runtime evidence is host-owned and recorded separately from
game-package semantics.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for the branch, review, testing, and
design-record practices. Security reports belong in [SECURITY.md](SECURITY.md),
not in public issues.

## License

The current source and documentation are released under the [MIT License](LICENSE).
