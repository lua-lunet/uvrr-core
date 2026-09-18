# Lab book — 2026-09-18 — experimental campaign run sheet

Dated evidence entry. This page seeds the campaign's run records. Every
figure in the paper's experimental section maps to one completed row of the
table below; no figure enters the paper with its row blank. Placeholders are
`X.XX` / `XX.X` (latencies, per the rig's scale) and `XXX` (counts); a
quoted number without a completed row is a fabrication.

## Rig identity (record once per environment change)

| field | value |
|---|---|
| rig | loopback / local VM / cloud (region, zone, instance class) |
| kernel + runtime versions | `XXX` |
| build | commit SHA of vrr-core, lunet-locks, harness; feature flags |
| clock discipline | monotonic tick source; wall-clock regression clamp on/off |
| disk class | device, mount options, `fsync` behaviour as measured |

## Knobs (record once per configuration)

| knob | value |
|---|---|
| heartbeat | `X.XX` ms |
| election | `XXX` ms |
| phi window / floor | `XXX` / `X.XX` ms |
| lease cadence | `XXX` ms |
| iterations | `XXX` (cold counts: soak between iterations) |

## Run rows (one per figure)

| figure | run id | rig | measured p50 | p90 | p99 | max | bound (§9) | bound pass? | anomalies in window | quotable number |
|---|---|---|---|---|---|---|---|---|---|---|
| E1 rejoin p50 | `XXX` | local VM | `X.XX` ms | `X.XX` ms | `XX.X` ms | `XX.X` ms | 2 RTT + floor = `X.XX` ms | y/n | `none / GC / reconfig / scheduler` | `X.XX` ms |
| E2 flush delta | `XXX` | local VM | `X.XX` ms | `X.XX` ms | `XX.X` ms | `XX.X` ms | device fsync = `X.XX` ms | y/n | | `X.XX` ms |
| E5 failover total (ours) | `XXX` | local VM | `X.XX` ms | | | | floor + 2 RTT = `X.XX` ms | y/n | | `X.XX` ms |
| E5 failover total (flush arm) | `XXX` | local VM | `XX.X` ms | | | | + 2 flushes | y/n | | `XX.X` ms |
| E5 flush in RTT units | `XXX` | local VM | `X.X` RTTs | | | | ≥ 1 RTT | y/n | | `X.X` RTTs |
| E6 phi median vs RTT median | `XXX` | local VM | `X.XX` / `X.XX` ms | | | | phi ≈ RTT | y/n | | |
| E6 outlier attribution | `XXX` | local VM | GC `XX.X` % / sched `XX.X` % / unexplained `XX.X` % | | | | shares sum to 100 | y/n | | |
| E1/E2 cloud legs | `XXX` | cloud | `XX.X` ms | `XX.X` ms | `XXX` ms | | cloud 2 RTT + floor | y/n | | `XX.X` ms |
| E3 Maelstrom | `XXX` runs × 3 and 5 nodes | local | linearizability pass y/n per run | | | | all pass | y/n | | |

## Raw artifacts

| run id | capture path | sha256 |
|---|---|---|
| `XXX` | `.tmp/runs/XXX/` | `XXX` |

## Notes for the day

(What broke, what was fixed, what the phi evidence actually showed, and any
decision taken — floor changes, estimator changes, harness fixes — with the
reason recorded before the next run starts.)
