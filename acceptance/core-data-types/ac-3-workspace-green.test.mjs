// AC-3: Every crate compiles against the new facet-core and the workspace passes all tests and
// lints (fmt, clippy -D warnings, build, test), with the shared types re-exported from the
// facet-core root and no external crate added to facet-core's normal dependencies.
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

const run = (cmd, args, opts = {}) =>
  spawnSync(cmd, args, { cwd: ROOT, encoding: 'utf8', maxBuffer: 256 * 1024 * 1024, ...opts });

test('AC-3: facet-core re-exports every shared type and keeps no external dependencies', () => {
  assert.ok(existsSync(join(CORE, 'Cargo.toml')), 'crates/core/Cargo.toml missing');

  const meta = run('cargo', ['metadata', '--format-version', '1', '--no-deps']);
  assert.equal(meta.status, 0, `cargo metadata failed:\n${meta.stderr}`);
  const core = JSON.parse(meta.stdout).packages.find((p) => p.name === 'facet-core');
  assert.ok(core, 'facet-core is not a workspace package');
  const normal = core.dependencies.filter((d) => d.kind === null).map((d) => d.name);
  assert.deepEqual(normal, [], 'facet-core must not gain normal dependencies (bitset is hand-written)');

  const dir = mkdtempSync(join(tmpdir(), 'facet-ac-3-'));
  try {
    writeFileSync(join(dir, 'Cargo.toml'), [
      '[package]',
      'name = "facet-ac-3"',
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
    writeFileSync(join(dir, 'src', 'lib.rs'), '');
    mkdirSync(join(dir, 'tests'));
    copyFileSync(join(HERE, 'ac-3-exports.rs'), join(dir, 'tests', 'ac_3.rs'));

    const r = run('cargo', ['test', '--test', 'ac_3'], {
      cwd: dir,
      env: { ...process.env, CARGO_TARGET_DIR: join(dir, 'target') },
    });
    const out = `${r.stdout}\n${r.stderr}`;
    assert.equal(r.status, 0, `facet-core exports check failed:\n${out}`);
    assert.match(r.stdout, /test result: ok\. 1 passed; 0 failed/, `exports test did not run:\n${out}`);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

for (const [name, args] of [
  ['cargo fmt --all --check', ['fmt', '--all', '--check']],
  ['cargo clippy --workspace --all-targets -- -D warnings',
    ['clippy', '--workspace', '--all-targets', '--', '-D', 'warnings']],
  ['cargo build --workspace', ['build', '--workspace']],
  ['cargo test --workspace', ['test', '--workspace']],
]) {
  test(`AC-3: ${name} exits 0`, () => {
    assert.ok(existsSync(join(ROOT, 'Cargo.toml')), 'Cargo.toml missing at repo root');
    const r = run('cargo', args);
    assert.equal(r.status, 0, `${name} failed (exit ${r.status}):\n${r.stdout}\n${r.stderr}`);
  });
}
