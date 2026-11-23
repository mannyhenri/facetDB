// AC-2: with a passing gate, `scripts/record_gate.sh 3 --decision "kept B-tree index"` runs
// ./run_gates.sh 3, appends a `### Phase 3` entry (time spent 30 min ±1, `phase 3: PASS`, the
// decision), adds `- Phase 3: kept B-tree index` under Decisions, and commits only PROGRESS.md as
// `Phase 3: gate passed`. With a failing gate nothing is recorded or committed.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import {
  changedInHead, fixtureRepo, gateCalls, gateEntry, git, head, minutesSpent, read, record, section,
  subject,
} from './progress-harness.mjs';

test('AC-2a: a passing gate is recorded and committed with the phase number', () => {
  const { dir, cleanup } = fixtureRepo({
    previousGate: 'Phase 2: gate passed', minutesAgo: 30,
    gateOutput: ['PASS cargo test --workspace', 'phase 3: PASS'],
  });
  try {
    // An unrelated staged change must not end up in the gate commit.
    writeFileSync(join(dir, 'notes.txt'), 'changed\n');
    git(dir, ['add', 'notes.txt']);
    const before = git(dir, ['rev-list', '--count', 'HEAD']);

    const r = record(dir, ['3', '--decision', 'kept B-tree index']);
    assert.equal(r.status, 0, `record_gate.sh must exit 0:\n${r.out}`);
    assert.deepEqual(gateCalls(dir), ['3'], 'record_gate.sh must run ./run_gates.sh 3 exactly once');

    const text = read(dir);
    const entry = gateEntry(text, 3);
    const mins = minutesSpent(entry);
    assert.ok(mins >= 29 && mins <= 31, `time spent must be 30 min (±1), got ${mins}`);
    assert.match(entry, /phase 3: PASS/, 'the entry must include the gate result');
    assert.match(entry, /kept B-tree index/, 'the entry must include the decision');

    const decisions = section(text, 'Decisions').map((l) => l.trim()).filter(Boolean);
    assert.ok(decisions.includes('- Phase 3: kept B-tree index'),
      `Decisions must list "- Phase 3: kept B-tree index", got:\n${decisions.join('\n')}`);
    assert.ok(!decisions.includes('(none)'), '"(none)" must be gone once a decision is recorded');

    assert.equal(subject(dir), 'Phase 3: gate passed');
    assert.equal(Number(git(dir, ['rev-list', '--count', 'HEAD'])), Number(before) + 1, 'exactly one new commit');
    assert.deepEqual(changedInHead(dir), ['PROGRESS.md'], 'the gate commit must change only PROGRESS.md');
    assert.equal(git(dir, ['status', '--porcelain', '--', 'PROGRESS.md']), '', 'PROGRESS.md must be fully committed');
  } finally {
    cleanup();
  }
});

test('AC-2b: a failing gate records nothing and commits nothing', () => {
  const { dir, cleanup } = fixtureRepo({
    previousGate: 'Phase 2: gate passed', minutesAgo: 30, gateStatus: 1,
    gateOutput: ['FAIL cargo test --workspace', 'phase 3: FAIL (1 of 5 checks failed)'],
  });
  try {
    const before = read(dir);
    const headBefore = head(dir);
    const r = record(dir, ['3', '--decision', 'kept B-tree index']);
    assert.notEqual(r.status, 0, `record_gate.sh must exit non-zero when the gate fails:\n${r.out}`);
    assert.deepEqual(gateCalls(dir), ['3'], 'record_gate.sh must run ./run_gates.sh 3');
    assert.equal(read(dir), before, 'PROGRESS.md must be byte-identical');
    assert.equal(head(dir), headBefore, 'no new commit');
  } finally {
    cleanup();
  }
});
