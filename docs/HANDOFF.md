# SESSION HANDOFF — aia_comp-sim tennis parity (updated 2026-09-10, evening)

**Read this + `docs/TENNIS_V014_PARITY_NOTES.md`; generic method now lives in
`docs/RE_PLAYBOOK.md` (read it first for anything new).** Goal framing from
the user, verbatim: **"We don't clone the game 1:1 — if we play any 2 AIs
against each other the results must be the same as in the game."**
(Viewer/editor = QOL only; Bevy non-headless viewer deferred by user.)

## 1. Locations (canonical)

| Thing | Path |
|---|---|
| **Rust sim repo** (git, GitHub `titaniummachine1/aia_comp-sim`) | `C:\gitProjects\aia_comp-sim` (standalone — do NOT move back into AIA_tennis) |
| Capture tooling + modded game | `C:\gitProjects\AIA_tennis\modhost\` (v0.14 install in `modhost\v0.14\`; parallel workers in `modhost\v0.14_w2..w4\`) |
| Mined fixtures + replay tests | `aia_comp-sim\tests\fixtures\tennis-v014\` |
| Deep game-knowledge doc | `aia_comp-sim\docs\TENNIS_V014_PARITY_NOTES.md` |
| RE methodology doc | `aia_comp-sim\docs\RE_PLAYBOOK.md` |
| Version/disclaimer doc | `aia_comp-sim\docs\GAME_VERSIONS.md` |
| Bot saves (82) | `%USERPROFILE%\AppData\LocalLow\Unicorn One\AIComp\Saves\Tennis\` |
| Probe builds | `modhost\build_paritymod_event.ps1` (event); natural = same sources with `-DV014_METADATA -DV014_NATURAL_TRACE -DPARITYMOD` (no script yet — command in git history `89953cf` context); backups in `modhost\paritymod\` |

## 2. Golden rules (user-enforced)

1. **Never trigger a Bevy/dep rebuild** — crate-only builds, no feature or
   profile flips, no folder renames. Background any long build with a log.
2. **Mode-first VM**: no implicit soccer default. `Lowerer::compile(graph,
   Option<GameSpec>)`; `compile_pure` for unit tests; simulation requires
   explicit `GameSpec` (`tennis_v014()` = latest default,
   `tennis_builder()` = v0.012 builder order for on-disk graphs).
   Mode gate: `src/mode.rs`.
3. Do not redistribute game binaries.
4. Only kill `Aialanders.exe` processes whose path is inside a
   `modhost\v0.14*` dir (`modctl.game_pids_in_gamedir()` — env-aware now).
5. **Agent shells kill child processes** — long runs go through
   **scheduled tasks** (`schtasks /Create /Run`, `/TR` max 261 chars →
   use .cmd launcher files, see `modhost\worker_w*.cmd`).
6. Game is **multi-instance**: up to 4 concurrent instances, one game dir
   each (env `MODHOST_GAMEDIR`/`MODHOST_PARITY_EXE`); saves dir shared;
   ~2 GB commit per instance — monitor free RAM (`modhost\ram_watch.csv`).

## 3. Current numbers

- lib tests: **159 green** (fatigue sensors wired + pinned tests, `d36455b`)
- Shot-solver parity vs live 210-case matrix: **200/210 ≤0.25 m/s**
  (excluded 10 = fallback rows)
- Landing parity vs NthLanding matrix: **8/10** (one ~0.55 m curve residual
  = predictor doesn't zero curve on bounce — open fit)
- **Outcome parity: exact measurement in progress.** The noisy 71.9%
  (23/32, `722f5db`) is superseded: probe now logs per-point winners
  (`point_winners[]` in `parity_state.json`, smoke-tested: [0,0] for a 2-0
  home match), scoring scripts rewritten (`e409ec9`).
- Sim tournament (82 bots vs titanium54, sim-side): **83 results, 0
  failures** (`e99a892`).

## 4. In flight RIGHT NOW (2026-09-10 late evening)

**A. Parallel game tournament (status: 70/83 rows recorded — running).**
4-way parallel, points=8, scheduled tasks `aia_tour_w1..w4`:
- Rows: `modhost\game_tournament_results_w1..w4.jsonl` (17/14/19/20)
- Logs: `modhost\tour_p8_w{k}.log`; RAM: `ram_watch.csv`
- When done: merge rows (dedupe away+seed) → `python scripts\run_sim_tournament_pairs.py titanium54` (replays 8-point matches, exact attribution via `match_result()` walking point winners) → `python scripts\score_parity.py` → EXACT outcome parity %.
- Sim bin now emits `point_winners[]` (delta-based, reset-safe) and scoring compares winner SEQUENCES first, leader fallback (`3c37ed9`).
- Legacy serial artifacts: `game_tournament_results_points4.jsonl.bak`.

**B. SERVE PARITY BLOCKER (diagnosed — fix is the top simulator task).**
Sim matches are decided entirely by double faults: even titanium54 vs
stock = `faults:[8,0], double_faults:[4,0]` — 0 rallies. Traced serve:
strike at toss apex fires **backward/down** (`vel [-6.6,-18.2]`) —
`on_swing_release` uses the LIVE `cmd.move_or_aim` as serve aim, but the
bot's Vector31 at that tick is its movement/stance output. Game
semantics: `TennisController` has ONE `Vector31` = "move-to / on-hit aim"
(phase-dependent), and the player separately holds
`<ServeAimHint>k__BackingField` + `HasServeAimHint` (captured: home aim
(13.75,-4.5) = legal box, while moveDestination (-14.99,-10.97) = stance).
Hypothesis: the game latches the serve aim from the graph output at serve
announcement; the live Vector31 during Toss is NOT the strike aim.
Fix plan: (a) pin the ServeAimHint lifecycle from a live capture (extend
the event probe to poll player fields across serve transitions);
(b) sim: latch serve aim at ServeSetup entry (validated into the legal
box), strike uses the latch, fallback `legal_serve_target`;
(c) verify: titanium54 vs stock must produce a real rally point;
(d) rerun the sequence spot-check for real parity numbers.

## 6. Open work queue (SIMULATOR session — top-down)

1. **Serve blocker (§4B) — the #1 task.** Fix, then verify:
   titanium54 vs stock must rally; rerun the 10-pair sequence spot-check.
2. **Finish the parity measurement** (§4A): merge shards →
   `run_sim_tournament_pairs.py titanium54` → `score_parity.py` → post %.
3. **Sim-vs-game channel diff** using auto-exported timeplots
   (`modhost\parse_timeplots.py`) — first real per-tick ground truth test.
4. **Remote export API** (button-free): enumerate the TimePlot classes'
   methods via the probe's class dump (`Gates\TimePlot.cs` etc. — names
   found in global-metadata.dat strings), wire an `export` parity_cmd.
   Currently optional (quit-flush works) but is the robust end state.
5. Slice crossed-net **+0.05 s** fit term in the solver.
6. Getter-items capture (failed twice) → sensor label ABI (35/51/15/5 v0.14
   tables) + `Ball Incoming`-false-on-first-bounce quirk verification.
7. Bevy non-headless viewer (QOL, user-approved deferral).

## 6b. COMPILER — separate session/repo (do NOT mix with simulator work)

The graph compiler lives in its OWN repo: `C:\gitProjects\aia_graphc`
(git, root commit `1008c97`). Its handoff is `aia_graphc\README.md` +
`aia_graphc\PROGRESS.md`. The sim repo's only tie-in is the CI test
`runtime_brain::tests::graphc_poc_demo_replays` (replays a compiled bot).

## 7. Key commands

```
# build/test (crate-only!)
cd C:\gitProjects\aia_comp-sim
cargo test --lib
cargo test --test tennis_v014_solver_parity -- --nocapture   # 200/210
cargo test --test tennis_v014_landing_parity -- --nocapture  # 8/10
cargo run --bin tennis_tournament -- --home titanium54 --away aia3 --seed 7 --points 4

