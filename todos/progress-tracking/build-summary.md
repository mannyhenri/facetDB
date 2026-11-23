# Progress tracking in PROGRESS.md: build summary

- Branch: `feature/progress-tracking` in `/Users/mannyhenri/code/facetDB`, base
  `feature/run-gates-script` @ 4ab9225 (neither `facet-gen.rs` nor `run_gates.sh` is on `main`).
- Plan: `todos/progress-tracking/plan.md`.

## BLOCKED: locked tests contradict each other

`acceptance/cargo-workspace/ac-1-scaffold-runs.test.mjs:155-162` (locked by the cargo-workspace
feature, commit 3c6678f) asserts "PROGRESS.md and REPORT.md are title-only placeholders"
(`lines.length === 1`). This feature's locked AC-1 requires Schedule, Decisions and Gates sections in
PROGRESS.md, and the brief requires `./run_gates.sh 0` to stay green and the real Phase 0 gate to be
recorded. Both cannot hold:

- `./run_gates.sh 0` in this checkout: 19 of 20 checks PASS, `FAIL
  acceptance/cargo-workspace/ac-1-scaffold-runs.test.mjs` ("PROGRESS.md must contain a title line
  only", 24 !== 1), `phase 0: FAIL (1 of 20 checks failed)`.
- Locked run-gates tests from fresh clones at ece2962: AC-1 fail (same cause), AC-2 pass 1 / fail 1
  (case b expects exactly one failing check, now two), AC-3 pass.
- Therefore step 8 (record the real Phase 0 gate) was NOT done: `scripts/record_gate.sh 0` refuses
  to record a failing gate by design, and recording by hand is forbidden.

Workarounds rejected: removing `cargo-workspace` from the phase 0 list breaks the locked run-gates
AC-1 (it requires every cargo-workspace file to PASS); keeping PROGRESS.md title-only breaks this
feature's locked AC-1. Editing any locked file is not allowed.

Needed from the person: unlock `acceptance/cargo-workspace/ac-1-scaffold-runs.test.mjs` and change
that one test case to check REPORT.md only (PROGRESS.md was a placeholder until this feature). Then
re-run `./run_gates.sh 0`, record Phase 0 with `scripts/record_gate.sh 0 --decision …`, and re-run
the locked run-gates tests.

## Results at 0855b6c

| Suite | Result |
|---|---|
| Locked AC-1 `ac-1-schedule-and-decisions.test.mjs` | pass 1, fail 0 |
| Locked AC-2 `ac-2-record-passed-gate.test.mjs` | pass 2, fail 0 |
| Locked AC-3 `ac-3-overrun-requires-cut.test.mjs` | pass 2, fail 0 |
| `scripts/record-gate.test.mjs` | pass 17, fail 1 ("the real Phase 0 gate is recorded and committed": blocked) |
| `scripts/run-gates.test.mjs` | pass 16, fail 0 |
| `cargo test --workspace` | 87 passed, 0 failed |
| `./run_gates.sh 0` | exit 1, 19/20 PASS (cargo-workspace title-only check) |

Lint: `shellcheck` clean on `run_gates.sh` and `scripts/record_gate.sh`; `bash -n` (3.2) clean.

## Files changed (vs 4ab9225)

- `scripts/record_gate.sh` (new, 100755), `PROGRESS.md` (schedule, decisions, gates), `README.md`
  ("Recording a gate"), `run_gates.sh` (`cargo test: N passed, M failed` line after the cargo test
  check; `progress-tracking` in phase 0)
- `scripts/record-gate.test.mjs` (new, 18 tests), `scripts/run-gates.test.mjs` (2 counts tests,
  phase 0 fixture includes progress-tracking)
- `acceptance/progress-tracking/*` (4 locked files, unchanged)
- `todos/progress-tracking/{plan.md,build-summary.md}`

## Deviations

1. Worked in the repo path, not `../progress-tracking-work` (stage focus).
2. `run_gates.sh` prints the cargo test counts (the brief wants them in each entry and the gate did
   not print them).
3. Test fixture fix after the tests step: the fake `cargo` in `scripts/run-gates.test.mjs` printed
   its test output only after the injected-failure exit, so the "counts line also follows a failing
   cargo test check" case could never see counts; the output now comes first. No assertion changed.
4. Simplify pass: applied the cargo test check name defined once (reuse), a PROGRESS.md section
   guard before the gate runs (it could otherwise commit without an entry), phases limited to 0-7
   (8-9 have no gate), live gate output via `tee` + `PIPESTATUS`. Skipped: removing the
   `NODE_TEST_CONTEXT` unset (a unit test pins it), merging the decision/entry loops, and five
   efficiency notes (negligible at this size).
5. Step 8 not done (blocked, above).
