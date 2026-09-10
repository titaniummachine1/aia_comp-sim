# Tennis v0.14 parity — master notes (survival document)

Everything we learned about the real game, the capture pipeline, and the
Rust port. Written so nobody has to re-read code or re-do archaeology.
**If one doc survives, this one.**

---

## 1. The effective approach (what worked, in order)

1. **Do not guess the game — invoke it.** The ParityMod probe
   (`modhost/paritymod-src/`) launches the REAL game with a stripped native
   bootstrap that resolves IL2CPP metadata by **managed class/method/field
   names** (RVAs are provenance labels only, never hardcoded offsets), then
   **directly invokes** the game's own math on the live managed ball object
   and serializes raw f32 words as JSONL.
2. **Capture matrix, not traces.** One launch yields 168 direct
   `ComputeShotVelocity` results + 10 `TryPredictNthLandingFrom` +
   13 `Simulate` + 26 serve-direct + 29 curve + 8 clamp cases — each with
   full input words, result words, exception flags and snapshot restore
   checks. That is the replay corpus for the Rust sim.
3. **Bit-verify constants.** Every constant the Rust sim uses must trace to
   a captured f32 word (or the committed
   `tennis-sim/tests/fixtures/v014-pinned-scene-context.jsonl`). If a test
   pins a constant, re-measure — never edit the expected value (soccer rule).
4. **Replay in CI.** `cargo test --test tennis_v014_solver_parity` replays
   the whole matrix against the Rust solver and prints worst-case deltas.
   Parity regressions are loud.

## 2. Pipeline commands (all state lives in `modhost/`)

```
# one-time: build the event-capture probe (natural-trace is the other mode)
powershell -File modhost\build_paritymod_event.ps1
#   gcc (w64devkit @ %TEMP%\opencode\w64devkit\bin) with
#   -DV014_METADATA -DV014_EVENT_CAPTURE -DPARITYMOD
#   NOTE: build fixed Block-A nesting (config block was nested inside
#   V014_NATURAL_TRACE) and links -lgcc for ___chkstk_ms.

# mine the game (launches real v0.14 game ~90 s, then quits it)
python modhost/capture_fixtures.py --home titanium54 --away aia3 --seed 20260907
#   -> modhost/captures/<run>/ with per-kind JSONL
#   terminal row: {"kind":"metadata_capture_complete","fixture":
#     "simulate+nth-landing+direct-shot+direct-serve+curve-v014", ...}

# emit Rust fixtures
python modhost/emit_rust_fixtures.py <run>
#   -> aia_comp-sim/tests/fixtures/tennis-v014/{shot_solver,nth_landing,
#      simulate,curve}.jsonl + scene.json

# replay
cd aia_comp-sim && cargo test --test tennis_v014_solver_parity -- --nocapture

# quirk evidence scripts
python modhost/bounce_evidence.py <run>   # bounce plane / landing y
python modhost/test_quirks.py <run>
```

`modctl.py` (same dir) is the driver: `saves | launch --home X --away Y
--seed N --points P | state | wait | quit | summary`. Game v0.14 install:
`modhost/v0.14/` (Aialanders-original.exe is the pristine swap-back).

## 3. Game structure hints (decompiled/IL2CPP)

All resolved by name from `GameAssembly.dll` (v0.14, verified SHA in
tennis-sim evidence). Key managed surface:

