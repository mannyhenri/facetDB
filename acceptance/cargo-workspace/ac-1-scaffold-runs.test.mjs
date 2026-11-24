// AC-1: Scaffold runs successfully.
// Contract for the "Cargo workspace with the eight crates" goal: the workspace
// has the exact shape from the scope, and `cargo test` / `cargo bench --no-run`
// succeed across all eight crates.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');

const CRATES = ['core', 'log', 'delta', 'layouts', 'planner', 'engine', 'check', 'bench'];
const pkg = (dir) => `facet-${dir}`;

// Internal (facet-*) normal dependencies, along the architecture's data flow.
const INTERNAL_DEPS = {
  core: [],
  log: ['facet-core'],
  delta: ['facet-core', 'facet-log'],
  layouts: ['facet-core', 'facet-delta'],
  planner: ['facet-core', 'facet-layouts'],
  engine: ['facet-core', 'facet-log', 'facet-delta', 'facet-layouts', 'facet-planner'],
  check: ['facet-core', 'facet-engine', 'facet-log'],
  bench: ['facet-core', 'facet-engine'],
};
const PROPTEST_CRATES = ['core', 'delta', 'check'];
const LAYOUT_MODULES = ['row', 'column', 'vector', 'contract'];

const read = (rel) => readFileSync(join(ROOT, rel), 'utf8');
const run = (cmd, args) =>
  spawnSync(cmd, args, { cwd: ROOT, encoding: 'utf8', maxBuffer: 256 * 1024 * 1024 });

