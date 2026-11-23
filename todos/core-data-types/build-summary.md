# Core data types — build summary

- Branch: `feature/core-data-types`; base `main` @ 6d4454e (PR #1 merged before this stage, so the branch was fast-forwarded to main instead of branching from feature/cargo-workspace).
- Commits: `1eb74a8` locked acceptance tests + `docs/core-data-types/plan.md`; `ef44e7c` core types; this summary.
- Code: `crates/core/src/{lib,lsn,value,schema,row,change,bitmap,column}.rs`. No other crate touched; facet-core has no normal dependencies.
- Decisions: `todos/core-data-types/decisions.md`.

## Checks (all exit 0)

| Command | Result |
|---|---|
| `cargo fmt --all --check` | ok |
| `cargo clippy --workspace --all-targets -- -D warnings` | ok |
| `cargo build --workspace` | ok |
| `cargo test --workspace` | ok; facet-core 22 unit tests incl. 1 proptest; other crates 1 each |
| `node --test acceptance/core-data-types/ac-1-change-batch.test.mjs` | pass |
| `node --test acceptance/core-data-types/ac-2-column-batch.test.mjs` | pass |
| `node --test acceptance/core-data-types/ac-3-workspace-green.test.mjs` | pass |
| `node --test acceptance/cargo-workspace/ac-1-scaffold-runs.test.mjs` | pass |
| `cargo tree --workspace -e normal,dev` | no forbidden database crates |

Locked acceptance files are byte-identical to the approved versions (shasum checked).

## For review

- `Row::key` returns `&Value` and panics on a row shorter than the schema (documented); an `Option` would be the alternative.
- `ColumnBatch::from_rows` validates every row (O(rows × cols) extra pass), which means the typed arrays can never be wrong.
- `Bitmap::iter_ones` walks bit by bit; fine for now, and a word-skipping version is a later optimisation.