| Class | Method | What it does |
|---|---|---|
| `TennisBall` | `ComputeShotVelocity(from, aim, type, power)` → Vector3 | the ballistic solve (RVA 0xc1a4a0) |
| `TennisBall` | `RebuildLaunchForAim` (0xc1f0a0) | candidate-time rescan 0.28→2.4 step 0.06, exit err²≤0.0064 |
| `TennisBall` | `SteerVelocityToAim` (0xc20c70) | ≤36 horizontal corrections vel += d/t, exit err²≤0.01 |
| `TennisBall` | `EnforceLobLoft` (0xc1b9a0) | lob net-clearance vertical bound |
| `TennisBall` | `TryPredictNthLandingFrom` (0xc222a0) | horizons 6 s (1st) / 10 s (2nd), adaptive substeps ≤0.2/\|v\| |
| `TennisBall` | `TryCrossNetPlane` (0xc21dd0) | tape plane y = netHeight + radius·0.35 |
| `TennisBall` | `ApplyFloorBounce` (0xc18270) | per-shot pending flags, consumed 1st bounce |
| `TennisBall` | `ApplyNetRebound` (0xc18400) | dead-cat bounce off net |
| `TennisBall` | fields | `flightPace`, `arcadeGravity` (−28), `maxSpeed` (46), `<Radius>` (0.31), `lastHitPower`, `pendingTopspinKick/SliceBounce/DropBounce`, `cachedPredictedBounce(+Time/Age)` |
| `TennisGameManager` | getters | `get_CourtLength` (**28** — manager's serialized `courtLength=24` is NOT used), `get_NetHeight` (0.95), `get_CourtY` (0), `GetDefaultAimTarget` (±0.48·length), `aimNetGap` (0.85) |
| Serve | `HandleServeFault` (0xc32990), `MustLetServeBounce` (0xc341e0), `OnBallStruck` (0xc35310), `OnBallBounced` (0xc344f0), `OnBallHitNet` (0xc34e10) | fault/let/volley rules |
| Score | `TennisScore.AwardPoint` (0xc68e90) | game = max(1,4) points + 2 lead |
| Stances | `GetServeStand`/`GetReceiveStand`/`GetDiagonalServiceBox`/`GetCenterOfBack` | 14.9167 / 8.15 / box ±(L/4, S/2)+0.075 / 0.375·L |

RNG: VM uses SplitMix64 per player (`seed ^ 0x83a2f347` home /
`^ 0x157cc26d` away). Unity xorshift128 (`ReferenceRandom`,
`Random.InitState` state `[seed, 1812433253·s+1 …]`) drives fatigue (2
`range(-1,1)` draws), trick variants, `RandomRangeInt(0,7)` rerolls
(then x-then-z `range`), deciding-set coin.

## 4. Solver algorithm (EMPIRICALLY VERIFIED — fixture matrix)

**Game ARG order** (the 4th arg of ComputeShotVelocity; NOT the graph
dropdown order): `0 Topspin, 1 Slice, 2 Flat, 3 Lob, 4 Drop, 5 CurveLeft,
6 CurveRight`. Dropdown→arg mapping: `0→0, 1→1, 2→2, 4→4, 5→3, 6→5, 7→6`;
Trick (dropdown 3) is resolved by a seeded variant roll before the solver.

```
speed = base · (0.85 + (high−0.85)·q)      high = 1.3 for args {0,2}
                                                else 0.85+0.42·0.45 = 1.039
time t per arg:
  0,2,5,6 (straight): t = max(minT, d/speed)         minT = .14/.12/.14
  1 Slice:            t = 0.5 + 0.12·q + 0.04·0.08   (FIXED — distance ignored!)
  3 Lob:              t = max(0.82+0.14q+6·0.14, 0.8+0.08q+0.08)
  4 Drop:             t = 0.63 + 0.13q + 0.27·0.13
vy = (target_y − from.y)/t + 14·t                   target_y ≈ 0.31 (floor plane)
vxz = dir · d/t
cap loop: while |v| > 46: t += 0.05 (≤12, bounded max(2.4, t+0.35))
```

Verified exact at q=0/0.5/1 for args 0–6 (e.g. flat 20 m q=1: vx=36.4 =
28·1.3 ✓; slice t=0.6232 at q=1 ✓; drop t=0.7951 ✓).

## 5. Discoveries & quirks (the stuff you'd never get from code reading)

1. **Net avoidance lives in the STEER chain, not the raw solve.** The raw
   `ComputeShotVelocity` is net-blind for EVERY shot type — proven by the
   net-lip fixture (aim 0.25 past the net, profile added 2026-09-10): raw
   results cross at y≈0.4 (below tape) for all 7 args, unchanged
   pure-t/tail/floor times. The post-chain `RebuildLaunchForAim` +
   `SteerVelocityToAim` re-predicts against the net-colliding flight and
   stretches/lofts until the arc clears the tape AND lands on the aim —
   the steered output clears for all 7 args (top/slice/flat/drop converge
   to the identical arc (5.889, 28.788): the target arc is
   shot-independent). The in-game "flat/topspin eats the net" cases =
   when the steer chain FAILS (no arc exists that both lands and clears)
   and the raw net-blind velocity ships as-is. With pure aim and no
   inaccuracy the ball lands exactly on the aim point — the pure-aimbot
   contract. (Rust: steer uses the tape-failing predictor; seed stays
   net-blind — verified 200/210 against the live matrix.)
2. **The ball "bounces" before its center reaches y=0.** Floor plane =
   `courtY + radius` = **0.3100000024** (bit-exact in
   `floor-bounce-drop` capture: bounce counters increment with position
   y = 0.31). v0.14 interpolates rendered ball position, so this reads
   fair in-game. My model: `BOUNCE_FLOOR_Y = 0.31` ✓ already exact.
3. **Bounce/landing positions are ground-projected to y=0 in the API.**
   `<LastBouncePosition>` = (x, **0.0**, z) and all 10 NthLanding
   predictions return landing y = 0.0 — the physics plane (0.31) and the
   reported plane (0) differ by design.
4. **Slice ignores distance for its flight time** — fixed tail
   `0.5+0.12q+lift·0.08`. A 20 m and a 24 m slice take the same time; the
   speed simply scales with distance (39.7 vs 43.4 m/s at q=0).
5. **Lob/Drop likewise: time comes from floors, not d/speed** — the
   speed base (12 / 7.6) barely matters; q shifts the floor.
6. **`flightPace` is a per-ball scalar** (captured live = 1.0; fixture
   exercises 1.0/1.5/1.75). Flight forces scale by pace² (curve, gravity,
   decay) — shape-preserving. Do NOT confuse it with speed/46.
7. **Gravity sign convention**: solver math is z-up with gravity −28, so
   the vertical solve is `vy = Δy/t + 14t` (UPWARD launch). Getting this
   sign wrong produces downward lobs that fall out of the sky.
8. **Manager's serialized `courtLength=24` is a decoy** — all decision
   bodies call the getter (28). Trust getters over serialized fields.
9. **`Ball Incoming` (bool 27) reportedly turns false at the FIRST bounce**
   (not on direction change) — user report; **NOT yet verified from
   captured data** (getter_items capture failed that run; needs the
   getter-delegate value probe). Implemented in `tennis/api.rs` as the
   v0.14 behavior, flagged unverified.
10. **Serve tape = fault, rally tape = rebound** (`OnBallHitNet` branch);
    rebound: x = ∓(r+0.08), vx = ∓max(2.4, |vx|·0.38), vy = max(2.2,
    |vy|·0.45+1.4), vz·0.72. Tape plane = 0.95 + 0.31·0.35 = 1.0585;
    net half-width 9.25.
11. **Bounce pending flags** (1st bounce only): topspin vx/vz·1.2 and
    restitution·1.42; slice q-blended (forward 0.90→0.6724, rest 0.82→1.008);
    drop forward 0.40, rest·1.08, hop clamp 9.6–11.8. Base: rest 0.78,
    forward 0.94, min 2.4, spin floor 0.45.
12. **Scoring**: game at 4 points + 2 lead; server = (first_server +
    games_total) % 2; ad court = points_played & 1; receiver vollying
    serve before bounce = server's point; serve tape/out/box-miss →
    fault → double fault.
13. **Fallback aim**: when the aim is degenerate (~zero distance), the
    solver does NOT produce a zero vector — it uses the incoming velocity
    direction at full charged speed (fixture "fallback" rows: game
    (−36.4, 2.387) for flat q=1 = |28·1.3| along −input-vel dir).
    **Rust does not implement this yet — it's the current parity gap**
    (122/168 cases; fallback-profile rows are the failures).
14. **Slice crossed-net delta (+~0.05 s over the tail)** — open fit; only
    slice rows at crossed-net profile sit slightly above the tail.
15. **Curve force basis (NthLanding fixture-fitted)**: the ball's curve
    field is a z-axis acceleration applied along a FIXED axis — signed by
    the shot's curve direction (CurveRight = +Z, everything else = -Z in
    the fixture fits) — with the slow 0.75/s decay while the field is
    non-zero. NOT velocity-perp, NOT team-flipped in the two mined cases.
    The same two cases also suggest the PREDICTOR does not zero curve on
    bounce (residual ~0.55 m on 
egative-curve-team-one); live
    ApplyFloorBounce does zero it. Open fit.
