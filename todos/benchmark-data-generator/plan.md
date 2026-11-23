# Benchmark data generator: plan

No Phase 2 plan doc exists; this plan is derived from the locked scope brief and the three locked
acceptance tests in `acceptance/benchmark-data-generator/` (copied byte-identical from the main
working tree, where NeonCore wrote them untracked).

## Steps

1. **Failing unit tests** in `crates/bench` (small row counts): every row fits its schema
   (`Value::fits`), unique primary keys, exact row counts, byte-identical file output for the same
   seed across different thread counts, different output for a different seed.
2. **PRNG + generators**: in-crate SplitMix64-seeded PRNG; `facet_bench::datagen` with `events`,
   `orders`, `vectors` generators, each exposing its `facet_core::Schema` and yielding `Row`s for
   a (seed, row range).
3. **`facet-gen` binary** (`[[bin]]` in `crates/bench/Cargo.toml`): `<events|orders|vectors>
   [--rows N] [--seed S] --out <dir>`; writes `events.csv`, `orders.csv`, `vectors.f32` +
   `vectors_meta.csv` exactly as the brief specifies.
4. **Measure and tune**: `time target/release/facet-gen events --rows 10000000 --out <tmp>`;
   chunked `std::thread` parallelism with per-chunk seeds, hand-rolled integer formatting, large
   buffered writes, until well under 60 s.
5. **Docs**: `todos/benchmark-data-generator/build-summary.md` (updated in place) and
   `docs/solutions/benchmark-data-generator.md`; `PROGRESS.md` stays title-only.
6. **Full check set** (CLAUDE.md), commit, then every acceptance test file on its own; delete
   generated temp data.
