# Core data types — build summary

- Branch: `feature/core-data-types`; base `main` @ 6d4454e (PR #1 merged before work started, so the branch was fast-forwarded to main).
- Commits (main..HEAD): locked acceptance tests + plan; core types; build summary; simplify pass; Display tests + verification; P3 review fixes; P2 fix (`SelectionMut`).
- Decisions: `todos/core-data-types/decisions.md`.

## Pre-merge verification

- **Tests:** 34 passing / 0 failing with `cargo test --workspace` (facet-core 27 unit + 1 `compile_fail` doctest, including set-based key-type and fits tests, including the from_rows → to_rows proptest; 1 each in bench, check, delta, engine, log, planner; 0 in layouts). Acceptance: 15 passing / 0 failing across the three locked `acceptance/core-data-types` files and `acceptance/cargo-workspace/ac-1-scaffold-runs`.
- **Type check:** clean, `cargo check --workspace --all-targets` (0 errors, 0 warnings).
- **Lint:** clean, `cargo clippy --workspace --all-targets -- -D warnings` (0 issues) and `cargo fmt --all --check`.
- **Coverage:** not measured as a percentage, because no coverage tool (cargo-llvm-cov, tarpaulin) is installed and the project sets no threshold. A manual audit found that every public function and every private helper in `crates/core/src` is exercised by a unit or acceptance test. The only line never reached is the defensive `unreachable!` arm at `column.rs:49`. The three error `Display` impls were the last gap; they gained tests in this step (`schema.rs` `errors_display_readably`, `column.rs` `from_rows_rejects_an_invalid_row`).
- **Forbidden crates:** none (`cargo tree --workspace -e normal,dev`). facet-core has no normal dependencies.
- **Locked files:** the 6 `acceptance/core-data-types` files are byte-identical to the approved versions.

## Files changed (vs main)

- `crates/core/src/lib.rs`, `lsn.rs`, `value.rs`, `schema.rs`, `row.rs`, `change.rs`, `bitmap.rs`, `column.rs`
- `docs/core-data-types/plan.md`
- `todos/core-data-types/decisions.md`, `todos/core-data-types/build-summary.md`
- `acceptance/core-data-types/*` (locked, committed unchanged)

## Deviations from plan

- Based on `main` instead of `feature/cargo-workspace`, because PR #1 had merged.
- Implementation landed in one commit (`ef44e7c`) rather than one WIP commit per plan step.
- Otherwise none: all 10 plan steps are implemented as written.

## Review outcome

Triage in `p1.md` (none) and `p2.md` (one, resolved: `SelectionMut`). P3s resolved: `Row::key` returns `Option`, `WORD_BITS`, `ChangeBatch::len/is_empty` dropped.

## Review notes (non-blocking, pre-review)

- `row.rs:25`: `Row::key` panicked on a short row; resolved in triage (now returns `Option<&Value>`).
- `column.rs:130-134`: `from_rows` validates every row before building, an extra O(rows × cols) pass that keeps `column.rs:49` unreachable.
- `schema.rs:155,170`: `ColId(i as u32)` would truncate past 2³² columns. Not reachable in practice.
- `lib.rs:2`: the forward-looking Fragment note stays, because the locked cargo-workspace test requires the core doc to name Fragment.
