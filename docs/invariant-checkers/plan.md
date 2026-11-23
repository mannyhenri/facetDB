# Plan: Invariant checkers

Written from the scope brief's In scope list (no separate plan existed).

1. New module `crates/core/src/invariants.rs`, `pub mod invariants` in `lib.rs`; plain functions over facet-core types and std only, returning `Result<(), Violation>`.
2. `pub enum Violation` (Debug, Clone, PartialEq, Display) with one variant per failure below, each carrying the index/LSN/key/id needed to find the problem.
3. `check_ack_after_durable(trace: &[AckEvent])`, `AckEvent` = `Durable(Lsn)` | `Acked(Lsn)`; fails with `AckBeforeDurable { index, lsn, durable }` when an `Acked(n)` appears while the highest `Durable` seen so far is below `n`.
4. `check_watermarks(observations: &[(Lsn, Lsn)])` over (layout watermark, log last_durable) pairs; fails with `WatermarkRegressed { index, from, to }` and `WatermarkPastDurable { index, watermark, durable }`.
5. `check_same_rows(schema: &Schema, a: &[Row], b: &[Row])`, order-insensitive by primary key; fails with `RowsDiffer { only_in_a, only_in_b, different }` listing primary-key Values; Float compares by `f64::total_cmp` equality so NaN matches NaN.
6. `check_segments_unchanged(before: &[(u64, u64)], after: &[(u64, u64)])` over (segment id, digest); fails with `SegmentModified { id, before, after }`; new and removed ids are allowed.
7. `pub fn segment_digest(bytes: &[u8]) -> u64`: hand-written FNV-1a 64-bit.
8. `check_batch_sizes(sizes: &[usize], available: usize)` using `BATCH_ROWS`: `EmptyBatch { index }`, `BatchTooSmall { index, size, remaining }`, `ShortNotFinal { index, size, remaining }`, `TotalMismatch { expected, actual }`.
9. Unit tests in `invariants.rs`: per checker at least one passing trace and one failing trace per Violation variant, plus a proptest per checker generating valid traces that must pass.
10. Module doc comment with the table invariant → checker → feature that writes its real test first → phase.
11. A short "Invariant checkers" section in CLAUDE.md with the test-first rule and the same table.
12. Decisions recorded in `todos/invariant-checkers/` (PROGRESS.md stays title-only).

Acceptance (locked): `node --test acceptance/invariant-checkers/ac-{1-ack-after-durable,2-watermarks,3-same-rows,4-segments-unchanged,5-batch-sizes}.test.mjs`, plus AC-6 (fmt, clippy, build, test).
