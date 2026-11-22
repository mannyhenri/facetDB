# Build summary: cargo-workspace (pre-merge verification)

Branch `feature/cargo-workspace`, verified at `49aa88c`.

- Tests: 15 passing / 0 failing
  - `cargo test --workspace`: 7 unit tests pass (one each in facet-core, -log, -delta, -planner, -engine, -check, -bench); facet-layouts has no tests by design; 0 doc tests.
  - `node --test acceptance/cargo-workspace/ac-1-scaffold-runs.test.mjs` (locked AC-1): 8 / 8 pass.
- Type check: clean (`cargo check --workspace --all-targets`, 0 errors, 0 warnings)
- Lint: clean (`cargo clippy --workspace --all-targets -- -D warnings`: 0 issues; `cargo fmt --all --check`: clean)
- Coverage: not measured (N/A). The crates have no executable code outside `#[cfg(test)]` modules, so there is nothing to cover. No coverage tool (cargo-llvm-cov, tarpaulin) is installed and the project sets no threshold.
- Files changed (vs `main`):
  - `.gitignore`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `PROGRESS.md`, `REPORT.md`
  - `crates/{core,log,delta,layouts,planner,engine,check,bench}/Cargo.toml` and `src/lib.rs`
  - `crates/layouts/src/{row,column,vector,contract}.rs`
  - `crates/bench/benches/placeholder.rs`
  - `acceptance/cargo-workspace/ac-1-scaffold-runs.test.mjs` (locked, unchanged since `3c6678f`)
  - `docs/cargo-workspace/plan.md`, `docs/cargo-workspace/build-summary.md`
- Deviations from plan:
  - Steps 1-8 were committed together in `394ff80` rather than as per-step WIP commits.
  - rustfmt gives the four empty layouts modules a single trailing newline (`96a59ce`).
  - `crates/bench/benches/placeholder.rs` was added: the plan's placeholder `[[bench]]` target needs a source file.
  - Crate doc comments are written from the scope brief and vision, because build spec section 3 is not in the repo.
