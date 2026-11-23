# Plan: Cargo workspace with the eight crates

Written from the scope brief's In scope list (no separate plan existed).

1. Root `Cargo.toml`: virtual workspace, `resolver = "2"`, members `crates/{core,log,delta,layouts,planner,engine,check,bench}`; `[workspace.package]` edition 2021, version 0.1.0; `[workspace.dependencies]` criterion + proptest.
2. `rust-toolchain.toml` pinning `channel = "stable"`.
3. Eight library crates, package names `facet-<dir>`, inheriting version and edition from the workspace.
4. `crates/layouts/src/` has `lib.rs` plus empty modules `row.rs`, `column.rs`, `vector.rs`, `contract.rs`, declared in `lib.rs`.
5. Every other crate's `src/lib.rs` has a crate doc comment naming its responsibility and one trivial passing unit test.
6. Internal dependencies: log, delta, layouts, planner, engine, check, bench -> facet-core; delta -> facet-log; layouts -> facet-delta; planner -> facet-layouts; engine -> facet-log, facet-delta, facet-layouts, facet-planner; check -> facet-engine, facet-log; bench -> facet-engine. proptest dev-dependency on core, delta, check; criterion dev-dependency on bench with one placeholder `[[bench]]` (harness = false).
7. `PROGRESS.md` and `REPORT.md` with a title line only.
8. `/target` in `.gitignore`; commit the workspace.

Acceptance: `node --test acceptance/cargo-workspace/ac-1-scaffold-runs.test.mjs` (locked).
