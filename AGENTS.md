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
| `docs/RE_PLAYBOOK.md` | **Generic RE method** — start here for any new game/version. **§8 = re-target a VERSION** (host dir → same-flags build → diagnostic ladder → the two failure classes: byte-pinned instruction hooks & vanished named fields → trace-diff technique → changelog-first drift triage). |
| `docs/HANDOFF.md` | Session state; **§0 = latest session READ FIRST**, §13–§18 = this session, §18 = v0.15f re-target case study. |
| `docs/GAME_VERSIONS.md` | Version scope + re-target steps; v0.14 pinned physics, v0.15f capture verified. |
| `docs/AIA_UPSTREAM_QUIRKS.md` | Locked Unity measurements |
| `docs/API_GAPS.md` | Unimplemented API getters |
| `bevy_sim_params_v05.json` | Runtime physics constants |
| `data/reference/` | API dumps and measured data (read-only) |
| `C:\gitProjects\implementation_plan.md` | **Plan of record** (phases A–F) — outside the repo, read it before starting work. |

## Brain Types

`chase` | `idle` | `test1` | `test2` | `perfect` | `aia` | `graph:<path>`

Use `soccer_headless` for batch testing. Parse stdout JSON for results.

## Script / Data Organization (autonomous rule — no asking)

- `scripts/*.py` = live pipeline only. Prefix rule: `run_*` (execute), `score_*`/`compare_*`/`analyze_*` (read-only scoring), `gate_*`/`promote_*` (promotion). One-off probes go to `scripts/archive/` (already holds the 6 `_*.py` temps moved 2026-09-11). Never leave `_tmp_*` or `__pycache__/` at top level — delete pycache, archive temps on sight.
- Tennis results canonical: game side `C:\gitProjects\AIA_tennis\modhost\game_tournament_results.jsonl` (merged; shards `*_w1..w4` + `*fix2*` merge with dedupe home+away+seed, keep stateful rows). Sim side `data/tennis/sim_pairs_results.jsonl` (append mode — resume-safe; truncate only via python, never PowerShell `Set-Content` = BOM). Timeplots: game `modhost/captures/timeplots-<date>/` + `modhost/game_timeplots/<bot>/`, parsed only via `modhost/parse_timeplots.py` (locale-comma rule). Fixtures: `tests/fixtures/tennis-v014/` (committed, CI replays).
- Validity 2026-09-11: `run_sim_tournament_pairs.py` replays `--points 8` vs merged game file (does NOT yet auto-include fix2 shards — merge first); `score_parity.py` aggregates `agree-seq`/`agree-leader` vs `disagree` (fixed: `startswith("agree")` check); `parse_timeplots.py` + `compare_timeplots.py` valid.
- Multi-seed **distributional** parity (2026-09-12): game side is `modhost\restart_sweep.jsonl` (one launch, `restart_match` per seed; a row is usable only when `state.restart_epoch` is present — pre-fix rows lack it and are skipped because the boot match's snapshot was misattributed; `serving_team` = THAT seed's setup server). `scripts/run_sim_seed_sweep.py` replays the same (home, away, seed) triples headless with matched `--points` + `AIA_FIRST_SERVER` from `serving_team` → `data/tennis/sim_seed_sweep.jsonl` (append, resume-safe). `scripts/score_distribution.py` is READ-ONLY: per-pairing + pooled home-win / server-hold / mean-points and the total-variation distance between the game and sim leader distributions, with exact per-seed agreement demoted to a secondary column. This is the HANDOFF §11 "valid ground truth" metric — never headline the exact % (first-server mapping + RNG draw sites are still unpinned).

