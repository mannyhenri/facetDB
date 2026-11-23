---
title: Core data types
slug: core-data-types
categories: [data, architecture, api, testing]
tags: [rust, facet-core, newtype, bitmap, columnar, proptest, node-test, scope-component]
date: 2026-11-23
---

## Problem

The log, the delta buffer, the layouts and the checker all pass changes and rows to each other. Before Phase 1 could start, they needed one shared vocabulary in `facet-core`: log sequence numbers, values, schemas, rows, change batches and column batches. The stated requirement was a list of types (scope brief, "In scope"). The real need was a set of **invariants every later crate can rely on without re-checking**:

- A value in a typed column array always has that column's type.
- A selection always has exactly one bit per row.
- A schema always has a valid primary key.
- An `Lsn` cannot be mixed up with a row count.

Three locked acceptance tests (`acceptance/core-data-types/ac-{1,2,3}-*`) fixed the public names and shapes before any code was written.

## Solution

- One private module per type in `crates/core/src/` (`lsn`, `value`, `schema`, `row`, `change`, `bitmap`, `column`), all re-exported from `crates/core/src/lib.rs:12-18`, with `BATCH_ROWS = 1024` at `lib.rs:21`.
- Newtypes `Lsn(pub u64)` (`lsn.rs:5`, `ZERO` at `:9`, an overflow-checked `next()` at `:12-14`) and `ColId(pub u32)` (`schema.rs:10`).
- `Value` has the five spec variants plus `Null` (`value.rs:6-14`), deriving `PartialEq` only. `DataType` mirrors the non-null variants, with `Vector { dim }` (`value.rs:17-24`). `Value::fits` (`value.rs:41-46`) is the single type check, and `Schema::validate` uses it.
- `Schema::new` validates instead of trusting its input. It returns `Result` and rejects a primary key that is out of range, nullable, or not Int/Str/Bytes, as well as duplicate names (`schema.rs:116-136`). Fields are private, so the invariant holds afterwards.
- `ChangeBatch` is plain data with public fields: one `lsn`, plus `inserts`, `updates` (full new rows) and `deletes` (primary-key `Value`s) (`change.rs:8-17`).
- `ColumnBatch` (`column.rs:79-84`) holds `ColumnData` arrays (`column.rs:13-24`; vectors are one flat `Vec<f32>`, with row `i` at `values[i*dim..(i+1)*dim]`), per-column validity bitmaps and a separate selection bitmap. Null slots get placeholders (`column.rs:53-60`), so every array has one entry per row. `from_rows` validates every row first (`column.rs:130-134`), which keeps the `unreachable!` at `column.rs:49` unreachable. `to_rows` iterates only selected rows (`column.rs:159-164`).
- `Bitmap` is a hand-written bitset of packed `u64` words with bits past `len` kept zero (`bitmap.rs:5-10`), so `count_ones` is a popcount sum. There is no external crate.
- Tests: 27 unit tests plus one `compile_fail` doctest in core, including a from_rows → to_rows proptest with random selections (`column.rs:351-370`). The 3 locked acceptance files are run by `node --test`.

## What Worked

- **Locking the API in acceptance tests first.** The constructor shapes (`Column::new`, `Schema::new(Vec<Column>, ColId)`, `ColumnData::Vector { dim, values }`, bitmap `set(i, bool)`) were never in doubt during the build; `todos/core-data-types/decisions.md` lists exactly which names the tests fixed.
- **Making invalid states unrepresentable instead of documenting them.** Examples are private `Schema`/`ColumnBatch` fields, deriving `ColumnBatch::len` from the selection length (no separate counter that could drift) and the `SelectionMut` handle. Each removed a whole class of bug rather than one instance.
- **A `compile_fail` doctest as a regression test for an API restriction** (`column.rs:89-94`). It proves that `*batch.selection_mut() = Bitmap::all_set(4)` does not compile, which a normal unit test cannot express.
- **Set-based tests with compile-time exhaustiveness guards.** `primary_key_accepts_exactly_non_nullable_int_str_bytes` (`schema.rs:324`) and `fits_exactly_the_matching_type_for_every_kind` (`value.rs:105`) iterate every `DataType` × nullable flag against an explicit table. An exhaustive `match` forces the tests to be updated when a variant is added. This applies the CLAUDE.md "set-based tests" pattern to unit tests, not only to `cargo metadata` acceptance tests.
- **Finite float ranges in the proptest.** `NaN != NaN` would fail the round-trip equality without a real bug (`decisions.md`, "Proptest").

