// Unit tests for scripts/record_gate.sh beyond the locked acceptance tests in
// acceptance/progress-tracking. They reuse that directory's harness (read-only): each case builds
// a temp git repo with a copy of the script, a fixture PROGRESS.md, a stub ./run_gates.sh that logs
// its arguments, an `init` commit 5 h old and one more commit `minutesAgo` old.
//
// Run: node --test scripts/record-gate.test.mjs
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { chmodSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import {
  ROOT, fixtureRepo, gateCalls, gateEntry, git, head, minutesSpent, read, record, section, subject,
} from '../acceptance/progress-tracking/progress-harness.mjs';

const pass = (n) => [`PASS cargo test --workspace`, `phase ${n}: PASS`];
const decisionsOf = (dir) => section(read(dir), 'Decisions').map((l) => l.trim()).filter(Boolean);

/** Asserts the run changed nothing: same PROGRESS.md bytes, same HEAD. */
function assertUnchanged(dir, before, headBefore, r) {
  assert.equal(read(dir), before, `PROGRESS.md must be unchanged:\n${r.out}`);
  assert.equal(head(dir), headBefore, 'no new commit');
}

// --- time spent and the previous gate ----------------------------------------------------------

test('phase 0 time is measured from the first commit; an older "Phase 0: set up …" commit is not a gate', () => {
  const { dir, cleanup } = fixtureRepo({
    previousGate: 'Phase 0: set up cargo workspace with the eight facet crates', minutesAgo: 10,
    gateOutput: pass(0),
  });
  try {
    const r = record(dir, ['0']);
    assert.equal(r.status, 0, r.out);
    const mins = minutesSpent(gateEntry(read(dir), 0));
    assert.ok(mins >= 299 && mins <= 301, `phase 0 time must run from the first commit (300 min), got ${mins}`);
    assert.equal(subject(dir), 'Phase 0: gate passed');
  } finally { cleanup(); }
});

test('phase N > 0 without a "Phase N-1: gate passed" commit: exit 2, gate not run, nothing recorded', () => {
  for (const previousGate of [
    'Phase 0: set up cargo workspace with the eight facet crates', // looks like a gate, is not
    'Phase 1: gate passed', // a gate, but not the previous phase's
    'Phase 2: gate passed (draft)', // not the exact subject
  ]) {
    const { dir, cleanup } = fixtureRepo({ previousGate, minutesAgo: 10, gateOutput: pass(3) });
    try {
      const before = read(dir);
      const headBefore = head(dir);
      const r = record(dir, ['3']);
      assert.equal(r.status, 2, `previous commit "${previousGate}": exit 2 expected\n${r.out}`);
      assert.match(r.out, /Phase 2: gate passed/, 'the message must name the missing gate commit');
      assert.deepEqual(gateCalls(dir), [], 'the gate must not run');
      assertUnchanged(dir, before, headBefore, r);
    } finally { cleanup(); }
  }
});

test('the most recent previous gate commit is used when there are several', () => {
  const { dir, cleanup } = fixtureRepo({ previousGate: 'Phase 2: gate passed', minutesAgo: 400, gateOutput: pass(3) });
  try {
    const now = Math.floor(Date.now() / 1000) - 45 * 60;
    git(dir, ['commit', '-q', '--allow-empty', '-m', 'Phase 2: gate passed'],
      { GIT_AUTHOR_DATE: `@${now} +0000`, GIT_COMMITTER_DATE: `@${now} +0000` });
    const r = record(dir, ['3']);
    assert.equal(r.status, 0, r.out);
    const mins = minutesSpent(gateEntry(read(dir), 3));
    assert.ok(mins >= 44 && mins <= 46, `must measure from the latest Phase 2 gate (45 min), got ${mins}`);
  } finally { cleanup(); }
});

// --- arguments ---------------------------------------------------------------------------------

test('invalid arguments: exit 2, gate not run, nothing recorded', () => {
  const { dir, cleanup } = fixtureRepo({ previousGate: 'Phase 2: gate passed', minutesAgo: 10, gateOutput: pass(3) });
  try {
    const before = read(dir);
    const headBefore = head(dir);
    for (const args of [[], ['x'], ['10'], ['-1'], ['3.0'], ['3', '--bogus'], ['3', '--decision'], ['3', '--cut'], ['3', 'extra']]) {
      const r = record(dir, args);
      assert.equal(r.status, 2, `${JSON.stringify(args)} must exit 2\n${r.out}`);
      assert.match(r.out, /usage|unknown|requires|invalid/i, `${JSON.stringify(args)}: explain the error`);
    }
    assert.deepEqual(gateCalls(dir), [], 'the gate must never run on a usage error');
    assertUnchanged(dir, before, headBefore, { out: '' });
  } finally { cleanup(); }
});

// --- budget and cut ----------------------------------------------------------------------------

test('a budget of "—" never requires a cut', () => {
  const { dir, cleanup } = fixtureRepo({ previousGate: 'Phase 2: gate passed', minutesAgo: 500, gateOutput: pass(3) });
  try {
    const r = record(dir, ['3']);
    assert.equal(r.status, 0, `no budget, no cut required:\n${r.out}`);
    assert.doesNotMatch(gateEntry(read(dir), 3), /cut:/);
  } finally { cleanup(); }
});

test('exactly 20 minutes over budget does not require a cut; 21 does', () => {
  for (const [minutesAgo, status] of [[80, 0], [81, 2]]) {
    const { dir, cleanup } = fixtureRepo({ budgets: { 4: 60 }, previousGate: 'Phase 3: gate passed', minutesAgo, gateOutput: pass(4) });
    try {
      const r = record(dir, ['4']);
      assert.equal(r.status, status, `${minutesAgo} min against a 60 min budget\n${r.out}`);
    } finally { cleanup(); }
  }
});

test('an overrun without --cut is refused before the gate runs', () => {
  const { dir, cleanup } = fixtureRepo({ budgets: { 4: 60 }, previousGate: 'Phase 3: gate passed', minutesAgo: 120, gateOutput: pass(4) });
  try {
    const r = record(dir, ['4']);
    assert.equal(r.status, 2, r.out);
    assert.match(r.out, /run-length and bit-packing/);
    assert.deepEqual(gateCalls(dir), [], 'no point running a gate whose record would be refused');
  } finally { cleanup(); }
});

// --- decisions -------------------------------------------------------------------------------

test('decisions: the first replaces "(none)", later ones are appended in order and earlier ones kept', () => {
  const { dir, cleanup } = fixtureRepo({ previousGate: 'Phase 2: gate passed', minutesAgo: 10, gateOutput: pass(3) });
  try {
    assert.equal(record(dir, ['3', '--decision', 'first']).status, 0);
    writeFileSync(join(dir, 'run_gates.sh'), "#!/bin/sh\necho \"$*\" >> \"$(dirname \"$0\")/gate-calls.log\"\necho 'phase 4: PASS'\n");
    chmodSync(join(dir, 'run_gates.sh'), 0o755);
    const r = record(dir, ['4', '--decision', 'second', '--decision', 'third']);
    assert.equal(r.status, 0, r.out);
    assert.deepEqual(decisionsOf(dir), ['- Phase 3: first', '- Phase 4: second', '- Phase 4: third']);
    const entry = gateEntry(read(dir), 4);
    assert.ok(entry.indexOf('second') < entry.indexOf('third'), 'entry keeps the decisions in order');
    assert.match(gateEntry(read(dir), 3), /first/, 'the earlier entry is kept');
  } finally { cleanup(); }
});

test('decision and cut text with quotes, slashes, &, backslashes and $ is written verbatim', () => {
  const tricky = `it's "quoted" a/b & c \\ d $HOME %s \`x\``;
  const { dir, cleanup } = fixtureRepo({ budgets: { 4: 60 }, previousGate: 'Phase 3: gate passed', minutesAgo: 120, gateOutput: pass(4) });
  try {
    const r = record(dir, ['4', '--decision', tricky, '--cut', tricky]);
    assert.equal(r.status, 0, r.out);
    assert.deepEqual(decisionsOf(dir), [`- Phase 4: ${tricky}`]);
    const entry = gateEntry(read(dir), 4);
    assert.ok(entry.includes(tricky), `entry must contain the decision verbatim:\n${entry}`);
    assert.ok(entry.includes(`cut: ${tricky}`), `entry must contain the cut verbatim:\n${entry}`);
  } finally { cleanup(); }
});

// --- the entry ---------------------------------------------------------------------------------

test('the entry has the date, the gate summary, the number of checks and the cargo test counts', () => {
  const { dir, cleanup } = fixtureRepo({
    previousGate: 'Phase 2: gate passed', minutesAgo: 10,
    gateOutput: [
      'PASS cargo fmt --all --check', 'PASS cargo test --workspace', '    cargo test: 87 passed, 0 failed',
      'PASS acceptance/x/ac-1.test.mjs', 'phase 3: PASS',
    ],
  });
  try {
    assert.equal(record(dir, ['3']).status, 0);
    const entry = gateEntry(read(dir), 3);
    const today = new Date();
    const ymd = [today.getFullYear(), String(today.getMonth() + 1).padStart(2, '0'), String(today.getDate()).padStart(2, '0')].join('-');
    assert.ok(entry.includes(ymd), `entry must carry today's date ${ymd}:\n${entry}`);
    assert.match(entry, /phase 3: PASS/);
    assert.match(entry, /\b3 checks\b/, 'number of PASS/FAIL check lines');
    assert.match(entry, /87 passed, 0 failed/, 'cargo test counts from the gate output');
  } finally { cleanup(); }
});

test('a gate that prints no cargo test counts is still recorded', () => {
  const { dir, cleanup } = fixtureRepo({ previousGate: 'Phase 2: gate passed', minutesAgo: 10, gateOutput: ['phase 3: PASS'] });
  try {
    const r = record(dir, ['3']);
    assert.equal(r.status, 0, r.out);
    assert.match(gateEntry(read(dir), 3), /phase 3: PASS/);
  } finally { cleanup(); }
});

// --- gate run and commit ---------------------------------------------------------------------

test('a failing gate: its output is shown and its exit status returned', () => {
  const { dir, cleanup } = fixtureRepo({
    previousGate: 'Phase 2: gate passed', minutesAgo: 10, gateStatus: 3,
    gateOutput: ['FAIL cargo test --workspace', 'phase 3: FAIL (1 of 5 checks failed)'],
  });
  try {
    const r = record(dir, ['3']);
    assert.equal(r.status, 3, r.out);
    assert.match(r.out, /phase 3: FAIL \(1 of 5 checks failed\)/);
  } finally { cleanup(); }
});

test('the gate runs without NODE_TEST_CONTEXT', () => {
  const { dir, cleanup } = fixtureRepo({ previousGate: 'Phase 2: gate passed', minutesAgo: 10, gateOutput: pass(3) });
  try {
    writeFileSync(join(dir, 'run_gates.sh'),
      "#!/bin/sh\nprintf '%s' \"${NODE_TEST_CONTEXT-unset}\" > \"$(dirname \"$0\")/ctx.txt\"\necho 'phase 3: PASS'\n");
    chmodSync(join(dir, 'run_gates.sh'), 0o755);
    const r = spawnSync('scripts/record_gate.sh', ['3'], {
      cwd: dir, encoding: 'utf8', env: { ...process.env, NODE_TEST_CONTEXT: 'child-v8' },
    });
    assert.equal(r.status, 0, `${r.stdout}${r.stderr}`);
    assert.equal(readFileSync(join(dir, 'ctx.txt'), 'utf8'), 'unset');
  } finally { cleanup(); }
});

test('only PROGRESS.md is committed; other staged and unstaged changes stay as they were', () => {
  const { dir, cleanup } = fixtureRepo({ previousGate: 'Phase 2: gate passed', minutesAgo: 10, gateOutput: pass(3) });
  try {
    writeFileSync(join(dir, 'notes.txt'), 'staged\n');
    git(dir, ['add', 'notes.txt']);
    writeFileSync(join(dir, 'notes.txt'), 'staged then edited\n');
    writeFileSync(join(dir, 'new.txt'), 'untracked\n');
    assert.equal(record(dir, ['3']).status, 0);
    assert.deepEqual(git(dir, ['show', '--name-only', '--format=', 'HEAD']).split('\n'), ['PROGRESS.md']);
    assert.equal(git(dir, ['diff', '--cached', '--name-only']), 'notes.txt', 'notes.txt is still staged');
    assert.equal(git(dir, ['show', ':notes.txt']), 'staged', 'the staged content is unchanged');
    assert.equal(readFileSync(join(dir, 'notes.txt'), 'utf8'), 'staged then edited\n');
    assert.match(git(dir, ['status', '--porcelain']), /\?\? new\.txt/, 'untracked file left alone');
  } finally { cleanup(); }
});

test('the script never amends: the previous gate commit is still an ancestor', () => {
  const { dir, cleanup } = fixtureRepo({ previousGate: 'Phase 2: gate passed', minutesAgo: 10, gateOutput: pass(3) });
  try {
    const prev = head(dir);
    assert.equal(record(dir, ['3']).status, 0);
    assert.equal(git(dir, ['rev-parse', 'HEAD~1']), prev);
  } finally { cleanup(); }
});

// --- the real repository -----------------------------------------------------------------------

test('record_gate.sh is committed executable with a bash shebang, and README documents it', () => {
  const ls = spawnSync('git', ['ls-files', '-s', 'scripts/record_gate.sh'], { cwd: ROOT, encoding: 'utf8' });
  assert.match(ls.stdout, /^100755 /, 'scripts/record_gate.sh must be committed with mode 100755');
  assert.equal(readFileSync(join(ROOT, 'scripts', 'record_gate.sh'), 'utf8').split('\n')[0], '#!/usr/bin/env bash');
  const readme = readFileSync(join(ROOT, 'README.md'), 'utf8');
  assert.match(readme, /Recording a gate/);
  assert.match(readme, /scripts\/record_gate\.sh <phase>/);
});

test('run_gates.sh lists progress-tracking for phase 0', () => {
  const gates = readFileSync(join(ROOT, 'run_gates.sh'), 'utf8');
  const line = gates.split('\n').find((l) => /^\s*0\) acceptance_dirs=/.test(l));
  assert.ok(line, 'no phase 0 entry in run_gates.sh');
  assert.match(line, /\bprogress-tracking\b/);
});

test('the real Phase 0 gate is recorded and committed', () => {
  const subjects = spawnSync('git', ['log', '--format=%s'], { cwd: ROOT, encoding: 'utf8' }).stdout.split('\n');
  assert.ok(subjects.includes('Phase 0: gate passed'), 'no "Phase 0: gate passed" commit');
  const text = readFileSync(join(ROOT, 'PROGRESS.md'), 'utf8');
  const entry = gateEntry(text, 0);
  assert.match(entry, /phase 0: PASS/);
  assert.match(entry, /\d+ passed, 0 failed/, 'the real entry carries the cargo test counts');
  assert.ok(section(text, 'Decisions').some((l) => /^- Phase 0: .+ \(source: .+\)$/.test(l.trim())),
    'Phase 0 deviations are listed with their sources');
});
