// AC-3: `facet-gen orders --seed 42 --out <tmp>` (default 1,000,000 rows) writes orders.csv with header
// `order_id,customer_id,product_id,quantity,price,status` + 1,000,000 rows whose order_id values are
// unique; the same seed always produces the same data: orders, vectors and events (--rows 100000) run
// twice with --seed 42 give byte-identical files (equal SHA-256), and --seed 43 gives a different
// orders.csv.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { closeSync, existsSync, mkdtempSync, openSync, readFileSync, readSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
const ORDERS = 1_000_000;
const HEADER = 'order_id,customer_id,product_id,quantity,price,status';

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

function sha256(path) {
  assert.ok(existsSync(path), `${path} was not written`);
  const h = createHash('sha256');
  const buf = Buffer.alloc(16 << 20);
  const fd = openSync(path, 'r');
  try {
    for (let n; (n = readSync(fd, buf, 0, buf.length, null)) > 0;) h.update(buf.subarray(0, n));
  } finally {
    closeSync(fd);
  }
  return h.digest('hex');
}

test('AC-3: orders is a keyed table and every dataset is deterministic per seed', () => {
  const exe = buildGenerator();
  const base = mkdtempSync(join(tmpdir(), 'facet-gen-ac-3-'));
  const run = (name, args) => {
    const out = join(base, name);
    const r = spawnSync(exe, [...args, '--out', out], { cwd: ROOT, encoding: 'utf8', maxBuffer: 64 << 20 });
    assert.equal(r.status, 0, `facet-gen ${args.join(' ')} --out ${out} exited ${r.status}:\n${r.stderr}`);
    return out;
  };
  try {
    // Narrow keyed orders table.
    const o1 = run('orders-42-a', ['orders', '--seed', '42']);
    const lines = readFileSync(join(o1, 'orders.csv'), 'utf8').split('\n');
    if (lines.at(-1) === '') lines.pop();
    assert.equal(lines[0].replace(/\r$/, ''), HEADER, 'orders.csv header');
    assert.equal(lines.length - 1, ORDERS, 'number of data rows in orders.csv');
    const ids = new Set();
    for (let i = 1; i < lines.length; i++) {
      const f = lines[i].replace(/\r$/, '').split(',');
      if (f.length !== 6) assert.fail(`orders line ${i} has ${f.length} fields: ${lines[i]}`);
      if (!/^-?\d+$/.test(f[0])) assert.fail(`orders line ${i}: order_id ${JSON.stringify(f[0])} is not an integer`);
      if (ids.has(f[0])) assert.fail(`orders line ${i}: duplicate order_id ${f[0]}`);
      ids.add(f[0]);
    }
    assert.equal(ids.size, ORDERS, 'distinct order_id values');

    // Same seed -> byte-identical files, for every dataset.
    const o2 = run('orders-42-b', ['orders', '--seed', '42']);
    assert.equal(sha256(join(o2, 'orders.csv')), sha256(join(o1, 'orders.csv')), 'orders.csv differs between two --seed 42 runs');

    const v1 = run('vectors-42-a', ['vectors', '--seed', '42']);
    const v2 = run('vectors-42-b', ['vectors', '--seed', '42']);
    for (const f of ['vectors.f32', 'vectors_meta.csv']) {
      assert.equal(sha256(join(v2, f)), sha256(join(v1, f)), `${f} differs between two --seed 42 runs`);
    }
    rmSync(v1, { recursive: true, force: true });
    rmSync(v2, { recursive: true, force: true });

    const e1 = run('events-42-a', ['events', '--rows', '100000', '--seed', '42']);
    const e2 = run('events-42-b', ['events', '--rows', '100000', '--seed', '42']);
    assert.equal(sha256(join(e2, 'events.csv')), sha256(join(e1, 'events.csv')), 'events.csv differs between two --seed 42 runs');

    // A different seed -> different data.
    const o3 = run('orders-43', ['orders', '--seed', '43']);
    assert.notEqual(sha256(join(o3, 'orders.csv')), sha256(join(o1, 'orders.csv')), 'orders.csv is the same for --seed 42 and --seed 43');
  } finally {
    rmSync(base, { recursive: true, force: true });
  }
});
