# SESSION HANDOFF — aia_comp-sim tennis parity (updated 2026-09-12)

**Read this + `docs/RE_PLAYBOOK.md` (generic method; §8 = version re-target) +
`docs/TENNIS_V014_PARITY_NOTES.md`.** Goal framing from the user, verbatim:
**"We don't clone the game 1:1 — if we play any 2 AIs against each other the
results must be the same as in the game."** (Viewer/editor = QOL only.)

## 0. LATEST SESSION (2026-09-12) — READ THIS FIRST

**Plan of record: `C:\gitProjects\implementation_plan.md`** (phases A–F, with
Phase F's status; open decisions at the end). Read it before starting anything.

### Landed this session
| Area | What | Where |
|---|---|---|
| Ground truth | Multi-seed distributional parity harness (game sweep → sim replay → TV distance) | `scripts/run_sim_seed_sweep.py`, `scripts/score_distribution.py`, §13 |
| Interpreter | Corpus certification: reference `GraphBrain` vs shipping O1 VM vs pass-free O0 VM, per-tick traces | `tests/tennis_interpreter_certification.rs`, §14 |
| Interpreter | Semantics battery: 46 TimePlot channels probing every ambiguous semantic | `scripts/gen_interpreter_battery.py`, `tests/interpreter_semantics_battery.rs`, §15 |
| Interpreter | `compare_traces` now diffs the tennis controller (was vacuous for tennis) | `src/graph_vm/trace.rs` |
| Interpreter | **2 reference gaps FIXED**: `GraphBrain` had no `TimePlot` arm (emitted no channels) and no `ClampFloat` arm (zeroed everything downstream) → cert went **7/12 → 9/12 clean** | `src/graph/eval.rs` |
| v0.15f mod | Mod host + build script + launcher; probe runs a **full match** on v0.15f | `modhost/v0.15f/`, `build_paritymod_v015.ps1`, `launch_v015.cmd`, §18 |
| v0.15f mod | **2 version blockers FIXED**: instruction-shape preamble matching; version-optional field allowlist | `paritymod-src/metadata_probe.h` |

### Pinned numbers (do not regress silently)
- `cargo test --lib` → **164 passed, 0 failed, 2 ignored**.
- Certification → **12/14 clean**; remaining: `bat` (aim + O0≠O1),
  `Pixel_Heart` (`Round(RandomFloat)` — world-RNG placeholder).
- Battery → 46 channels, 4 tick-varying, **0 divergences**
  (`PASS: reference == O0 == O1 on every channel and trace`).
- **Game-read policy truth (§17)** — the game ran the battery (50 ch, 1060
  samples): div/mod-by-zero = IEEE inf/nan; `ConditionalSetFloat` **and**
  `ConditionalSetVector3` unwired-false = **previous-tick HOLD** (confirmed via
  `Magnitude`); unwired input = 0; all 16 `Operation` indices confirmed.
  **New game-sourced open items:** the game is not memoized per tick
  (`Vector3Split` reads 0 where `Magnitude` holds); same-tick variable reads are
  visible; Bool→Float wires are dropped (no coercion).
- **`bat` O0≠O1 FIXED (§17b)**: `RandomF` was mis-marked `Pure`, so CSE merged its
  draws (`ConstructVec(r,r,r)`) — now `OpEffect::Write`. `PASS_BISECT=bat` →
  `all prefixes == O0`. bat's remaining cert divergence = the same world-RNG
  placeholder as `Pixel_Heart` (`RandomFloat → 0.0`), not an optimizer bug.
- **Serve-clock refusal modelled (§17c)**: `SERVE_CLOCK = 5.0`; on expiry the
  serve is **awarded to the opponent** for the rest of the game
  (`Score::award_serve`), not re-armed. Lib tests **167 passed / 0 failed**
  (was 164). Exact `serveDeadlineTick` window + scope (game vs set) still to be
  read from the probe (`modhost/read_serve_deadline.py`).
- **Phase C channel diff RUN (§20)**: `scripts/channel_diff.py` rewritten; first
  run shows `ChargePct` (game 0.28, sim **0.00**) as a total model error, fixed
  by making `swing_hold_gate` the **default** (`AIA_SWING_MODEL=legacy` restores
  the old default). Open: `SwingHeld` (sim 0.09 vs game 0.54), `Chase`/`Mode`
  ~2× overshoot, `Bounced`/`Incoming` overshoot.
- **Phase C scale-up (§20b)**: reintroduced the 78-match game set (tournament
  row format) — sim home-win **55%** vs game **58%**, **TV distance = 0.078**.
  Exact per-match sequence parity stays low (RNG/first-server unpinned).
- **Phase C first-server-matched sweep (§22)**: `restart_sweep.py --seeds 1-16
  --points 8` → **exact parity 11/17 = 64.7%** (vs 32.1% unmatched!), hold rate
  **53% vs 53%**; residual is *strength* (game 5.6-1.0 vs sim 3.8-2.1, TV 0.20) —
  the remaining rally-quality gap.
- **Simulator hub / v0.15 target (GAME_VERSIONS)**: `GameSpec::tennis_v015()` is
  now the latest tennis spec (`tennis_v014()` explicit); the only v0.15
  world-model delta is the **swept (frame-interpolated) ball contact**, gated by
  `GameVersion::interpolates_ball()` + `TennisWorld::for_spec`. Lib **169/0**.
- v0.15f `parity_state.json` → `tick 422 / callbacks 3239 / points_done 1 /
  done:true / success:true`; quit flushed a **748471-byte** TimePlot.
- Exact outcome parity **25/78 = 32.1%** — still brittle; the distributional
  metric is the headline now.

### Next actions, in order (see the plan for detail)
1. **Phase A + B DONE.** A2 (hold) is game-**CONFIRMED** for Float and Vector.
   Remaining interpreter/VM item: **`bat` O0≠O1 = the CSE pass** (§17b) — write a
   focused CSE unit test on bat's aim cone before touching CSE.
2. **Simulator/VM**: decide the two game-sourced open items (§17) — same-tick
   variable visibility and the Bool→Float wire drop — only with a confirming
   graph each; both change real-bot semantics, so measure first.
3. **Phase C**: scale the multi-seed sweep (`--points 8`, ~50 seeds), then the
   never-run per-tick channel diff against the game auto-export.
4. **Phase F4**: re-mine v0.15 fixtures and triage drift against the §18
   changelog map before touching the sim.

### Housekeeping / gotchas for the next session
- **Nothing is committed.** `aia_comp-sim`, `aia_graphc` and `AIA_tennis` are
  all dirty; stage deliberately (never `git add -A` — `AIA_tennis` holds
  `c`, `titan`, `opencode.json`, `v0.15f/`, `v0.14*/`, upstream trees).
- Never commit game binaries. `modhost/v0.15f/` and `modhost/paritymod/*.exe`
  are working artifacts.
- Long runs (builds, game launches, test suites) must go through **scheduled
  tasks** — the agent shell kills child process trees. `.cmd` launchers, logs
  to `%TEMP%`.
- The tool's ~30 s command cap is real: `Start-Sleep 25` + work is the safe
  budget; poll in slices.



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

0. **Ground-truth harness — LANDED 2026-09-12 (see §13).** Scale the game
   restart sweep (`--points 8`, ~50 seeds/pairing), then
   `scripts/run_sim_seed_sweep.py` + `scripts/score_distribution.py`. Score
   distributions (home-win / server-hold / TV distance); the exact per-seed %
   is a secondary column until first-server mapping + RNG draw sites are pinned.
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

---

# SESSION 2026-09-12 — compiler modes/compaction + parity harness (done)

## 10. What landed (sim side)

- **Compiler optimization modes + source-free compaction** (repo
  `aia_graphc`, see its `PROGRESS.md` Session 10). For the sim the relevant
  artifacts are:
  - `scripts/run_compiler_mode_parity.py` regenerates
    `data/compiler_probes/mode_parity/{o0,o1,o2}.txt` (+ `.desc.json`) from
    one behavior-bearing bot.
  - `tests/compiler_mode_parity.rs`: the SAME bot at o0/o1/o2 emits
    **identical controller output for 40 ticks**; o0 keeps working
    TimePlot channels, o1/o2 strip them. This is the guard that the
    compiler modes never change strength.
  - `tests/titanium_compact_parity.rs` (opt-in): set `TITANIUM_ORIG` +
    `TITANIUM_COMPACT` (a `graphc-rs` o2 output) and it runs both, home
    side, tick for tick. Validated: Titanium 4.64 MB → 1.54 MB, 60 ticks
    identical.
  - `tests/compiler_probe.rs` pins updated to probe **93 nodes/O0 90/O1 80**
    (was 94/91/80; the one-node drop is a safe identity fold).
- lib suite now **164 green**; `compiler_probe` + `compiler_mode_parity`
  green.

## 11. Best path from here (decided)

Compiler is effectively **v1** (feature-complete). The blocker for any
further compiler work is **sim parity being unmeasurable**, so the next
session is sim-first:

1. **Valid ground truth first**: statistical parity harness (multi-seed
   outcome *distributions* per pairing) + matched start conditions. The
   exact metric is unusable (first-server 8/78; RNG shelved).
2. **Certify the interpreter** (separates VM bugs from world-model bugs):
   sweep many real saves (AIA/AIA3/Titanium) + compiler outputs through
   `GraphBrain` vs `graph_vm` with `ObservableTrace`/`compare_traces`,
   asserting identical var commits + controllers. Extends the existing
   `runtime_aia_trace_matches_graph_brain` to a corpus.
3. **Then** the known tennis gaps: rally-aim semantics, first-server
   mapping, serve regime, the 18 "uncertain" getters (ROADMAP).

## 12. Enabler in flight — restart match WITHOUT relaunching the game

Goal: fast multi-seed parity sampling (a launch is ~90 s + wedge-prone).
Existing groundwork in `modhost`:
- `paritymod-src/metadata_probe.h` already has `reset_match` (invokes
  `TennisGameManager.ResetMatch`), `rngtest` (`Random.InitState` +
  `RandomRangeInt`), `methods` (dump a class's methods by name), `snapshot`,
  `quit`; driven by `reset_sweep.py` (`reset_sweep.jsonl`).
- Startup path (natural trace) does `Random.InitState(seed)` then
  `TennisGameManager.QueueStartMatchWhenGraphsReady()`.
- Next: a `restart_match` cmd that re-seeds + `ResetMatch` +
  `QueueStartMatchWhenGraphsReady` in one shot (and, if the methods exist,
  reloads team graphs so home/away can change per sample). Method names are
  discovered with the existing `methods` probe — no full decompile needed
  (IL2CPP names, per `RE_PLAYBOOK.md`).

### Status: game-verified (2026-09-12) + native timeplot export

- `restart_match` (optional `seed`/`points`): `CoreRandom.InitState(seed)` ->
  `ResetMatch()` -> `QueueStartMatchWhenGraphsReady()`, re-arms per-point
  accounting, logs `seed_ok`/`reset_ok`/`queue_ok` + before/after
  `ServingTeam` + RNG. **Verified**: sweeps seeds in-place without
  relaunch (`python modhost/restart_sweep.py --seeds 1-3 --points 1`);
  the epoch advances in ~2 s while the match is live.
- `restart_epoch` added to `parity_state.json`: the first sweep seed used to
  read the *boot* match's snapshot (same seed as the launch), so drivers now
  wait for `restart_epoch > prev` and attribution is exact.
- Native timeplot export via `TimePlot.ExportToJson` (singleton via
  `get_Instance`), invoked by the mod on: `restart_match`
  (`reason:restart`, before `ResetMatch`), `quit` (`reason:quit`), a new
  on-demand `{"cmd":"export_timeplot"}` (`reason:api`), and an interactive
  close via a `UnityWndClass` subclass flushing on `WM_CLOSE` /
  `WM_ENDSESSION` (no preamble pinning; build now links `user32`). Each call
  logs a `parity_cmd`/`export_timeplot` row with `ok`/seed/epoch/serving/team.
- `restart_sweep.py` names each export from the home graph's tick-0 samples
  (`timeplot_naming.py`):
  `timeplot_<time>_<left>_vs_<right>_<hB|aB>_srv<S>_seed<N>.json`
  (`hB` = home on -X/near; home-on-left from -Z; both are flippable
  constants). Verified end-to-end: boot match -> `srv1_seed1`, restart to
  seed2 captures seed1 -> `srv0_seed1`, ... final `quit` export ->
  `srv0_seed3`.
- Rebuilt: `build_paritymod_reset.ps1` -> `Aialanders-paritymod-reset.exe`
  (68952 B). Remaining gap: in the sampled seeds the layout token was always
  `hB` (home consistently on -X); confirm the convention on a match where the
  camera/side actually flips before trusting `aB`.

## 13. Ground-truth harness — multi-seed distributional parity (2026-09-12)

The §11 item 1 ("valid ground truth first") now has a working three-stage
pipeline. It exists because the exact per-seed metric is brittle while the
seed->first-server mapping and RNG draw sites are unpinned.

1. **Game (multi-seed):** `modhost/restart_sweep.py --seeds 1-50 --points 8`
   restarts one live match per seed and appends to `restart_sweep.jsonl`,
   one row per seed carrying `state.point_winners` (the outcome), `seed`,
   `serving_team` (that seed's SETUP server) and `state.restart_epoch`.
   **`restart_epoch` is the attribution guard** — pre-fix rows lack it and are
   discarded (the first sweep seed used to read the boot match's snapshot).
2. **Sim (matched replay):** `scripts/run_sim_seed_sweep.py` reads those rows
   and replays each `(home, away, seed)` headless with the same `--points` and
   `AIA_FIRST_SERVER` derived from `serving_team`, appending to
   `data/tennis/sim_seed_sweep.jsonl` (append, resume-safe; `--game`, `--home`,
   `--max-ticks`, `--timeout`). Prints the exact per-seed verdict as it goes.
3. **Score (read-only):** `scripts/score_distribution.py` joins the two files
   on `(home, away, seed, first_server)` and reports, per pairing and pooled:
   **home win rate, server-hold rate, mean points for/against** and the
   **total-variation distance** between the game and sim leader distributions.
   Exact per-seed agreement is a secondary column, never the headline.
   `--min-seeds N` flags pairings too thin to trust (rates are noise below it).

Verified end-to-end 2026-09-12 on the 3 post-fix sweep rows
(titanium54 vs aia3, points=1, matched servers): `agree-seq` 2/3, and the
scorer reports game vs sim home-win 67% vs 100%, server-hold 33% vs 67%,
TV 0.33 — i.e. exactly the kind of stable, sample-size-aware signal wanted.
**Scale before interpreting:** the current sample is 3 seeds / points 1, so the
rates are noise. Run the game sweep at `--points 8` for ~50 seeds per pairing,
then re-run stages 2-3.

## 14. Interpreter certification — reference `GraphBrain` vs the VM (2026-09-12)

> **UPDATED 2026-09-12 (Phase A) — now 12/14 clean.** `Adam` was a *false*
> divergence (NaN-aware comparator, §16); `nqvxf22` was a second empty-`Operation`
> panic (same fix as Adam). Remaining: `bat` (aim `(0,0)` vs VM, and O0≠O1) and
> `Pixel_Heart` (`Round(RandomFloat)` — a world-RNG placeholder, not an
> interpreter bug). Full landings table in §16.

HANDOFF §11.2 landed as `tests/tennis_interpreter_certification.rs`.

Prerequisite change: `compare_traces` now also diffs the **tennis controller**.
Before this it only compared the 4 soccer `commands`, which a tennis graph
leaves at their defaults — so a tennis trace identity passed vacuously. Soccer
behavior is unchanged (both sides `None`).

What it does: for each tennis save in a curated corpus it drives the reference
tree-walker (`graph::GraphBrain`) and **two** VM builds — the shipping O1 VM
(`RuntimeBrain::compile_for`, byte-identical lowering to `TennisBrain::compile`)
and a pass-free O0 VM (`Lowerer::compile_for` + `ProgramBuilder.pack` +
`RuntimeBrain::from_program`) — through a live `TennisWorld` for 40 ticks,
comparing Pass 1..8 commits + `TennisController` every tick. The O0 arm is what
attributes a divergence: **O1-only => optimizer/pass bug; O0 and O1 both =>
lowering/interpreter bug.**

Report-only by default so CI stays green; `TENNIS_CERT_STRICT=1` fails on any
divergence. `TENNIS_CERT_MAX` (default 12) / `TENNIS_CERT_TICKS` (default 40).
Per-bot panics are caught and reported (with the bot name) so one bad save
cannot abort the corpus.

First run (2026-09-12, before the §15 `ClampFloat` fix): **12 graphs, 7 clean**.
After that fix: **9 clean** — `titanium54`, `aia3`, `aia`, `ignore_ball31`,
`sim_titanium31`, `cross_court`, `open_court`, `alternator`,
`graphc_serve_latch`. Findings, in priority order:

1. **Aim-request divergence — RESOLVED for `titanium54`/`sim_titanium31`,
   `bat` remains.** The reference's aim x was systematically **0.0**
   (titanium54 `(0.0,5.0)` vs VM `(1.3538578,5.0)`; sim_titanium31
   `(0.0,6.0)` vs `(3.0625,6.0)`; bat `(0.0,0.0)` vs `(0.8833,0.8833)`).
   Root cause: **a missing `ClampFloat` arm in `GraphBrain`** (§15), which
   zeroed the entire aim subtree in the REFERENCE — not a VM bug. With the
   reference able to clamp, `titanium54` and `sim_titanium31` are clean. `bat`
   still diverges (aim `(0,0)` vs VM `(0.8833,…)`) and additionally has
   O0 != O1 (finding 2). Tool: `scripts/analyze_aim_chain.py <bot> [--depth N]`
   walks the aim chain on the save JSON with no compile; bat's aim source is a
   `ConditionalSetVector3` gated by `TennisGetBool 'Is Self Serving'` whose
   false branch is a `ConditionalSetVector3 mod='1'` — the next target.
2. **`bat`: O0 != O1** — aim `(0.8833,0.0264)` (o0) vs `(0.8833,0.8833)` (o1).
   A genuine O1 pass divergence, independent of the reference: the first known
   counterexample to "O1 never changes behavior" outside the single soccer
   probe.
3. **`Adam` panics the reference** on an empty `Operation` modifier
   (`dropdowns::OperationKind::from_modifier("")`). Both interpreters reject
   "" by design ("never returns a default op"), but `GraphBrain`
   force-evaluates every SetVariable / debug sink / root function, so it hits a
   node the DCE'd VM never touches. This is the open `T_op_empty` probe
   question (the game's policy for an empty Operation modifier) — do not
   "fix" it by defaulting until that is read from the game.
4. **`Pixel_Heart`: `shot_type` diverges** (reference 0.0 vs VM 1.0, in BOTH O0
   and O1) while move/aim/swing/sprint agree — a shot-type dropdown wire
   divergence, independent of the aim chain. Unexplained; next after `bat`.

Commands:
```
cargo test --test tennis_interpreter_certification -- --nocapture
TENNIS_CERT_STRICT=1 cargo test --test tennis_interpreter_certification -- --nocapture
```

## 15. Interpreter semantics battery — behaviour proof via TimePlot channels (2026-09-12)

> **UPDATED 2026-09-12 (Phase A/B):** all three divergence classes below are
> **CLOSED** (§16) — the battery now reports
> `PASS: reference == O0 == O1 on every channel and trace`. The same graph was
> then loaded in the **game** and the `B.*` channels read back (§17): IEEE
> inf/nan, Float hold, unwired=0 and all 16 `Operation` indices are confirmed;
> **Vector hold** and **Bool→Float coercion** are new game-sourced open items.
> The fixture now also emits a channel sidecar
> (`data/interpreter_probes/semantics_battery.channels.json`).

`scripts/gen_interpreter_battery.py` -> `data/interpreter_probes/semantics_battery.txt`
(371 nodes, 46 `B.*` channels). `tests/interpreter_semantics_battery.rs` runs the
reference vs **O0** (channels + trace) and the reference vs **O1** (trace only,
because O1 strips debug sinks) for 12 ticks, every tick. Report-only by default;
`BATTERY_STRICT=1` fails on any divergence. Companion to §14: that one replays
what real bots happen to touch, this one probes semantics directly. The same
`.txt` is game-loadable (a tennis bot with a direct-wired idle controller), so a
game TimePlot export can be diffed channel-by-channel the same way.

**Two reference gaps found and fixed to make it observable at all:**
1. `GraphBrain::exec_debug_draw` had **no `TimePlot` arm** — the reference
   emitted zero channels, so no graph-internal value was observable through it.
2. `GraphBrain::eval_node_output` had **no `ClampFloat` arm** — every clamped
   graph evaluated to Null/0 and silently zeroed all downstream values (the VM
   always had both; note eval.rs *does* guard Div/Mod by 1e-12, the VM does not).

**Divergent (reference vs shipping VM), 3 classes:**
- **Div/Mod-by-zero policy.** Reference guards `|b| < 1e-12 -> 0.0`; the VM
  computes IEEE inf/nan (visible as ±1e30 after the battery's own clamp).
  `B.div_p0`/`B.div_n0`/`B.div_00`, `B.mod_zero`.
- **ConditionalSet unwired-false HOLD rule.** On the false arm the reference
  returns zero/false; the VM holds the previous tick's value (the game
  "Memory" behaviour per the `lower.rs` comment). `B.hold_f` ref 0 vs VM 7,
  `B.hold_b` 0 vs 1, `B.hold_v_*` 0 vs 1/2/3 — diverging exactly on the ticks
  the false arm is taken.
- **Initial latch value.** Reference starts `Null`; VM starts `Bool(false)`
  (deliberate — `runtime_brain.rs`). Trace Pass 1 tick 0, var `B_b`:
  Null vs Bool(false). Coerced reads mask it (both read as 0.0).

**Agreed — previously unproven, now measured identical in both:**
- unwired `AddFloats` input reads 0 (`B.null_add` = 3).
- `Modulo(-7, 3)` = -1 (sign policy shared).
- `CompareFloats` (5,5) = 1,0,0,1,1 for indices 0..4; (3<5) = 1.
- `ClampFloat` hi/lo/pass-through = 1/-1/0.5.
- `Power(2,-1)` = 0.5; `Not(Not(true))` = 1.
- all 16 `Operation` indices resolve (`op_0`=7 abs, `op_10`=2.645751 sqrt 7,
  `op_11`=1 sign, `op_12`=1.945910 ln 7, `op_14`=1096.633 e^7, ...).
- vector construct/split round trip = 1.5/2.5/3.5.
- same-tick store order agrees (`B.latch_a`=12, `B.latch_b`=11 at tick 12) —
  the read/write ordering inside a tick is identical in both interpreters.

Still unproven (deliberately not in the battery): `Operation` with an EMPTY
modifier (both interpreters panic by design, §14 finding 3) and `Lerp`/`Min`/`Max`.

```
python scripts/gen_interpreter_battery.py
cargo test --test interpreter_semantics_battery -- --nocapture
BATTERY_STRICT=1 cargo test --test interpreter_semantics_battery -- --nocapture
```

## 16. Phase A — reference↔VM closure LANDED (2026-09-12)

The three §15 divergence classes are closed and the battery is fully clean:

| Fix | Where | Result |
|---|---|---|
| Div/Mod `1e-12` guards removed → IEEE `a/b`, `a%b` | `src/graph/eval.rs` | `B.div_*`/`B.mod_zero` divergences gone |
| ConditionalSet unwired-false **previous-tick HOLD** (per-port latch) | `src/graph/eval.rs` (`latch`, `hold_value`) | `B.hold_*` divergences gone |
| `init_vars` switch (`VarInit::Null`/`BoolFalse`), default `BoolFalse` | `src/graph/eval.rs` | `B_b` tick-0 trace divergence gone |
| Empty `Operation` modifier tolerated (0.0 + approximation record) | `src/graph/eval.rs`, `dropdowns::try_from_modifier` | `Adam`/`nqvxf22` no longer panic the reference |
| NaN-aware trace comparison (`VmValue::same_value`, `nan_eq`) | `src/graph_vm/values.rs`, `trace.rs` | `Adam`'s `Vector(NaN,NaN,NaN)` no longer a false divergence |

`cargo test --lib` = **164 passed / 0 failed / 2 ignored**;
battery = `PASS: reference == O0 == O1 on every channel and trace`
(46 channels, 4 tick-varying);
certification (`TENNIS_CERT_MAX=14`) = **12/14 clean** —
`titanium54, aia3, aia, Adam, ignore_ball31, sim_titanium31, cross_court,
open_court, alternator, graphc_serve_latch, graphc_rival, nqvxf22`.
Only `bat` (aim, o0≠o1) and `Pixel_Heart` (`RandomFloat`) remain.

`Pixel_Heart` root cause (was "unexplained", §14 finding 4): its controller
`Float1` (shot type) is `Operation mod='1'` (Round) of **`RandomFloat`**. The
reference deliberately has no RNG stream (0.0); the VM uses a persistent
SplitMix64 placeholder. Both are placeholders for the game's seeded
`UnityEngine.Random` (RNG-from-world is explicitly shelved), so this is a
*world-RNG* divergence, not an interpreter bug.

## 17. Phase B — game adjudication of the policy questions (2026-09-12)

`scripts/gen_interpreter_battery.py` now also emits
`data/interpreter_probes/semantics_battery.channels.json` (per channel: `kind`,
`policy`+`question`, or `golden`). `scripts/read_battery_from_game.py` parses a
game TimePlot export (reusing `modhost/parse_timeplots.py`'s locale-comma rule)
and prints the game's value + a verdict per channel.

Run (one launch, v0.14 natural-traced probe `Aialanders-paritymod-reset.exe`):

```
copy data/interpreter_probes/semantics_battery.txt "<Saves>\Tennis\"
set MODHOST_PARITY_EXE=...\paritymod\Aialanders-paritymod-reset.exe
python modhost/modctl.py launch --home semantics_battery --away aia3 --seed 1 --points 1
python modhost/modctl.py wait --timeout 1200 ; python modhost/modctl.py quit
python scripts/read_battery_from_game.py "<Saves>\Tennis\Timeplots"
```

Result (round 1, 46-ch): the game ran the battery and exported all `B.*`
channels. Round 2 (`timeplot_2026-09-12_04-00-01`, 50-ch, seed 2, **1060
samples**) added disambiguation channels and **superseded an initial misread**
(a 16-sample window hid the steady state). Corrected game truth:

| question | game value | verdict |
|---|---|---|
| `1/0`, `-1/0`, `0/0` | `1e30`, `-1e30`, `nan` | **IEEE inf/-inf/nan** — VM/reference right; the old reference guard was a bug |
| `5 % 0` | `nan` | **IEEE NaN** |
| `ConditionalSetFloat` unwired false | holds `7` across a **long false stretch** (`sel_wired`==22 constant) | **HOLD (previous tick) CONFIRMED** — A2 stands |
| `ConditionalSetVector3` unwired false | `holdv_mag` holds `3.7417` across the same stretch | **HOLD CONFIRMED** (via `Magnitude`) |
| `ConditionalSetVector3` read via `Vector3Split` | `hold_v_*` == `0` on the false ticks | ⚠ **consumer-path anomaly** (below) |
| `ConditionalSetBool` unwired false | `0` | Bool not float-observable; consistent with hold-false |
| unwired `AddFloats` input | `3` | unwired input = 0 confirmed |
| all 16 `Operation` indices | abs 7, sqrt 2.645751, sign 1, ln 1.945910, e^ 1096.633, 10^ 1e7, asin/acos nan, … | **all match the VM/reference exactly** |
| `ClampFloat` / vector round-trip / `Power(2,-1)` | 1/-1/0.5, 1.5/2.5/3.5, 0.5 | match |
| same-tick var read (`B.latch_a` vs `B.latch_b`) | game: **identical** (`B_b == B_n`); sim: `latch_b == latch_a - 1` | ⚠ game exposes **same-tick store visibility**; the sim lags one tick |
| Bool into a Float arithmetic input | `AddFloats(bool,0)`==0 **even when the bool is true**; `MultiplyFloats(bool,1)`==1 | ⚠ game appears to **drop the Bool→Float wire** (input takes the op identity: Add 0 / Mul 1) → **no coercion**; `as_float(Bool)`=1.0 is suspect |

**The ⚠ vector row is a consumer-path artifact, not "the game zeroes
vectors".** `holdv_mag` (via `Magnitude`) holds the previous vector, while
`hold_v_*` (via `Vector3Split`) reads zero for the *same* node — and `B.vec_*`
proves `Vector3Split` is fine on a plain `ConstructVector3`. So the game's
evaluation is **not globally memoized per tick**: a node feeding several
consumers (here the `TimePlot` chains) is re-evaluated per consumer, and a
self-latching conditional's later consumers see the value written earlier in the
tick. The VM's global per-tick cache + double-buffered latch matches the *first*
consumer (the dominant path for real bots) and therefore the `Magnitude`
reading — so **A2 is right for parity**; the `Vector3Split`-after-`Magnitude`
ordering is a genuine modelling gap, worth revisiting only if a real bot depends
on it (HANDOFF §17 finding 1).

The two remaining ⚠ rows (same-tick var visibility, bool→float wire) are
game-sourced open items; the sim was NOT changed on their account.

### 17b. `bat` O0≠O1 — root cause FOUND and FIXED: `RandomF` was CSE'd (2026-09-12)

`tests/pass_bisect.rs` (+ `PassManager::o1_prefix`) localised the flip to the CSE
pass. `tests/cse_ir_diff.rs` dumped bat's IR before/after CSE and showed the bug
outright: CSE had merged the three `RandomF` instructions, turning
`ConstructVec(RandomF, RandomF, RandomF)` into `ConstructVec(r, r, r)` (the aim's
components became identical) and dropping two RNG draws.

Root cause: `OpCode::effect()` listed **`RandomF` as `Pure`**. It is not — each
evaluation draws a new value from the per-tick stream, so two textual `RandomF`s
are different values. Fixed by classifying `RandomF` as `OpEffect::Write` (it
advances VM RNG state), which keeps it out of CSE and const-fold. Regression test:
`cse::tests::does_not_cse_random_f`.

Result: `PASS_BISECT=bat` now reports **`all prefixes == O0`**; lib **168/0**;
battery PASS. bat's remaining certification divergence is now the *same* single
cause as `Pixel_Heart` — the reference has no RNG stream (`RandomFloat → 0.0`), a
world-RNG placeholder (RNG-from-world is shelved) — **not** an optimizer bug.

```
PASS_BISECT=bat cargo test --test pass_bisect -- --nocapture
CSE_DIFF=bat  cargo test --test cse_ir_diff  -- --nocapture
```




### 17c. Serve-clock refusal — 5 s, then the serve is awarded to the opponent (2026-09-12)

User-observed game rule: after the server idles, a **5 s countdown**
(`<ServeCountdownDisplay>`) runs; when it hits 0 the **opponent is awarded the
serve** (no point is scored). The sim previously had `SERVE_CLOCK = 15.0`
(unmeasured, from the first commit) and on expiry just **re-armed the same
server** — the rule was missing.

Implemented (`aia_comp-sim`):
- `params::SERVE_CLOCK = 5.0` (was 15.0), documented as the observed countdown.
- `Score::award_serve(side)` + `Score.serve_override`: the awarded serve lasts
  the rest of the **current game**; `award_point` clears the override so rotation
  resumes at the next game. `Score.serve_forfeits` counts them.
- `TennisWorld.serve_clock_t` spans ServeSetup + Toss and is **not** reset by a
  recatch re-toss; `on_serve_deadline()` awards the serve to the opponent and
  re-runs `setup_serve()`. The old `phase_t >= SERVE_CLOCK → setup_serve` re-arm
  is gone.
- Tests: `score::tests::serve_clock_forfeit_awards_the_serve_to_the_opponent`,
  `world::serve_clock_tests::{serve_clock_expiry_awards_the_serve_to_the_opponent,
  serve_clock_survives_a_recatch}`. Lib **167 passed / 0 failed**; battery PASS;
  cert still 12/14.

The sim's serve is automatic (auto-toss + `TennisAutoSwing` auto-strike), so the
clock does not fire in normal play — it fires only if the serve phase stalls,
which is the game's refusal case.

**Still to pin from the game** (the probe already lists these in
`metadata_trace_manager_fields`, `paritymod-src/metadata_probe.h`):
`serveDeadlineTick` (is the window 5 s, or a longer deadline with a 5 s
display?), `<ServeCountdownDisplay>`, `lastServeCountdownShown`, and whether the
awarded serve lasts the game or the whole set. Reader added:
`modhost/read_serve_deadline.py <probe-startup.jsonl>` (works once the capture
contains `manager_fields` rows).

## 18. v0.15f re-target — mod VERIFIED end-to-end (2026-09-12)


Goal: mod the latest free release the same way v0.14 was modded, then start
v0.15 parity. **Status: the probe runs a full match on v0.15f** — seed, hook,
snapshot, ticks, point resolved, quit, and a TimePlot auto-export.

- Host: `modhost/v0.15f/` = copy of the pristine `AIA_tennis/v0.015f/` install
  with the untouched launcher preserved as `Aialanders-original.exe`
  (667648 B). `GameAssembly.dll` is 128212480 B (v0.14: 128072704 B).
- Build: `modhost/build_paritymod_v015.ps1` → `paritymod/Aialanders-paritymod-reset-v015.exe`
  (same sources/flags as the v0.14 reset build; distinct output name).
- Driven by `MODHOST_GAMEDIR=modhost/v0.15f` +
  `MODHOST_PARITY_EXE=…paritymod-reset-v015.exe` (no driver change).

The metadata-driven design paid off: **3167 methods, 1943 fields, 1786 class
catalog rows, 109 managed classes all resolved by name**, and both graphs
loaded Sim-ready with byte-identical structure to v0.14 (titanium54
5383 nodes / 7932 conns; aia3 54/46). Exactly **two** version-coupled
assumptions had to be relaxed:

1. **`Stamina.OnSimulationTick` preamble was matched byte-for-byte.**
   The first 16 bytes contain `cmp byte ptr [rip+disp32], imm8` where the
   disp32 is a *static-field address*: v0.14 `…,14,215,122,6,0` vs v0.15f
   `…,94,228,124,6,0`. The instruction is identical; only the address moved.
   The matcher now compares the **instruction shape** (bytes 0..7 and 12..15)
   and skips the disp32, and adds a version-tolerant sanity check that the
   re-encoded guard points *inside* GameAssembly. (The trampoline already
   re-encoded from the OBSERVED bytes, so pinning the address was pure
   build-coupling.) Failure signature before the fix:
   `natural_hook_status.reason = "preamble_mismatch"` → `installed:false` →
   `natural_trace_status = stamina_hook_install_failed`.

2. **Three named ball fields no longer exist in v0.15f.** The field-group
   reader treats a missing named field as a read error → `ok=0` →
   `state_read_ok:false` → terminal `seed_or_initial_snapshot_failed`.
   Diffing the v0.14 vs v0.15f initial snapshots showed the field *name sets*
   identical (31 ball fields) but three values `null` in v0.15f:

   | field | v0.14 | v0.15f |
   |---|---|---|
   | `cachedPredictedBounceTime` | `System.Single` @0x140 | **removed** |
   | `cachedPredictedSecondBounceTime` | `System.Single` @0x144 | **removed** |
   | `predictedBounceAge` | `System.Single` @0x14c | **removed** |
   | `hasCachedPredictedBounce` | @0x148 | @0x140 (moved) |

   These are exactly the ball bounce-time *cache* fields — the v0.15 changelog's
   "fixed an issue with ball time to ground being cached". `metadata_probe.h`
   now has an explicit `metadata_trace_field_optional()` allowlist: an absent
   optional field emits `null` without failing the group, so a typo or a real
   read error still fails loudly.

Verdict rows from the working run: `natural_hook_status.installed:true`
(`installed_dynamic_method_entry`), `parity_state.json` reaching
`tick 422, callbacks 3239, points_done 1, point_winners [0], done:true,
success:true`, and `quit` flushing a **748471-byte** TimePlot
(the failed runs wrote 4304-byte stubs).

### v0.15 changelog → parity impact (authoritative triage map)

| v0.15 change | Parity impact |
|---|---|
| **Ball position interpolated between frames** so fast balls can still be hit | **Real behaviour change.** Contact/swing-range detection must become a swept/continuous test against the interpolated position. Sim surface: `src/tennis/ball.rs`, `src/tennis/world.rs` (`Ball In Swing Range`, hit resolution). Expect this to be the main v0.15↔v0.14 divergence. |
| **Ball time-to-ground caching fixed** | **Real behaviour change.** Matches the removal of the three cache fields above. The public `Ball Time To Ground` getter / `TryPredictNthLandingFrom` path is the parity surface; the sim's `src/predict/mod.rs` cache assumption is exactly what v0.15 changed — and the v0.14 landing residual (8/10, curve-on-bounce) may be a symptom of the same caching. |
| Space opens a searchable node menu | QoL/UI only — **no parity impact** |

### Next (Phase F4+)

```
python modhost/capture_fixtures.py --home titanium54 --away aia3 --seed 20260907
python modhost/emit_rust_fixtures.py <run>
cd aia_comp-sim && cargo test --test tennis_v014_solver_parity -- --nocapture
```
then triage each drift row against the table above before touching the sim.



## 20. Phase C — the per-tick channel diff has now RUN (2026-09-12)

`scripts/channel_diff.py` was rewritten (clean GAME/SIM/Δ table, repeatable
`--env K=V`, `--json`) and run for the first time. It compares the **sim** graph's
TimePlot channels (`tennis_tournament --trace` + `AIA_TRACE_CHANNELS=1` → `hch`)
against the **game's** native export of the same graph — no same-seed replay
needed, so it isolates **model** drift (how often a sensor fires) from outcome
drift.

Run (titanium54 vs aia3, seed 7, points 4; game capture
`captures/timeplots-20260910/timeplot_2026-09-10_15-27-03.json`):

| channel | game (frac>0.5 / mean) | sim **before** | sim **after** (hold default) |
|---|---|---|---|
| `v44_ChargePct` | 0.292 / 0.279 | **0.000 / 0.000** | **0.249 / 0.254** |
| `v44_SwingHeld` | 0.539 | 0.031 | 0.089 |
| `v44_RacketDist` | 0.970 / 9.76 | 0.994 / 12.13 | 0.976 / 13.51 |
| `v44_Chase` | 0.332 | 0.700 | 0.592 |
| `v44_Mode` | 0.332 | 0.700 | 0.592 |
| `v44_Incoming` | 0.150 | 0.407 | 0.240 |
| `v44_Bounced` | 0.096 | 0.253 | 0.304 |
| `v44_TIntercept` | 0.772 / 267.8 | 0.701 / 512.9 | 0.691 / 294.2 |
| `v44_AimX` | 0.966 / 3.58 | 1.000 / 1.35 | 1.000 / 3.78 |
| `v44_BallX` | 0.329 / 0.24 | 0.545 / −2.59 | 0.435 / −0.76 |

Conclusions:

1. **`ChargePct` was a total model error** — the game charges (mean 0.279) and the
   default sim was pegged at **0.000**. The already-implemented `swing_hold_gate`
   closes it (0.249). ⇒ the gate is now the **default**
   (`AIA_SWING_MODEL=legacy` / `off` restores the old default so the recorded
   sweep verdicts stay reproducible). AimX / TIntercept / RacketDist means also
   line up much better under the gate.
2. `SwingHeld` is still far off (sim 0.089 vs game 0.539) — the sim's hold is much
   shorter than the game's 0.6-1.2 s approach holds. **Open.**
3. `Chase`/`Mode` overshoot (sim 0.59-0.70 vs 0.33) — the sim's movement/decision
   channels fire ~2× as often. **Open** (rally/movement model).
4. `Bounced`/`Incoming` overshoot and `BallX` mean sign-flips — the sim spends
   more of the point in the ball-approach phase and more on the far side. **Open.**

This converts "rally aim semantics" from a guess into ranked, measurable gaps.
Sim suite after the default flip: lib **168/0**, battery PASS, cert 12/14.

### 20b. Scale-up: 78-match distribution over the game's own result set (2026-09-12)

`run_sim_seed_sweep.py` now also accepts the **tournament** row format
(`game_tournament_results*.jsonl`: a fresh launch per seed, so self-attributed;
`first_server` is not exposed, so the hold metric is skipped). Replaying all 78
game matches (`points=8`) and scoring with `score_distribution.py`:

```
POOLED  game: n=77  home win 58%  mean points 5.0-3.0
        sim : n=77  home win 55%  mean points 4.1-3.6
        TV distance = 0.078  (0 = identical outcome distribution)
```

So across the 78-match set the sim's **aggregate outcome distribution is within
0.078 TV of the game's** (home-win 55% vs 58%). Exact per-match sequence parity
stays low (RNG draw sites + first-server still unpinned) — which is exactly why
the plan demotes the exact metric and makes the distribution the headline. The
sim's point margins are narrower (4.1-3.6 vs 5.0-3.0): the sim plays more
balanced matches than the game. Open.

Caveat: tournament rows do not expose the match's *first* server, so those
replays use the sim's own derived server. A first-server-matched sweep
(`restart_sweep.jsonl`) remains the exact-parity path.


## 21. Phase F4 — v0.15f fixture re-mine (partial, 2026-09-12)

New tooling: `build_paritymod_event_v015.ps1` builds the v0.15f **event** probe
(`Aialanders-paritymod-event-v015.exe`, distinct name so the v0.14 event probe
is untouched); `emit_rust_fixtures.py` gained `--out=<dir>` so a new version's
corpus lands in `tests/fixtures/tennis-v015/` without clobbering the pinned
v0.14 fixtures.

**Blocker found and fixed.** The event fixture failed on v0.15f with
`event_fixture_failure reason=snapshot_fields_unavailable`. Root cause: the
event snapshot helpers (`metadata_event_capture_group`,
`metadata_event_group_matches`, `metadata_event_restore_group` + the
`V014ServeFieldSpec` variants) failed the **whole** snapshot on any unreadable
field, while the trace writer already honoured `metadata_trace_field_optional()`.
They now tolerate the version-optional fields (and `required==0` serve fields),
so a build that removed a field no longer aborts the capture. No-op for v0.14
(all fields present).

**Second blocker found and fixed.** The shot fixture's **own** snapshot
(`metadata_event_shot_capture_snapshot`) still failed
(`event_fixture_failure component=shot`). Cause: `v014_shot_ball_fields` marked
the three v0.15-removed ball cache fields (`cachedPredictedBounceTime`,
`cachedPredictedSecondBounceTime`, `predictedBounceAge`) `required:1`. Flipped to
`0` (the shot restore/match helpers were already `required`-aware).

**After both fixes** (v0.15f, titanium54 vs aia3 seed 20260907):

| fixture | v0.15f |
|---|---|
| `shot_fixture` | **complete 210/210 cases, 840/840 calls** |
| `curve_fixture` | **complete 29/29** |
| `serve_direct` | 26 rows |
| `simulate` | 13 rows |
| `nth_landing` | 10 rows |
| `getter_items` | pending (`graph_not_ready` at the startup fixture) |

Full corpus emitted to `tests/fixtures/tennis-v015/`.

**Result: the v0.15 shot solver is IDENTICAL to v0.14.**
`cargo test --test tennis_v014_solver_parity`:

```
v0.14 shot solver parity: 200/210 within 0.25 m/s, max_err=23.6888 m/s
v0.15 shot solver parity: 200/210 within 0.25 m/s, max_err=23.6888 m/s
```

Same ratio, same max error, same worst row (the documented `fallback`
world-state case) → v0.15's solver-adjacent changes did **not** move
`ComputeShotVelocity`. So the sim's solver fixtures need **no** v0.15 re-pin;
v0.15's real behaviour changes are confined to ball-position interpolation +
the time-to-ground cache (§18/§20 triage map).

**Remaining F4:** `getter_items` (the capture must run after the graph loads),
and the non-solver v0.15 behaviours (frame interpolation, time-to-ground cache)
— world-model work, not fixture work.


## 22. Phase C — first-server-matched sweep (2026-09-12)

`restart_sweep.py --seeds 1-16 --home titanium54 --away aia3 --points 8 --keep-alive`
(v0.14 reset probe; one launch, restarted per seed) produced 16
**first-server-matched** game matches (~4-5 min/seed). The chained
`run_sim_seed_sweep.py` + `score_distribution.py` replayed and scored them:

```
exact (this run): 11/17 = 64.7%          <- WITH the game's own first server
pairing titanium54 vs aia3: n=20
  game: home 95%  hold 53%  pts 5.6-1.0
  sim : home 75%  hold 53%  pts 3.8-2.1
  TV distance 0.20     exact 13/20 (seq 3)
```

Three headline facts:

1. **Exact per-match parity jumps to ~65% once the first server matches** (the
   pooled tournament set without server matching was 32.1%). So the RNG /
   first-server mismatch was the **dominant** source of exact-parity error, not
   the world model.
2. **Server-hold rate matches exactly (53% vs 53%)** — the serve/return model is
   faithful on this pairing.
3. The residual is **strength**: the game's titanium54 beats aia3 **5.6-1.0**
   (home 95%), the sim only **3.8-2.1** (home 75%) — the sim **under-rates the
   stronger bot**. That is the rally-quality cliff measured in §20
   (`SwingHeld` 0.09 vs 0.54, `Chase`/`Mode` ~2× overshoot, `Bounced`
   overshoot), and it is the next world-model target.

Data: `modhost/restart_sweep.jsonl` (game) + `data/tennis/sim_seed_sweep.jsonl`
(sim), both committed. Reproduce the sim side any time with
`python scripts/score_distribution.py`.

## 23. Fair tick-by-tick comparison → the serve-setup phase is the first gap (2026-09-12)

`scripts/tick_diff.py` replays the SAME (seed, first server) the game's restart
sweep used and diffs the graph's `v44_*` TimePlot channels sample-by-sample
against the sim's `hch` trace — the fair comparison the distributional diff
cannot give. First run (`--seed 1 --srv 1`, game
`timeplot_2026-09-12_05-13-22_..._srv1_seed1.json`):

```
game_n=1923   sim_n=2723   44 shared channels   mean|diff| 10.18
first-diff: tick 0 for positions/ball, tick 39 for AimX/AimZ/AimZAdj
```

Side-by-side samples (game/sim) expose **why tick 0 differs** — it is not a
phase shift (best offset = 0), it is the **serve-setup window**:

| t | `v44_BallX` g/s | `v44_OppX` g/s | `v44_SelfX` g/s |
|---|---|---|---|
| 0 | 0.00 / 14.92 | 8.23 / 14.92 | −14.84 / −14.00 |
| 24 | −14.55 / 14.92 | 8.14 / 14.92 | −14.84 / −14.00 |
| 100 | −4.17 / −3.05 | 4.65 / 10.03 | ... |

Findings:

1. **The game runs the graph before the ball is placed**: its ball reads `0.00`
   for ~23 ticks, and the players sit at *spawn* positions (home −14.84, away
   8.23) — **not** at the serve/receive stances. The sim teleports the server to
   its serve stance (14.917) with the ball in hand from tick 0.
   ⇒ the fair comparison needs the sim to start from the game's captured initial
   state (`trace_initial` / `serve_fixture_context`), not its own stance.
2. **The serve setup is ~4× longer in the sim**: the game's ball leaves the hand
   by ~tick 24 (0.5 s); the sim's at ~tick 100 (2 s) — the sim's `0.6 s` settle
   + toss vs the game's actual. Matches the §20/§22 picture (the sim plays longer
   points).
3. **Serve-phase aim already agrees**: `v44_AimX`/`AimZ`/`AimZAdj` match for the
   first ~39 ticks, then drift — so the stance/serve branch is right; the rally
   aim is where it leaves.

### 23b. The shot-deviation model has no timing term (the "bell curve" question)

The sim's only aim perturbation is **fatigue scatter**
(`world.rs::do_strike`): `unity.range(-1,1) * FATIGUE_LATERAL * singles_width *
active` laterally, `* FATIGUE_DEPTH * court_length * active` in depth. That is a
**zero-mean uniform** draw; there is no bell curve, and **no early/late contact
term at all** — so "is the bell centred on the smallest-deviation point?" cannot
be answered from the sim side.

The game's timing/accuracy deviation lives in its **contact-grading methods**
(`GradeServeContact`, `GradeRallyContact`, `EvaluateHitAccuracy` — in
`metadata_probe.h`), which we have **not captured**: the shot fixture feeds
`ComputeShotVelocity(from, aim, power, shot_type, is_serve)` and has **no contact
timing input**. So the next fixture is a **contact-grading capture**: vary the
contact-timing offset, read the graded error, and fit the deviation-vs-timing
curve (then replace/augment the fatigue-only scatter with it).

**Priority after §23:** (1) make the sim start from the game's captured initial
state so ticks align from the first meaningful sample; (2) shorten/µ-match the
serve setup; (3) build the contact-grading fixture.

## 24. The cold-launch boot match is not a fair state (fixed in the mod driver)

On a **cold launch** the game spawns the ball at court centre and awards a
spurious point to the right side before the serve setup — so the *boot match* is
not a fair/correct state. This is exactly what the §23 tick diff saw (`v44_BallX`
`0.00` for the first ~23 samples) and why `restart_sweep.py` restarts before each
seed (its `restart_epoch` guard exists for the same reason).

The other drivers did **not** restart, so they recorded the boot dud:
`game_tournament.py` (the 78-match set) called `modctl launch` then waited for
`done` — capturing the ball-at-centre match. That is a real contributor to the
low exact parity on that set.

Fixed (`AIA_tennis`):

- `modctl.py` gains a **`settle`** subcommand: after a cold launch it waits for
  the mod to come up, writes `{"cmd":"restart_match",…}`, and waits for a fresh
  `restart_epoch`. Docstring records *why*.
- `game_tournament.py::play_match` now calls `modctl.settle` right after launch,
  so the match it records is the clean one.
- `restart_sweep.py` already did this; `capture_fixtures.py` is unaffected (the
  event fixture runs at startup, not through a match).

**Sim side:** no change needed — the sim starts with the ball at the server's
hand (a clean state), so it was already "correct" here; the comparison tooling
(`scripts/tick_diff.py`) should simply use a **restarted** match's timeplot, not
the boot export.

## 25. Fair tick-by-tick alignment: sim DOES replicate the serve stance

Question asked: after settling, can the sim replicate how the game behaves? The
`tick_diff.py` tool now answers it fairly.

**Anchor (new default `--anchor strike`).** The game holds the ball at the
**origin** while the match is *not started*, then **teleports it to the server's
hand**, the server settles, and at the **strike** the ball launches. The sim
holds the ball at the hand from tick 0. So the fair "same serve stance" instant
is the ball **release/strike**, not raw tick 0:

- release detector uses `REL_EPS = 0.3` (a launched ball = >0.3 m/tick = 15 m/s)
  so the placement settle / in-hand hover (<0.15 m/tick) does **not** false-fire;
- `mad_at` slices **both** sides at their own onset (`g[ga:]`, `s[sa:]`), rather
  than shifting one side by `ga - sa`.

**Sentinel handling.** The game emits `999.0` for "time to intercept =
unreachable". `_absdiff`/`_diverge` treat it categorically (equal sentinels
match; sentinel vs real = 1.0 of "wrong state"), so it no longer swamps the mean.

Settled match, `titanium54 vs aia3`, seed 7, srv home (`timeplot_..12-40-45`):

```
game  game_n=191 sim_n=352 | 44 shared channels | anchor v44_BallX
      (game onset 86, sim onset 69) | aligned 105 | mean|diff| 1.4818
serve phase (v44_BallX): game placed@22 released@86 setup=64 | sim placed@0 released@69 setup=69
```

**Findings**

1. **Serve setup matches within 5 ticks** (game 64, sim 69). The earlier "1 vs
   69" scare was the placement-settle false-firing as a release.
2. **Serve stance matches**: `SelfX` -14.01/-14.92, `OppMeetX` 4.01/3.31 at the
   strike.
3. **The first ~25-30 ticks (~0.5-0.6 s) of the rally track closely**:
   `RacketDist`/`DNow` 1.36/0.14 -> 5.70/5.44 (t10) -> 13.12/14.27 (t25);
   `SelfX` -12.93/-13.30 (t10).
4. **Beyond ~t30 the rally diverges** and by t100 is fully uncorrelated. This is
   expected: the rally is chaotic multi-agent dynamics, so *any* residual (a
   one-tick decision difference, an RNG draw, a contact micro-timing) amplifies.
   Tick-exact rally parity therefore requires matching every decision and random
   draw, not just the world model.

**Net answer:** the sim replicates the game's serve stance and the early rally
(within ~0.5 s the named channels agree to a few cm / tenths); it cannot follow a
full rally tick-for-tick, and that is a property of the system, not a bug.

**Next (still to do):** (a) re-mine the 78-match tournament now that `settle`
discards the boot dud; (b) build the **contact-grading fixture**
(`GradeRallyContact` / `EvaluateHitAccuracy`, still uncaptured) to measure the
early/late hit-deviation distribution - the "is the bell-curve centre at the
minimum-deviation point?" question; the sim currently has **no timing-deviation
term** (only zero-mean uniform fatigue scatter), so this fixture is what pins it.


### 25b. Aggregate over the matched sweep (15 seeds)

`scripts/tick_diff_sweep.py` runs the fair diff for every seed the restart sweep
left a named timeplot for (auto-detecting the first server from the file name):

```
aggregate over 15 matched seeds: mean 2.578  median 2.558  min 2.263  max 2.968
serve setup: game 64-67 vs sim 69 (every seed)
onset:       game 86/89 vs sim 69   (seed 15 sim 164 - a sim re-toss, see below)
```

Per-channel mean|diff| across the 15 seeds (worst first):

| channel | mean | comment |
|---|---|---|
| `v44_OppMeetX` | 14.84 | opponent meet/intercept X - WORST |
| `v44_BallX` | 11.08 | ball trajectory divergence |
| `v44_RacketDist` / `v44_DNow` | 8.92 | racket-to-ball distance |
| `v44_AimReq` | 7.87 | requested aim |
| `v49_AimZAdj` / `v44_AimZ` | 5.18 | aim Z (adjustment) |
| `v44_SelfX` / `v44_RacketX` | 5.05 | server/racket X |
| `v44_AimX` | 4.38 | aim X |

So the opponent's planned meet point diverges more than the ball itself - the
intercept / world-model path is the weakest subsystem, ahead of ball trajectory.
That is the next thing to attack for rally fidelity.

`--json` was added to `tick_diff.py` to feed this aggregation.

Seed 15 sim onset 164 (vs 69 for every other seed): the sim's first serve took an
extra ~95 ticks, i.e. a re-toss / recatch on the first serve (`step_toss`
recatches when the toss is not struck). The input seed only changes RNG, not the
toss gate, so a first serve that fails to auto-strike is a real (if rare) sim
behaviour gap worth a look.


## 26. Turn rule (double-hit guard) + the non-headless viewer

### 26a. Turn rule

User-confirmed: tennis is TURN-BASED - once a side strikes, only the OPPONENT may
strike next; a side can never hit the ball it just hit, not even after a bounce.
The sim had half of this (`strike_lock`, cleared on bounce = no re-strike while
my shot is in flight) but allowed the striker to hit its own shot again after the
bounce. Fixed in `on_swing_release`: the guard is now
`strike_lock == Some(side) || hit_by == Some(side)` - `hit_by` (last striker,
survives the bounce) IS the turn marker. Tests: `turn_rule_tests`
(`a_side_cannot_strike_twice_in_a_row`, `the_turn_flips_when_the_opponent_strikes`).

### 26b. `tennis_viewer` - the non-headless tennis simulator

`cargo run --bin tennis_viewer -- --home titanium54 [--away aia3] [--seed 7]`
(2D Bevy window, 1280x760, real-time fixed-dt; empty side = stock bot).
Fixes that made it usable: B0001 query conflict -> `ParamSet`; camera scale
20.0 -> 1.0 (was ~39 px of court); everything player/ball-shaped drawn as
CIRCLES; scoreboard rebuilt (outline body + lit inner disc per set, spread
between the score labels - the old sync rescaled every ring sub-spawn and
clumped the board into one blob); reach rings anchored on the RACKET CENTER
(what the model measures), not the player; blue backdrop (court lighter than
apron for contrast); `PERFECT_RADIUS` 1.05 -> 1.0 (aia3 uses exactly 1.0).
The top-down view drops the height axis, so the ball now renders with a ground
SHADOW (true court XZ) plus height lift/scale, and the phase line shows
`ball y` in metres - lobs/toss/net-tape flights read as 3D.
Note: the running viewer locks `tennis_viewer.exe` - stop it before cargo test.

### 26c. Diagnostics test race (fixed)

`graph_vm::diagnostics` tests mutate a process-global registry from parallel
test threads; under load another test's `clear()` landed between
`record_unimplemented` and `assert_sound` and swallowed the expected panic
(flaky `assert_sound_panics_on_unimplemented`). All four tests now serialize on
a test mutex.

### 26d. Is the sim fully 3D? (user question)

Yes where it matters; the viewer is a projection of it, not the model:
- Ball flight is full 3D: `ball.vel.y -= GRAVITY * pace^2 * h` (adaptive
  substeps <= 0.2/|v|), floor bounce at `BOUNCE_FLOOR_Y`, net plane crossing
  resolved at the interpolated crossing point with height
  `at_net.y < NET_TAPE_HEIGHT` (tape/rebound at height - game
  `ApplyNetRebound`), curve accel with decay, speed cap 46*pace.
- Shots are solved in 3D per the game's own `ComputeShotVelocity`
  (`vy = (target_y - from.y)/t + 0.5*g*t`, target on the bounce floor plane,
  per-family speed/lift/flight-time tables).
- Contact is a 3D sphere test against the racket center at `STRIKE_HEIGHT`
  1.25 m (v0.15: swept segment test).
- Parity evidence: shot solver 200/210 within 0.25 m/s on both v0.14 and v0.15
  live-captured fixtures; curve 29/29; serve stance + early rally track in the
  tick diff.
- Deliberate simplifications (honest list): players are ground-plane 2D (the
  game's movers are too) with a constant racket height instead of an animated
  swing; aim scatter is uniform (no timing-deviation term yet).

### 26e. Stamina modelled (measured, was hardcoded 1.0)

The sim reported `Self/Opponent Stamina Pct` as a constant 1.0 and the viewer
stamina bar was decoration. Game truth (probe hooks `Stamina.OnSimulationTick`
natively; native timeplot channel `v44_Stamina`, seed-10 match, 1928 ticks):

- sprinting: **-0.010/tick** (118 drain ticks = bursts of 3-21 sprint ticks;
  1.00 -> 0.10 across the match),
- walking: **neutral** (flat plateaus of 100+ ticks mid-rally),
- standing: **+0.001/tick** (345 regen ticks, fills during point pauses /
  serve setup),
- **no reset at point boundaries** (continuous meter across the match).

Modelled as `TennisPlayer.stamina` with `STAMINA_SPRINT_DRAIN` /
`STAMINA_REGEN` in params.rs; sensors now report the real meter; the viewer
bar shows it; the graph-replayed `v44_Stamina` TimePlot channel now carries
the sim value (same-name channel as the game for the tick diff).
Open: whether low stamina reduces speed/power in the game (the v0.12
rally/deuce fatigue formulas remain the pinned power/scatter mechanism).

### 26f. Serve charge gate + per-point divergence horizon

Game truth (settled export, v44_ sensors during setup): the bot holds SwingHeld=1
for the WHOLE setup, yet the game manager keeps ChargePct at 0.00 until the toss
is released, then ramps 0 -> 0.79 over ~20 ticks; the strike fires at the TOSS
APEX at charge 0.79 (serve ~33.8 m/s). The sim charged from setup entry (1.0,
36.4 m/s).

Fixes (all measured, not guessed):
- charge accrues only while the ball is NOT held (`!ball_held`) - the game
  manager ignores the swing bit while the ball is in hand;
- toss target = the contact height (`TOSS_APEX_Y` = `TOSS_STRIKE_Y` = 3.45) and
  launch ~10.6 m/s so the rise takes 0.4 s = the measured 20-tick charge ramp;
- `SETUP_SETTLE` 0.88 s so placement -> strike = ~64 ticks (game 64).

Measured after: strike charge 0.76 (game 0.79), serve 33.4 m/s (game ~33.8),
rise 20 ticks (game 20), placement->strike 66 (game 64), struck at the apex.
Whole-match tick diff on seed 10: mean|diff| 6.58 -> 3.26; sim rally ticks
4103 -> 2963 (game 1928).

New tool: `tick_diff.py --per-point` aligns EACH point at its own serve strike
(serve launches detected as slow->fast transitions; the game held ball drifts so
plateau tests fail) with a +-3 tick best-shift search. Result on seed 10:
points 0-2 track ~30 ticks (~0.6 s, the serve flight + first return) then
diverge; later points cannot be compared 1:1 because the sim re-serves where
the game does not, shifting point indices.

Remaining divergence suspects (ranked):
1. rally length (sim ~1.5-3x the game tick count per point; the largest gap);
2. serve aim target inside the box (unmeasured);
3. hitter decision channels (SwingHeld sim 0.09 vs game 0.54 during rallies);
4. sim extra serves/faults where the game holds serve.
