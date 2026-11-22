---
title: Cargo workspace with the eight crates
slug: cargo-workspace
categories: [architecture, devops, testing]
tags: [rust, cargo, cargo-workspace, criterion, proptest, node-test, scope-system]
date: 2026-11-22
---

## Problem

Facet needed somewhere for every later Phase 0+ feature to put its code: a Rust Cargo workspace with the eight crates from the build spec (core, log, delta, layouts, planner, engine, check, bench), wired along the architecture's data flow. The stated requirement was "scaffold runs successfully". The real need was a *contract* for the shape of the workspace: exact members, exact dependency edges, no cycles, stable toolchain, no database crates. Later features can then add code without re-litigating structure.

## Solution

- A virtual workspace (`Cargo.toml`, resolver 2) with `[workspace.package]` edition 2021 / version 0.1.0, pinned to stable via `rust-toolchain.toml`.
- Eight `facet-<dir>` library crates. Internal edges are declared once in `[workspace.dependencies]` and inherited with `.workspace = true`. criterion and proptest are also declared at the workspace level and used only as dev-dependencies (proptest: core, delta, check; criterion: bench with one `harness = false` placeholder bench).
- One set-based acceptance test (`acceptance/cargo-workspace/ac-1-scaffold-runs.test.mjs`, Node's built-in runner, no npm deps). It reads `cargo metadata --no-deps` and asserts the exact member list and each crate's exact internal dependency set, iterates all four layouts modules, and requires a passing unit-test section for every crate in `cargo test --workspace` output.

## What Worked

- **`cargo metadata --no-deps` as the test oracle.** It gives resolved package names, manifest paths, edition, version, targets and dependency kinds without touching the network. Asserting with `deepEqual` per crate makes the test set-based: removing a single edge fails it (checked before locking).
- **Validating the locked test against a throwaway reference scaffold** in a scratch directory before submitting. It caught a real test bug (see below) that would otherwise have been locked in.
- **`facet-` package prefix with unchanged directory names.** It avoids the `core`/`log` clashes and keeps the spec's directory layout.
- **Centralising internal path deps in `[workspace.dependencies]`.** Each later edge is now a one-line `facet-x.workspace = true` edit.

## What Did Not Work

- **Parsing `cargo test` output from stdout + stderr concatenated.** `Running unittests …` goes to stderr and `test result:` to stdout, so concatenation loses the pairing. Fix: run `sh -c 'cargo test --workspace 2>&1'` and split on `Running `.
- **`git worktree add ../<slug>-work feature/<slug>`** (from the CE workflow template) fails when the pipeline already runs in a worktree on that branch. Reuse the existing worktree.
- **Declaring every crate in `[workspace.dependencies]`.** `facet-check` was added but nothing depends on it, so the entry was dead and was removed. Declare only crates that have dependents.
- **Writing empty module files with zero bytes.** rustfmt wants a trailing newline, so `cargo fmt --check` fails on them.

## Prevention

- **Missing spec.** The brief referenced "section 3 of the build spec", which is not in the repo, so crate doc comments were written from the scope brief and left as a human check. Next time, put the spec (or the relevant section) into the repo before the build stage, or have the acceptance test assert the spec's exact wording.
- **Conflicting instructions.** The brief said "record design defaults in PROGRESS.md" while also requiring PROGRESS.md to be title-only, and earlier stages said "NeonCore opens the PR" while the Open PR step said to open it. Spot these at planning time and resolve them in the brief.
- **Network-dependent acceptance tests.** The first `cargo test` needs crates.io for criterion and proptest unless they are already cached. Note this when locking the test, or vendor/warm the cache in CI.
- **Run `cargo fmt --all --check` and clippy in the build stage,** not only in review, to catch formatting before WIP commits.
- **Check new workspace entries for use:** `cargo metadata` can list `[workspace.dependencies]` entries nothing inherits.

## Related Docs

- No prior docs existed in `docs/solutions/`; this is the first. What was missing: build spec section 3 (crate responsibilities) is not in the repo.
- `CLAUDE.md` (repo root): the conventions distilled from this feature for later phases.
- `docs/cargo-workspace/plan.md`: the plan (scope brief's In scope list).
- `todos/cargo-workspace/build-summary.md`, `todos/cargo-workspace/p1.md`, `todos/cargo-workspace/p2.md`: verification and triage.
- PR: https://github.com/mannyhenri/facetDB/pull/1

## Key Code References

- `Cargo.toml:1-26`: virtual workspace, members, `[workspace.package]`, `[workspace.dependencies]` (internal crates + criterion/proptest).
- `rust-toolchain.toml:2`: `channel = "stable"`.
- `crates/engine/Cargo.toml:7-11`: the widest internal fan-in (core, log, delta, layouts, planner).
- `crates/bench/Cargo.toml`: criterion dev-dependency and the `[[bench]] harness = false` placeholder (`crates/bench/benches/placeholder.rs`).
- `crates/layouts/src/lib.rs:3-6`: the four empty layout modules.
- `acceptance/cargo-workspace/ac-1-scaffold-runs.test.mjs`: the locked, set-based contract (expected dependency graph in `INTERNAL_DEPS`).