## What Did Not Work

- **`selection_mut() -> &mut Bitmap`** (review P2, fixed in 43878ad). The caller could assign a bitmap of another length: a shorter one silently dropped rows from `len()`/`to_rows()`, and a longer one panicked. A `&mut` to an internal invariant-carrying field is a hole even when every field is private. It was replaced by `SelectionMut` (`column.rs:95-110`: `set` plus read-only `Deref`, no `DerefMut`), which keeps the locked `batch.selection_mut().set(3, false)` call (`acceptance/core-data-types/ac-2-column-batch.rs:147`) compiling.
- **`Row::key` panicking on a short row** (P3, d6dc0ee). `Row`'s field is public, so short rows can exist. It now returns `Option<&Value>` (`row.rs:25`), like `Schema::column`/`col_id`.
- **`ChangeBatch::len`/`is_empty`** were removed in triage (d6dc0ee): a total over inserts + updates + deletes mixes different kinds and had no caller.
- **Line citations in `todos/core-data-types/build-summary.md` went stale** when `SelectionMut` was added above them (`column.rs:48` → `:49`, `103-106` → `130-134`). They were corrected in this stage, which is the CLAUDE.md "unchecked file:line citations" anti-pattern again.
- **The first quality gate rejected example-based tests** for the key-type rule and `fits`, so they were rewritten as set-based tests (9425720).

## Prevention

- **Review every `&mut` accessor to a private field** for whether the caller can replace the whole value, not just mutate it. A narrow handle with `Deref` but no `DerefMut` is the cheap fix, and a `compile_fail` doctest locks it in. This class of bug would have been caught at build time by asking "which invariants does this struct carry, and which public methods can break them?" for each struct.
- **When a struct's field is `pub`, no method may panic on a malformed value of it.** Return `Option`/`Result` instead.
- **Write unit tests set-based from the start** (every variant × every flag, with an exhaustive-match guard) rather than waiting for the quality gate to ask.
- **Regenerate citations after the last code commit.** Grep for `\.rs:[0-9]` in docs and re-check with `sed -n` before committing.
- **Locked tests can pin wording.** The cargo-workspace test requires the core crate doc to name `Fragment` (`acceptance/cargo-workspace/ac-1-scaffold-runs.test.mjs:143`), so `lib.rs:2` mentions a type that does not exist yet. Before locking a test, check whether it pins prose that a later feature will want to change.
- **Conflicting pipeline instructions recurred**: "do not open a PR, NeonCore does" versus a Review step that opened PR #2, and "do not push" versus this stage's "commit and push". As before, the specific step was followed and the conflict named in the report.

## Related Docs

- `docs/solutions/cargo-workspace.md`: the only prior doc. Its lessons applied directly: `sh -c 'cargo test --workspace 2>&1'` for test counts, reusing the existing worktree, recording decisions outside `PROGRESS.md`, and checking citations. What was missing there was guidance on in-crate API invariants; this doc adds it.
- `CLAUDE.md` "Core types": the conventions later features need.
- `docs/core-data-types/plan.md`: the plan.
- `todos/core-data-types/decisions.md`: every design default with its reason.
- `todos/core-data-types/build-summary.md`, `p1.md`, `p2.md`: verification and triage.
- PR: https://github.com/mannyhenri/facetDB/pull/2

## Key Code References

- `crates/core/src/lib.rs:12-18`: re-exports (the public API). `lib.rs:21`: `BATCH_ROWS`.
- `crates/core/src/value.rs:6-14`: `Value` (PartialEq only). `:17-24`: `DataType`. `:41-46`: `fits`.
- `crates/core/src/schema.rs:116-136`: `Schema::new` primary-key and duplicate-name rules. `:159`: `validate`.
- `crates/core/src/row.rs:25`: `Row::key -> Option<&Value>`.
- `crates/core/src/change.rs:8-17`: `ChangeBatch`.
- `crates/core/src/column.rs:13-24`: `ColumnData` (flat vectors). `:79-84`: `ColumnBatch` fields. `:95-110`: `SelectionMut`. `:130`: `from_rows`. `:159-164`: `to_rows`.
- `crates/core/src/bitmap.rs:5-10`: `Bitmap` layout and its zero-tail invariant.
- `crates/core/src/column.rs:351-370`: round-trip proptest.
