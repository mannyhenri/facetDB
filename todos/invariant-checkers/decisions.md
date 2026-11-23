# Invariant checkers — design decisions

Recorded here because PROGRESS.md stays title-only until its own feature.

- **Public module, not re-exports.** The brief asks for `pub mod invariants`, and the locked acceptance tests import `facet_core::invariants::{...}`. This is the one exception to "import core types from the crate root"; CLAUDE.md says so.
- **API fixed by the locked acceptance tests:** `AckEvent::{Durable, Acked}(Lsn)`; field types `index/size/remaining/expected/actual: usize`, `lsn/durable/from/to/watermark: Lsn`, `id/before/after: u64`, and `RowsDiffer` lists as `Vec<Value>`.
- **`Violation` also implements `std::error::Error`**, like `SchemaError` and `RowError`, so a test can `?` it.
- **Ack checker:** durability starts at `Lsn::ZERO`, so `Acked(Lsn(0))` passes with nothing durable. A `Durable` lower than an earlier one does not lower the high-water mark. A durable LSN that goes backwards is not reported here: the log feature owns that check.
- **Watermark checker:** compared with the previous observation's watermark. When one observation both regresses and passes durable, `WatermarkRegressed` is reported. `last_durable` going backwards is not checked (same reason as above).
- **Same-rows matching:** rows are grouped by primary key and compared after sorting by a private total order on `Value` (kind first, then value; floats by `total_cmp`). That is O(n log n) without `Hash`/`Ord` on `Value`, which stays `PartialEq` only.
  - All three lists in `RowsDiffer` are sorted by key (Int numeric, Str/Bytes lexicographic), so output is deterministic and independent of input order.
  - Value equality is bitwise for floats, as the brief says (`f64::total_cmp`), and vectors use `f32::total_cmp` the same way. So NaN matches NaN, but `0.0` does not match `-0.0` and NaNs with different payloads differ. Layouts that store the same bits pass.
  - A key that appears more than once on a side counts as different if the key is on both sides, or only-in-that-side otherwise (the brief: duplicates are a difference).
  - A row too short to have a key is keyed by `Value::Null` rather than panicking (CLAUDE.md: no panics on malformed `pub` data).
- **Segments:** only an id present before and after with another digest fails. With a duplicate id, every after-digest must equal every before-digest; the first mismatch in `before` order is reported. `after` is sorted once, then each before-id is found by binary search.
- **FNV-1a 64:** offset basis `0xcbf29ce484222325`, prime `0x100000001b3`, xor then multiply. Unit test pins three published test values (`""`, `"a"`, `"foobar"`). Not cryptographic; documented.
- **Batch sizes:** per-batch rules are checked in order (empty, then too small, then short-not-final), and the total last. A batch larger than what remains is not "short"; it shows up as `TotalMismatch`, as does a batch after the rows ran out. The sum saturates at `usize::MAX` instead of overflowing.
- **Proptests:** one per checker, generating only valid traces. The same-rows one draws any `f64` (NaN, infinities, `-0.0` included), unlike the finite-range rule in CLAUDE.md, because bitwise equality is exactly what is under test and a row always equals itself bitwise.
- **Out of scope, not written:** any Log, Layout, Fragment, BatchStream, segment or watermark type, and the real invariant tests (owned by the Phase 1–5 features). No new dependency: facet-core still has none for normal builds; proptest stays dev-only.
- **Worktree:** the pipeline already runs in `facetDB-feature-invariant-checkers` on `feature/invariant-checkers`, so `../invariant-checkers-work` was not created (CLAUDE.md anti-pattern).