16. **Predictor net policy**: rally predictions (stopOnTape=false) PASS
    THROUGH a below-tape net crossing and keep integrating — only serve /
    stopOnTape predictions fail there (
egative-curve-team-one crossed
    at y=0.44 and still predicted its 2nd bounce).
17. **TryPredictNthLandingFrom returns Nullable<bool>** — when false,
    the serialized out words are zeroed placeholders, NOT a position.
    Always read return_bool_word before trusting landing_f32_words.
18. **Timeplot export (v0.14): two triggers.** (a) The panel's
    "Export JSON" button — a manual click writes
    `Saves\Tennis\Timeplots\timeplot_<ts>.json` immediately (no
    menu-return needed; verified 2026-09-10 with the user clicking Export
    4× during a live match). (b) 2026-09-10 addition: a **graceful quit
    also flushes plots when the panel is visible** (unattended tournament
    matches produced files with no click; hard kills never do). Panel
    visibility/settings persist between sessions. The file is
    locale-comma JSON: comma between digits = decimal point, comma+space
    = array separator (parse with `modhost\parse_timeplots.py`). Content:
    `simTime` + `series[]` per TimePlot channel (`name`, `color`,
    per-tick `x[]` sim-seconds at 0.019 steps, `y[]` values), cumulative
    across the whole match so far. Exports carry ALL plotted channels
    from BOTH bots (44 in the titanium54-vs-aia3 capture, incl.
    `v49_FatActive`). Captures: `modhost\captures\timeplots-20260910\`
    (+ live per-bot folders under `modhost\game_timeplots\`).
    This is the per-tick game-side ground truth for the sim-vs-game
    channel diff (replaces the failed sim_probe.txt in-game approach).
    Attribution in parallel tournaments: result rows carry UTC
    match_started/match_ended windows; collect preserves mtimes (copy2);
    filename timestamps fall inside a row's window.
19. **Menu scoreboard can show a stale point after paritymod launch**
    (0-15 before any play; cleared when the match starts) and the game
    AUTO-RESTARTS a finished match after ~1 min if left unattended —
    quit via menu promptly after the target point count.

## 6. Current state (commit where this doc was added)

- `aia_comp-sim`: 157 lib tests green; tennis game mode complete (world,
  scoring, serve, net, bounce, VM wiring, mode gate, GameSpec versions).
- Solver replay = **164/168 within 0.25 m/s**; landing replay = **8/10**. Remaining failures: fallback-aim rows (discovery #13) and
  slice crossed-net (#14). Fix order: (a) fallback direction from input
  velocity, (b) slice net term, (c) tighten gate to 0.05 m/s, then
  bit-compare.
- `GameSpec`: `tennis_v014()` latest-default; `tennis_builder()` for
  v0.012-order graphs (titanium54); unknown v0.14 labels rejected (never
  invented). `Ball Incoming` quirk implemented, unverified (see #9).
- Version docs: `aia_comp-sim/docs/GAME_VERSIONS.md` (v0.15f = free;
  Patreon features; sim targets v0.14 only — disclaimer included).

## 7. If the session dies — pickup checklist

1. `cd aia_comp-sim && cargo test` — everything must be green before
   touching anything.
2. Read this doc §4/§5 before touching `tennis/shot.rs` or `ball.rs`.
3. To re-mine after any game/approach change:
   `python modhost/capture_fixtures.py …` (§2). Never run captures while a
   real match is running (exe swap).
4. Never rebuild Bevy/deps: crate-only `cargo test --lib`; no feature or
   profile flips; no folder renames (each nukes the cache and costs ~40 min).
5. Commit after every green milestone; the fixture JSONLs are committed so
   replays work without the game.
