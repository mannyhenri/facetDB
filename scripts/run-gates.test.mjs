// Fast unit tests for run_gates.sh. They cover what the locked acceptance tests in
// acceptance/run-gates-script do not: each runs a copy of the script in a throwaway fixture repo
// with a fake `cargo` and tiny acceptance files, on PATH=<shims>:/usr/bin:/bin (macOS /bin/bash
// 3.2, no real cargo or node), so no case needs a real build.
//
// Run: node --test scripts/run-gates.test.mjs
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import {
  chmodSync, copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync,
  symlinkSync, writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const SCRIPT = join(ROOT, 'run_gates.sh');
const PHASE0_DIRS = [
  'cargo-workspace', 'core-data-types', 'invariant-checkers', 'benchmark-data-generator',
  'progress-tracking',
];
// Checks in a default fixture: the shared ones plus one acceptance file per Phase 0 directory.
const DEFAULT_CHECKS = 5 + PHASE0_DIRS.length;
const SHARED = [
  ['fmt --all --check', /^(PASS|FAIL) .*cargo fmt/],
  ['clippy --workspace --all-targets -- -D warnings', /^(PASS|FAIL) .*cargo clippy/],
  ['build --workspace', /^(PASS|FAIL) .*cargo build/],
  ['test --workspace', /^(PASS|FAIL) .*cargo test/],
  ['tree --workspace -e normal,dev', /^(PASS|FAIL) .*(cargo tree|forbidden)/],
];
const PASSING = "import { test } from 'node:test';\ntest('ok', () => {});\n";
const FAILING = "import { test } from 'node:test';\ntest('bad', () => { throw new Error('x'); });\n";

// Fake cargo: logs its arguments; fails the subcommands listed in FAKE_CARGO_FAIL (printing
// lines that start with PASS/FAIL, which the gate must not pass through unindented); `cargo tree`
// prints FAKE_CARGO_TREE_OUT; `cargo test` prints FAKE_CARGO_TEST_OUT.
const FAKE_CARGO = `#!/bin/sh
echo "$*" >> "$FAKE_CARGO_LOG"
case " $FAKE_CARGO_FAIL " in *" $1 "*)
  echo "FAIL injected $1"; echo "PASS injected $1"; exit 101 ;;
esac
if [ "$1" = tree ]; then printf '%s\\n' "\${FAKE_CARGO_TREE_OUT:-facet-core v0.1.0}"; fi
if [ "$1" = test ] && [ -n "$FAKE_CARGO_TEST_OUT" ]; then printf '%s\\n' "$FAKE_CARGO_TEST_OUT"; fi
exit 0
`;

/**
 * A fixture repo: run_gates.sh copied in, the four Phase 0 acceptance dirs with one passing test
 * each (override with `files`: dir -> {name: source} or null for "no dir"), and a shim dir.
 */
function fixture({ files = {}, cargo = true, node = 'real' } = {}) {
  const top = mkdtempSync(join(tmpdir(), 'run-gates-unit-'));
  const repo = join(top, 'repo');
  const shims = join(top, 'shims');
  mkdirSync(repo);
  mkdirSync(shims);
  assert.ok(existsSync(SCRIPT), 'run_gates.sh does not exist at the repo root');
  copyFileSync(SCRIPT, join(repo, 'run_gates.sh'));
  chmodSync(join(repo, 'run_gates.sh'), 0o755);
  for (const d of PHASE0_DIRS) {
    const entries = d in files ? files[d] : { 'ac-1-ok.test.mjs': PASSING };
    if (entries === null) continue;
    mkdirSync(join(repo, 'acceptance', d), { recursive: true });
    for (const [name, src] of Object.entries(entries)) writeFileSync(join(repo, 'acceptance', d, name), src);
  }
  if (cargo) {
    writeFileSync(join(shims, 'cargo'), FAKE_CARGO);
    chmodSync(join(shims, 'cargo'), 0o755);
  }
  if (node === 'real') symlinkSync(process.execPath, join(shims, 'node'));
  else if (node !== null) {
    writeFileSync(join(shims, 'node'), `#!/bin/sh\n[ "$1" = --version ] && echo ${node} && exit 0\nexit 0\n`);
    chmodSync(join(shims, 'node'), 0o755);
  }
  const log = join(top, 'cargo.log');
  return { top, repo, log, shims, cleanup: () => rmSync(top, { recursive: true, force: true }) };
}

function run(fx, args, extraEnv = {}) {
  const env = {
    HOME: process.env.HOME, PATH: `${fx.shims}:/usr/bin:/bin`, FAKE_CARGO_LOG: fx.log, ...extraEnv,
  };
  const r = spawnSync('./run_gates.sh', args, { cwd: fx.repo, env, encoding: 'utf8', timeout: 30_000 });
  assert.equal(r.error, undefined, `could not run run_gates.sh: ${r.error}`);
  const all = `${r.stdout}${r.stderr}`;
  const lines = r.stdout.split('\n').filter((l) => l.trim() !== '');
  return {
    status: r.status, stdout: r.stdout, stderr: r.stderr, all,
    last: lines[lines.length - 1] ?? '',
    checks: r.stdout.split('\n').filter((l) => /^(PASS|FAIL) \S/.test(l)),
    cargoCalls: existsSync(fx.log) ? readFileSync(fx.log, 'utf8').trim().split('\n') : [],
  };
}

const ctx = (r) => `\n--- stdout ---\n${r.stdout}\n--- stderr ---\n${r.stderr}`;

function lineFor(r, re, what) {
  const hits = r.checks.filter((l) => re.test(l));
  assert.equal(hits.length, 1, `expected one PASS/FAIL line for ${what}${ctx(r)}`);
  return hits[0];
}

const fileLine = (d, f) => new RegExp(`^(PASS|FAIL) .*${d}/${f.replace('.', '\\.')}`);

// --- happy path -------------------------------------------------------------------------------

test('phase 0 with every check passing: exit 0, one PASS per check, cargo called as specified', () => {
  const fx = fixture({ files: { 'core-data-types': { 'ac-1-a.test.mjs': PASSING, 'ac-2-b.test.mjs': PASSING } } });
  try {
    const r = run(fx, ['0']);
    assert.equal(r.status, 0, `exit 0 expected${ctx(r)}`);
    for (const [, re] of SHARED) assert.match(lineFor(r, re, re.source), /^PASS /);
    for (const d of PHASE0_DIRS) {
      const names = d === 'core-data-types' ? ['ac-1-a.test.mjs', 'ac-2-b.test.mjs'] : ['ac-1-ok.test.mjs'];
      for (const f of names) assert.match(lineFor(r, fileLine(d, f), `${d}/${f}`), /^PASS /);
    }
    assert.equal(r.checks.length, DEFAULT_CHECKS + 1, `exactly one line per check${ctx(r)}`);
    assert.equal(r.last, 'phase 0: PASS');
    assert.deepEqual(r.cargoCalls, SHARED.map(([a]) => a), 'cargo must be called once per shared check, in order');
  } finally { fx.cleanup(); }
});

test('the gate writes nothing into the repository', () => {
  const fx = fixture();
  try {
    const list = (d) => readdirSync(d, { recursive: true }).sort();
    const before = list(fx.repo);
    assert.equal(run(fx, ['0']).status, 0);
    assert.deepEqual(list(fx.repo), before);
  } finally { fx.cleanup(); }
});

// --- failures still run every check -----------------------------------------------------------

test('a failing cargo check: exit 1, FAIL line, every later check still runs, output indented', () => {
  const fx = fixture();
  try {
    const r = run(fx, ['0'], { FAKE_CARGO_FAIL: 'fmt' });
    assert.equal(r.status, 1, `exit 1 expected${ctx(r)}`);
    assert.match(lineFor(r, SHARED[0][1], 'cargo fmt'), /^FAIL /);
    for (const [, re] of SHARED.slice(1)) assert.match(lineFor(r, re, re.source), /^PASS /);
    assert.deepEqual(r.cargoCalls, SHARED.map(([a]) => a), 'every cargo check must still run');
    assert.match(r.all, /injected fmt/, 'the failing output must be shown');
    assert.ok(!r.checks.some((l) => /injected/.test(l)), `failing output must not start a line with PASS/FAIL${ctx(r)}`);
    assert.equal(r.last, `phase 0: FAIL (1 of ${DEFAULT_CHECKS} checks failed)`);
  } finally { fx.cleanup(); }
});

test('several failures are all counted', () => {
  const fx = fixture({ files: { 'invariant-checkers': { 'ac-1-bad.test.mjs': FAILING } } });
  try {
    const r = run(fx, ['0'], { FAKE_CARGO_FAIL: 'clippy test' });
    assert.equal(r.status, 1);
    assert.equal(r.checks.filter((l) => l.startsWith('FAIL ')).length, 3, ctx(r));
    assert.equal(r.last, `phase 0: FAIL (3 of ${DEFAULT_CHECKS} checks failed)`);
  } finally { fx.cleanup(); }
});

test('a failing acceptance file is a FAIL even when NODE_TEST_CONTEXT is inherited', () => {
  const fx = fixture({ files: { 'benchmark-data-generator': { 'ac-9-bad.test.mjs': FAILING } } });
  try {
    for (const env of [{}, { NODE_TEST_CONTEXT: 'child-v8' }]) {
      const r = run(fx, ['0'], env);
      assert.equal(r.status, 1, `exit 1 expected with ${JSON.stringify(env)}${ctx(r)}`);
      assert.match(lineFor(r, fileLine('benchmark-data-generator', 'ac-9-bad.test.mjs'), 'bad file'), /^FAIL /);
      assert.equal(r.last, `phase 0: FAIL (1 of ${DEFAULT_CHECKS} checks failed)`);
    }
  } finally { fx.cleanup(); }
});

// --- acceptance directories: missing or empty is a FAIL, never a skip ---------------------------

test('a listed acceptance directory that is missing is a FAIL naming it', () => {
  const fx = fixture({ files: { 'invariant-checkers': null } });
  try {
    const r = run(fx, ['0']);
    assert.equal(r.status, 1, `exit 1 expected${ctx(r)}`);
    assert.match(lineFor(r, /^(PASS|FAIL) .*acceptance\/invariant-checkers/, 'missing dir'), /^FAIL /);
    assert.equal(r.last, `phase 0: FAIL (1 of ${DEFAULT_CHECKS} checks failed)`);
  } finally { fx.cleanup(); }
});

test('a listed acceptance directory with no *.test.mjs files is a FAIL naming it', () => {
  const fx = fixture({ files: { 'core-data-types': { 'README.md': '# none\n', 'helper.mjs': 'export {};\n' } } });
  try {
    const r = run(fx, ['0']);
    assert.equal(r.status, 1, `exit 1 expected${ctx(r)}`);
    assert.match(lineFor(r, /^(PASS|FAIL) .*acceptance\/core-data-types/, 'empty dir'), /^FAIL /);
    assert.ok(!r.checks.some((l) => /helper\.mjs|README/.test(l)), 'only *.test.mjs files are acceptance checks');
  } finally { fx.cleanup(); }
});

// --- forbidden-crate check ----------------------------------------------------------------------

test('the forbidden-crate check FAILs on each forbidden crate and on a failing cargo tree', () => {
  for (const crate of ['rusqlite', 'sled', 'rocksdb', 'duckdb', 'redb', 'lmdb']) {
    const fx = fixture();
    try {
      const r = run(fx, ['0'], { FAKE_CARGO_TREE_OUT: `facet-core v0.1.0\n└── ${crate} v1.2.3` });
      assert.equal(r.status, 1, `${crate} must fail the gate${ctx(r)}`);
      assert.match(lineFor(r, SHARED[4][1], 'forbidden'), /^FAIL /, crate);
    } finally { fx.cleanup(); }
  }
  const fx = fixture();
  try {
    // cargo tree printing nothing forbidden but exiting non-zero must not pass.
    const r = run(fx, ['0'], { FAKE_CARGO_FAIL: 'tree' });
    assert.equal(r.status, 1, `a failing cargo tree must fail the gate${ctx(r)}`);
    assert.match(lineFor(r, SHARED[4][1], 'forbidden'), /^FAIL /);
  } finally { fx.cleanup(); }
});

// --- exit 2: undefined phases, prerequisites, usage --------------------------------------------

test('phases 1-7 are not defined yet: exit 2 with the message, no cargo call', () => {
  const fx = fixture();
  try {
    for (let n = 1; n <= 7; n++) {
      const r = run(fx, [String(n)]);
      assert.equal(r.status, 2, `phase ${n}${ctx(r)}`);
      assert.match(r.all, new RegExp(`phase ${n} gate is not defined yet`));
      assert.doesNotMatch(r.all, /^PASS |: PASS$/m);
    }
    assert.deepEqual(r0(fx), [], 'no cargo call for undefined phases');
  } finally { fx.cleanup(); }
});

function r0(fx) {
  return existsSync(fx.log) ? readFileSync(fx.log, 'utf8').trim().split('\n').filter(Boolean) : [];
}

test('more invalid arguments exit 2 with usage on stderr and no cargo call', () => {
  const fx = fixture();
  try {
    for (const args of [[''], ['3.0'], ['0x1'], ['+1'], ['10'], ['0', '0'], ['--help']]) {
      const r = run(fx, args);
      assert.equal(r.status, 2, `${JSON.stringify(args)}${ctx(r)}`);
      assert.match(r.stderr, /usage/i, `${JSON.stringify(args)}: usage on stderr`);
    }
    assert.deepEqual(r0(fx), []);
  } finally { fx.cleanup(); }
});

test('missing cargo: exit 2 naming cargo, no check runs', () => {
  const fx = fixture({ cargo: false });
  try {
    const r = run(fx, ['0']);
    assert.equal(r.status, 2, ctx(r));
    assert.match(r.stderr, /cargo/);
    assert.deepEqual(r.checks, []);
  } finally { fx.cleanup(); }
});

test('missing node: exit 2 naming node, no check runs', () => {
  const fx = fixture({ node: null });
  try {
    const r = run(fx, ['0']);
    assert.equal(r.status, 2, ctx(r));
    assert.match(r.stderr, /node/);
    assert.deepEqual(r.checks, []);
    assert.deepEqual(r0(fx), [], 'no cargo check may run before prerequisites pass');
  } finally { fx.cleanup(); }
});

test('node older than 18: exit 2; node 18 and newer are accepted', () => {
  for (const [version, ok] of [['v16.20.2', false], ['v17.9.1', false], ['v18.0.0', true], ['v23.11.0', true]]) {
    const fx = fixture({ node: version, files: {} });
    try {
      // The fake node ignores test files, so the acceptance checks "pass"; only the version matters.
      const r = run(fx, ['0']);
      if (ok) assert.equal(r.status, 0, `${version} must be accepted${ctx(r)}`);
      else {
        assert.equal(r.status, 2, `${version} must be rejected${ctx(r)}`);
        assert.match(r.stderr, /node/);
        assert.deepEqual(r.checks, []);
      }
    } finally { fx.cleanup(); }
  }
});

// --- the committed script and README -------------------------------------------------------------

test('run_gates.sh is committed executable with a bash shebang, and README documents it', () => {
  const ls = spawnSync('git', ['ls-files', '-s', 'run_gates.sh'], { cwd: ROOT, encoding: 'utf8' });
  assert.match(ls.stdout, /^100755 /, 'run_gates.sh must be committed with mode 100755');
  assert.equal(readFileSync(SCRIPT, 'utf8').split('\n')[0], '#!/usr/bin/env bash');
  const readme = readFileSync(join(ROOT, 'README.md'), 'utf8');
  assert.match(readme, /\.\/run_gates\.sh <phase>/);
  for (const code of ['0', '1', '2']) assert.match(readme, new RegExp(`\\b${code}\\b.*(pass|fail|usage|error|defined)`, 'i'));
  assert.match(readme, /acceptance\//, 'README must say how a phase adds acceptance directories');
});

// --- cargo test counts (read by scripts/record_gate.sh) ------------------------------------------

test('the cargo test check is followed by an indented line with the summed pass/fail counts', () => {
  const fx = fixture();
  try {
    const out = [
      '   Running unittests src/lib.rs (target/debug/deps/facet_core-1)',
      'test result: ok. 54 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s',
      'test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s',
      '   Doc-tests facet_core',
      'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s',
    ].join('\n');
    const r = run(fx, ['0'], { FAKE_CARGO_TEST_OUT: out });
    assert.equal(r.status, 0, ctx(r));
    const lines = r.stdout.split('\n');
    const at = lines.findIndex((l) => /^PASS .*cargo test/.test(l));
    assert.ok(at >= 0, ctx(r));
    assert.equal(lines[at + 1], '    cargo test: 87 passed, 0 failed', `counts line after the check${ctx(r)}`);
    assert.equal(r.checks.length, DEFAULT_CHECKS, 'the counts line is not a check line');
  } finally { fx.cleanup(); }
});

test('the counts line also follows a failing cargo test check', () => {
  const fx = fixture();
  try {
    const out = 'test result: FAILED. 9 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s';
    const r = run(fx, ['0'], { FAKE_CARGO_TEST_OUT: out, FAKE_CARGO_FAIL: 'test' });
    assert.equal(r.status, 1, ctx(r));
    const lines = r.stdout.split('\n');
    const at = lines.findIndex((l) => /^FAIL .*cargo test/.test(l));
    assert.equal(lines[at + 1], '    cargo test: 9 passed, 2 failed', ctx(r));
  } finally { fx.cleanup(); }
});
