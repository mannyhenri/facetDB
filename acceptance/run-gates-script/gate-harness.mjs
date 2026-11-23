// Shared helpers for the run-gates-script acceptance tests: clone the repo at HEAD into a temp dir
// (a clean checkout: no target/, no untracked files) and run ./run_gates.sh there.
//
// These tests run ./run_gates.sh themselves, so acceptance/run-gates-script must never be listed
// in any phase of run_gates.sh: that would recurse.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, readdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');

/** Acceptance directories the Phase 0 gate must cover (more may be added later). */
export const PHASE0_DIRS = [
  'cargo-workspace', 'core-data-types', 'invariant-checkers', 'benchmark-data-generator',
];

/** Clones ROOT at HEAD into a fresh temp dir and returns its path. */
export function cleanClone() {
  const parent = mkdtempSync(join(tmpdir(), 'facet-gates-'));
  const dir = join(parent, 'facetDB');
  const c = spawnSync('git', ['clone', '-q', ROOT, dir], { encoding: 'utf8' });
  assert.equal(c.status, 0, `git clone failed:\n${c.stderr}`);
  assert.ok(!existsSync(join(dir, 'target')), 'a fresh clone must not contain target/');
  assert.ok(existsSync(join(dir, 'run_gates.sh')), 'run_gates.sh is not committed at HEAD');
  return { dir, cleanup: () => rmSync(parent, { recursive: true, force: true }) };
}

/** Every *.test.mjs under acceptance/<d>/ for each Phase 0 dir, as "<d>/<file>". */
export function phase0Files(dir) {
  const files = [];
  for (const d of PHASE0_DIRS) {
    const names = readdirSync(join(dir, 'acceptance', d)).filter((f) => f.endsWith('.test.mjs'));
    assert.ok(names.length > 0, `acceptance/${d} has no *.test.mjs files`);
    for (const f of names.sort()) files.push(`${d}/${f}`);
  }
  return files;
}

/**
 * Runs `./run_gates.sh <args>` in `dir` with stdout and stderr merged in write order, offline
 * (the cargo cache must be warm) and with cargo's default target dir (the clone's own target/).
 */
export function runGates(dir, args) {
  const env = { ...process.env, CARGO_NET_OFFLINE: 'true' };
  delete env.CARGO_TARGET_DIR;
  // Under `node --test`, children inherit NODE_TEST_CONTEXT and a nested `node --test <file>` then
  // exits 0 even when its tests fail. Clear it so the gate sees real results, as from a shell.
  delete env.NODE_TEST_CONTEXT;
  const quoted = args.map((a) => `'${a}'`).join(' ');
  const r = spawnSync('sh', ['-c', `./run_gates.sh ${quoted} 2>&1`], {
    cwd: dir, env, encoding: 'utf8', maxBuffer: 512 << 20,
  });
  assert.equal(r.error, undefined, `could not run ./run_gates.sh: ${r.error}`);
  const lines = r.stdout.split('\n').map((l) => l.replace(/\r$/, ''));
  const nonEmpty = lines.filter((l) => l.trim() !== '');
  return {
    status: r.status,
    out: r.stdout,
    lines,
    last: nonEmpty[nonEmpty.length - 1] ?? '',
    checkLines: lines.filter((l) => /^(PASS|FAIL) \S/.test(l)),
  };
}

/** The PASS/FAIL line for the check matching `re`, asserting there is exactly one. */
export function checkLine(run, re, what) {
  const hits = run.checkLines.filter((l) => re.test(l));
  assert.equal(hits.length, 1,
    `expected exactly one PASS/FAIL line for ${what}, got ${hits.length}:\n${run.checkLines.join('\n')}\n--- full output tail ---\n${run.out.slice(-4000)}`);
  return hits[0];
}

/** Shared-check patterns: each check line must name its command. */
export const SHARED_CHECKS = [
  [/cargo fmt/, 'cargo fmt --all --check'],
  [/cargo clippy/, 'cargo clippy --workspace --all-targets -- -D warnings'],
  [/cargo build/, 'cargo build --workspace'],
  [/cargo test/, 'cargo test --workspace'],
  [/cargo tree|forbidden/, 'the forbidden-crate check'],
];

export function fileRe(rel) {
  return new RegExp(rel.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'));
}
