# SESSION HANDOFF — aia_comp-sim tennis parity (2026-09-10)

**Read this + `docs/TENNIS_V014_PARITY_NOTES.md` first.** Everything below is
verified against the repos on disk. Goal framing from the user, verbatim:
**"We don't clone the game 1:1 — if we play any 2 AIs against each other the
results must be the same as in the game."** (Viewer/editor = QOL only.)

## 1. Locations (canonical)

| Thing | Path |
|---|---|
| **Rust sim repo** (git, GitHub `titaniummachine1/aia_comp-sim`) | `C:\gitProjects\aia_comp-sim` (standalone — user moved it out of AIA_tennis deliberately; do NOT move back) |
| Capture tooling + modded game | `C:\gitProjects\AIA_tennis\modhost\` (v0.14 install in `modhost\v0.14\`) |
| Mined fixtures + replay tests | `aia_comp-sim\tests\fixtures\tennis-v014\` |
| Deep game-knowledge doc | `aia_comp-sim\docs\TENNIS_V014_PARITY_NOTES.md` |
| Version/disclaimer doc | `aia_comp-sim\docs\GAME_VERSIONS.md` |
| Bot saves (82) | `%USERPROFILE%\AppData\LocalLow\Unicorn One\AIComp\Saves\Tennis\` |
| Probe build (w64devkit gcc) | `%TEMP%\opencode\w64devkit\bin\gcc.exe`; script `modhost\build_paritymod_event.ps1` (defines `-DV014_METADATA -DV014_EVENT_CAPTURE -DPARITYMOD`, link `-lkernel32 -lgcc`); natural exe backed up as `Aialanders-paritymod-natural.exe` |

## 2. Golden rules (user-enforced)

1. **Never trigger a Bevy/dep rebuild** — crate-only builds, no feature or
   profile flips, no folder renames. Background any long build with a log.
2. **Mode-first VM**: no implicit soccer default. `Lowerer::compile(graph,
   Option<GameSpec>)`; `compile_pure` for unit tests (mode-owned nodes are
   hard errors); simulation requires explicit `GameSpec`
   (`tennis_v014()` = latest default, `tennis_builder()` = v0.012 builder
   order for all on-disk graphs). Mode gate: `src/mode.rs`.
3. Do not redistribute game binaries (user handles the legacy-zip repo
   themselves; I declined that task — don't revisit).
4. Only kill `Aialanders.exe` processes whose path is inside
   `modhost\v0.14\` (`modctl.game_pids_in_gamedir()` does this).

## 3. Current numbers

- lib tests: **154 green** (as of handoff; fatigue WIP is compiling)
- Shot-solver parity vs live 210-case matrix: **200/210 ≤0.25 m/s**
  (excluded 10 = fallback rows; direction comes from live LastHitter state)
- Landing parity vs NthLanding matrix: **8/10** (one ~0.55 m curve residual
  = predictor doesn't zero curve on bounce — open fit)
- **Outcome parity vs game tournament: 71.9% (23/32)** — NOISY: game side
  only recorded post-reset points snapshot; shut-out games can't be
  attributed. To make it exact: probe rebuild must add `games` + per-point
  winners to `parity_state.json`, then rerun matches with 8+ points, then
  rerun `scripts\run_sim_tournament_pairs.py` + `scripts\score_parity.py`.
- Game tournament: 81 recorded, 49 skipped on a file-lock race (resumable —
  `game_tournament.py` skips bots already recorded; delete
  `game_tournament_results.jsonl` rows or just rerun).

## 4. In-flight / half-done (pick up here)

1. **Fatigue WIP (compile OK, tests green, not committed, not finished):**
   `src\tennis\params.rs` has the exact recovered formulas
   (`fatigue_points`, `deuce_fatigue_points`, `rally_fatigue_points`,
   `fatigued_charge`, constants 0.03/0.012/0.025, grace 12/2, floor 0.55).
   `world.rs` applies them at strike: rally_hits counter, aim scatter via
   two Unity RNG draws (lateral then depth; right = cross((0,1,0),attack)
   → (0,0,∓1) on z; attack = ∓x sign), charge multiplied by
   `max(0.55, 1−0.025·active)`.
   **TODO**: wire sensors (`Rally Fatigue` = `rally_fatigue_points`,
   `Deuce Fatigue` = `deuce_fatigue_points(extra)`; currently hardcoded 0.0
   in `tennis/api.rs`), add pinned tests, commit. This is the
   anti-stalemate mechanism the user explicitly wants honored.
2. **Sim tournament rerun** (post node fixes: `RandomFloat`, `CrossProduct`,
   `ConditionalSetString`, `RandomColor`, empty-Operation): background run
   was started then aborted; old JSONL has stale rows — DELETE
   `data\tennis\sim_tournament.jsonl` before rerunning
   `scripts\run_sim_tournament.py titanium54 7 4`.
3. **Game probe channel diff**: probe graph `sim_probe.txt` (built by
   `scripts\build_interpreter_probe.py`) ran in-game twice but produced
   **0 timeplot files** (see #6) — the sim-vs-game semantic diff is still
   blocked on that. Sim-side values are in `probe_sim.json`.
4. **`T_latch_x` stuck at 0.0 in sim** — SetVariable/GetVariable chain
   (`probe_x = probe_x + 1`) doesn't accumulate. Real bug or real semantic
   difference; needs a minimal unit test (hand-built RawGraph → RuntimeBrain
   → settle ticks → var check).
5. **Timeplot export mystery** (v0.14): 0 files ever, despite graceful
   `CloseMainWindow()` shutdowns, `exportResults=true` +
   `timeplotVisible=true` set in settings. Soccer game exports fine (July
   files). Hypotheses: export needs menu-return after match end, or the
   panel opened during play, or flush on the game's OWN exit (our quit =
   `ExitProcess(0)` in paritymod — bypasses Unity teardown).
   **User will manually test at home** (open in-game timeplot panel during a
   match, quit via game menu, check `Saves\Tennis\Timeplots`). UI automation
   toolkit ready: `pip install pyautogui` DONE (PIL 12.3 present); plan =
   screenshot → Read (image) → annotated click plan → pyautogui click.
6. **49 locked-out game matches** — rerun `game_tournament.py` (resumable);
   add a wait-for-process-exit loop before each launch (lock race caused
   `PermissionError` on the exe swap).
7. **Slice crossed-net +0.05 s term** — only slice rows at the crossed-net
   profile sit above the tail; open fit in the solver.
8. **Getter-items capture** failed both runs → sensor label ABI (35/51/15/5
   v0.14 tables) + the `Ball Incoming`-turns-false-on-first-bounce quirk
   (implemented, UNVERIFIED) remain to be pinned from a live capture.

## 5. Big recent discoveries (full list in the notes doc §5)

- **Raw `ComputeShotVelocity` is net-blind for ALL shots** (net-lip fixture,
  aim 0.25 past the net, 210 cases). Net avoidance lives in the
  Rebuild/Steer chain (steer re-predicts with tape-failing policy →
  stretches/lofts). "Flat/topspin eats the net" = steer-chain FAILURE cases.
- **Div/Mod must be IEEE** — the author confirmed n/0=+inf, −n/0=−inf,
  0/0=nan; the v0.15 fix only sanitizes DebugDraw inputs to 0 (that bug =
  the LeBlock black screen we reproduced). Both my interpreter AND the
  const-fold pass had guards — FIXED (verify: `tests\graph_vm_div_probe.rs`).
- **Fatigue = anti-stalemate**: `active = max(0,extra_deuce−2) +
  max(0,rally_shots−11)`, extra_deuce = points≥3/≥3 ? sum−6 : 0; two Unity
  RNG draws per fatigued hit; aim scatter ±(0.03·12·active) lateral,
  ±(0.012·28·active) depth; charge × max(0.55, 1−0.025·active).
- **Game re-serializes loaded graphs** on exit (aia3.txt rewritten 12:59).
- **TimePlot** ports: String1=channel, Float1=value, sink-only; my VM
  `OpCode::TimePlot` → `debug_draw::plot(name, v)`; `tennis_probe` bin
  drains per tick → `probe_sim.json` (Python json accepts inf/NaN).
- **Corpus stats** (`scripts\analyze_graphs.py` →
  `data\tennis\graph_corpus_report.json`): 53 node types, 384 distinct edge
  pairs, all 82 graphs contain feedback cycles, 336 type-mismatch edges,
  45 empty Operation modifiers.

## 6. Key commands

```
# build/test (crate-only!)
cd C:\gitProjects\aia_comp-sim
cargo test --lib
cargo test --test tennis_v014_solver_parity -- --nocapture   # 200/210
cargo test --test tennis_v014_landing_parity -- --nocapture  # 8/10
cargo run --bin tennis_viewer -- --home titanium54 --away Zudan6 --seed 7
cargo run --bin tennis_probe -- --ticks 300 --out probe_sim.json
cargo run --bin tennis_tournament -- --home titanium54 --away aia3 --seed 7 --points 4

