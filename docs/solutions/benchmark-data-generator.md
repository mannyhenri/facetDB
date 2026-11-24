---
title: Deterministic benchmark data generator in facet-bench
slug: benchmark-data-generator
categories: [benchmarking, testing]
tags: [rust, facet-bench, data-generation, determinism, splitmix64, csv, std-thread, performance]
date: 2026-11-23
---

## Problem

Every later benchmark compares Facet with DuckDB and an in-memory key-value store, and the
project rule is "same data for every system". Phase 0's gate also needs 10M wide event rows
written in under 60 seconds. Three datasets were needed: a wide web-events table (10M rows, a
ClickBench-hits-style subset), a narrow keyed orders table and 1M 128-dimension vectors with a
category and price. They had to be loadable by DuckDB (files) and by Facet (rows), bit-for-bit the
same.

## Solution

`facet_bench::datagen` (`crates/bench/src/datagen.rs`) plus the `facet-gen` binary:

```
cargo build --release -p facet-bench --bin facet-gen
target/release/facet-gen <events|orders|vectors> [--rows N] [--seed S] --out <dir>
```

Defaults: events 10,000,000 rows, orders 1,000,000, vectors 1,000,000, seed 42.

- `Dataset::{Events, Orders, Vectors}`: `schema()` (a `facet_core::Schema`, key column 0),
  `default_rows()`, `file_names()`, `name()` / `from_name()`.
- `rows(dataset, seed, ids)` yields `facet_core::Row`s; `write_files(dataset, seed, rows, out,
  threads)` writes the files. `facet_bench::cli::parse_args` parses the command line.

**Files.**

| Dataset | Files | Columns |
|---|---|---|
| events | `events.csv` | event_id, event_time, user_id, session_id, url, referer, title, search_phrase, country, region_id, browser, os, device_type, is_mobile, resolution_width, resolution_height, duration_ms, revenue |
| orders | `orders.csv` | order_id, customer_id, product_id, quantity, price, status |
| vectors | `vectors.f32`, `vectors_meta.csv` | raw row-major little-endian f32, N x 128, no header; meta: id, category, price |

Each CSV has one header line and ends with a newline. An empty field is NULL only in a nullable
column (`revenue`); `search_phrase` and `referer` are non-null strings that are often empty. No
string contains `,`, `"`, CR or LF, so no quoting is ever needed (a test checks this).

**Distributions.** Skewed choices use `n * u^3` for uniform `u` (Zipf-like: index 0 gets about half
the rows of a 7-item list). Events: `event_id` = row index; `event_time` starts 2024-01-01T00:00:00Z
and advances ~250 ms per row; `user_id` skewed over 1M, `session_id` = user x 1000 + 0..999; url
and title from 10 sections x 10,000 pages; 8 referers (first is empty = direct); `search_phrase`
two words for 15 % of rows; 20 two-letter countries; `region_id` = country x 100 + skewed 0..99;
7 browsers, 6 OSes, 3 device types with matching resolutions and `is_mobile`; `duration_ms` skewed
up to 600,000; `revenue` present (0.01..500.00) for 3 % of rows. Orders: `order_id` = row index;
customer over 200,000, product over 20,000, quantity 1..10, price 1.99..499.98, six statuses.
Vectors: 100 categories `cat_00`..`cat_99` (skewed); centroids uniform in [-1, 1]^128 per
(seed, category); each vector = centroid + roughly normal noise (sd ~0.1); price 1.00..999.99.

**Determinism.** Row `i` uses its own SplitMix64, seeded `mix64(mix64(seed ^ dataset_salt) + i)`
(`write_row`, `datagen.rs:261`). Nothing depends on the total row count, the thread count or the
chunk split. `write_files` hands chunks of 65,536 rows to worker `c % threads`, each with a
2-deep `sync_channel`, and the writer reads the workers round-robin, so chunks land in order with
bounded memory (`datagen.rs:161`).

**Measured.** 10M events: 2.38 / 2.21 / 2.26 s (three runs, release, Intel Xeon W-3265M, 48 threads,
macOS). The file is ~1.46 GB. 1M vectors: 0.44 s; 1M orders: 0.03 s.

## What Worked

- **Per-row seeding** instead of per-chunk seeding: thread-count and range independence come for
  free, and `rows(ds, seed, a..b)` gives random access for tests and loaders.
- **One source of truth for files and rows**: `Generator::row` (`datagen.rs:234`) parses the CSV
  line `write_row` produced (plus the vector bytes). The "same data for every system" invariant
  then holds by construction, and the test `csv_files_hold_exactly_the_generated_rows` checks it.
- **Hand-rolled integer and cents formatting** into per-chunk `Vec<u8>`s (`push_int`, `push_cents`)
  and plain `std::thread::scope`: no new crates, 25x under the limit on the first measurement.
- **Byte-hash check around refactors**: hashing all four files for a fixed (seed, rows) before and
  after the simplify pass caught a reordered RNG draw that unit tests did not (they compare
  generator output with itself).

## What Did Not Work

- The first simplify pass moved the `event_time` jitter draw after the section and page draws. All
  24 unit tests still passed, but `events.csv` changed for every seed. Fixed by restoring the draw
  order; the SHA-256 comparison is what caught it.

## Prevention

- **Any change to a generator's draw order or ranges changes the data** and invalidates earlier
  benchmark numbers. Before and after touching `write_event` / `write_order` / `write_vector`, hash
  `facet-gen <ds> --rows 200003 --seed 7` output; if bytes change on purpose, say so in
  PROGRESS.md and rerun the affected benchmarks.
- Loaders for DuckDB and the key-value store must read these files, and Facet loaders should use
  `rows()` or the same files, never a re-implementation.
- Keep string pools free of `,` `"` and newlines (`strings_never_need_csv_escaping`).

## Key Code References

- `crates/bench/src/datagen.rs:127` `rows`; `:135` `write_files`; `:161` ordered parallel writer
- `crates/bench/src/datagen.rs:234` `Generator::row`; `:261` `write_row` (per-row seed)
- `crates/bench/src/datagen.rs:363` `write_event`; `:406` `write_order`; `:275` `write_vector`
- `crates/bench/src/datagen.rs:416` `Line` (comma-separated field writer); `:484` `Rng` (SplitMix64)
- `crates/bench/src/cli.rs:23` `parse_args`; `crates/bench/src/bin/facet-gen.rs`
- Tests: `crates/bench/tests/datagen.rs`; acceptance: `acceptance/benchmark-data-generator/`
