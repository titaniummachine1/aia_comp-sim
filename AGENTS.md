# Agent Guide — AIComp Soccer Sim

This crate is an **offline game copy** for AI development. Self-test with headless matches and `cargo test` instead of asking users to run Unity.

## Build & Run

```bat
cargo run                              # Viewer (dev, fast incremental)
cargo run --release                    # Viewer (release)
cargo run --release --bin soccer_headless -- --secs 30 --home chase --away chase
cargo test --lib                       # Unit / parity tests
```

**Fast edit → compile loop:** `cargo run` (dev) rebuilds only this crate. Bevy stays cached in `target/debug`. Do not switch features or profiles between small edits — that triggers a full Bevy rebuild.

**If the build is stale:**
1. `scripts\rebuild_crate.bat` — this crate only
2. `scripts\rebuild_deep.bat` — full rebuild (last resort)

## Key Rules

- **Physics constants live in two places:** `src/params.rs` (Rust defaults) and `bevy_sim_params_v05.json` (runtime override). The JSON **overrides** Rust at load time. Edit both or only the JSON for runtime changes.
- **Measured constants are locked.** `params::measured_constants_tests` asserts values from real-game recordings. If a test fails, re-measure — do not change the expected value.
- **Do not compare Unity to a sim that started from a different state.** Freeze the Unity snapshot, inject it, then measure RMSE.
- **NN train harness** is gated behind `--features nn_train` to keep default compiles fast.

## Truth Files

| File | Purpose |
|------|---------|
| `docs/AIA_UPSTREAM_QUIRKS.md` | Locked Unity measurements |
| `docs/API_GAPS.md` | Unimplemented API getters |
| `bevy_sim_params_v05.json` | Runtime physics constants |
| `data/reference/` | API dumps and measured data (read-only) |

## Brain Types

`chase` | `idle` | `test1` | `test2` | `perfect` | `aia` | `graph:<path>`

Use `soccer_headless` for batch testing. Parse stdout JSON for results.

## Script / Data Organization (autonomous rule — no asking)

- `scripts/*.py` = live pipeline only. Prefix rule: `run_*` (execute), `score_*`/`compare_*`/`analyze_*` (read-only scoring), `gate_*`/`promote_*` (promotion). One-off probes go to `scripts/archive/` (already holds the 6 `_*.py` temps moved 2026-09-11). Never leave `_tmp_*` or `__pycache__/` at top level — delete pycache, archive temps on sight.
- Tennis results canonical: game side `C:\gitProjects\AIA_tennis\modhost\game_tournament_results.jsonl` (merged; shards `*_w1..w4` + `*fix2*` merge with dedupe home+away+seed, keep stateful rows). Sim side `data/tennis/sim_pairs_results.jsonl` (append mode — resume-safe; truncate only via python, never PowerShell `Set-Content` = BOM). Timeplots: game `modhost/captures/timeplots-<date>/` + `modhost/game_timeplots/<bot>/`, parsed only via `modhost/parse_timeplots.py` (locale-comma rule). Fixtures: `tests/fixtures/tennis-v014/` (committed, CI replays).
- Validity 2026-09-11: `run_sim_tournament_pairs.py` replays `--points 8` vs merged game file (does NOT yet auto-include fix2 shards — merge first); `score_parity.py` aggregates `agree-seq`/`agree-leader` vs `disagree` (fixed: `startswith("agree")` check); `parse_timeplots.py` + `compare_timeplots.py` valid.
- Compiler probe (graphc verification, game-vs-sim): `scripts/run_compiler_probe.py` regenerates `data/compiler_probes/compiler_probe.txt` (+`.desc.json`) and `live_position_fib.txt` via the real graphc pipeline; `tests/compiler_probe.rs` pins goldens + costs (probe 94 nodes/O0 91/O1 80; live 94/94/85) + 40-tick LIVE self-consistency; `scripts/compare_compiler_probe.py <game-export.json>` scores a game TimePlot (static exact, order canaries, LIVE relations). Cost model is lexicographic: transitions first, size breaks ties.

## Scope / Legal Boundary (binding)

Goal = **100% outcome parity** (any 2 AIs → same result as game), NOT a game clone. Rust accelerated sim + accelerated graph VM (run millions of games headless for AI training) is required — VM must exist to run AI in sim. Never copy: in-game editor, 3D graphics/assets, game binaries (never in git). Viewer stays 2D. This is a training toolset / hub for building ridiculously strong AI, not a game copy.

## Tennis Parity Model (2026-09-11, game-proven from native timeplots)

- ServeAimHint latch: strike uses the FIRST-TOSS-TICK request, not
  setup-entry (setup wire is best-point junk; serving flag goes live with
  the Toss and the aim wire switches to the serve branch then). In-box
  honored, else box fallback; live strike-tick wire already flipped back
  to rally default. Proven 2026-09-11 from titanium54 native timeplots:
  8/8 serves land (7.0077, 0.0000), sub-mm repeatable per side, while the
  live wire reads (1.3x..13, ±5). Sim (separate-aim + toss latch) lands
  (7.18, ±0.02) — 18 cm long, open: strike-state decomposition (sim
  q=1.0 vs game 0.79, serve-shot-2 vs rally-shot-6/1, contact 2.63 vs
  3.53 m) and/or serve-regime solver error. Legacy (move wire) unchanged:
  stance rejected at both timings, same fallback.
- `scripts/gate_landings.py` now takes a file arg (default = Sep-10
  capture) and skips post-teleport settle false positives + stops landing
  scans at resets; `scripts/channel_diff.py` takes a game-capture path arg.
- Rally gate = pass-through: game landings equal the aim request within ~3 cm
  (mode-1 strikes). `TennisAutoAim` passes through; the aim dies in
  `TennisAutoMove` lowering (V32 dropped) — fixed via `TennisCommand.aim`
  (`AIA_AIM_MODEL=separate`).
- `TennisAutoSwing` = approach-hold (54% of ticks, 0.6-1.2 s, charges
  0.17-1.0); sim flicker pegged charge at 0. Fixed via world hold-gate
  (`AIA_SWING_MODEL=hold`). Player speeds verified exact (13.0/8.5).
- Serve setup forces the SERVER to stance only; the receiver is never
  placed and walks free (game truth 2026-09-11: receiver holds (8.15,
  2.52), wanders to (10.48, 10.92) mid-setup — sim no longer teleports it
  to receive_stance; it keeps its end-of-last-point spot). Serve fault =
  first bounce outside the diagonal box judged AT the landing, line-grown
  box inclusive (landing on the line is in, default tennis rules).
  Implementation already matched; placement change shifts game-2+ receiver
  starts, so the 78-row sim baseline needs a re-score (bat/171012 game 2
  flips vs the stale recorded row; converging pairs unaffected).
- Sweep harness: `scripts/sweep_rally.py` (env variants, `--out` verdicts).
  Leader parity 2026-09-11: legacy 25/78, latch-off 30/78, side-gate 29/78,
  swing-hold 24/78. Forensics: `scripts/return_forensics.py`,
  `scripts/gate_landings.py`, `scripts/channel_diff.py`.
- Known structural gaps: sim holds 100% of service games (stock-vs-stock
  proves it is world-structural, not VM); first-server mapping unproven
  (10/28 on hold-rows); raw `Ball Incoming` rule needs live getter capture.
