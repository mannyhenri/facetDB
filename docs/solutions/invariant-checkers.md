---
title: Invariant checkers in facet-core
slug: invariant-checkers
categories: [testing, architecture]
tags: [rust, facet-core, invariants, test-first, test-oracle, proptest, fnv-1a, total-cmp, node-test, scope-component]
date: 2026-11-23
---

## Problem

The build spec's section 2 lists five engine invariants and asks for their tests to be written before the code they guard. In Phase 0 none of that code exists: there is no log (Phase 1), no applier, watermarks or Layout trait (Phase 3), no column segments (Phase 4) and no vector layout (Phase 5). The stated requirement was "invariant tests written first". The real need was to make that rule **cheap and uniform** for five later features:

- Each owning feature should be able to write its invariant test first, by recording what its code did and handing the record to a ready, already-proven checker.
- It should not invent its own oracle under deadline pressure, with its own idea of what "same rows" or "batch of 1,024" means.

A checker that wrongly accepts a bad trace is worse than none: it turns a real bug into a green test. So the work was judged on **false passes**, not just on happy paths.

## Solution

The solution is one public module, `facet_core::invariants` (`crates/core/src/invariants.rs`, `pub mod invariants;` at `crates/core/src/lib.rs:7`). It holds plain functions over core types and `std` that return `Result<(), Violation>`, and adds no dependency. Each checker takes **observations**, not engine objects, so it was proven now on synthetic traces and later features only have to record a trace.

`Violation` (`invariants.rs:35-77`) has one variant per failure, each carrying the index/LSN/key/id needed to find the problem. It implements Debug, Clone, PartialEq, Display (`:79-140`) and `Error` (`:142`).

### What each checker guarantees

| Checker | Accepts | Rejects with |
|---|---|---|
| `check_ack_after_durable(&[AckEvent])` (`:147-163`) | every `Acked(n)` preceded by some `Durable(d)` with `d >= n`. Durability is the running **max** of `Durable` events and starts at `Lsn::ZERO`, so a lower `Durable` never lowers it | `AckBeforeDurable { index, lsn, durable }` at the first early ack |
| `check_watermarks(&[(watermark, last_durable)])` (`:168-190`) | watermarks that never go below the previous one and never exceed `last_durable`, **at every index including 0** | `WatermarkRegressed { index, from, to }` (checked first), `WatermarkPastDurable { index, watermark, durable }` |
| `check_same_rows(&Schema, a, b)` (`:197-236`) | the same rows in any order, matched by primary key; floats compare **bitwise** (`total_cmp`), so NaN == NaN but `0.0 != -0.0` | `RowsDiffer { only_in_a, only_in_b, different }`, reporting **every** differing key, each list sorted by key. A key duplicated on either side is `different` (or `only_in_*`) |
| `check_segments_unchanged(before, after)` over `(id, digest)` (`:241-260`) | new ids and removed ids (merge/compaction) | `SegmentModified { id, before, after }` when an id in both has another digest, duplicates included |
| `segment_digest(&[u8]) -> u64` (`:264-270`) | FNV-1a 64 (basis `0xcbf29ce484222325`, prime `0x100000001b3`, xor then multiply), deterministic. Not cryptographic | — |
| `check_batch_sizes(sizes, available)` (`:276-307`) | while at least `BATCH_ROWS` remain, every batch has at least `BATCH_ROWS`; once fewer remain, the next batch takes all of them; the sizes sum exactly to `available` | `EmptyBatch`, then `BatchTooSmall`, then `ShortNotFinal` per batch in order, then `TotalMismatch { expected, actual }`. A batch that overshoots what remains, or comes after the end, is a `TotalMismatch` |

### How an owning feature records a trace

These are sketches: the engine APIs they call (`log.append`, `applier.step`, `layout.scan`, ...) do not exist yet and their names are placeholders. The point is **where** each observation comes from. Write the test first, watch it fail to compile or fail, then build the code.

