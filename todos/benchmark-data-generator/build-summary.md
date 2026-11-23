# Benchmark data generator: build summary

- Branch: `feature/benchmark-data-generator`, base `main` @ 220a393. Worktree: `../benchmark-data-generator-work`.
- Plan: `todos/benchmark-data-generator/plan.md` (no Phase 2 plan doc; the scope brief is the plan).
- Usage and design: `docs/solutions/benchmark-data-generator.md`.

## Pre-merge verification

Last re-run: 2026-11-23 in `/Users/mannyhenri/code/facetDB` on this branch @ 7a6ad20 (clean tree); every number below re-checked there. `cargo bench -p facet-bench --no-run` also exits 0. The acceptance total (23) is summed by script from the `ℹ pass` lines; the Run Tests step report said 22, a hand-sum error.

- **Tests:** 87 passing / 0 failing (`sh -c 'cargo test --workspace 2>&1'`, summed with awk): facet-bench 3 unit + 24 integration (`tests/datagen.rs`), facet-core 54 unit + 1 `compile_fail` doctest, 1 each in check, delta, engine, log, planner. Acceptance: 23 passing / 0 failing across all 12 files under `acceptance/`, each run alone with `node --test` (benchmark-data-generator AC-1..AC-3 1 each, cargo-workspace 8, core-data-types 1 + 1 + 5, invariant-checkers 5 x 1).
- **Type check:** clean, `cargo check --workspace --all-targets` (0 errors, 0 warnings).
- **Lint:** clean, `cargo clippy --workspace --all-targets -- -D warnings` (0 issues) and `cargo fmt --all --check`. `RUSTDOCFLAGS="-D warnings" cargo doc -p facet-bench --no-deps` clean.
- **Coverage:** not measured as a percentage: no coverage tool (cargo-llvm-cov, tarpaulin) is installed and the project sets no threshold. Manual audit of `crates/bench/src`:
  - Every public item is exercised: `Dataset` methods, `rows`, `write_files` (0 rows, missing dir, threads 1 vs 7, seed change, `threads == 0`, `out` is a file), `parse_args` (defaults, all flags, 11 bad inputs).
  - The binary's success path is exercised by the acceptance tests; its error paths were checked by hand: no arguments and `--rows x` exit 2 with the usage/error message, `--out <existing file>` exits 1 with `File exists`.
  - Not reached by tests: an I/O error in the middle of writing (`datagen.rs:185-187`), the worker-panic `expect` (`datagen.rs:184`) and the unreachable Bytes/Vector arm of `parse_field` (`datagen.rs:449`).
- **Dependencies:** none added; no forbidden crates (`cargo tree --workspace -e normal,dev`).
- **Locked files:** the 3 `acceptance/benchmark-data-generator` files match the locked hashes (`shasum -a 256`).
- **Non-blocking notes (P3):** row counts near `u64::MAX` would overflow `(c + 1) * CHUNK_ROWS` (`datagen.rs:168`) and `i * 250` in `event_time` (`datagen.rs:365`) (panic in debug, wrap in release). Unreachable in practice (the file would be exabytes), so left as is.

## Measured

`cargo build --release -p facet-bench --bin facet-gen`, then
`time target/release/facet-gen events --rows 10000000 --seed 42 --out <tmp>`:
real 2.38 s, 2.21 s, 2.26 s (three runs; Intel Xeon W-3265M, 48 threads, macOS, local SSD).
Limit: 60 s. events.csv is 1,462,542,213 bytes.
Vectors (1M x 128): 0.44 s. Orders (1M): 0.03 s.

## Tests

- `crates/bench/tests/datagen.rs`: 24 passing (written first, unchanged since b987b6e).
- `crates/bench/src/datagen.rs` unit tests: 2 passing (integer/cents formatting, skew range); `crate_builds`: 1.
- Acceptance: `acceptance/benchmark-data-generator` AC-1, AC-2, AC-3 pass, each run alone with `node --test`; all earlier acceptance files still pass.

## Files changed (vs main)

- `crates/bench/src/datagen.rs` (new), `crates/bench/src/cli.rs` (new), `crates/bench/src/bin/facet-gen.rs` (new)
- `crates/bench/src/lib.rs` (`pub mod cli; pub mod datagen;`), `crates/bench/Cargo.toml` (`[[bin]] facet-gen`)
- `crates/bench/tests/datagen.rs` (new)
- `acceptance/benchmark-data-generator/*.test.mjs` (3 locked files, committed unchanged)
- `todos/benchmark-data-generator/{plan.md,build-summary.md}`, `docs/solutions/benchmark-data-generator.md`

No new dependencies. `PROGRESS.md` unchanged (title-only).

## Deviations from the brief

1. **Per-row seeds instead of per-chunk seeds.** Row `i` draws from its own SplitMix64 seeded with
   `mix64(mix64(seed ^ dataset_salt) + i)`. This is strictly stronger than per-chunk seeding: output
   does not depend on thread count, chunk size or range split, and `rows(ds, seed, a..b)` can start
   anywhere. Chunks (65,536 rows) are still the unit of parallel work.
2. **`rows()` parses the CSV line that `write_files` writes** (plus the vector bytes), so the
   in-memory rows and the files cannot drift apart.
3. **No `--threads` flag:** the binary uses `std::thread::available_parallelism()`; the library's
   `write_files` takes a thread count (0 is `InvalidInput`).
4. Decisions the brief left open: vectors column named `vector`; categories `cat_00`..`cat_99`
   (skewed); events start at 2024-01-01T00:00:00Z, about 250 ms apart (10M rows span ~29 days);
   `search_phrase` is a non-null `Str` (empty string), `revenue` the only nullable column.

## Gap stage (stage 3): acceptance evidence location

NeonCore's retest after stage 2 ran the locked tests in `/Users/mannyhenri/code/facetDB`, which was
still on `main` (220a393), and all three failed with `no bin target named facet-gen`. The code was
only in the separate worktree `../benchmark-data-generator-work` (the CE template's
`../<slug>-work` name), never checked out in the repo path and never pushed. Fix, no code change:
the untracked duplicate test copies in the repo path (hash-identical to the branch) were deleted,
the clean worktree was removed, and `feature/benchmark-data-generator` was checked out in
`/Users/mannyhenri/code/facetDB`, where AC-1..AC-3 then passed. Lesson: the acceptance check runs in
the repo path, so the feature branch must be checked out (or pushed) there before a stage ends.
