# Phase gate script (run_gates.sh): plan

No Phase 2 plan doc exists (`docs/run-gates-script/plan.md`); the scope brief is the plan.

- Branch: `feature/run-gates-script`, from `feature/benchmark-data-generator` @ f2e4e9a, because
  `crates/bench/src/bin/facet-gen.rs` is not on `main` (220a393) yet.
- Work happens in `/Users/mannyhenri/code/facetDB` itself, not a `../run-gates-script-work`
  worktree: NeonCore runs the locked tests in this path.
- Locked acceptance tests: `acceptance/run-gates-script/{ac-1,ac-2,ac-3}*.test.mjs` +
  `gate-harness.mjs`, committed unchanged.

## Steps

1. **Commit the locked tests and this plan** unchanged on the feature branch.
2. **Extra failing tests** outside `acceptance/` (`scripts/test-run-gates.sh`, which proves the
   script from throwaway clones): a listed acceptance directory that is missing or has no
   `*.test.mjs` is a FAIL (not a skip); missing `cargo` or `node` (or node < 18) exits 2;
   phases 1-7 print `phase N gate is not defined yet` and exit 2; a failing nested
   `node --test` is still a FAIL when `NODE_TEST_CONTEXT` is set in the environment.
3. **`run_gates.sh`** at the repo root, `#!/usr/bin/env bash`, mode 755, bash 3.2:
   - validate arguments before anything else (no cargo call before validation): exactly one
     argument, an integer 0-7; otherwise usage on stderr, exit 2;
   - per-phase acceptance list in a `case`; phase 0 = cargo-workspace, core-data-types,
     invariant-checkers, benchmark-data-generator; phases 1-7: `phase N gate is not defined yet`,
     exit 2;
   - prerequisites: `cargo`, and `node` >= 18; otherwise exit 2;
   - `cd` to the script's directory; `unset NODE_TEST_CONTEXT`;
   - shared checks: cargo fmt, clippy -D warnings, build, test, forbidden-crate check
     (`cargo tree` status checked on its own, then grep the captured output);
   - every `acceptance/<dir>/*.test.mjs` run alone with `node --test <file>`; a missing or
     empty directory is one FAIL line;
   - no `set -e`, no pipes on gating commands; each check's status recorded; every check runs;
   - output: `PASS <check>` / `FAIL <check>` (failing output indented below the FAIL line so no
     output line starts with `PASS `/`FAIL `), last line `phase N: PASS` or
     `phase N: FAIL (k of m checks failed)`; exit 0 / 1.
4. **README.md** usage section: the command, the exit codes, how a later phase adds its
   acceptance directories (and never `acceptance/run-gates-script`: it would recurse).
5. **Simplify pass** (`/ce-simplify-code`), re-run the focused tests.
6. **Prove from a fresh clone** of the committed branch: `./run_gates.sh 0` → exit 0; the AC-2
   injections → exit 1; the AC-3 arguments → exit 2. Then the full check set in this path.
7. **Build summary** `todos/run-gates-script/build-summary.md`: branch, commit, base, real output
   of the three AC runs, scripted test totals.

## Not in this stage

Phases 1-7 gates, an `all` mode, PROGRESS.md/REPORT.md, CI, Phase 4 review.