```rust
use facet_core::invariants::*; // the one module-path import in core (CLAUDE.md "Core types")

// Ack after durable: Group commit / Log API (Phase 1).
// Record every durable advance and every ack in the order they happen, from one shared log.
let trace = std::sync::Mutex::new(Vec::new());
log.on_durable(|lsn| trace.lock().unwrap().push(AckEvent::Durable(lsn)));
for c in commits { let lsn = log.commit(c).await?; trace.lock().unwrap().push(AckEvent::Acked(lsn)); }
check_ack_after_durable(&trace.into_inner().unwrap())?;

// Watermarks: Background applier (Phase 3).
// Sample (watermark, last_durable) at the same moment, after every applier step.
let mut obs = Vec::new();
while applier.step()? { obs.push((layout.watermark(), log.last_durable())); }
check_watermarks(&obs)?;

// Same rows: Merged base-plus-delta read (Phase 3), then column and vector layouts (Phases 4-5).
// Read the same snapshot S from two layouts (or from a layout and a naive reference).
let s = log.last_durable();
check_same_rows(&schema, &row_layout.scan_at(s)?, &column_layout.scan_at(s)?)?;

// Immutable segments: Row layout with sorted runs (Phase 3), Column segments (Phase 4).
// Digest every segment's bytes before and after an apply; record digests, not copies.
let digests = |l: &Layout| l.segments().map(|s| (s.id(), segment_digest(s.bytes()))).collect::<Vec<_>>();
let before = digests(&layout);
layout.apply(batch)?;
check_segments_unchanged(&before, &digests(&layout))?;

// Batch sizes: Layout contract (Phase 3).
// Record the row count of every batch one scan returns, plus how many rows it could return.
let sizes: Vec<usize> = layout.scan(&query)?.map(|b| b.len()).collect();
check_batch_sizes(&sizes, expected_row_count)?;
```

Tests: 27 unit tests in `invariants.rs`, plus the five locked acceptance tests `acceptance/invariant-checkers/ac-{1..5}-*.test.mjs`.
- Each `Violation` variant has a pass and a fail case produced through its checker.
- Every variant's `Display` text is checked against a table with an exhaustiveness guard.
- A 6×6 kind-order test covers the private value order.
- There is one proptest per checker that generates only valid traces (`:993-1018`).

## What Worked

- **Observations instead of engine types.** Every checker could be proven on synthetic good and bad inputs months before the code it guards, without creating a single Log/Layout/segment type (a no-go). Later features couple to these signatures, not the reverse.
- **Proving the locked tests before submitting them.** Before submitting, each acceptance fixture was run against a throwaway reference implementation in a scratch copy, where it had to pass. It was also run against a deliberately broken copy, where it had to fail. The broken copies were an off-by-one durable/watermark bound, `==` instead of `total_cmp`, FNV-1 instead of FNV-1a, and a short final batch accepted. That last check is why AC-4 pins the published FNV-1a test values (`""`, `"a"`) instead of only checking that `"abc" != "abd"`; plain FNV-1 also passes that weaker check.
- **A private total order instead of `Hash`/`Ord` on `Value`.** `Value` stays `PartialEq` only (CLAUDE.md). `cmp_values` (`:336-351`, kind rank then value, floats by `total_cmp`) lets `check_same_rows` sort and merge in O(n log n), and makes the three result lists deterministic.
- **Set-based tests with exhaustiveness guards**, the CLAUDE.md pattern from core-data-types:
  - `every_violation_displays_where_to_look` (`:869`) with its guard at `:785` and a count assert.
  - `cmp_values_orders_every_pair_of_kinds_by_rank` (`:599`), guarded by the exhaustive `kind_rank` match.
- **A three-reviewer simplify pass (reuse, quality, efficiency).** The efficiency reviewer replaced per-key `Vec` groups in `check_same_rows` with runs over one sorted list (`sorted_by_key` `:312`, `run_end` `:322`), with no behaviour change.

## What Did Not Work

- **Saturating the batch total** (review P3, fixed in 4273b2b). `sum.saturating_add(s)` made an overflowing total equal `usize::MAX`. So `check_batch_sizes(&[usize::MAX, 5], usize::MAX)` returned `Ok(())`, confirmed in a scratch crate. Saturation silently turns "too many" into "exactly the maximum", which is a false pass for any equality check. The total now uses `try_fold` + `checked_add` (`:300`), so overflow is always `TotalMismatch`. It is covered by `an_overflowing_total_never_matches_usize_max` (`:768`).
- **A coverage gap only a manual audit found.** No coverage tool is installed, so a line-by-line audit stood in. It found that `kind_rank`'s Float/Bytes/Vector/Null arms were never reached: cross-kind comparison was only tested for Int vs Str. The gap was closed by the 6×6 set test.
- **Committing after running the acceptance set.** `acceptance/cargo-workspace/ac-1-scaffold-runs.test.mjs:177` asserts a clean working tree. Running it with an uncommitted test change gave a red result that was not a regression.
- **A clippy failure slipped into a commit.** It was a `type_complexity` lint on a proptest strategy type. Chaining `clippy && git commit` without `set -e` semantics let the commit through, and it had to be amended. It was fixed with a `Digests` type alias.
- **Conflicting pipeline instructions, a third time.** "Open a PR titled …" and "commit and push" (stage briefs) versus "do NOT open a PR / NeonCore pushes" (branch section). No PR was opened and nothing was pushed; the conflict was named in each report.

