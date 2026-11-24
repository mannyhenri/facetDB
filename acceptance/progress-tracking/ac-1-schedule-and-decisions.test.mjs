// AC-1: PROGRESS.md at the repo root has a '## Schedule' table with phases exactly 0..9 in order,
// Gate `./run_gates.sh N` for 0-7 and 'none' for 8-9, Phase 4's cut 'run-length and bit-packing';
// a '## Decisions' section that is exactly '(none)' or only `- Phase N: … (source: <existing path>)`
// bullets; and a '## Gates' section.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { ROOT, section } from './progress-harness.mjs';

const cells = (row) => row.trim().replace(/^\||\|$/g, '').split('|').map((c) => c.trim());

test('AC-1: PROGRESS.md has the 0-9 schedule, a decisions list and a gates section', () => {
  const text = readFileSync(join(ROOT, 'PROGRESS.md'), 'utf8');

  const table = section(text, 'Schedule').filter((l) => l.trim().startsWith('|'));
  assert.ok(table.length >= 2, 'the Schedule section has no markdown table');
  const header = cells(table[0]).map((c) => c.toLowerCase());
  assert.equal(header[0], 'phase', `first column must be Phase, got ${header[0]}`);
  const gateCol = header.indexOf('gate');
  assert.ok(gateCol > 0, `no Gate column in ${table[0]}`);
  const cutCol = header.findIndex((h) => h.startsWith('cut'));
  assert.ok(cutCol > 0, `no "Cut …" column in ${table[0]}`);
  assert.match(table[1], /^\s*\|[\s:|-]+\|\s*$/, 'second table line must be the separator row');
  const rows = table.slice(2).map(cells);

  assert.deepEqual(rows.map((r) => r[0]), ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'],
    'schedule rows must be phases 0..9, each once, in order');
  for (const r of rows) {
    const n = Number(r[0]);
    const gate = r[gateCol].replace(/`/g, '').trim();
    assert.equal(gate, n <= 7 ? `./run_gates.sh ${n}` : 'none', `phase ${n} gate`);
    assert.equal(r.length, header.length, `phase ${n} row has ${r.length} cells, header has ${header.length}`);
  }
  assert.match(rows[4][cutCol], /run-length and bit-packing/, "phase 4's cut");

  const decisions = section(text, 'Decisions').map((l) => l.trim()).filter(Boolean);
  assert.ok(decisions.length > 0, 'the Decisions section is empty (expected "(none)" or bullets)');
  if (!(decisions.length === 1 && decisions[0] === '(none)')) {
    for (const d of decisions) {
      const m = /^- Phase ([0-9]): .+ \(source: ([^()]+)\)$/.exec(d);
      assert.ok(m, `decision line not of the form "- Phase N: … (source: <path>)": ${d}`);
      for (const p of m[2].split(',').map((s) => s.trim().replace(/`/g, '').replace(/#.*$/, ''))) {
        assert.ok(existsSync(join(ROOT, p)), `decision source does not exist: ${p} (in "${d}")`);
      }
    }
  }

  section(text, 'Gates');
});
