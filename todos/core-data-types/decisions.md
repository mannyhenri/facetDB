# Core data types — design decisions

Recorded here because PROGRESS.md stays title-only until its own feature.

- **One module per type** in `crates/core/src/` (`lsn`, `value`, `schema`, `row`, `change`, `bitmap`, `column`), private modules re-exported from `lib.rs`, so callers only write `facet_core::X`.
- **API fixed by the locked acceptance tests:** `Column::new(name, DataType, nullable)`, `Schema::new(Vec<Column>, ColId)`, `ColId(pub u32)`, `Row::new(Vec<Value>)`, `ChangeBatch` with public `lsn/inserts/updates/deletes`, `ColumnBatch::{from_rows, to_rows, columns, selection, selection_mut}`, `ColumnData::{Int, Float, Str, Bytes, Vector { dim, values }}`, bitmap `len/count_ones/get/set(i, bool)`.
- **`Value` is `PartialEq` only** (no `Eq`/`Hash`) because `Float` holds an `f64`. `Row`, `ChangeBatch`, `ColumnData` and `ColumnBatch` follow it.
- **`Value::data_type()` returns `Option<DataType>`:** `None` for `Null`, and a vector's `dim` is its length. `Value::fits(DataType, nullable)` is the single type check; `Schema::validate` uses it.
- **`Schema::new` returns `Result<Schema, SchemaError>`.** It rejects a primary key that is out of range, nullable, or not Int/Str/Bytes (brief: "one primary-key column of type Int, Str or Bytes"), and duplicate column names. Schema fields are private so these invariants hold.
- **`Schema::validate` returns `Result<(), RowError>`.** The error is either `Arity` or the first `Mismatch` (column, expected type, nullable, actual type).
- **`Row(pub Vec<Value>)`**, matching the `Lsn(pub u64)` style. `Row::key(&Schema)` returns `&Value` and panics on a row shorter than the schema; rows that pass `validate` never are.
- **`ColumnBatch::from_rows` validates every row** and returns `Result<_, BatchError { row, error }>`, so the typed arrays can never hold a value of the wrong type. All rows start selected.
- **Null slots hold placeholders** (0, 0.0, empty string/bytes, `dim` zeros) and are marked in a per-column validity bitmap, so every typed array has exactly one entry per row (vectors: `len × dim` floats).
- **`Bitmap`** is packed `u64` words with bits past `len` kept zero, so `count_ones` is a popcount sum. Out-of-range `get`/`set` panic. Only `all_set` is provided as a constructor (validity starts all-valid; nulls are cleared). `iter_ones` was added for `to_rows`.
- **No row cap** on `ColumnBatch`; `BATCH_ROWS = 1024` is only the target size.
- **Proptest:** random rows over a schema where every non-key column is nullable (dim 3), with a random selection. Floats are drawn from finite ranges, because NaN != NaN would break the equality check without a real bug.
- **Out of scope, not written:** Fragment, Bid, Stats, Layout trait, binary encoding, column encodings, vectorised operators. The crate doc comment still says "Fragment will join them with the layout contract", because the locked cargo-workspace test requires the core doc to name LSN, ChangeBatch and Fragment.
- **Base branch:** PR #1 merged before this stage started, so `feature/core-data-types` was fast-forwarded to `main` (6d4454e) rather than branched from `feature/cargo-workspace`.