## Prevention

- **For any checker or oracle, audit for false passes first.** Ask of every arithmetic and comparison: can a bad input collapse onto the accepted value? Saturating or wrapping arithmetic, `unwrap_or(default)`, and `==` on floats are the usual suspects. Write the adversarial case (overflow, NaN, `-0.0`, duplicate keys, index 0) as a test before trusting the green suite.
- **Prove a locked test against a deliberately broken implementation, and choose breakages that a weak assertion would miss** (FNV-1 vs FNV-1a, `<=` vs `<`). If the broken copy passes, the assertion is too weak.
- **Bitwise float equality is a contract, not an accident.** `check_same_rows` says that layouts must store the same bits. A feature that canonicalises floats (e.g. folds `-0.0` to `0.0`) must decide that explicitly; it should not weaken the checker in passing.
- **Promote `cmp_values` on its second caller.** It is the only total order on `Value` in the workspace. The first other feature that needs a key order (likely Row layout with sorted runs) should move it to one shared core helper instead of writing a second order with different float or kind rules (`todos/invariant-checkers/decisions.md`).
- **`RowsDiffer` Display prints every key.** If a large mismatch makes failure output unreadable, truncate only the `Display` text and keep the full lists in the variant.
- **Run the acceptance set on a committed tree**, and run `clippy` as its own step before committing.
- **Re-check citations after the last code commit.** The triage fix shifted test line numbers; the build summary's citations were re-checked with `sed -n` each time.

## Related Docs

- `docs/solutions/core-data-types.md`. Its lessons applied directly:
  - set-based tests with exhaustiveness guards
  - finite float ranges in proptests (deliberately *not* followed in the same-rows proptest, where NaN/`-0.0` are exactly what is under test)
  - no panics on `pub` data (a short row is keyed by `Value::Null`)
  - checking citations after the last commit
  - What was missing there: how to test an oracle for false passes. This doc adds it.
- `docs/solutions/cargo-workspace.md`: `sh -c 'cargo test --workspace 2>&1'` for test counts, reusing the existing worktree, running each `*.test.mjs` file explicitly.
- `CLAUDE.md` "Invariant checkers" (`CLAUDE.md:33-44`): the test-first rule and the owner/phase table. `CLAUDE.md:24` names `facet_core::invariants` as the one module-path import.
- `docs/invariant-checkers/plan.md`: the plan.
- `todos/invariant-checkers/decisions.md`: every design default with its reason.
- `todos/invariant-checkers/build-summary.md`, `p1.md`, `p2.md`, `p3.md`: verification and triage.
- PR: not opened by the pipeline yet (see "What Did Not Work").

## Key Code References

- `crates/core/src/invariants.rs:1-17`: module doc with the invariant → checker → owning feature → phase table.
- `crates/core/src/invariants.rs:26-31`: `AckEvent`. `:35-77`: `Violation`. `:79-140`: `Display`. `:142`: `Error`.
- `crates/core/src/invariants.rs:147-163`: `check_ack_after_durable`. `:168-190`: `check_watermarks`.
- `crates/core/src/invariants.rs:197-236`: `check_same_rows`. `:309-362`: its helpers (`MISSING_KEY`, `sorted_by_key`, `run_end`, `rows_equal`, `cmp_values`, `kind_rank`).
- `crates/core/src/invariants.rs:241-260`: `check_segments_unchanged`. `:264-270`: `segment_digest`.
- `crates/core/src/invariants.rs:276-307`: `check_batch_sizes`; the checked total is at `:299-306`.
- `crates/core/src/invariants.rs:768`: overflow regression test. `:785`/`:869`: Display set test and guard. `:993-1018`: proptests.
- `crates/core/src/lib.rs:7`: `pub mod invariants;`.
- `acceptance/invariant-checkers/ac-{1..5}-*.rs`: the locked contract (API names, field types, expected values).
