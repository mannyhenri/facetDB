// AC-4: Segment digests before and after an apply: new and removed segments are allowed, an edited
// one is rejected.
// Compiles ac-4-segments-unchanged.rs as an integration test of a throwaway crate that depends on
// crates/core by path, so it exercises facet_core::invariants exactly as a sibling crate would.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, '..', '..');
const CORE = join(ROOT, 'crates', 'core');

test('AC-4: new and removed segments allowed; an edited segment is rejected', () => {
  assert.ok(existsSync(join(CORE, 'Cargo.toml')), 'crates/core/Cargo.toml missing');
  const dir = mkdtempSync(join(tmpdir(), 'facet-inv-ac-4-'));
  try {
    writeFileSync(join(dir, 'Cargo.toml'), [
      '[package]',
      'name = "facet-inv-ac-4"',
      'version = "0.0.0"',
      'edition = "2021"',
      'publish = false',
      '',
      '[dependencies]',
      `facet-core = { path = ${JSON.stringify(CORE)} }`,
      '',
      '[workspace]',
      '',
    ].join('\n'));
    mkdirSync(join(dir, 'src'));
    writeFileSync(join(dir, 'src', 'lib.rs'), '\n');
    mkdirSync(join(dir, 'tests'));
    copyFileSync(join(HERE, 'ac-4-segments-unchanged.rs'), join(dir, 'tests', 'ac_4.rs'));

    const r = spawnSync('cargo', ['test', '--test', 'ac_4'], {
      cwd: dir,
      encoding: 'utf8',
      env: { ...process.env, CARGO_TARGET_DIR: join(dir, 'target') },
      maxBuffer: 256 * 1024 * 1024,
    });
    const out = `${r.stdout}\n${r.stderr}`;
    assert.equal(r.status, 0, `AC-4 fixture failed:\n${out}`);
    assert.match(r.stdout, /test result: ok\. 5 passed; 0 failed/, `AC-4 tests did not all run:\n${out}`);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
