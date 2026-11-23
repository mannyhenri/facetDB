# Facet progress

## Schedule

| Phase | Name | Gate | Budget (min) | Cut if over budget (section 7) |
|---|---|---|---|---|
| 0 | Setup | `./run_gates.sh 0` | — | — |
| 1 | Log | `./run_gates.sh 1` | — | — |
| 2 | Delta and snapshots | `./run_gates.sh 2` | — | — |
| 3 | Layout contract and row layout | `./run_gates.sh 3` | — | — |
| 4 | Column layout | `./run_gates.sh 4` | — | run-length and bit-packing |
| 5 | Vector layout | `./run_gates.sh 5` | — | — |
| 6 | Planner | `./run_gates.sh 6` | — | — |
| 7 | Investment loop | `./run_gates.sh 7` | — | — |
| 8 | Benchmarks | none | — | — |
| 9 | Report and definition of done | none | — | — |

Rebuilt from the project's feature list because the build spec is not in the repo: fill in the budgets and section 7 cuts from the spec.

## Decisions

(none)

## Gates
