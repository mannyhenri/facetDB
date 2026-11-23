# Plan: Core data types

Written from the scope brief's In scope list (no separate plan existed).

1. `crates/core/src/lsn.rs`: `pub struct Lsn(pub u64)` (Copy, Ord, Hash, Debug) with `Lsn::ZERO` and `next()`.
2. `crates/core/src/value.rs`: `Value` { Int(i64), Float(f64), Str(String), Bytes(Vec<u8>), Vector(Vec<f32>), Null } and `DataType` { Int, Float, Str, Bytes, Vector { dim } }; `Value::data_type()` and a check that a Value fits a DataType (Null fits any nullable column).
3. `crates/core/src/schema.rs`: `ColId(pub u32)`; `Schema` = ordered columns (name, DataType, nullable) + primary-key ColId; lookup by name; `validate(&Row)` reporting the first mismatch.
4. `crates/core/src/row.rs`: `Row` = values in schema column order, with `key(&Schema)`.
5. `crates/core/src/change.rs`: `ChangeBatch` { lsn, inserts, updates (full rows), deletes (primary keys) }.
6. `crates/core/src/bitmap.rs`: hand-written bitset (set, get, count_ones, len, all-selected constructor); no external crate.
7. `crates/core/src/column.rs`: `ColumnBatch` with one typed array per column (flat `Vec<f32>` for vectors), per-column validity bitmaps, a selection bitmap; `from_rows(&Schema, &[Row])` and `to_rows()` over selected rows.
8. `pub const BATCH_ROWS: usize = 1024`; everything re-exported from `lib.rs`.
9. Unit tests in core for the acceptance criteria, plus one proptest that `from_rows` -> `to_rows` round-trips random rows.
10. Decisions recorded in `todos/core-data-types/` (PROGRESS.md stays title-only).

Acceptance (locked): `node --test acceptance/core-data-types/ac-{1-change-batch,2-column-batch,3-workspace-green}.test.mjs`.
