# Build summary: cargo-workspace (Phase 3, implement)

Branch: feature/cargo-workspace. Workspace commit: 394ff80 (test + plan: 3c6678f).

Assumption: no section-3 build spec is in the repo, so crate doc comments describe each crate's role from the scope brief and vision.

## `cargo build --workspace` (exit 0)

```
    Updating crates.io index
     Locking 102 packages to latest compatible versions
      Adding criterion v0.5.1 (available: v0.8.2)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.29s
```

## `cargo test --workspace` (exit 0)

```
     Running unittests src/lib.rs (target/debug/deps/facet_bench-f96dc41fd1c3c1dc)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/lib.rs (target/debug/deps/facet_check-00d8818df8e3b6c6)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/lib.rs (target/debug/deps/facet_core-cb0940847584de1b)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/lib.rs (target/debug/deps/facet_delta-13239a76edee41a5)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/lib.rs (target/debug/deps/facet_engine-db27b6cacce914bd)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/lib.rs (target/debug/deps/facet_layouts-d8c27a28dbd48111)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/lib.rs (target/debug/deps/facet_log-e8fe7e853c3434a1)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/lib.rs (target/debug/deps/facet_planner-7be218355a012490)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests facet_bench
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests facet_check
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests facet_core
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests facet_delta
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests facet_engine
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests facet_layouts
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests facet_log
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests facet_planner
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

## `cargo bench -p facet-bench --no-run` (exit 0)

```
    Finished `bench` profile [optimized] target(s) in 23.61s
  Executable benches src/lib.rs (target/release/deps/facet_bench-b74798b10535a970)
  Executable benches/placeholder.rs (target/release/deps/placeholder-a85ea78ed88c56ba)
```

## `cargo tree --workspace -e normal,dev` (exit 0)

```
facet-bench v0.1.0 (./crates/bench)
├── facet-core v0.1.0 (./crates/core)
│   [dev-dependencies]
│   └── proptest v1.12.0
│       ├── bit-set v0.11.1
│       │   └── bit-vec v0.10.2
│       ├── bit-vec v0.10.2
│       ├── bitflags v2.13.2
│       ├── chacha20 v0.10.2
│       │   ├── cfg-if v1.0.5
│       │   ├── cpufeatures v0.3.1
│       │   └── rand_core v0.10.1
│       ├── core_detect v1.0.0
│       ├── num-traits v0.2.19
│       ├── rand v0.10.4
│       │   ├── getrandom v0.4.3
│       │   │   ├── cfg-if v1.0.5
│       │   │   ├── libc v0.2.191
│       │   │   └── rand_core v0.10.1
│       │   └── rand_core v0.10.1
│       ├── rand_xorshift v0.5.0
│       │   └── rand_core v0.10.1
│       ├── regex-syntax v0.8.12
│       ├── rusty-fork v0.3.1
│       │   ├── fnv v1.0.7
│       │   ├── quick-error v1.2.3
│       │   ├── tempfile v3.27.0
│       │   │   ├── fastrand v2.5.0
│       │   │   ├── getrandom v0.4.3 (*)
│       │   │   ├── once_cell v1.21.4
│       │   │   └── rustix v1.1.6
│       │   │       ├── bitflags v2.13.2
│       │   │       ├── errno v0.3.15
│       │   │       │   └── libc v0.2.191
│       │   │       └── libc v0.2.191
│       │   └── wait-timeout v0.2.1
│       │       └── libc v0.2.191
│       ├── tempfile v3.27.0 (*)
│       └── unarray v0.1.4
└── facet-engine v0.1.0 (./crates/engine)
    ├── facet-core v0.1.0 (./crates/core) (*)
    ├── facet-delta v0.1.0 (./crates/delta)
    │   ├── facet-core v0.1.0 (./crates/core) (*)
    │   └── facet-log v0.1.0 (./crates/log)
    │       └── facet-core v0.1.0 (./crates/core) (*)
    │   [dev-dependencies]
    │   └── proptest v1.12.0 (*)
    ├── facet-layouts v0.1.0 (./crates/layouts)
    │   ├── facet-core v0.1.0 (./crates/core) (*)
    │   └── facet-delta v0.1.0 (./crates/delta) (*)
    ├── facet-log v0.1.0 (./crates/log) (*)
    └── facet-planner v0.1.0 (./crates/planner)
        ├── facet-core v0.1.0 (./crates/core) (*)
        └── facet-layouts v0.1.0 (./crates/layouts) (*)
