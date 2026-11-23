#!/usr/bin/env bash
# Records a passed phase gate in PROGRESS.md and commits it. See "Recording a gate" in README.md.
#
#   scripts/record_gate.sh <phase> [--decision "<text>"]... [--cut "<text>"]
#
# Runs ./run_gates.sh <phase>. Only when it exits 0 does it append a `### Phase N` entry under
# "## Gates" (date, time spent, gate result, decisions, cut), add the decisions under
# "## Decisions", and commit PROGRESS.md alone as `Phase N: gate passed`.
# Exit 0: recorded. 2: usage error, previous gate not recorded, or an overrun without --cut.
# Otherwise: the gate's own exit status (nothing recorded). Works with macOS /bin/bash 3.2.

usage() {
  echo "usage: scripts/record_gate.sh <phase 0-9> [--decision \"<text>\"]... [--cut \"<text>\"]" >&2
  exit 2
}

[ $# -ge 1 ] || usage
case $1 in
  [0-9]) phase=$1 ;;
  *) usage ;;
esac
shift
decisions=()
cut=""
while [ $# -gt 0 ]; do
  case $1 in
    --decision | --cut)
      [ $# -ge 2 ] || { echo "record_gate.sh: $1 requires a value" >&2; usage; }
      if [ "$1" = --decision ]; then decisions+=("$2"); else cut=$2; fi
      shift 2
      ;;
    *)
      echo "record_gate.sh: unknown argument '$1'" >&2
      usage
      ;;
  esac
done

cd "$(dirname "$0")/.." || exit 2

# Time spent runs from the previous phase's gate commit (exact subject), or for phase 0 from the
# repository's first commit. A loose match would take e.g. "Phase 0: set up cargo workspace".
if [ "$phase" -eq 0 ]; then
  since=$(git log --max-parents=0 --format=%ct | sort -n | head -1)
else
  previous="Phase $((phase - 1)): gate passed"
  since=$(git log --format='%ct %s' | awk -v want="$previous" \
    '{ ct = $1; sub(/^[0-9]+ /, "") } $0 == want { print ct; exit }')
  if [ -z "$since" ]; then
    echo "record_gate.sh: no '$previous' commit: record phase $((phase - 1)) first" >&2
    exit 2
  fi
fi
minutes=$((($(date +%s) - since) / 60))

# The phase's schedule row: | Phase | Name | Gate | Budget (min) | Cut if over budget (section 7) |
schedule_cell() {
  awk -F'|' -v phase="$phase" -v col="$1" '
    { p = $2; gsub(/^[ \t]+|[ \t]+$/, "", p) }
    p == phase && NF >= 7 { c = $col; gsub(/^[ \t]+|[ \t]+$/, "", c); print c; exit }' PROGRESS.md
}
budget=$(schedule_cell 5)
case $budget in '' | *[!0-9]*) budget="" ;; esac
if [ -n "$budget" ] && [ "$minutes" -gt $((budget + 20)) ] && [ -z "$cut" ]; then
  echo "record_gate.sh: phase $phase took $minutes min, more than 20 over its $budget min budget." >&2
  echo "Apply the section 7 cut and record it: --cut \"$(schedule_cell 6)\"" >&2
  exit 2
fi

# Under a parent `node --test`, a nested `node --test <file>` exits 0 even when its tests fail.
unset NODE_TEST_CONTEXT
gate_output=$(./run_gates.sh "$phase" 2>&1)
gate_status=$?
printf '%s\n' "$gate_output"
if [ "$gate_status" -ne 0 ]; then
  echo "record_gate.sh: phase $phase gate failed (exit $gate_status); nothing recorded" >&2
  exit "$gate_status"
fi

summary=$(printf '%s\n' "$gate_output" | grep "^phase $phase: " | tail -n 1)
checks=$(printf '%s\n' "$gate_output" | grep -cE '^(PASS|FAIL) ')
counts=$(printf '%s\n' "$gate_output" | sed -n 's/^ *\(cargo test: .*\)$/\1/p' | tail -n 1)
result="$summary ($checks checks${counts:+; $counts})"

new_decisions=""
entry="### Phase $phase

- Date: $(date +%Y-%m-%d)
- Time spent: $minutes min
- Gate: $result"
for d in "${decisions[@]}"; do
  new_decisions="$new_decisions${new_decisions:+
}- Phase $phase: $d"
  entry="$entry
- Decision: $d"
done
[ -n "$cut" ] && entry="$entry
- cut: $cut"

# Rewrite PROGRESS.md section by section: each "## " section is re-emitted as heading, blank line,
# body, blank line; new decisions replace "(none)", the entry goes at the end of "## Gates". The
# texts come through ENVIRON, so awk applies no escape processing to them.
tmp=$(mktemp "${TMPDIR:-/tmp}/progress.XXXXXX") || exit 1
DECISIONS=$new_decisions ENTRY=$entry awk '
  function close_section(   i, first) {
    if (section == "") { for (i = 1; i <= n; i++) print body[i]; n = 0; return }
    first = 1
    while (first <= n && body[first] == "") first++
    while (n >= first && body[n] == "") n--
    print ""
    for (i = first; i <= n; i++) print body[i]
    if (section == "## Decisions") {
      if (ENVIRON["DECISIONS"] != "") print ENVIRON["DECISIONS"]
      else if (n < first) print "(none)"
    }
    if (section == "## Gates") { if (n >= first) print ""; print ENVIRON["ENTRY"] }
    n = 0
  }
  /^## / { close_section(); if (section != "") print ""; section = $0; print; next }
  section == "## Decisions" && $0 == "(none)" { next }
  { body[++n] = $0 }
  END { close_section() }
' PROGRESS.md >"$tmp" || { rm -f "$tmp"; exit 1; }
mv "$tmp" PROGRESS.md

git commit -q -m "Phase $phase: gate passed" -- PROGRESS.md || exit 1
echo "record_gate.sh: recorded phase $phase ($minutes min) and committed 'Phase $phase: gate passed'"
