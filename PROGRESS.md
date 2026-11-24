# Facet progress

## Schedule

| Phase | Name | Gate | Budget (min) | Cut if over budget (section 7) |
|---|---|---|---|---|
| 0 | Setup | `./run_gates.sh 0` | — | — |
| 1 | Log | `./run_gates.sh 1` | — | — |
| 2 | Delta and snapshots | `./run_gates.sh 2` | — | — |
| 3 | Layout contract and row layout | `./run_gates.sh 3` | — | — |
| 4 | Column layout | `./run_gates.sh 4` | — | run-length and bit-packing |
| 5 | Vector layout | `./run_gates.sh 5` | — | — |
| 6 | Planner | `./run_gates.sh 6` | — | — |
| 7 | Investment loop | `./run_gates.sh 7` | — | — |
| 8 | Benchmarks | none | — | — |
| 9 | Report and definition of done | none | — | — |

Rebuilt from the project's feature list because the build spec is not in the repo: fill in the budgets and section 7 cuts from the spec.

## Decisions

- Phase 0: crate packages are named facet-<dir> (facet-core, facet-log, ...) so they do not shadow Rust's core crate or the crates.io log crate; directory names stay as in the spec (source: docs/solutions/cargo-workspace.md)
- Phase 0: Value derives PartialEq only, no Eq/Ord/Hash, because of the f64; Row, ChangeBatch and ColumnBatch follow it (source: todos/core-data-types/decisions.md)
- Phase 0: column nulls are placeholders in the typed array plus a cleared validity bit, so every array has one entry per row (source: todos/core-data-types/decisions.md)
- Phase 0: invariant checkers live in the public module facet_core::invariants instead of root re-exports, and compare floats bitwise with total_cmp (source: todos/invariant-checkers/decisions.md)
- Phase 0: the data generator seeds every row from (dataset, seed, row) instead of per chunk, so output does not depend on thread count or chunk size (source: docs/solutions/benchmark-data-generator.md)
- Phase 0: generated data is CSV plus raw little-endian f32 for vectors, no Parquet or Arrow (source: docs/solutions/benchmark-data-generator.md)
- Phase 0: phase gates are not cumulative and phases without a defined gate exit 2 instead of passing (source: todos/run-gates-script/plan.md)
- Phase 0: the PROGRESS.md schedule was rebuilt from the feature list because the build spec is not in the repo; budgets and section 7 cuts other than Phase 4's are blank until filled in (source: todos/progress-tracking/plan.md)

## Gates

### Phase 0

- Date: 2026-11-23
- Time spent: 2086 min
- Gate: phase 0: PASS (20 checks; cargo test: 87 passed, 0 failed)
- Decision: crate packages are named facet-<dir> (facet-core, facet-log, ...) so they do not shadow Rust's core crate or the crates.io log crate; directory names stay as in the spec (source: docs/solutions/cargo-workspace.md)
- Decision: Value derives PartialEq only, no Eq/Ord/Hash, because of the f64; Row, ChangeBatch and ColumnBatch follow it (source: todos/core-data-types/decisions.md)
- Decision: column nulls are placeholders in the typed array plus a cleared validity bit, so every array has one entry per row (source: todos/core-data-types/decisions.md)
- Decision: invariant checkers live in the public module facet_core::invariants instead of root re-exports, and compare floats bitwise with total_cmp (source: todos/invariant-checkers/decisions.md)
- Decision: the data generator seeds every row from (dataset, seed, row) instead of per chunk, so output does not depend on thread count or chunk size (source: docs/solutions/benchmark-data-generator.md)
- Decision: generated data is CSV plus raw little-endian f32 for vectors, no Parquet or Arrow (source: docs/solutions/benchmark-data-generator.md)
- Decision: phase gates are not cumulative and phases without a defined gate exit 2 instead of passing (source: todos/run-gates-script/plan.md)
- Decision: the PROGRESS.md schedule was rebuilt from the feature list because the build spec is not in the repo; budgets and section 7 cuts other than Phase 4's are blank until filled in (source: todos/progress-tracking/plan.md)
