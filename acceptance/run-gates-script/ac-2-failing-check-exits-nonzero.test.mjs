// AC-2: when a Phase 0 check fails, `./run_gates.sh 0` exits 1, names the failed check, still runs
// every other check, and ends with `phase 0: FAIL (k of m checks failed)`.
// Case A: a panicking #[test] appended to crates/core/src/lib.rs → the cargo test check FAILs.
// Case B: one Phase 0 acceptance file replaced (in the clone only) by a failing test → exactly
// that file FAILs and every other check PASSes.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { appendFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import {
  SHARED_CHECKS, checkLine, cleanClone, fileRe, phase0Files, runGates,
} from './gate-harness.mjs';

const SUMMARY = /^phase 0: FAIL \((\d+) of (\d+) checks failed\)$/;

function summary(run, ctx) {
  const m = SUMMARY.exec(run.last);
  assert.ok(m, `last line must be "phase 0: FAIL (k of m checks failed)", got ${JSON.stringify(run.last)}${ctx}`);
  return { failed: Number(m[1]), total: Number(m[2]) };
}

test('AC-2a: a failing Rust test makes the gate exit 1 and the cargo test check FAIL', () => {
  const { dir, cleanup } = cleanClone();
  try {
    const files = phase0Files(dir);
    appendFileSync(join(dir, 'crates', 'core', 'src', 'lib.rs'),
      '\n#[test]\nfn gate_must_fail() {\n    panic!()\n}\n');
    const run = runGates(dir, ['0']);
    const ctx = `\n--- output tail ---\n${run.out.slice(-6000)}`;

    assert.equal(run.status, 1, `./run_gates.sh 0 must exit 1${ctx}`);
    assert.match(checkLine(run, /cargo test/, 'cargo test --workspace'), /^FAIL /,
      'the cargo test check must FAIL');
    // Every other check still ran: one PASS/FAIL line each.
    for (const [re, what] of SHARED_CHECKS) checkLine(run, re, what);
    for (const f of files) checkLine(run, fileRe(f), f);
    const { failed, total } = summary(run, ctx);
    assert.ok(failed >= 1 && failed <= total, `bad counts ${failed} of ${total}`);
    assert.ok(total >= SHARED_CHECKS.length + files.length,
      `total ${total} is less than the ${SHARED_CHECKS.length + files.length} checks Phase 0 must run`);
  } finally {
    cleanup();
  }
});

test('AC-2b: one failing acceptance file makes the gate exit 1 and only that file FAIL', () => {
  const { dir, cleanup } = cleanClone();
  try {
    const files = phase0Files(dir);
    const broken = 'invariant-checkers/ac-5-batch-sizes.test.mjs';
    assert.ok(files.includes(broken), `${broken} must be a Phase 0 acceptance file`);
    writeFileSync(join(dir, 'acceptance', broken),
      "import { test } from 'node:test';\n"
      + "test('injected failure', () => { throw new Error('injected by AC-2b'); });\n");
    const run = runGates(dir, ['0']);
    const ctx = `\n--- output tail ---\n${run.out.slice(-6000)}`;

    assert.equal(run.status, 1, `./run_gates.sh 0 must exit 1${ctx}`);
    assert.match(checkLine(run, fileRe(broken), broken), /^FAIL /, `${broken} must FAIL`);
    for (const [re, what] of SHARED_CHECKS) {
      assert.match(checkLine(run, re, what), /^PASS /, `${what} must still PASS`);
    }
    for (const f of files.filter((f) => f !== broken)) {
      assert.match(checkLine(run, fileRe(f), f), /^PASS /, `${f} must still PASS`);
    }
    const { failed, total } = summary(run, ctx);
    assert.equal(failed, 1, `exactly 1 check failed, summary says ${failed}${ctx}`);
    assert.equal(total, run.checkLines.length,
      `summary total ${total} must equal the ${run.checkLines.length} PASS/FAIL lines printed`);
  } finally {
    cleanup();
  }
});
