# Contributing

Thank you for helping with nworlds. The repository is early and design clarity
matters as much as code volume.

## Before You Start

- Open an issue or discussion for a substantial behavior, API, or architecture
  change.
- Check the active roadmap and VINE graph before starting work.
- Keep game meaning independent of host, transport, storage, and device choices.
- Do not commit credentials, tokens, private data, generated build output, or
  local machine configuration.

## Workflow

- Create a focused branch from `main`.
- Keep commits small enough to review and describe the evidence they add.
- Update the owning proposal or VINE task when behavior or ownership changes.
- Open a pull request against `main`.
- Maintainers may perform an administrative merge while the project has a small
  contributor base; normal changes still go through the same review and CI
  requirements.

## Required Checks

Run these from the repository root:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test --manifest-path tests/conformance/Cargo.toml --locked
cargo check --manifest-path crates/caravan-demo/Cargo.toml --bin caravan-windows --locked
```

Also run `git diff --check` and validate any edited VINE graph. Keep manual
Windows observations separate from automated test claims.

## Pull Requests

A pull request should explain:

- the problem or requirement that binds the change;
- the owning layer and files changed;
- tests, measurements, or manual observations performed; and
- any remaining limitation or deferred decision.

Avoid unrelated formatting churn and do not introduce a generic abstraction
until a concrete repeated boundary requires it.
