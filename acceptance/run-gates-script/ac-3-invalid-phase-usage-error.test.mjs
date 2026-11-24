// AC-3: without a valid phase number (no argument, 8, -1, x, or an extra argument),
// `./run_gates.sh` exits 2 within a few seconds, prints a usage line to stderr, reports no
// PASS, and never invokes cargo (a recording `cargo` shim sits first on PATH).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { chmodSync, existsSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { ROOT } from './gate-harness.mjs';

const CASES = [[], ['8'], ['-1'], ['x'], ['0', '1']];

test('AC-3: invalid phase arguments exit 2 with usage and run no cargo command', () => {
  assert.ok(existsSync(join(ROOT, 'run_gates.sh')), 'run_gates.sh does not exist at the repo root');
  const shimDir = mkdtempSync(join(tmpdir(), 'facet-gates-shim-'));
  try {
    const marker = join(shimDir, 'cargo-was-called');
    writeFileSync(join(shimDir, 'cargo'), `#!/bin/sh\necho "$@" >> '${marker}'\nexit 0\n`);
    chmodSync(join(shimDir, 'cargo'), 0o755);
    const env = { ...process.env, PATH: `${shimDir}:${process.env.PATH}` };

    for (const args of CASES) {
      const label = `./run_gates.sh ${args.join(' ')}`.trim();
      const r = spawnSync(join(ROOT, 'run_gates.sh'), args, {
        cwd: ROOT, env, encoding: 'utf8', timeout: 10_000,
      });
      assert.equal(r.error, undefined, `${label}: could not run or timed out: ${r.error}`);
      assert.equal(r.status, 2, `${label} must exit 2, got ${r.status}\nstdout:\n${r.stdout}\nstderr:\n${r.stderr}`);
      assert.match(r.stderr, /usage/i, `${label}: stderr must contain the usage line`);
      assert.match(r.stderr, /run_gates\.sh/, `${label}: the usage line must name run_gates.sh`);
      assert.doesNotMatch(r.stdout + r.stderr, /^PASS |: PASS$/m, `${label} must not report PASS`);
      assert.ok(!existsSync(marker), `${label} invoked cargo`);
    }
  } finally {
    rmSync(shimDir, { recursive: true, force: true });
  }
});