- Compiler probe (graphc verification, game-vs-sim): `scripts/run_compiler_probe.py` regenerates `data/compiler_probes/compiler_probe.txt` (+`.desc.json`) and `live_position_fib.txt` via the real graphc pipeline; `tests/compiler_probe.rs` pins goldens + costs (probe 93 nodes/O0 90/O1 80; live 94/94/85) + 40-tick LIVE self-consistency; `scripts/compare_compiler_probe.py <game-export.json>` scores a game TimePlot (static exact, order canaries, LIVE relations). Cost model is lexicographic: transitions first, size breaks ties. Optimization-mode parity: `scripts/run_compiler_mode_parity.py` regenerates `data/compiler_probes/mode_parity/{o0,o1,o2}.txt`; `tests/compiler_mode_parity.rs` asserts the same bot is behaviorally identical at every mode (only debug sinks/chrome may differ) and that o0 keeps working debug TimePlots.

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
- Interpreter certification (2026-09-12): `tests/tennis_interpreter_certification.rs` drives the reference `GraphBrain` + the shipping O1 VM + a pass-free O0 VM through a live `TennisWorld` (40 ticks) and diffs Pass 1..8 commits + `TennisController` each tick (`compare_traces` now covers tennis_command — previously a tennis trace passed vacuously). The O0 arm attributes a divergence (O1-only = pass bug; both = lowering/interpreter). Report-only by default; `TENNIS_CERT_STRICT=1` fails. First run: 12 graphs, 7 clean; after the §15 `ClampFloat` reference fix, 9 clean; **after Phase A (HANDOFF §16), 12/14 clean** — `Adam` was a *false* divergence (NaN-aware comparator) and `nqvxf22` a second empty-`Operation` panic. Remaining: `bat` (aim + O0!=O1) and `Pixel_Heart` (`Round(RandomFloat)`, a world-RNG placeholder). The `titanium54`/`sim_titanium31` aim divergence was a REFERENCE bug (missing ClampFloat), not a VM bug. Details: HANDOFF §14/§16.
- Interpreter semantics battery (2026-09-12): `scripts/gen_interpreter_battery.py` emits `data/interpreter_probes/semantics_battery.txt` (46 `B.*` TimePlot channels: every Operation index, CompareFloats, ConditionalSet each kind incl. unwired-false, coercions, div/mod-by-zero, unwired inputs, latches, vector round trip, clamp, power/mod edges). `tests/interpreter_semantics_battery.rs` diffs reference-vs-O0 channels + reference-vs-O0/O1 traces each tick (report-only; `BATTERY_STRICT=1` fails). Building it exposed and fixed **two reference gaps**: `GraphBrain` had no `TimePlot` arm (emitted no channels at all) and no `ClampFloat` arm (zeroed everything downstream). Originally DIVERGENT (div/mod-by-zero guards, ConditionalSet unwired-false hold, initial latch value) — **all three CLOSED in Phase A (HANDOFF §16)**; the battery now reports `PASS: reference == O0 == O1 on every channel and trace`. It also emits a channel sidecar (`semantics_battery.channels.json`) marking policy vs golden. Details: HANDOFF §15/§16.
- Game-truth policy readout (2026-09-12, Phase B): `scripts/read_battery_from_game.py` reads the game's TimePlot export of `semantics_battery.txt` (v0.14 natural probe; `parse_timeplots` locale-comma rule). Round 2 (50 ch, 1060 samples) **corrected an initial misread**: div/mod-by-zero = **IEEE inf/nan** (A1 correct), `ConditionalSetFloat` **and** `ConditionalSetVector3` unwired-false = **previous-tick HOLD** (the vector hold confirmed via `Magnitude` — the earlier "Vector == 0" was a `Vector3Split` consumer artifact), unwired input = 0, and **all 16 `Operation` indices** match. **New game-sourced open items:** the game is **not memoized per tick** (a node feeding several consumers can give them different values — `Vector3Split` reads 0 where `Magnitude` holds), same-tick variable reads are visible, and Bool→Float wires are dropped (no coercion). Sim NOT changed on their account. Details: HANDOFF §17.
- `bat` O0≠O1 FIXED (2026-09-12): opt-in `tests/pass_bisect.rs` + `PassManager::o1_prefix(k)` localised it to CSE; `tests/cse_ir_diff.rs` dumped the IR and showed CSE merging `ConstructVec(RandomF,RandomF,RandomF)` into `ConstructVec(r,r,r)`. Root cause: `OpCode::effect()` marked **`RandomF` `Pure`** — now `OpEffect::Write`. `PASS_BISECT=bat` → `all prefixes == O0`; regression `cse::tests::does_not_cse_random_f`; lib **168/0**. bat's remaining cert divergence is now the same world-RNG placeholder as `Pixel_Heart`. Run: `PASS_BISECT=bat cargo test --test pass_bisect -- --nocapture`, `CSE_DIFF=bat cargo test --test cse_ir_diff -- --nocapture`. Details: HANDOFF §17b.
- Serve-clock refusal (2026-09-12): the game awards the **serve to the opponent** when a 5 s `<ServeCountdownDisplay>` expires (`serveDeadlineTick`); the sim used to re-arm the same server with an unmeasured `SERVE_CLOCK = 15.0`. Now `SERVE_CLOCK = 5.0`, `Score::award_serve`/`serve_override` (lasts the current game; cleared at game end) + `Score.serve_forfeits`, and `TennisWorld.serve_clock_t`/`on_serve_deadline` (survives recatches). Tests added; lib **167/0**. Exact deadline window + scope (game vs set) still to read from the probe via `modhost/read_serve_deadline.py`. Details: HANDOFF §17c.
- **v0.15f re-target (2026-09-12)**: mod host `modhost/v0.15f/` + `build_paritymod_v015.ps1` + `launch_v015.cmd` (`MODHOST_GAMEDIR`/`MODHOST_PARITY_EXE`); the probe runs a full match (`tick 422 / done:true`, 748 KB TimePlot). Two version blockers fixed in `paritymod-src/metadata_probe.h`: (a) instruction-SHAPE preamble matching (never pin a RIP-relative address byte); (b) `metadata_trace_field_optional()` allowlist for named fields the new build deleted (`cachedPredictedBounceTime`/`cachedPredictedSecondBounceTime`/`predictedBounceAge`). Sim physics is still v0.14-pinned — v0.15 changed real behaviour (frame-interpolated ball position; time-to-ground cache fixed). Recipe: `docs/RE_PLAYBOOK.md` §8, case study §18.
- Known structural gaps: sim holds 100% of service games (stock-vs-stock
  proves it is world-structural, not VM); first-server mapping unproven
  (10/28 on hold-rows); raw `Ball Incoming` rule needs live getter capture.
