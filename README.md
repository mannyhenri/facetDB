# facetDB

Created with NeonCore.

## Phase gates

`./run_gates.sh <phase>` runs the gate for one phase (0-7) from the repo root: `cargo fmt --all --check`,
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo build --workspace`,
`cargo test --workspace`, the forbidden-crate check (`cargo tree --workspace -e normal,dev` must not
list rusqlite, sled, rocksdb, duckdb, redb or lmdb), then each `*.test.mjs` file in the phase's
acceptance directories, one `node --test <file>` at a time. Every check runs even after a failure.
Each prints `PASS <check>` or `FAIL <check>` (failure output follows, indented), and the last line
is `phase N: PASS` or `phase N: FAIL (k of m checks failed)`.

Exit codes:

- `0`: every check passed.
- `1`: at least one check failed.
- `2`: usage error (no argument, extra arguments, not 0-7), a phase whose gate is not defined yet,
  or a missing prerequisite (`cargo`, `node` >= 18).

Requirements: Rust via `rust-toolchain.toml`, Node 18 or newer, and a warm cargo cache or network
access for the first build.

Adding a phase: when the feature that closes phase N lands, add a `N)` entry to the `case $phase`
list in `run_gates.sh` naming its directories under `acceptance/`, e.g.
`1) acceptance_dirs="log-records group-commit" ;;`. Until then `./run_gates.sh N` prints
`phase N gate is not defined yet` and exits 2. A listed directory that is missing or holds no
`*.test.mjs` file is a failure, not a skip. Never list `acceptance/run-gates-script`: its tests run
`./run_gates.sh` themselves.

## Recording a gate

`scripts/record_gate.sh <phase> [--decision "<text>"]... [--cut "<text>"]` records a passed gate in
`PROGRESS.md`. It runs `./run_gates.sh <phase>`; if the gate fails it records and commits nothing
and exits with the gate's status. On a pass it appends a `### Phase N` entry under `## Gates` (date,
time spent, the gate's summary line, number of checks, `cargo test` counts, decisions, cut), adds
each decision under `## Decisions`, and commits `PROGRESS.md` alone as `Phase N: gate passed`.

- Time spent is measured from the previous `Phase N-1: gate passed` commit (phase 0: from the first
  commit). Recording phase N without that commit exits 2: one phase at a time.
- If the phase has a budget in the schedule table and ran more than 20 minutes over it, `--cut` is
  required (the script names the phase's section 7 cut) and is recorded as `cut: <text>`.
- Write each decision that differs from the spec with its source, e.g.
  `--decision "per-row seeds instead of per-chunk seeds (source: docs/solutions/benchmark-data-generator.md)"`.
- Commit your work first: the gate's acceptance tests expect a clean tree.
