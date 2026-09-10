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

- lib tests: **162 green** (`bf1b69f`+)
- Shot-solver parity vs live 210-case matrix: **200/210 ≤0.25 m/s**
  (excluded 10 = fallback rows)
- Landing parity vs NthLanding matrix: **8/10** (one ~0.55 m curve residual
  = predictor doesn't zero curve on bounce — open fit)
- **Outcome parity (exact, 2026-09-11): 25/78 = 32.1%** (24 agree-leader +
  1 agree-seq, 53 disagree; no unknowns) — first exact measurement with
  serves that actually land (`7e9245c`). Before the serve session the sim
  was all double faults (no rallies at all). Remaining gap = rally aim
  semantics (see §4C).
- Sim tournament (82 bots vs titanium54, sim-side): **83 results, 0
  failures** (`e99a892`).

## 4. In flight RIGHT NOW (2026-09-11, after the serve session)

**A. Game tournament: DONE, 82 unique rows (77 done+success).**
4-way parallel run completed (`game_tournament_results_w1..w4.jsonl`),
merged (dedupe home+away+seed, keep stateful rows) into
`modhost\game_tournament_results.jsonl`. 5 pairings have NO state: the
GAME itself wedges vs Pixel_Heart / sim_probe / controller (>55 min, score
stalls) and titanium34 / ignore_ball31 needed >20 min (re-run with
`--timeout 3300` in flight as tasks `aia_tour_fix2_w3/w4`, results
`game_tournament_results_fix2_w*.jsonl` — re-merge + re-score when they
land). Scoring excludes stateless rows automatically (`state.done` filter).

**B. SERVE PARITY — FIXED this session (4 commits, all verified).**
1. `9b8af82` Serve aim latch (ServeAimHint model): strike aim latched at
   ServeSetup entry, in-box candidates honored, else `legal_serve_target`.
   The live Vector31 during Toss is the stance output — was the
   all-double-fault blocker.
2. `9796aab` Serve auto-strike (TennisAutoSwing): the toss serve fires on
   descent into the racket zone with the latch, whatever the bot's swing
   output (bots that hold/never press still serve). Plus double-hit
   strike lock until bounce (swing-pulse bots no longer reset their own
   serve in flight).
3. `bf1b69f` General auto-contact: a held swing strikes as soon as the
   ball is in the racket zone (charge = hold duration; toss on descent,
   server only; skipped when striking would foul). "Release-on-range-exit"
   brain patterns can never connect otherwise. Plus `Ball Has Bounced`
   stays true after the serve bounce (bounce count no longer reset).
4. `7e9245c` Rally aim latch: only opponent-court Vector31 outputs count
   as strike aims; positioning/chase outputs (own half, ball position —
   what brains emit while receiving) never overwrite the latch; deep
   default fallback. Returns became real shots; mixed sequences.

**C. TOP REMAINING PARITY BLOCKER — rally aim semantics (32.1% → ?).**
Trace evidence (`--trace` on tennis_tournament, per-tick brain aims):
titanium54's Vector31 during receive = its own position / the ball
position (pure positioning). The sim's dud-return chain is fixed by the
latch, but WHERE the game aims a return (and the default deep aim
(13.75,-4.5) captured at serve time) is still un-pinned. Next moves:
(a) extend the event probe to poll Vector31/ServeAimHint/moveDestination
    across a RECEIVE transition in a live game (needs one free instance);
(b) sweep the latch model variants in-sim against the 78-row game set
    (default target candidates: deep corner (13.75,-4.5), center-of-back,
    opponent position) and score each with score_parity.py;
(c) auto-contact charge for never-holding bots is currently the bot's
    charge (0 → 85% speed) — game truth unknown, sweep it too.
Note: sim matches are now long (8 pts ≈ 1-8k ticks); the pairs replay
takes ~5 min. `score_parity.py` chokes on a BOM — never empty
sim_pairs_results.jsonl with PowerShell `Set-Content` (use python).

## 6. Open work queue (SIMULATOR session — top-down)

1. **Rally aim semantics (§4C)** — the 32.1% → ? lever. Live receive
   capture (probe) + latch-variant sweep vs the 78-row game set.
2. **Re-merge + re-score** when `aia_tour_fix2_w4` lands (titanium34 +
   ignore_ball31 rows; w3's three pairings wedge the game itself — treat
   as permanent no-verdict).
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

1. Read `RE_PLAYBOOK.md` §0/§5 + this file; check
   `game_tournament_results_fix2_w4.jsonl` — did titanium34 +
   ignore_ball31 land with state?
2. If yes: re-merge (`merge_tour_shards.py` equivalent incl. fix2 files,
   keep stateful rows) → `run_sim_tournament_pairs.py titanium54` →
   `score_parity.py` → updated exact %.
3. Work §6.1: rally-aim latch variant sweep (cheap, in-sim, 78-row set)
   while the probe capture for the live receive transition is prepared.
4. Channel diff: sim probe values vs auto-exported timeplots, channel by
   channel (44 channels available).
5. Commit after every landed item; never empty sim_pairs_results.jsonl
   via PowerShell (BOM — see §4A note).