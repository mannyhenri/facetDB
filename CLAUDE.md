# Facet

Facet stores every change once in an ordered log and maintains several physical layouts (row, column, vector) of the same data, so each query reads one consistent snapshot and the engine picks the fastest layout per part of the query.

## Workspace layout

- Rust Cargo virtual workspace at the repo root; crates live in `crates/` (the spec's `facet/` directory *is* the repo root).
- Eight member crates, no more: `crates/{core,log,delta,layouts,planner,engine,check,bench}`.
- Package names are `facet-<dir>` (`facet-core`, `facet-log`, ...). Never name a package `core` (shadows Rust's built-in `core` crate) or `log` (clashes with the crates.io `log` crate). Directory names stay as in the spec.
- Every crate inherits `version.workspace = true` and `edition.workspace = true` (edition 2021, resolver 2).

## Dependency rules

- Internal edges point down toward core, never up, never in a cycle:
  `bench -> engine`, `check -> engine, log`, `engine -> planner, layouts, delta, log`, `planner -> layouts`, `layouts -> delta`, `delta -> log`; every crate also depends on `facet-core`.
- Internal crates are declared once in the root `[workspace.dependencies]` (`facet-x = { path = "crates/x" }`) and used as `facet-x.workspace = true`. Only declare a crate there once some other crate depends on it.
- External dependencies are added by the feature that needs them, declared in `[workspace.dependencies]` and inherited with `workspace = true`. Do not pre-add crates later phases "might" want.
- No external database crates anywhere (rusqlite, sled, rocksdb, duckdb, redb, lmdb, ...). Check with `cargo tree --workspace -e normal,dev`.
- Rust stable only (`rust-toolchain.toml`): no nightly, no `#![feature]`.

## Commands

- Build: `cargo build --workspace`
- Test: `cargo test --workspace` (must stay green)
- Lint: `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all --check`
- Bench compile check: `cargo bench -p facet-bench --no-run`

## Ownership

- `PROGRESS.md` content belongs to the "Progress tracking in PROGRESS.md" feature; `REPORT.md` to "Benchmark report".
- `crates/layouts/src/contract.rs` stays empty until the "Layout contract" feature (Phase 3).
- Files under `acceptance/` are locked acceptance tests: never edit them.
