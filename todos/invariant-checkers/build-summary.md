# Invariant checkers — build summary

- Branch: `feature/invariant-checkers`; base `main` @ d9273e2 (PR #1 and PR #2 merged). Worktree: `facetDB-feature-invariant-checkers` (reused; no `../invariant-checkers-work`).
- Commits (main..HEAD): locked acceptance tests + plan; `invariants` module; docs (CLAUDE.md section, decisions, this summary); simplify pass (`check_same_rows` walks key runs over one sorted list: no per-key allocation, unstable sort).
- Plan: `docs/invariant-checkers/plan.md`. Decisions: `todos/invariant-checkers/decisions.md`.

## Pre-merge verification

- **Tests:** `cargo test --workspace`: facet-core 52 unit tests (25 new in `invariants`, including 5 proptests) + 1 `compile_fail` doctest; 1 each in bench, check, delta, engine, log, planner; 0 in layouts. All passing.
- **Acceptance:** all 9 files under `acceptance/` pass, run one by one with `node --test`: invariant-checkers AC-1..AC-5 (1/1 each), core-data-types 1 + 1 + 5, cargo-workspace 8.
- **AC-6:** `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo build --workspace` ok; `cargo bench -p facet-bench --no-run` ok.
- **Docs:** `RUSTDOCFLAGS="-D warnings" cargo doc -p facet-core --no-deps` clean (the module table's intra-doc links resolve).
- **Dependencies:** facet-core still has no normal dependencies (proptest stays dev-only). No forbidden crates (`cargo tree --workspace -e normal,dev`).
- **Locked files:** the 10 `acceptance/invariant-checkers` files are byte-identical to the approved versions (`shasum -c` against hashes taken before implementation).
- **PROGRESS.md:** unchanged, title-only.

## Coverage of the brief

| Brief item | Where |
|---|---|
| `AckEvent`, `Violation` (Debug, Clone, PartialEq, Display; also `Error`) | `crates/core/src/invariants.rs:26`, `:35`, `:142` |
| `check_ack_after_durable` | `invariants.rs:147` |
| `check_watermarks` | `invariants.rs:168` |
| `check_same_rows` (by key, bitwise floats, duplicates differ) | `invariants.rs:197`, helpers `:309`-`:362` |
| `check_segments_unchanged`, `segment_digest` (FNV-1a 64) | `invariants.rs:241`, `:264` |
| `check_batch_sizes` | `invariants.rs:276` |
| One pass + one fail per variant | `invariants.rs:365` onward; Display tested for every variant with an exhaustiveness guard at `:752` |
| One proptest per checker | `invariants.rs:960` |
| Module doc table | `invariants.rs:1`-`:17` |
| CLAUDE.md "Invariant checkers" section | `CLAUDE.md:33`-`:44` |

## Deviations from plan

- None in scope. Additions: `impl std::error::Error for Violation` (matches `SchemaError`/`RowError`), and an extra FNV-1a test value (`"foobar"`).

## Notes for review

- `check_same_rows` compares floats bitwise, as the brief specifies: `0.0` vs `-0.0` and different NaN payloads count as different. A layout that canonicalises floats would fail it; the owning feature decides if that is a bug or the checker needs a mode.
- `check_batch_sizes` reports a batch that overshoots the rows left as `TotalMismatch`, not `ShortNotFinal` (it took all of them, and more).
- `lib.rs:7` places `pub mod invariants;` alphabetically among the private modules.