[dev-dependencies]
└── criterion v0.5.1
    ├── anes v0.1.6
    ├── cast v0.3.0
    ├── ciborium v0.2.2
    │   ├── ciborium-io v0.2.2
    │   ├── ciborium-ll v0.2.2
    │   │   ├── ciborium-io v0.2.2
    │   │   └── half v2.7.1
    │   │       ├── cfg-if v1.0.5
    │   │       └── zerocopy v0.8.64
    │   │           └── zerocopy-derive v0.8.64 (proc-macro)
    │   │               ├── proc-macro2 v1.0.107
    │   │               │   └── unicode-ident v1.0.27
    │   │               ├── quote v1.0.47
    │   │               │   └── proc-macro2 v1.0.107 (*)
    │   │               └── syn v2.0.119
    │   │                   ├── proc-macro2 v1.0.107 (*)
    │   │                   ├── quote v1.0.47 (*)
    │   │                   └── unicode-ident v1.0.27
    │   └── serde v1.0.229
    │       ├── serde_core v1.0.229
    │       └── serde_derive v1.0.229 (proc-macro)
    │           ├── proc-macro2 v1.0.107 (*)
    │           ├── quote v1.0.47 (*)
    │           └── syn v3.0.10
    │               ├── proc-macro2 v1.0.107 (*)
    │               ├── quote v1.0.47 (*)
    │               └── unicode-ident v1.0.27
    ├── clap v4.6.9
    │   └── clap_builder v4.6.7
    │       ├── anstyle v1.0.14
    │       └── clap_lex v1.1.2
    ├── criterion-plot v0.5.0
    │   ├── cast v0.3.0
    │   └── itertools v0.10.5
    │       └── either v1.19.0
    ├── is-terminal v0.4.17
    │   └── libc v0.2.191
    ├── itertools v0.10.5 (*)
    ├── num-traits v0.2.19
    ├── once_cell v1.21.4
    ├── oorandom v11.1.5
    ├── plotters v0.3.7
    │   ├── num-traits v0.2.19
    │   ├── plotters-backend v0.3.7
    │   └── plotters-svg v0.3.7
    │       └── plotters-backend v0.3.7
    ├── rayon v1.12.0
    │   ├── either v1.19.0
    │   └── rayon-core v1.13.0
    │       ├── crossbeam-deque v0.8.8
    │       │   ├── crossbeam-epoch v0.9.21
    │       │   │   └── crossbeam-utils v0.8.23
    │       │   └── crossbeam-utils v0.8.23
    │       └── crossbeam-utils v0.8.23
    ├── regex v1.13.2
    │   ├── regex-automata v0.4.19
    │   │   └── regex-syntax v0.8.12
    │   └── regex-syntax v0.8.12
    ├── serde v1.0.229 (*)
    ├── serde_derive v1.0.229 (proc-macro) (*)
    ├── serde_json v1.0.154
    │   ├── itoa v1.0.19
    │   ├── memchr v2.8.3
    │   ├── serde_core v1.0.229
    │   └── zmij v1.0.23
    ├── tinytemplate v1.2.1
    │   ├── serde v1.0.229 (*)
    │   └── serde_json v1.0.154 (*)
    └── walkdir v2.5.1
        └── same-file v1.0.6

facet-check v0.1.0 (./crates/check)
├── facet-core v0.1.0 (./crates/core) (*)
├── facet-engine v0.1.0 (./crates/engine) (*)
└── facet-log v0.1.0 (./crates/log) (*)
[dev-dependencies]
└── proptest v1.12.0 (*)

facet-core v0.1.0 (./crates/core) (*)

facet-delta v0.1.0 (./crates/delta) (*)

facet-engine v0.1.0 (./crates/engine) (*)

facet-layouts v0.1.0 (./crates/layouts) (*)

facet-log v0.1.0 (./crates/log) (*)

facet-planner v0.1.0 (./crates/planner) (*)
```

No database crates (rusqlite, sled, rocksdb, duckdb, redb, lmdb) appear in the tree. `git status` is clean and nothing under target/ is tracked.

Locked acceptance test: `node --test acceptance/cargo-workspace/ac-1-scaffold-runs.test.mjs` → 8 pass, 0 fail.