# game matches (single instance, manual)
cd C:\gitProjects\AIA_tennis\modhost
python modctl.py launch --home X --away Y --seed N --points P
python modctl.py wait --timeout 900
python modctl.py quit

# parallel tournament (running; see §4)
schtasks /Run /TN aia_tour_w2   # etc.
Get-Content tour_p8_w1.log -Tail 3

# parity scoring after merge
cd C:\gitProjects\aia_comp-sim
python scripts\run_sim_tournament_pairs.py titanium54
python scripts\score_parity.py
```

## 8. Pitfalls hit this session (don't repeat)

- PowerShell `python -c "..."` breaks on nested quotes — write scripts to
  `%TEMP%\opencode\*.py` and run them. `${k}` needed inside -f strings.
- The opencode shell tool kills the whole process tree on command exit —
  NEVER launch long runs as its children; scheduled tasks only.
- `schtasks /TR` caps at 261 chars → .cmd launcher files.
- `git add -A` mixes unrelated fixes — stage deliberately.
- Game holds trace/log files while running — stop processes first.
- Probe with raw inf channels can hang the game — clamp to ±1e30.
- Settings files need UTF-8 without BOM.
- `emit_rust_fixtures.py` RUST path fixed (resolves to
  `C:\gitProjects\aia_comp-sim`) — asserted, don't revert.
- First serial-run sweep misattributed stale timeplots to Adam — fixed by
  mtime filtering + UTC windows; Adam folder contents are suspect.

## 9. Suggested first five moves for the next session

1. Read `RE_PLAYBOOK.md` §0/§5 + this file; check `ram_watch.csv` and
   `tour_p8_w*.log` — is the tournament done?
2. If done: merge shards → `run_sim_tournament_pairs.py titanium54` →
   `score_parity.py` → post the exact parity %.
3. If not: check per-shard resumes (`--shard k --shards 4 --results ...`
   with env overrides), rerun only unfinished shards.
4. Channel diff: sim probe values vs auto-exported timeplots, channel by
   channel (44 channels available).
5. Work the open queue §6 top-down; commit after every landed item.