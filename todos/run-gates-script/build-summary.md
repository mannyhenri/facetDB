# Phase gate script (run_gates.sh): build summary

- Branch: `feature/run-gates-script`, checked out in `/Users/mannyhenri/code/facetDB` (no separate
  worktree: NeonCore runs the locked tests in this path).
- Base: `feature/benchmark-data-generator` @ f2e4e9a, because `crates/bench/src/bin/facet-gen.rs`
  is not on `main` (220a393). Neither branch is pushed.
- Plan: `todos/run-gates-script/plan.md` (no Phase 2 plan doc; the scope brief is the plan).
- Code verified at 79a0b59 (clean tree).

## Pre-merge verification (at ca0ab65)

- **Tests:** 105 passing / 0 failing: Rust 87 (`cargo test --workspace`, summed by awk) + run-gates
  Node tests 18 (locked AC-1 1, AC-2 2, AC-3 1, `scripts/run-gates.test.mjs` 14). `./run_gates.sh 0`:
  17 PASS / 0 FAIL, covering all 12 earlier acceptance files.
- **Type check:** clean. `bash -n` and `/bin/bash -n` (3.2) on `run_gates.sh`; `node --check` on the
  5 new `.mjs` files; `cargo check --workspace --all-targets` exit 0.
- **Lint:** clean. `shellcheck run_gates.sh` 0 issues (one `SC2329` false positive disabled at
  `run_gates.sh:68`); `cargo clippy --workspace --all-targets -- -D warnings` and
  `cargo fmt --all --check` exit 0.
- **Coverage:** no percentage (no coverage tool for bash; the project sets no threshold). Manual
  audit of `run_gates.sh`, line ranges to tests:
  - usage `:14-18`: AC-3 + "more invalid arguments"; undefined phase `:25-28`: "phases 1-7";
  - prerequisites `:31-33`: "missing cargo", "missing node"; version `:34-41`: "node older than 18";
  - `NODE_TEST_CONTEXT` `:45`: its own test; `check` `:52-65` both arms: happy path, "a failing cargo
    check", "several failures";
  - `forbidden_crates` `:69-73`: all 6 crates, failing `cargo tree`, clean tree;
  - acceptance loop `:82-92`: files, missing dir, empty dir; summary `:94-99`: PASS and FAIL.
  - Not reached: the non-numeric `node --version` fallback (`:37`) and `cd ... || exit 2` (`:43`).
- **Scope:** no change under `crates/`, `Cargo.toml`, `Cargo.lock`, `PROGRESS.md`, `REPORT.md`;
  locked files only in b9bd3aa.
- **Review findings (non-blocking):**
  - P3 `run_gates.sh:72`: the forbidden-crate match is exact-name, so a direct dependency on a
    binding crate (`librocksdb-sys`, `libduckdb-sys`, `lmdb-rkv`) would pass, where the CLAUDE.md
    substring grep would catch it. The six named crates themselves are caught. Candidate for Phase 4.
  - P3 `run_gates.sh:43`: `cd "$(dirname "$0")"` follows a symlinked invocation to the link's
    directory, not the repo. Only matters if someone symlinks the script.
  - P3 `run_gates.sh:55`: a check prints nothing until it ends (the 10M-events file takes ~10-60 s),
    so long checks look idle.

## Results at 79a0b59

Locked acceptance tests (`node --test <file>`, run alone):

| Test | Result | Time |
|---|---|---|
| AC-1 `ac-1-phase0-clean-checkout.test.mjs` | pass 1, fail 0 | 102 s (fresh clone, cold target/) |
| AC-2 `ac-2-failing-check-exits-nonzero.test.mjs` | pass 2, fail 0 | 194 s (two fresh clones) |
| AC-3 `ac-3-invalid-phase-usage-error.test.mjs` | pass 1, fail 0 | < 1 s |

Times are from the run at d930065 (79a0b59 only folds the two `command -v` lines into a loop);
79a0b59 was re-run with the same results.

Unit tests: `node --test scripts/run-gates.test.mjs`: pass 14, fail 0 (~12 s, fake cargo,
macOS /bin/bash 3.2).

`./run_gates.sh 0` in this checkout: exit 0, 17 PASS lines (5 shared checks + 12 acceptance
files), last line `phase 0: PASS`. Invalid arguments (none, `8`, `-1`, `x`, `0 1`) exit 2 with
`usage: ./run_gates.sh <phase>   (phase is 0-7)` on stderr; `3` exits 2 with
`phase 3 gate is not defined yet`.

Totals (summed by script): Rust 87 passed / 0 failed (`cargo test --workspace`); run-gates Node
tests 18 passed / 0 failed (4 locked + 14 unit); every earlier acceptance file passes inside the
phase 0 gate.

Lint: `shellcheck run_gates.sh` clean (one `SC2329` false positive disabled: `forbidden_crates` is
called through `check`); `bash -n` clean; fmt and clippy pass inside the gate.

Locked files unchanged (sha256 prefixes): 94dd1642bc42, 28d69afcb93e, f88b138e6bfb, 73f51737243d.

## Files changed (vs f2e4e9a)

- `run_gates.sh` (new, mode 100755)
- `README.md` ("Phase gates" section)
- `scripts/run-gates.test.mjs` (new, 14 unit tests)
- `acceptance/run-gates-script/*` (4 locked files, committed unchanged)
- `todos/run-gates-script/{plan.md,build-summary.md}`

No product code, no new crates. PROGRESS.md and REPORT.md unchanged.

## Deviations

1. Worked in the repo path, not `../run-gates-script-work` (stage focus overrides the template).
2. Unit tests are a Node test file (`scripts/run-gates.test.mjs`), not `scripts/test-run-gates.sh`,
   to match the repo's only test convention; plan.md updated.
3. Locked AC-2 case B replaces one acceptance file in the clone instead of deleting `facet-gen.rs`
   (decided when the tests were written: deleting the bin breaks the manifest, so every check
   would fail).
4. Simplify pass: applied 1 of 6 quality findings (prerequisite loop). Skipped: `node -e` version
   check (the unit tests' fake node only answers `--version`), the optional `found`-flag rewrite,
   and two comment/README trims (the README is for readers who do not open the script). Reuse and
   efficiency reviewers: no findings.

## Notes for later phases

- The gate unsets `NODE_TEST_CONTEXT`: under a parent `node --test`, a nested
  `node --test <file>` exits 0 even when its tests fail.
- Never list `acceptance/run-gates-script` in a phase: its tests run `./run_gates.sh 0`.
- A cold cargo cache needs crates.io; the AC tests run with `CARGO_NET_OFFLINE=true`.