# game mining / matches
cd C:\gitProjects\AIA_tennis\modhost
python capture_fixtures.py --home titanium54 --away aia3 --seed N   # fixture matrix
python game_tournament.py --home titanium54 --points 8             # sweep (resumable)
python modctl.py launch --home X --away Y --seed N --points P
python modctl.py quit      # ExitProcess — skips timeplot flush!
# graceful quit that preserves Unity teardown:
(Get-Process Aialanders).CloseMainWindow()

# UI automation (user-approved; visualize before clicking)
python -c "import pyautogui, PIL; pyautogui.screenshot('screen.png')"
```

## 7. Pitfalls hit this session (don't repeat)

- PowerShell `python -c "..."` breaks on nested quotes — write scripts to
  `%TEMP%\opencode\*.py` and run them.
- `git add -A src` after manual edits mixes unrelated fixes — stage
  deliberately, one logical commit per fix.
- The game holds `probe-startup.jsonl` while running — stop processes
  before deleting.
- Probe runs with raw inf channels can hang the v0.14 game (user's system
  felt crashed) — probe graph now CLAMPS plotted values to ±1e30.
- Settings files written by PowerShell get a UTF-8 BOM — the game may
  reject them; strip BOM (fixed already; keep `python -c` writes as
  `newline=''`, encoding utf-8 without BOM).
- `emit_rust_fixtures.py` RUST path resolves relative to modhost → points
  to `C:\gitProjects\aia_comp-sim` (fixed + asserted — don't revert).

## 8. Suggested first five moves for the next session

1. Read `docs/TENNIS_V014_PARITY_NOTES.md` §4/§5, then this file.
2. `cargo test --lib` (must be green) → commit the fatigue WIP with sensor
   wiring + tests.
3. Rerun sim tournament (clean JSONL first) → confirm 82/82 load post-fixes.
4. Probe rebuild: add `games` + per-point winners to
   `parity_write_snapshot` (natural exe; read score class fields like the
   clamp fixtures do), rerun `game_tournament.py --points 8`, rerun
   `run_sim_tournament_pairs.py` + `score_parity.py` → exact parity %.
5. When the user is home: the manual timeplot-panel test (§4.5), then wire
   the export trigger into `game_tournament.py`.
