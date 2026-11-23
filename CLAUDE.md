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

## Core types

- Shared types live in `crates/core/src/` (one private module per type) and are re-exported from `lib.rs`: import them as `facet_core::{Lsn, Value, DataType, ColId, Column, Schema, Row, ChangeBatch, ColumnBatch, ColumnData, Bitmap, BATCH_ROWS}`. Never use a module path.
- `Lsn(pub u64)` is a newtype (`Lsn::ZERO`, `next()`). `ColId(pub u32)` is a column's position in the schema.
- `Value` has `Int(i64)`, `Float(f64)`, `Str(String)`, `Bytes(Vec<u8>)`, `Vector(Vec<f32>)` and `Null`. It derives `PartialEq` only, with no `Eq`/`Ord`/`Hash`, because of the `f64`. Do not use it as a `HashMap`/`BTreeMap` key; `Row`, `ChangeBatch` and `ColumnBatch` inherit this. `Value::fits(DataType, nullable)` is the single type check.
- A `Schema` has exactly one primary-key column, which is non-nullable `Int`, `Str` or `Bytes`. `Schema::new` returns `Result` and enforces this.
- In a `ChangeBatch`, every update carries the full new row (matched by its key) and every delete carries only the primary-key `Value`. One `ChangeBatch` has one `Lsn`.
- `ColumnBatch` keeps one typed array per column. Vectors are one flat `Vec<f32>` of `len × dim`, and null slots hold placeholders. The per-column **validity** bitmaps (nulls) are separate from the one **selection** bitmap (one bit per row). `to_rows()` returns selected rows only, and `selection_mut()` can flip bits but not replace the bitmap.
- `BATCH_ROWS = 1024` is the target batch size. Calls across the layout contract carry at least 1,024 rows when that many are available, and `ColumnBatch` has no row cap.
- `Fragment`, `Bid`, `Stats` and the `Layout` trait do not exist yet; they come with the "Layout contract" feature (Phase 3). Details and rationale: `docs/solutions/core-data-types.md`.

## Commands

- Build: `cargo build --workspace`
- Test: `cargo test --workspace` (must stay green)
- Lint: `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all --check`
- Bench compile check: `cargo bench -p facet-bench --no-run`

## Ownership

- `PROGRESS.md` content belongs to the "Progress tracking in PROGRESS.md" feature; `REPORT.md` to "Benchmark report".
- `crates/layouts/src/contract.rs` stays empty until the "Layout contract" feature (Phase 3).
- Files under `acceptance/` are locked acceptance tests: never edit them.

## Patterns to repeat

- **Set-based acceptance tests from `cargo metadata`.** Use `cargo metadata --format-version 1 --no-deps` as the oracle (needs no network) and assert *exact* sets with deep equality: member list, each crate's internal dependencies, dev-dependencies. A test that lists every member fails on a partial fix; one that checks a couple of examples does not.
- **Prove a test before locking it.** Before submitting an acceptance test, run it against a throwaway reference implementation in a scratch directory (must pass) and a deliberately broken copy (must fail). Locked tests can never be edited, so bugs found later cannot be fixed.
- **Run the full check set at the end of every build stage:** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo build --workspace`, `cargo test --workspace`, the acceptance commands, and `cargo tree --workspace -e normal,dev` for forbidden crates.

## Anti-patterns (and how to spot them early)

- **Reading `cargo test` output from separate streams.** `Running unittests …` goes to stderr and `test result:` to stdout; concatenating them breaks the pairing. Signal: a per-crate output parser fails although every test passed. Fix: run `sh -c 'cargo test --workspace 2>&1'`.
- **Zero-byte `.rs` files.** rustfmt wants a trailing newline, so `cargo fmt --check` fails. Signal: fmt diffs on files you never edited. Give empty modules a single newline.
- **Dead `[workspace.dependencies]` entries.** `facet-check` was declared although no crate depends on it. Signal: an entry with no `.workspace = true` user. Check with `grep -r 'facet-x.workspace' crates/` before adding.
- **Duplicate status docs.** A second build summary went stale within one stage and the review had to remove it. Keep one build summary per feature (`todos/<slug>/build-summary.md`) and update it in place.
- **Unchecked `file:line` citations.** A cited range ran past the end of the file. Signal: any citation written from memory. Check with `wc -l` / `sed -n` before committing docs.
- **`git worktree add ../<slug>-work feature/<slug>` from the CE templates** fails when the pipeline already runs in a worktree on that branch. Check `git worktree list` first and reuse the existing worktree.

## Taste decisions

- `facet-<dir>` package names with spec directory names unchanged: avoids the `core`/`log` clashes without diverging from the spec layout.
- Internal crates are inherited from `[workspace.dependencies]` (like external ones) rather than per-crate `path = "../x"`, so each manifest has one style and adding an edge is a one-line change.
- `Cargo.lock` is committed: the workspace builds benches and is an application, not a published library.
- Placeholder unit tests are named `crate_builds`; the layouts modules are `pub mod`, ordered alphabetically in `lib.rs`.
- Acceptance tests use Node's built-in test runner (`node --test`, no npm dependencies) under `acceptance/<slug>/`, because the repo has no other test harness for cross-crate contracts.

## Caveats

- Build spec section 3 (crate responsibilities) is **not in the repo**. Crate doc comments in `crates/*/src/lib.rs` were written from the scope brief and still need a human check against the spec. Do not treat them as authoritative.
- The first `cargo test`/`cargo bench` on a fresh machine downloads criterion and proptest from crates.io; acceptance tests that run cargo are not offline until the cache is warm.
- `PROGRESS.md` must stay title-only until the "Progress tracking in PROGRESS.md" feature, even where a brief says "record decisions in PROGRESS.md". Record them in `todos/<slug>/` or `docs/solutions/` meanwhile.
- Pipeline instructions can conflict (e.g. "NeonCore opens the PR" vs a step that says to open it). Follow the specific step, and name the conflict in the report.
