// AC-3: Phase 4 has a 60-minute budget and cut 'run-length and bit-packing'. 85 min after the
// Phase 3 gate (+25), recording without --cut exits 2 naming the cut and changes nothing; with
// --cut the entry says `cut: run-length and bit-packing` and the commit is `Phase 4: gate passed`.
// 70 min after (+10, within 20) it records without --cut.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { fixtureRepo, gateEntry, head, read, record, subject } from './progress-harness.mjs';

const PASSING = ['PASS cargo test --workspace', 'phase 4: PASS'];

test('AC-3a: an overrun of more than 20 minutes requires the section 7 cut', () => {
  const { dir, cleanup } = fixtureRepo({
    budgets: { 4: 60 }, previousGate: 'Phase 3: gate passed', minutesAgo: 85, gateOutput: PASSING,
  });
  try {
    const before = read(dir);
    const headBefore = head(dir);
    const refused = record(dir, ['4']);
    assert.equal(refused.status, 2, `without --cut record_gate.sh must exit 2:\n${refused.out}`);
    assert.match(refused.out, /run-length and bit-packing/, 'the message must name the section 7 cut');
    assert.equal(read(dir), before, 'PROGRESS.md must be unchanged');
    assert.equal(head(dir), headBefore, 'no commit');

    const ok = record(dir, ['4', '--cut', 'run-length and bit-packing']);
    assert.equal(ok.status, 0, `with --cut record_gate.sh must exit 0:\n${ok.out}`);
    assert.match(gateEntry(read(dir), 4), /cut: run-length and bit-packing/);
    assert.equal(subject(dir), 'Phase 4: gate passed');
  } finally {
    cleanup();
  }
});

test('AC-3b: an overrun within 20 minutes records without a cut', () => {
  const { dir, cleanup } = fixtureRepo({
    budgets: { 4: 60 }, previousGate: 'Phase 3: gate passed', minutesAgo: 70, gateOutput: PASSING,
  });
  try {
    const r = record(dir, ['4']);
    assert.equal(r.status, 0, `+10 min must record without --cut:\n${r.out}`);
    assert.doesNotMatch(gateEntry(read(dir), 4), /cut:/, 'no cut line when none was taken');
    assert.equal(subject(dir), 'Phase 4: gate passed');
  } finally {
    cleanup();
  }
});