// Lines of a TOML table, from its [header] up to the next [header].
function tomlSection(text, header) {
  const lines = text.split(/\r?\n/);
  const start = lines.findIndex((l) => l.trim() === `[${header}]`);
  if (start === -1) return null;
  const out = [];
  for (const l of lines.slice(start + 1)) {
    if (/^\s*\[/.test(l)) break;
    out.push(l);
  }
  return out.join('\n');
}

function metadata() {
  const r = run('cargo', ['metadata', '--format-version', '1', '--no-deps']);
  assert.equal(r.status, 0, `cargo metadata failed:\n${r.stderr}`);
  return JSON.parse(r.stdout);
}

test('AC-1: root Cargo.toml is a virtual workspace with the eight members', () => {
  assert.ok(existsSync(join(ROOT, 'Cargo.toml')), 'Cargo.toml missing at repo root');
  const toml = read('Cargo.toml');
  assert.equal(tomlSection(toml, 'package'), null, 'root must be a virtual workspace (no [package])');

  const ws = tomlSection(toml, 'workspace');
  assert.ok(ws !== null, '[workspace] table missing');
  assert.match(ws, /^\s*resolver\s*=\s*"2"\s*$/m, 'resolver = "2" missing');
  const members = [...(ws.match(/members\s*=\s*\[([\s\S]*?)\]/)?.[1] ?? '').matchAll(/"([^"]+)"/g)]
    .map((m) => m[1].replace(/\/$/, ''))
    .sort();
  assert.deepEqual(members, CRATES.map((c) => `crates/${c}`).sort());

  const wsPkg = tomlSection(toml, 'workspace.package');
  assert.ok(wsPkg !== null, '[workspace.package] missing');
  assert.match(wsPkg, /^\s*edition\s*=\s*"2021"\s*$/m);
  assert.match(wsPkg, /^\s*version\s*=\s*"0\.1\.0"\s*$/m);

  const wsDeps = tomlSection(toml, 'workspace.dependencies');
  assert.ok(wsDeps !== null, '[workspace.dependencies] missing');
  assert.match(wsDeps, /^\s*criterion\s*=/m, 'criterion not in [workspace.dependencies]');
  assert.match(wsDeps, /^\s*proptest\s*=/m, 'proptest not in [workspace.dependencies]');
});

test('AC-1: rust-toolchain.toml pins the stable channel', () => {
  assert.ok(existsSync(join(ROOT, 'rust-toolchain.toml')), 'rust-toolchain.toml missing');
  assert.match(read('rust-toolchain.toml'), /^\s*channel\s*=\s*"stable"\s*$/m);
});

test('AC-1: eight facet-* packages with inherited edition/version and the right dependency graph', () => {
  const meta = metadata();
  const byName = new Map(meta.packages.map((p) => [p.name, p]));
  const memberNames = meta.packages
    .filter((p) => meta.workspace_members.includes(p.id))
    .map((p) => p.name)
    .sort();
  assert.deepEqual(memberNames, CRATES.map(pkg).sort());

  for (const dir of CRATES) {
    const p = byName.get(pkg(dir));
    assert.equal(resolve(p.manifest_path), join(ROOT, 'crates', dir, 'Cargo.toml'), `${p.name} location`);
    assert.equal(p.version, '0.1.0', `${p.name} version`);
    assert.equal(p.edition, '2021', `${p.name} edition`);

    const manifest = read(`crates/${dir}/Cargo.toml`);
    assert.match(manifest, /^\s*(version\.workspace\s*=\s*true|version\s*=\s*\{\s*workspace\s*=\s*true\s*\})/m,
      `${p.name} must inherit version from the workspace`);
    assert.match(manifest, /^\s*(edition\.workspace\s*=\s*true|edition\s*=\s*\{\s*workspace\s*=\s*true\s*\})/m,
      `${p.name} must inherit edition from the workspace`);

    assert.ok(p.targets.some((t) => t.kind.includes('lib')), `${p.name} must be a library crate`);

    const internal = p.dependencies
      .filter((d) => d.kind === null && d.name.startsWith('facet-'))
      .map((d) => d.name)
      .sort();
    assert.deepEqual(internal, [...INTERNAL_DEPS[dir]].sort(), `${p.name} internal dependencies`);

    const hasDevDep = (n) => p.dependencies.some((d) => d.kind === 'dev' && d.name === n);
    if (PROPTEST_CRATES.includes(dir)) {
      assert.ok(hasDevDep('proptest'), `${p.name} needs proptest as a dev-dependency`);
      assert.match(manifest, /proptest(\.workspace\s*=\s*true|\s*=\s*\{[^}]*workspace\s*=\s*true)/,
        `${p.name} proptest must come from [workspace.dependencies]`);
    }
  }

  const bench = byName.get('facet-bench');
  assert.ok(bench.dependencies.some((d) => d.kind === 'dev' && d.name === 'criterion'),
    'facet-bench needs criterion as a dev-dependency');
  assert.match(read('crates/bench/Cargo.toml'), /criterion(\.workspace\s*=\s*true|\s*=\s*\{[^}]*workspace\s*=\s*true)/,
    'facet-bench criterion must come from [workspace.dependencies]');
  const benchTargets = bench.targets.filter((t) => t.kind.includes('bench'));
  assert.equal(benchTargets.length, 1, 'facet-bench must have exactly one [[bench]] target');
  assert.match(read('crates/bench/Cargo.toml'), /\[\[bench\]\][\s\S]*?harness\s*=\s*false/,
    '[[bench]] target must set harness = false');
});

test('AC-1: crate sources — doc comments, layouts modules, unit tests', () => {
  for (const dir of CRATES) {
    const lib = `crates/${dir}/src/lib.rs`;
    assert.ok(existsSync(join(ROOT, lib)), `${lib} missing`);
    const src = read(lib);
    if (dir !== 'layouts') {
      assert.match(src, /^\s*\/\/!\s*\S/m, `${lib} needs a crate-level //! doc comment`);
      assert.match(src, /#\[test\]/, `${lib} needs a unit test`);
    }
  }
  const core = read('crates/core/src/lib.rs');
  const coreDoc = core.split(/\r?\n/).filter((l) => /^\s*\/\/!/.test(l)).join('\n');
  for (const word of ['LSN', 'ChangeBatch', 'Fragment']) {
    assert.ok(coreDoc.includes(word), `facet-core doc comment must name ${word}`);
  }

  const layouts = read('crates/layouts/src/lib.rs');
  for (const m of LAYOUT_MODULES) {
    assert.ok(existsSync(join(ROOT, `crates/layouts/src/${m}.rs`)), `crates/layouts/src/${m}.rs missing`);
    assert.match(layouts, new RegExp(`^\\s*(pub(\\([^)]*\\))?\\s+)?mod\\s+${m}\\s*;`, 'm'),
      `crates/layouts/src/lib.rs must declare mod ${m};`);
  }
});

// PROGRESS.md left this check when the progress-tracking feature filled it in (amendment approved by
// the person, 2026-11-23). REPORT.md stays a placeholder until the benchmark report feature.
test('AC-1: REPORT.md is a title-only placeholder', () => {
  for (const f of ['REPORT.md']) {
    assert.ok(existsSync(join(ROOT, f)), `${f} missing`);
    const lines = read(f).trim().split(/\r?\n/);
    assert.equal(lines.length, 1, `${f} must contain a title line only`);
    assert.match(lines[0], /^#\s+\S/, `${f} title must be a markdown heading`);
  }
});

test('AC-1: build output is git-ignored and the workspace is committed', () => {
  assert.match(read('.gitignore'), /^\/?target\/?\s*$/m, '.gitignore must ignore /target');

  const tracked = run('git', ['ls-files']).stdout.split('\n');
  const required = [
    'Cargo.toml', 'rust-toolchain.toml', 'PROGRESS.md', 'REPORT.md',
    ...CRATES.flatMap((c) => [`crates/${c}/Cargo.toml`, `crates/${c}/src/lib.rs`]),
    ...LAYOUT_MODULES.map((m) => `crates/layouts/src/${m}.rs`),
  ];
  for (const f of required) assert.ok(tracked.includes(f), `${f} is not committed`);
  assert.ok(!tracked.some((f) => f.startsWith('target/')), 'target/ must not be committed');

  const dirty = run('git', ['status', '--porcelain', '--', 'Cargo.toml', 'rust-toolchain.toml', 'crates']).stdout;
  assert.equal(dirty.trim(), '', `workspace has uncommitted changes:\n${dirty}`);
});

test('AC-1: cargo test runs every crate and passes', () => {
  // Merge stderr ("Running …") and stdout ("test result: …") so their order is kept.
  const r = run('sh', ['-c', 'cargo test --workspace 2>&1']);
  const out = r.stdout;
  assert.equal(r.status, 0, `cargo test --workspace failed:\n${out}`);

  // Each "Running unittests src/lib.rs (…/facet_<crate>-…)" section holds its own "test result:" line.
  const sections = out.split(/^\s*Running /m);
  for (const dir of CRATES) {
    const section = sections.find((s) =>
      new RegExp(`^unittests src/lib\\.rs \\([^)]*facet_${dir}-[0-9a-f]+\\)`).test(s));
    assert.ok(section, `no unit-test run for facet-${dir}`);
    const m = section.match(/test result: ok\. (\d+) passed; 0 failed/);
    assert.ok(m, `facet-${dir} unit tests did not pass`);
    if (dir !== 'layouts') assert.ok(Number(m[1]) >= 1, `facet-${dir} must run at least one unit test`);
  }
});

test('AC-1: the placeholder criterion bench compiles', () => {
  const r = run('cargo', ['bench', '-p', 'facet-bench', '--no-run']);
  assert.equal(r.status, 0, `cargo bench --no-run failed:\n${r.stdout}\n${r.stderr}`);
});
