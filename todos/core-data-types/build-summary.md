# Core data types — build summary

- Branch: `feature/core-data-types`; base `main` @ 6d4454e (PR #1 merged before work started, so the branch was fast-forwarded to main).
- Commits: `1eb74a8` locked acceptance tests + plan; `ef44e7c` core types; `b46e594` first summary; `9a8ee5a` simplify pass; then pre-merge verification (Display tests + this summary).
- Decisions: `todos/core-data-types/decisions.md`.

## Pre-merge verification

- **Tests:** 30 passing / 0 failing with `cargo test --workspace` (facet-core 24, including the from_rows → to_rows proptest; 1 each in bench, check, delta, engine, log, planner; 0 in layouts). Acceptance: 15 passing / 0 failing across the three locked `acceptance/core-data-types` files and `acceptance/cargo-workspace/ac-1-scaffold-runs`.
- **Type check:** clean, `cargo check --workspace --all-targets` (0 errors, 0 warnings).
- **Lint:** clean, `cargo clippy --workspace --all-targets -- -D warnings` (0 issues) and `cargo fmt --all --check`.
- **Coverage:** not measured as a percentage, because no coverage tool (cargo-llvm-cov, tarpaulin) is installed and the project sets no threshold. A manual audit found that every public function and every private helper in `crates/core/src` is exercised by a unit or acceptance test. The only line never reached is the defensive `unreachable!` arm at `column.rs:48`. The three error `Display` impls were the last gap; they gained tests in this step (`schema.rs` `errors_display_readably`, `column.rs` `from_rows_rejects_an_invalid_row`).
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

## Review notes (non-blocking)

- `row.rs:25-26`: `Row::key` indexes directly and panics on a row shorter than the schema. That is documented, and validated rows never trigger it; `Option<&Value>` is the alternative.
- `column.rs:103-106`: `from_rows` validates every row before building, an extra O(rows × cols) pass that keeps `column.rs:48` unreachable.
- `schema.rs:155,170`: `ColId(i as u32)` would truncate past 2³² columns. Not reachable in practice.
- `lib.rs:2`: the forward-looking Fragment note stays, because the locked cargo-workspace test requires the core doc to name Fragment.
