# Facet

Facet stores every change once in an ordered log and maintains several physical layouts (row, column, vector) of the same data, so each query reads one consistent snapshot and the engine picks the fastest layout per part of the query.

## Workspace layout

- Rust Cargo virtual workspace at the repo root; crates live in `crates/` (the spec's `facet/` directory *is* the repo root).
- Eight member crates, no more: `crates/{core,log,delta,layouts,planner,engine,check,bench}`.
- Package names are `facet-<dir>` (`facet-core`, `facet-log`, ...). Never name a package `core` (shadows Rust's built-in `core` crate) or `log` (clashes with the crates.io `log` crate). Directory names stay as in the spec.
- Every crate inherits `version.workspace = true` and `edition.workspace = true` (edition 2021, resolver 2).
- `docs/solutions/<slug>.md` holds one compound doc per finished feature (YAML frontmatter `categories`, `tags`; sections Problem, Solution, What Worked, What Did Not Work, Prevention, Key Code References). It is relevant when working in an area an earlier feature touched.

## Dependency rules

- Internal edges point down toward core, never up, never in a cycle:
  `bench -> engine`, `check -> engine, log`, `engine -> planner, layouts, delta, log`, `planner -> layouts`, `layouts -> delta`, `delta -> log`; every crate also depends on `facet-core`.
- Internal crates are declared once in the root `[workspace.dependencies]` (`facet-x = { path = "crates/x" }`) and used as `facet-x.workspace = true`. Only declare a crate there once some other crate depends on it.
- External dependencies are added by the feature that needs them, declared in `[workspace.dependencies]` and inherited with `workspace = true`. Do not pre-add crates later phases "might" want.
- No external database crates anywhere (rusqlite, sled, rocksdb, duckdb, redb, lmdb, ...). Check with `cargo tree --workspace -e normal,dev`.
- Rust stable only (`rust-toolchain.toml`): no nightly, no `#![feature]`.

## Core types

- Shared types live in `crates/core/src/` (one private module per type) and are re-exported from `lib.rs`: import them as `facet_core::{Lsn, Value, DataType, ColId, Column, Schema, Row, ChangeBatch, ColumnBatch, ColumnData, Bitmap, BATCH_ROWS}`. Never use a module path, except the public `facet_core::invariants` module (see Invariant checkers).
- `Lsn(pub u64)` is a newtype (`Lsn::ZERO`, `next()`). `ColId(pub u32)` is a column's position in the schema.
- `Value` has `Int(i64)`, `Float(f64)`, `Str(String)`, `Bytes(Vec<u8>)`, `Vector(Vec<f32>)` and `Null`. It derives `PartialEq` only, with no `Eq`/`Ord`/`Hash`, because of the `f64`. Do not use it as a `HashMap`/`BTreeMap` key; `Row`, `ChangeBatch` and `ColumnBatch` inherit this. `Value::fits(DataType, nullable)` is the single type check.
- A `Schema` has exactly one primary-key column, which is non-nullable `Int`, `Str` or `Bytes`. `Schema::new` returns `Result` and enforces this.
- In a `ChangeBatch`, every update carries the full new row (matched by its key) and every delete carries only the primary-key `Value`. One `ChangeBatch` has one `Lsn`.
- `ColumnBatch` keeps one typed array per column. Vectors are one flat `Vec<f32>` of `len × dim`, and null slots hold placeholders. The per-column **validity** bitmaps (nulls) are separate from the one **selection** bitmap (one bit per row). `to_rows()` returns selected rows only, and `selection_mut()` can flip bits but not replace the bitmap.
- `BATCH_ROWS = 1024` is the target batch size. Calls across the layout contract carry at least 1,024 rows when that many are available, and `ColumnBatch` has no row cap.
- `Fragment`, `Bid`, `Stats` and the `Layout` trait do not exist yet; they come with the "Layout contract" feature (Phase 3). Details and rationale: `docs/solutions/core-data-types.md`.

## Invariant checkers

- `crates/core/src/invariants.rs` holds one checker per section 2 invariant. They take plain observations (LSN traces, `(watermark, last_durable)` pairs, row slices, `(segment id, digest)` pairs, batch sizes) and return `Result<(), Violation>`. Import them by module path, `facet_core::invariants::{check_watermarks, Violation, ...}`: `invariants` is the one public module in core.
- **Rule:** the feature that owns an invariant writes its invariant test first, by calling its checker, before writing the code the test guards. Do not write a second checker for the same invariant; extend this one.

| Invariant | Checker | Feature that writes the real test first | Phase |
|---|---|---|---|
| Ack only after the log record is durable | `check_ack_after_durable` | Group commit / Log API | 1 |
| Watermark never moves backwards or past last durable LSN | `check_watermarks` | Background applier | 3 |
| Same rows at snapshot S from every layout | `check_same_rows` | Merged base-plus-delta read, then column and vector layouts | 3, then 4–5 |
| Base segments are immutable | `check_segments_unchanged` + `segment_digest` | Row layout with sorted runs, then Column segments | 3, then 4 |
| At least 1,024 rows per batch when available | `check_batch_sizes` | Layout contract | 3 |

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
- **Make invalid states unrepresentable, then lock the restriction in with a test.** Keep invariant-carrying fields private, derive values that could drift (e.g. `ColumnBatch::len` is the selection length, with no separate counter), and hand out narrow handles (`SelectionMut`: `set` plus read-only `Deref`, no `DerefMut`) instead of `&mut` to a field. When the guarantee is "this must not compile", lock it with a `compile_fail` doctest (`crates/core/src/column.rs:89-94`).
- **Set-based unit tests with an exhaustiveness guard.** For a rule over an enum (primary-key types, `Value::fits`), iterate every variant × every flag against an explicit expected table, and add an exhaustive `match` so adding a variant breaks the test build (`schema.rs:324`, `value.rs:105`). Write them this way first: the quality gate rejected example-based versions in core-data-types.
- **Proptests over floats draw from finite ranges.** `NaN != NaN` fails round-trip equality without a real bug.
- **Run the full check set at the end of every build stage:** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo build --workspace`, `cargo test --workspace`, the acceptance commands, and `cargo tree --workspace -e normal,dev` for forbidden crates.

## Anti-patterns (and how to spot them early)

- **Reading `cargo test` output from separate streams.** `Running unittests …` goes to stderr and `test result:` to stdout; concatenating them breaks the pairing. Signal: a per-crate output parser fails although every test passed. Fix: run `sh -c 'cargo test --workspace 2>&1'`.
- **Zero-byte `.rs` files.** rustfmt wants a trailing newline, so `cargo fmt --check` fails. Signal: fmt diffs on files you never edited. Give empty modules a single newline.
- **Dead `[workspace.dependencies]` entries.** `facet-check` was declared although no crate depends on it. Signal: an entry with no `.workspace = true` user. Check with `grep -r 'facet-x.workspace' crates/` before adding.
- **Duplicate status docs.** A second build summary went stale within one stage and the review had to remove it. Keep one build summary per feature (`todos/<slug>/build-summary.md`) and update it in place.
- **Unchecked `file:line` citations.** A cited range ran past the end of the file. Signal: any citation written from memory. Check with `wc -l` / `sed -n` before committing docs.
  Citations also go stale: in core-data-types, adding `SelectionMut` moved `column.rs` lines and left `build-summary.md` pointing at the wrong ones, and the compound doc's first draft had three off-by-one ranges. Re-check every citation after the **last** code commit: `grep -n '\.rs:[0-9]' docs todos -r`, then `sed -n` each one.
- **`&mut` accessor to a private, invariant-carrying field.** `ColumnBatch::selection_mut() -> &mut Bitmap` let a caller assign a bitmap of another length (rows silently dropped, or a panic in `to_rows`). The review caught it as a P2. Signal: any `pub fn x_mut(&mut self) -> &mut T` where `T` has a size or shape the struct depends on. Return a narrow handle instead.
- **Methods that panic on malformed values of a `pub` field.** `Row(pub Vec<Value>)` can be short, so `Row::key` panicking was wrong (review P3). Signal: `expect`/indexing on data a caller can construct freely. Return `Option`/`Result`.
- **Helpers with no caller.** `ChangeBatch::len`/`is_empty` summed inserts + updates + deletes (different kinds) and had no caller; triage removed them. Signal: a public method used only by its own test. Do not add it until a later crate needs it.
- **`node --test <directory>`** reported `fail 1` on `acceptance/cargo-workspace/` although its only test file passes. Run each `*.test.mjs` file explicitly (`for t in acceptance/<slug>/*.test.mjs; do node --test "$t"; done`).
- **`git worktree add ../<slug>-work feature/<slug>` from the CE templates** fails when the pipeline already runs in a worktree on that branch. Check `git worktree list` first and reuse the existing worktree.

## Taste decisions

- `facet-<dir>` package names with spec directory names unchanged: avoids the `core`/`log` clashes without diverging from the spec layout.
- Internal crates are inherited from `[workspace.dependencies]` (like external ones) rather than per-crate `path = "../x"`, so each manifest has one style and adding an edge is a one-line change.
- `Cargo.lock` is committed: the workspace builds benches and is an application, not a published library.
- Placeholder unit tests are named `crate_builds`; the layouts modules are `pub mod`, ordered alphabetically in `lib.rs`.
- Acceptance tests use Node's built-in test runner (`node --test`, no npm dependencies) under `acceptance/<slug>/`, because the repo has no other test harness for cross-crate contracts.
- Core types: one private module per type, re-exported from `lib.rs`, so the module layout can change without breaking callers.
- Plain-data newtypes expose their field (`Lsn(pub u64)`, `ColId(pub u32)`, `Row(pub Vec<Value>)`). Types with invariants (`Schema`, `ColumnBatch`, `Bitmap`) keep fields private behind a validating constructor that returns `Result` with a typed per-module error (`SchemaError`, `RowError`, `BatchError`).
- `Value` is `PartialEq` only (no `Eq`/`Ord`/`Hash`), because of the `f64`. Containers of values follow it rather than wrapping floats in an ordered type.
- Column nulls are placeholders in the typed array plus a cleared validity bit, so every array has exactly one entry per row (vectors: `len × dim`).

## Caveats

- Build spec section 3 (crate responsibilities) is **not in the repo**. Crate doc comments in `crates/*/src/lib.rs` were written from the scope brief and still need a human check against the spec. Do not treat them as authoritative.
- The first `cargo test`/`cargo bench` on a fresh machine downloads criterion and proptest from crates.io; acceptance tests that run cargo are not offline until the cache is warm.
- `PROGRESS.md` must stay title-only until the "Progress tracking in PROGRESS.md" feature, even where a brief says "record decisions in PROGRESS.md". Record them in `todos/<slug>/` or `docs/solutions/` meanwhile.
- Pipeline instructions can conflict (e.g. "NeonCore opens the PR" vs a step that says to open it). Follow the specific step, and name the conflict in the report.
  This recurred in core-data-types: "do not open a PR / do not push, NeonCore does" versus the Review step opening PR #2 and the Compound step saying "commit and push".
- Brief assumptions about branch state go stale. The core-data-types brief said "PR #1 is not merged, branch from feature/cargo-workspace", but PR #1 had merged before work started. Check with `gh pr view <n> --json state,baseRefName` before choosing a base or a PR target.
- Locked acceptance tests can pin prose. The cargo-workspace test requires the facet-core crate doc to name `Fragment` (`acceptance/cargo-workspace/ac-1-scaffold-runs.test.mjs:143`), so `crates/core/src/lib.rs:2` mentions a type that does not exist yet. Keep it until the Layout contract feature adds `Fragment`. When writing a new locked test, avoid asserting wording a later feature will want to change.
