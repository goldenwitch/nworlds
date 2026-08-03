# Conformance Matrix

Run the executable proof package with:

```text
cargo test --manifest-path tests/conformance/Cargo.toml
cargo run --manifest-path tests/conformance/Cargo.toml -- --report evidence/conformance-report.json
```

The first command runs every catalogued case. The second command reruns the
same cases and writes machine-readable results. The existing demo integration
test additionally checks the complete stdout trace against
`crates/caravan-demo/snapshots/anchor-trace.txt`.

| Clause | Executable case | Evidence | Coverage |
| --- | --- | --- | --- |
| Void, empty journal, exact `t_` | `empty-journal` | `checks.rs` | Runtime pass |
| Journal-owned creation and 91 tiles | `create-saucer` | `checks.rs` | Runtime pass |
| Target inclusion, postdating, equal-time order | `journal-time` | `checks.rs` | Runtime pass |
| Arbitrary and non-monotonic query order | `query-order` | `checks.rs` | Runtime pass |
| Fixed-journal stability without a crossed boundary | `within-tick` | `checks.rs` | Runtime pass: distinct 4,000/4,500 fixed-point samples share tick 4 and automaton data while retaining distinct logical times |
| Exact journal-time discontinuity | `journal-discontinuity` | `checks.rs` | Runtime pass |
| Independent terrain, actor, and effect layers | `layer-separation` | `checks.rs` | Runtime pass |
| Farmer, wheat, forester, fire, fighter, arborist differences | `farmer`, `wheat`, `forester`, `arsonist-fire`, `fighter`, `arborist` | `checks.rs` | Runtime pass |
| Seeded journal construction and reproducibility | `seeded` | `checks.rs` | Runtime pass; RNG ownership is behavioral evidence |
| Prefix agreement and parent/child branch isolation | `branches` | `checks.rs` | Runtime pass |
| Fixed-journal future query and lookahead | `lookahead` | `checks.rs` | Runtime pass |
| Playback, reverse scrub, rendering, animation, branch presentation | `presentation` | `checks.rs` | Runtime pass |
| Runnable anchor observables | `demo-trace` | `anchor-trace.txt` and demo snapshot test | Runtime pass |

## Explicit Gaps

- Runtime tests cannot establish compiler-enforced purity, forbidden callback
  injection, or mutation impossibility. The roadmap places those checks in the
  later purity-hardening stage.
- `LogicalTime` and `Tau` are distinct APIs and are exercised as such, but this
  package does not add compile-fail tests for accidental interchange.