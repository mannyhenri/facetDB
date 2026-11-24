// Shared helpers for the progress-tracking acceptance tests. AC-2 and AC-3 never touch the real
// repo's gate: each builds a throwaway git repo with a copy of scripts/record_gate.sh, a fixture
// PROGRESS.md (the brief's format), a stub ./run_gates.sh that logs its arguments, and backdated
// gate commits. (The real run_gates.sh runs these files for phase 0, so calling it would recurse.)
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import {
  chmodSync, copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
export const SCRIPT = join(ROOT, 'scripts', 'record_gate.sh');

const NAMES = [
  'Setup', 'Log', 'Delta and snapshots', 'Layout contract and row layout', 'Column layout',
  'Vector layout', 'Planner', 'Investment loop', 'Benchmarks', 'Report and definition of done',
];

/** A PROGRESS.md in the brief's format; `budgets` maps phase -> minutes (default '—'). */
export function progressFixture(budgets = {}) {
  const rows = NAMES.map((name, n) => {
    const gate = n <= 7 ? `\`./run_gates.sh ${n}\`` : 'none';
    const budget = n in budgets ? String(budgets[n]) : '—';
    const cut = n === 4 ? 'run-length and bit-packing' : '—';
    return `| ${n} | ${name} | ${gate} | ${budget} | ${cut} |`;
  });
  return [
    '# Facet progress', '',
    '## Schedule', '',
    '| Phase | Name | Gate | Budget (min) | Cut if over budget (section 7) |',
    '|---|---|---|---|---|',
    ...rows, '',
    'Rebuilt for the test fixture.', '',
    '## Decisions', '',
    '(none)', '',
    '## Gates', '',
  ].join('\n');
}

function git(dir, args, env = {}) {
  const r = spawnSync('git', args, { cwd: dir, encoding: 'utf8', env: { ...process.env, ...env } });
  assert.equal(r.status, 0, `git ${args.join(' ')} failed:\n${r.stderr}`);
  return r.stdout.trim();
}

/**
 * A temp git repo holding scripts/record_gate.sh, PROGRESS.md and a stub ./run_gates.sh that
 * appends its arguments to gate-calls.log, prints `gateOutput` and exits `gateStatus`.
 * Commits: `init` (5 h ago), then `<previousGate>` (`minutesAgo` ago).
 */
export function fixtureRepo({ budgets = {}, previousGate, minutesAgo, gateStatus = 0, gateOutput }) {
  assert.ok(existsSync(SCRIPT), 'scripts/record_gate.sh does not exist');
  const parent = mkdtempSync(join(tmpdir(), 'progress-tracking-'));
  const dir = join(parent, 'repo');
  mkdirSync(join(dir, 'scripts'), { recursive: true });
  copyFileSync(SCRIPT, join(dir, 'scripts', 'record_gate.sh'));
  chmodSync(join(dir, 'scripts', 'record_gate.sh'), 0o755);
  writeFileSync(join(dir, 'PROGRESS.md'), progressFixture(budgets));
  writeFileSync(join(dir, 'run_gates.sh'),
    `#!/bin/sh\necho "$*" >> "$(dirname "$0")/gate-calls.log"\n${gateOutput.map((l) => `echo '${l}'`).join('\n')}\nexit ${gateStatus}\n`);
  chmodSync(join(dir, 'run_gates.sh'), 0o755);
  writeFileSync(join(dir, '.gitignore'), 'gate-calls.log\n');
  writeFileSync(join(dir, 'notes.txt'), 'original\n');
  git(dir, ['init', '-q']);
  git(dir, ['config', 'user.name', 'Fixture']);
  git(dir, ['config', 'user.email', 'fixture@example.invalid']);
  git(dir, ['config', 'commit.gpgsign', 'false']);
  const now = Math.floor(Date.now() / 1000);
  const at = (secs) => ({ GIT_AUTHOR_DATE: `@${secs} +0000`, GIT_COMMITTER_DATE: `@${secs} +0000` });
  git(dir, ['add', '-A']);
  git(dir, ['commit', '-q', '-m', 'init'], at(now - 5 * 3600));
  git(dir, ['commit', '-q', '--allow-empty', '-m', previousGate], at(now - minutesAgo * 60));
  return { dir, cleanup: () => rmSync(parent, { recursive: true, force: true }) };
}

/** Runs scripts/record_gate.sh in the fixture; stdout+stderr merged. */
export function record(dir, args) {
  const r = spawnSync('scripts/record_gate.sh', args, { cwd: dir, encoding: 'utf8', timeout: 60_000 });
  assert.equal(r.error, undefined, `could not run scripts/record_gate.sh: ${r.error}`);
  return { status: r.status, out: `${r.stdout}${r.stderr}` };
}

export const read = (dir) => readFileSync(join(dir, 'PROGRESS.md'), 'utf8');
export const head = (dir) => git(dir, ['rev-parse', 'HEAD']);
export const subject = (dir) => git(dir, ['log', '-1', '--format=%s']);
export const changedInHead = (dir) => git(dir, ['show', '--name-only', '--format=', 'HEAD']).split('\n').filter(Boolean);
export const gateCalls = (dir) => {
  const f = join(dir, 'gate-calls.log');
  return existsSync(f) ? readFileSync(f, 'utf8').trim().split('\n').filter(Boolean) : [];
};
export { git };

/** Lines of the `## <name>` section (up to the next `## ` heading). */
export function section(text, name) {
  const lines = text.split('\n');
  const start = lines.findIndex((l) => l.trim() === `## ${name}`);
  assert.ok(start >= 0, `PROGRESS.md has no "## ${name}" section`);
  const rest = lines.slice(start + 1);
  const end = rest.findIndex((l) => /^## /.test(l));
  return end < 0 ? rest : rest.slice(0, end);
}

/** The `### Phase N` entry under `## Gates` (up to the next `###`/`##` heading), as text. */
export function gateEntry(text, n) {
  const gates = section(text, 'Gates');
  const start = gates.findIndex((l) => l.trim() === `### Phase ${n}`);
  assert.ok(start >= 0, `no "### Phase ${n}" entry under "## Gates":\n${gates.join('\n')}`);
  const rest = gates.slice(start + 1);
  const end = rest.findIndex((l) => /^#{2,3} /.test(l));
  return (end < 0 ? rest : rest.slice(0, end)).join('\n');
}

/** Minutes on the entry's time-spent line. */
export function minutesSpent(entry) {
  const line = entry.split('\n').find((l) => /time spent/i.test(l));
  assert.ok(line, `entry has no time-spent line:\n${entry}`);
  const m = /(\d+)\s*min/i.exec(line);
  assert.ok(m, `time-spent line has no "<n> min": ${line}`);
  return Number(m[1]);
}
