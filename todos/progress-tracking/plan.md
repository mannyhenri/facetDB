# Progress tracking in PROGRESS.md: plan

No Phase 2 plan doc exists (`docs/progress-tracking/plan.md`); the scope brief is the plan.

- Branch: `feature/progress-tracking`, from `feature/run-gates-script` @ 4ab9225, because neither
  `crates/bench/src/bin/facet-gen.rs` nor `run_gates.sh` is on `main` (220a393).
- Work happens in `/Users/mannyhenri/code/facetDB` itself (NeonCore runs the locked tests here).
- Locked acceptance tests: `acceptance/progress-tracking/{ac-1,ac-2,ac-3}*.test.mjs` +
  `progress-harness.mjs`, committed unchanged.

## Steps

1. **Commit the locked tests and this plan** unchanged on the feature branch.
2. **Extra failing tests** in `scripts/record-gate.test.mjs` (Node runner, temp git repos, stub
   `run_gates.sh`, like `scripts/run-gates.test.mjs`):
   - Phase 0 time is measured from the repo's first commit;
   - N > 0 with no `Phase N-1: gate passed` commit → exit 2, nothing recorded or committed;
   - an older commit titled `Phase 0: set up …` is not taken as a gate;
   - invalid arguments (none, `x`, `10`, `-1`, unknown flag, `--decision` without a value) → exit 2;
   - a budget of `—` never requires a cut; an overrun of exactly +20 min does not need one;
   - the first decision replaces `(none)`; later decisions are appended in order; existing
     decisions are kept;
   - decisions containing quotes, slashes, `&`, `\` or `$` are written verbatim;
   - only PROGRESS.md is committed when other files are dirty or staged, and the staged file stays
     staged;
   - the entry includes the gate's summary line, the number of checks and the `cargo test` counts
     when the gate prints them;
   - `NODE_TEST_CONTEXT` is unset before the gate runs.
   Plus a run_gates.sh unit test: after the `cargo test --workspace` check line, an indented
   `    cargo test: N passed, M failed` line (summed from the `test result:` lines).
3. **run_gates.sh**: print the indented cargo test counts line (needed by the entry; the locked
   run-gates tests only read lines starting with `PASS `/`FAIL `); add `progress-tracking` to the
   phase 0 list.
4. **scripts/record_gate.sh** (bash 3.2, mode 755): find the repo root from the script's
   location; validate arguments; locate the previous gate by exact subject with `%ct`; compute
   minutes with `date +%s`; read budget and cut from the schedule row; refuse (exit 2) on an
   overrun > 20 min without `--cut` before running the gate; run `./run_gates.sh N` (status not
   piped, `NODE_TEST_CONTEXT` unset); on failure print the output and exit with its status; on a
   pass rewrite PROGRESS.md through a temp file + mv (decisions into `## Decisions`, entry under
   `## Gates`) and `git commit -- PROGRESS.md -m "Phase N: gate passed"`.
5. **PROGRESS.md** with Schedule (rows 0-9, the rebuilt note), Decisions `(none)`, Gates; README
   "Recording a gate" paragraph.
6. **Simplify pass** (`/ce-simplify-code`), re-run focused tests.
7. **Verify**: unit tests, the 3 locked tests, the locked run-gates tests, `./run_gates.sh 0`.
8. **Record the real Phase 0 gate** on a clean tree with `scripts/record_gate.sh 0 --decision …`,
   one per Phase 0 deviation with its source path.
9. **Build summary** `todos/progress-tracking/build-summary.md`.

## Not in this stage

Gates for phases 1-9, budgets/cuts beyond Phase 4's, the build spec itself, push/PR, Phase 4 review.
