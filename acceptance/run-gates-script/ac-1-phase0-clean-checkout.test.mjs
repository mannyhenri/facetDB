// AC-1: in a fresh clone of HEAD, `./run_gates.sh 0` runs every Phase 0 check and passes:
// exit 0; one PASS line per shared check (fmt, clippy, build, test, forbidden crates) and per
// *.test.mjs under the four Phase 0 acceptance dirs (each file named); no FAIL line; the last
// line is exactly `phase 0: PASS`.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  SHARED_CHECKS, checkLine, cleanClone, fileRe, phase0Files, runGates,
} from './gate-harness.mjs';

test('AC-1: ./run_gates.sh 0 passes from a clean checkout', () => {
  const { dir, cleanup } = cleanClone();
  try {
    const files = phase0Files(dir);
    const run = runGates(dir, ['0']);
    const ctx = `\n--- output tail ---\n${run.out.slice(-6000)}`;

    assert.equal(run.status, 0, `./run_gates.sh 0 must exit 0${ctx}`);
    assert.deepEqual(run.checkLines.filter((l) => l.startsWith('FAIL ')), [], `no check may FAIL${ctx}`);
    for (const [re, what] of SHARED_CHECKS) {
      assert.match(checkLine(run, re, what), /^PASS /, `${what} must PASS`);
    }
    for (const f of files) {
      assert.match(checkLine(run, fileRe(f), f), /^PASS /, `${f} must PASS`);
    }
    assert.equal(run.last, 'phase 0: PASS', `last line must be "phase 0: PASS"${ctx}`);
  } finally {
    cleanup();
  }
});
