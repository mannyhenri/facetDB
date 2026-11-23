# Invariant checkers — build summary

- Branch: `feature/invariant-checkers`; base `main` @ d9273e2 (PR #1 and PR #2 merged). Worktree: `facetDB-feature-invariant-checkers` (reused; no `../invariant-checkers-work`).
- Commits (main..HEAD): locked acceptance tests + plan; `invariants` module; docs (CLAUDE.md section, decisions, this summary); simplify pass (`check_same_rows` walks key runs over one sorted list: no per-key allocation, unstable sort); pre-merge verification (kind-order test, this summary); review triage (P3 fixes: checked batch total, CLAUDE.md import exception).
- Plan: `docs/invariant-checkers/plan.md`. Decisions: `todos/invariant-checkers/decisions.md`.

## Pre-merge verification

- **Tests:** 61 passing / 0 failing with `cargo test --workspace`. That is facet-core 54 unit tests (27 in `invariants`, 5 of them proptests) + 1 `compile_fail` doctest, and 1 each in bench, check, delta, engine, log, planner (0 in layouts). Acceptance: 20 passing / 0 failing across all 9 files under `acceptance/`, each run with `node --test`: invariant-checkers AC-1..AC-5 (1 each), core-data-types 1 + 1 + 5, cargo-workspace 8.
- **Type check:** clean, `cargo check --workspace --all-targets` (0 errors, 0 warnings).
- **Lint:** clean, `cargo clippy --workspace --all-targets -- -D warnings` (0 issues) and `cargo fmt --all --check`.
- **Docs:** `RUSTDOCFLAGS="-D warnings" cargo doc -p facet-core --no-deps` clean (the module table's intra-doc links resolve).
- **Coverage:** not measured as a percentage. No coverage tool (cargo-llvm-cov, tarpaulin) is installed and the project sets no threshold.
  - A manual audit of `crates/core/src/invariants.rs` found every public function and private helper exercised by unit or acceptance tests. Every branch is reached: each `Violation` path, both merge-end arms of `check_same_rows`, duplicate ids in `check_segments_unchanged`, and the overflowing total in `check_batch_sizes`.
  - The one gap was `kind_rank`'s Float/Bytes/Vector/Null arms (cross-kind comparison only covered Int vs Str). It gained a set-based test in this step: `cmp_values_orders_every_pair_of_kinds_by_rank` (`invariants.rs:599`).
- **Bench:** `cargo bench -p facet-bench --no-run` compiles.
- **Dependencies:** facet-core still has no normal dependencies (proptest stays dev-only). No forbidden crates (`cargo tree --workspace -e normal,dev`).
- **Locked files:** the 10 `acceptance/invariant-checkers` files are byte-identical to the approved versions (`shasum -c` against hashes taken before implementation).
- **PROGRESS.md:** unchanged, title-only.

## Files changed (vs main)

- `crates/core/src/invariants.rs` (new), `crates/core/src/lib.rs` (`pub mod invariants;` at `lib.rs:7`)
- `CLAUDE.md` ("Invariant checkers" section)
- `docs/invariant-checkers/plan.md`
- `todos/invariant-checkers/decisions.md`, `todos/invariant-checkers/build-summary.md`
- `acceptance/invariant-checkers/*` (10 locked files, committed unchanged)

## Coverage of the brief

| Brief item | Where |
|---|---|
| `AckEvent`, `Violation` (Debug, Clone, PartialEq, Display; also `Error`) | `crates/core/src/invariants.rs:26`, `:35`, `:142` |
| `check_ack_after_durable` | `invariants.rs:147` |
| `check_watermarks` | `invariants.rs:168` |
| `check_same_rows` (by key, bitwise floats, duplicates differ) | `invariants.rs:197`, helpers `:309`-`:362` |
| `check_segments_unchanged`, `segment_digest` (FNV-1a 64) | `invariants.rs:241`, `:264` |
| `check_batch_sizes` | `invariants.rs:276` |
| One pass + one fail per variant | `invariants.rs:365` onward; Display tested for every variant with an exhaustiveness guard at `:785` |
| One proptest per checker | `invariants.rs:993` |
| Module doc table | `invariants.rs:1`-`:17` |
| CLAUDE.md "Invariant checkers" section | `CLAUDE.md:33`-`:44` |

## Deviations from plan

- None in scope.
- Additions: `impl std::error::Error for Violation` (matches `SchemaError`/`RowError`), an extra FNV-1a test value (`"foobar"`), and the kind-order test above.
- Steps 1–12 landed as whole commits rather than one WIP commit per step.

## Review outcome

Triage in `p1.md` (none), `p2.md` (none) and `p3.md` (4, all resolved): the checked batch total fixes a false pass at `usize::MAX`; CLAUDE.md:24 names the `facet_core::invariants` import exception; `cmp_values` promotion and `RowsDiffer` Display size are recorded as decisions.

## Notes for review

- `check_same_rows` compares floats bitwise, as the brief specifies: `0.0` vs `-0.0` and different NaN payloads count as different. A layout that canonicalises floats would fail it; the owning feature decides if that is a bug or the checker needs a mode.
- `check_batch_sizes` reports a batch that overshoots the rows left as `TotalMismatch`, not `ShortNotFinal` (it took all of them, and more).
- `acceptance/cargo-workspace/ac-1-scaffold-runs.test.mjs:177` requires a clean working tree, so run the acceptance set after committing.
