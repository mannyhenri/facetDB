#!/usr/bin/env bash
# Runs one phase gate: ./run_gates.sh <phase>, phase 0-7. See "Phase gates" in README.md.
# Exit 0: every check passed. 1: at least one check failed. 2: usage error, a phase whose gate is
# not defined yet, or a missing prerequisite. Works with macOS /bin/bash 3.2.
#
# No `set -e`: a failing check must not stop the run, every check's status is recorded instead.

usage() {
  echo "usage: ./run_gates.sh <phase>   (phase is 0-7)" >&2
  exit 2
}

# Validate the arguments before anything else runs (no cargo call on a usage error).
[ $# -eq 1 ] || usage
case $1 in
  [0-7]) phase=$1 ;;
  *) usage ;;
esac

# Acceptance directories (under acceptance/) each phase gate runs, besides the shared checks.
# The feature that closes a phase adds its entry here. Never list run-gates-script: its tests run
# this script, so the gate would call itself.
case $phase in
  0) acceptance_dirs="cargo-workspace core-data-types invariant-checkers benchmark-data-generator progress-tracking" ;;
  *)
    echo "phase $phase gate is not defined yet" >&2
    exit 2
    ;;
esac

for tool in cargo node; do
  command -v "$tool" >/dev/null 2>&1 || { echo "run_gates.sh: $tool not found on PATH" >&2; exit 2; }
done
node_version=$(node --version 2>/dev/null)
node_major=${node_version#v}
node_major=${node_major%%.*}
case $node_major in '' | *[!0-9]*) node_major=0 ;; esac
if [ "$node_major" -lt 18 ]; then
  echo "run_gates.sh: node >= 18 is required for node --test, found '$node_version'" >&2
  exit 2
fi

cd "$(dirname "$0")" || exit 2
# Under a parent `node --test`, a nested `node --test <file>` exits 0 even when its tests fail.
unset NODE_TEST_CONTEXT

total=0
failed=0

# check <name> <command...>: runs the command and prints `PASS <name>` or `FAIL <name>`. A
# failure's output follows, indented, so no output line can look like a check line.
check() {
  local name=$1 out status
  shift
  out=$("$@" 2>&1)
  status=$?
  total=$((total + 1))
  if [ "$status" -eq 0 ]; then
    echo "PASS $name"
  else
    failed=$((failed + 1))
    echo "FAIL $name"
  fi
  # scripts/record_gate.sh copies this line into the PROGRESS.md gate entry.
  if [ "$name" = "cargo test --workspace" ]; then
    printf '%s\n' "$out" | awk '/^test result:/ {
      for (i = 1; i < NF; i++) {
        if ($(i + 1) ~ /^passed/) passed += $i
        if ($(i + 1) ~ /^failed/) failed += $i
      }
    } END { printf "    cargo test: %d passed, %d failed\n", passed, failed }'
  fi
  if [ "$status" -ne 0 ]; then
    printf '%s\n' "$out" | sed 's/^/    /'
  fi
}

# Fails if cargo tree fails, or if it lists an external database crate. Called through `check`.
# shellcheck disable=SC2329
forbidden_crates() {
  local tree
  tree=$(cargo tree --workspace -e normal,dev 2>&1) || { printf '%s\n' "$tree"; return 1; }
  ! printf '%s\n' "$tree" | grep -E '(^|[[:space:]])(rusqlite|sled|rocksdb|duckdb|redb|lmdb) v[0-9]'
}

check "cargo fmt --all --check" cargo fmt --all --check
check "cargo clippy --workspace --all-targets -- -D warnings" cargo clippy --workspace --all-targets -- -D warnings
check "cargo build --workspace" cargo build --workspace
check "cargo test --workspace" cargo test --workspace
check "forbidden crates (cargo tree --workspace -e normal,dev)" forbidden_crates

# Each acceptance file runs alone: `node --test <dir>` has reported spurious failures.
for dir in $acceptance_dirs; do
  found=0
  for file in "acceptance/$dir"/*.test.mjs; do
    [ -f "$file" ] || continue
    found=1
    check "$file" node --test "$file"
  done
  if [ "$found" -eq 0 ]; then
    check "acceptance/$dir (missing, or has no *.test.mjs files)" false
  fi
done

if [ "$failed" -eq 0 ]; then
  echo "phase $phase: PASS"
  exit 0
fi
echo "phase $phase: FAIL ($failed of $total checks failed)"
exit 1
