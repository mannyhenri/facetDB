# Progress tracking in PROGRESS.md: build summary

- Branch: `feature/progress-tracking` in `/Users/mannyhenri/code/facetDB`, base
  `feature/run-gates-script` @ 4ab9225 (neither `facet-gen.rs` nor `run_gates.sh` is on `main`).
- Plan: `todos/progress-tracking/plan.md`.

## Resolved blocker: conflicting locked tests

`acceptance/cargo-workspace/ac-1-scaffold-runs.test.mjs` (locked by cargo-workspace, 3c6678f)
asserted "PROGRESS.md and REPORT.md are title-only placeholders", which contradicted this feature's
locked AC-1. The first three pipeline attempts stopped on it (the Run Tests gate failed after 2
retries). The person then asked to "do what's needed", and that one test case was amended to check
REPORT.md only (d4abc1c, person-approved; its comment records why). This feature's locked files
were not touched.

## Phase 0 gate (recorded)

`scripts/record_gate.sh 0 --decision …` (8 decisions) → gate `phase 0: PASS (20 checks; cargo test:
87 passed, 0 failed)`, commit **2ac7c42 `Phase 0: gate passed`** (changes only PROGRESS.md). Time
spent 2086 min (from the repo's first commit, as the brief defines Phase 0). Decisions recorded,
each with its source: facet-<dir> package names; Value PartialEq only; null placeholders + validity
bits; public facet_core::invariants with bitwise float comparison; per-row seeds; CSV + raw f32;
non-cumulative gates with undefined phases exiting 2; rebuilt schedule.

## Results at 2ac7c42 (clean tree)

| Suite | Result |
|---|---|
| Locked AC-1 `ac-1-schedule-and-decisions.test.mjs` | pass 1, fail 0 |
| Locked AC-2 `ac-2-record-passed-gate.test.mjs` | pass 2, fail 0 |
| Locked AC-3 `ac-3-overrun-requires-cut.test.mjs` | pass 2, fail 0 |
| Locked run-gates-script AC-1 / AC-2 / AC-3 (fresh clones) | 1/1, 2/2, 1/1 |
| `acceptance/cargo-workspace/ac-1-scaffold-runs.test.mjs` | pass 8, fail 0 |
| `scripts/record-gate.test.mjs` | pass 18, fail 0 |
| `scripts/run-gates.test.mjs` | pass 16, fail 0 |
| `./run_gates.sh 0` | exit 0, 20 PASS, `phase 0: PASS`; cargo test 87 passed, 0 failed |

Node test total (summed by script): 51 passed, 0 failed. `shellcheck run_gates.sh
scripts/record_gate.sh` clean.

## Files changed (vs 4ab9225)

- `scripts/record_gate.sh` (new, 100755), `PROGRESS.md` (schedule, decisions, gates), `README.md`
  ("Recording a gate"), `run_gates.sh` (`cargo test: N passed, M failed` line after the cargo test
  check; `progress-tracking` in phase 0)
- `scripts/record-gate.test.mjs` (new, 18 tests), `scripts/run-gates.test.mjs` (2 counts tests,
  phase 0 fixture includes progress-tracking)
- `acceptance/progress-tracking/*` (4 locked files, unchanged)
- `acceptance/cargo-workspace/ac-1-scaffold-runs.test.mjs` (one test case amended, person-approved, d4abc1c)
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
5. The locked cargo-workspace title-only check was amended (person-approved) so Phase 0 could be recorded.
