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
