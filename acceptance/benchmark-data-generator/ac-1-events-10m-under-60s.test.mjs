// AC-1: `facet-gen events --rows 10000000 --seed 42 --out <tmp>` writes 10,000,000 event rows in
// under 60 seconds (release build; compile time excluded because the binary is built first).
// events.csv = 1 header line with the 18 named columns + exactly 10,000,000 data lines of 18 fields,
// and event_id (first field) is exactly 0..9,999,999 in order.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { closeSync, existsSync, mkdtempSync, openSync, readSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { performance } from 'node:perf_hooks';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
const ROWS = 10_000_000;
const LIMIT_MS = 60_000;
const HEADER = [
  'event_id', 'event_time', 'user_id', 'session_id', 'url', 'referer', 'title', 'search_phrase',
  'country', 'region_id', 'browser', 'os', 'device_type', 'is_mobile', 'resolution_width',
  'resolution_height', 'duration_ms', 'revenue',
];

function buildGenerator() {
  const b = spawnSync('cargo', ['build', '--release', '-p', 'facet-bench', '--bin', 'facet-gen'], {
    cwd: ROOT, encoding: 'utf8', maxBuffer: 64 << 20,
  });
  assert.equal(b.status, 0, `release build of facet-gen failed:\n${b.stderr}`);
  const m = spawnSync('cargo', ['metadata', '--format-version', '1', '--no-deps'], {
    cwd: ROOT, encoding: 'utf8', maxBuffer: 64 << 20,
  });
  assert.equal(m.status, 0, `cargo metadata failed:\n${m.stderr}`);
  const exe = join(JSON.parse(m.stdout).target_directory, 'release',
    process.platform === 'win32' ? 'facet-gen.exe' : 'facet-gen');
  assert.ok(existsSync(exe), `facet-gen binary not found at ${exe}`);
  return exe;
}

// Calls onLine(bytes, start, end) for each line (without '\n'), streaming in 64 MB chunks.
function forEachLine(path, onLine) {
  const fd = openSync(path, 'r');
  try {
    let buf = Buffer.alloc(64 << 20);
    let carry = 0; // bytes of an unfinished line kept at the front of buf
    for (;;) {
      if (carry === buf.length) { const bigger = Buffer.alloc(buf.length * 2); buf.copy(bigger, 0, 0, carry); buf = bigger; }
      const n = readSync(fd, buf, carry, buf.length - carry, null);
      const end = carry + n;
      if (n === 0) { if (carry > 0) onLine(buf, 0, carry); return; }
      let start = 0;
      for (let i = buf.indexOf(10, carry); i !== -1 && i < end; i = buf.indexOf(10, i + 1)) {
        onLine(buf, start, i);
        start = i + 1;
      }
      buf.copy(buf, 0, start, end);
      carry = end - start;
    }
  } finally {
    closeSync(fd);
  }
}

test('AC-1: facet-gen events writes 10,000,000 rows in under 60 s', () => {
  const exe = buildGenerator();
  const out = mkdtempSync(join(tmpdir(), 'facet-gen-ac-1-'));
  try {
    const t0 = performance.now();
    const r = spawnSync(exe, ['events', '--rows', String(ROWS), '--seed', '42', '--out', out], {
      cwd: ROOT, encoding: 'utf8', maxBuffer: 64 << 20,
    });
    const elapsed = performance.now() - t0;
    assert.equal(r.status, 0, `facet-gen events exited ${r.status}:\n${r.stderr}`);
    assert.ok(elapsed < LIMIT_MS, `facet-gen events took ${(elapsed / 1000).toFixed(1)} s (limit 60 s)`);

    const file = join(out, 'events.csv');
    assert.ok(existsSync(file), 'events.csv was not written');
    let line = 0;
    let header = null;
    forEachLine(file, (buf, s, e) => {
      if (line === 0) {
        header = buf.toString('utf8', s, e).replace(/\r$/, '');
      } else {
        const want = line - 1;
        let commas = 0;
        let firstComma = -1;
        for (let i = s; i < e; i++) {
          if (buf[i] === 44) { if (firstComma === -1) firstComma = i; commas++; }
        }
        if (commas !== HEADER.length - 1) {
          assert.fail(`data line ${line} has ${commas + 1} fields, want 18: ${buf.toString('utf8', s, Math.min(e, s + 300))}`);
        }
        const id = buf.toString('latin1', s, firstComma);
        if (id !== String(want)) assert.fail(`data line ${line}: event_id ${JSON.stringify(id)}, want ${want}`);
      }
      line++;
    });
    assert.deepEqual(header === null ? null : header.split(','), HEADER, 'events.csv header');
    assert.equal(line - 1, ROWS, 'number of data lines in events.csv');
  } finally {
    rmSync(out, { recursive: true, force: true });
  }
});
