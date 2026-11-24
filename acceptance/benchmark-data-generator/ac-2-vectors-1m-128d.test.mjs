// AC-2: `facet-gen vectors --seed 42 --out <tmp>` (default 1,000,000 vectors) writes
//   vectors.f32: exactly 1,000,000 x 128 little-endian f32 = 512,000,000 bytes, every value finite;
//   vectors_meta.csv: header `id,category,price` + exactly 1,000,000 lines, id = 0..999,999 in order,
//   category non-empty from at most 100 distinct values, price a finite number > 0.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { closeSync, existsSync, mkdtempSync, openSync, readFileSync, readSync, rmSync, statSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
const N = 1_000_000;
const DIM = 128;

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

test('AC-2: facet-gen vectors writes 1M finite 128-d vectors with category and price', () => {
  const exe = buildGenerator();
  const out = mkdtempSync(join(tmpdir(), 'facet-gen-ac-2-'));
  try {
    const r = spawnSync(exe, ['vectors', '--seed', '42', '--out', out], {
      cwd: ROOT, encoding: 'utf8', maxBuffer: 64 << 20,
    });
    assert.equal(r.status, 0, `facet-gen vectors exited ${r.status}:\n${r.stderr}`);

    const vecPath = join(out, 'vectors.f32');
    assert.ok(existsSync(vecPath), 'vectors.f32 was not written');
    assert.equal(statSync(vecPath).size, N * DIM * 4, 'vectors.f32 size in bytes');

    // Every f32 is finite (little-endian; read in 64 MB aligned chunks).
    const chunk = Buffer.alloc(64 << 20);
    const fd = openSync(vecPath, 'r');
    try {
      let offset = 0;
      for (;;) {
        const n = readSync(fd, chunk, 0, chunk.length, null);
        if (n === 0) break;
        assert.equal(n % 4, 0, 'short read not aligned to f32');
        for (let i = 0; i < n; i += 4) {
          const v = chunk.readFloatLE(i);
          if (!Number.isFinite(v)) assert.fail(`vectors.f32 value #${(offset + i) / 4} is ${v}`);
        }
        offset += n;
      }
      assert.equal(offset, N * DIM * 4);
    } finally {
      closeSync(fd);
    }

    const metaPath = join(out, 'vectors_meta.csv');
    assert.ok(existsSync(metaPath), 'vectors_meta.csv was not written');
    const lines = readFileSync(metaPath, 'utf8').split('\n');
    if (lines.at(-1) === '') lines.pop();
    assert.equal(lines[0].replace(/\r$/, ''), 'id,category,price', 'vectors_meta.csv header');
    assert.equal(lines.length - 1, N, 'number of data lines in vectors_meta.csv');
    const categories = new Set();
    for (let i = 1; i < lines.length; i++) {
      const f = lines[i].replace(/\r$/, '').split(',');
      if (f.length !== 3) assert.fail(`meta line ${i} has ${f.length} fields: ${lines[i]}`);
      const [id, category, price] = f;
      if (id !== String(i - 1)) assert.fail(`meta line ${i}: id ${JSON.stringify(id)}, want ${i - 1}`);
      if (category.length === 0) assert.fail(`meta line ${i}: empty category`);
      categories.add(category);
      const p = Number(price);
      if (price.trim() === '' || !Number.isFinite(p) || !(p > 0)) {
        assert.fail(`meta line ${i}: price ${JSON.stringify(price)} is not a finite number > 0`);
      }
    }
    assert.ok(categories.size <= 100, `${categories.size} distinct categories (at most 100)`);
  } finally {
    rmSync(out, { recursive: true, force: true });
  }
});
