// AC-3: Two layouts' rows at the same snapshot are accepted in any order, and every differing primary
// key is reported.
// Compiles ac-3-same-rows.rs as an integration test of a throwaway crate that depends on
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

test('AC-3: same rows in any order pass; every differing key is reported', () => {
  assert.ok(existsSync(join(CORE, 'Cargo.toml')), 'crates/core/Cargo.toml missing');
  const dir = mkdtempSync(join(tmpdir(), 'facet-inv-ac-3-'));
  try {
    writeFileSync(join(dir, 'Cargo.toml'), [
      '[package]',
      'name = "facet-inv-ac-3"',
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
    copyFileSync(join(HERE, 'ac-3-same-rows.rs'), join(dir, 'tests', 'ac_3.rs'));

    const r = spawnSync('cargo', ['test', '--test', 'ac_3'], {
      cwd: dir,
      encoding: 'utf8',
      env: { ...process.env, CARGO_TARGET_DIR: join(dir, 'target') },
      maxBuffer: 256 * 1024 * 1024,
    });
    const out = `${r.stdout}\n${r.stderr}`;
    assert.equal(r.status, 0, `AC-3 fixture failed:\n${out}`);
    assert.match(r.stdout, /test result: ok\. 4 passed; 0 failed/, `AC-3 tests did not all run:\n${out}`);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
