# Benchmark data generator: build summary

- Branch: `feature/benchmark-data-generator`, base `main` @ 220a393. Worktree: `../benchmark-data-generator-work`.
- Plan: `todos/benchmark-data-generator/plan.md` (no Phase 2 plan doc; the scope brief is the plan).
- Usage and design: `docs/solutions/benchmark-data-generator.md`.

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
