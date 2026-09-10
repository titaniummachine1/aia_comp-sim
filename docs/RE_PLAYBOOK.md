# Cross-game RE playbook (how to reverse-engineer an AIA game efficiently)

Transferable methodology distilled from the soccer sim and tennis v0.14
sessions. Game-specific facts live in `TENNIS_V014_PARITY_NOTES.md`,
`SOCCER_GAME_MODEL.md`, `AIA_UPSTREAM_QUIRKS.md`, `API_GAPS.md`. When a new
game version or game lands, start here; only write a new game-specific notes
doc when this playbook's generic answers run out.

## 0. Goal framing (do not drift)

- **Outcome parity, not 1:1 cloning.** Any 2 AIs played in the sim must
  produce the same match result as in the game. Viewer/editor = QOL only.
- The game is a black box to **invoke**, never to redistribute (no binaries
  in git, ever — user handles legacy zips).
- Everything is measured against the live game. Never "fix" a test by
  editing the expected value (soccer golden rule).

## 1. First-hour checklist for a new game/version

1. **Locate install + verify build identity.** Hash the exe against the
   official build; a mismatched install means wrong metadata offsets.
2. **Find the metadata.** Unity IL2CPP:
   `<game>_Data/il2cpp_data/Metadata/global-metadata.dat`. You do NOT need a
   dump to start — plain ASCII strings extraction gives class names, script
   paths (`\Assets\_Scripts\...`), setting keys, UI labels. (One python
   regex pass over the .dat found the entire TimePlot UI surface in
   minutes; keep that script around.)
3. **Inventory what already exists**: prior sim repos, bot save dirs
   (`%USERPROFILE%\AppData\LocalLow\<Dev>\<Game>\Saves\...`), community
   graph libraries (AIGamePyLibrary pattern: nodes/edges with named
   operations), old dumps, the dev's other games (same engine = same
   patterns — tennis reused soccer's TimePlot concept).
4. **Establish the deterministic-launch recipe FIRST**: seeded RNG →
   spawn save → start match (tennis: `Random.InitState(seed)` →
   `Spawn("savename")` → `QueueStartMatchWhenGraphsReady()`). Every capture
   and every parity claim downstream depends on replays.
5. **Get ONE fully automated end-to-end match** (launch → run → quit →
   collect) before optimizing anything else. Everything else builds on it.

## 2. The ParityMod pattern (freestanding C probe, no CRT)

Structure that worked twice:

- **Launcher exe swap**: a tiny C launcher (gcc `-nostdlib -mwindows -e
  start`, `-fno-stack-protector -fno-builtin -fno-strict-aliasing`, link
  `-lkernel32 -lgcc`; provide `memcpy`/`__chkstk` locally) replaces the
  game exe, starts the real game, and resolves IL2CPP metadata **by managed
  class/method/field names** — RVAs are provenance labels only, never
  hardcoded offsets (survives rebuilds).
- **JSON-file API, no HTTP needed**: `paritymod.json` (config), 
  `parity_cmd.json` (poll-file commands: quit/snapshot/export), 
  `parity_state.json` (snapshots). Poll throttled (every 256 invokes).
- **Build variants by defines**, one source: 
  `V014_METADATA` (metadata access) + `PARITYMOD` (runtime driver:
  launch/wait/quit/snapshot) + `V014_EVENT_CAPTURE` (fixture-matrix miner)
  or `V014_NATURAL_TRACE` (unattended match driver with cmd poll).
  Natural build ≈ 62 KB, event build ≈ 160 KB.

Field/method access patterns (the ones that save hours):

| Pattern | Rule |
|---|---|
| C# auto-properties | Live as `<Name>k__BackingField` fields — read by that name first; chasing getters is the slow path |
| Statics | `il2cpp_field_static_get_value`; log `flags` (static bit) as provenance |
| Component links | A field whose declared type == a known class (e.g. `Player.Stamina: Stamina`) → follow the pointer directly, no instance hunt |
| Properties | Resolve getter by class+name, verify `invoke_ok`/`exception` per call; keep raw f32 words, decode nothing in C |
| Per-object dumps | Enumerate `methods[]` per class once (the probe already does) — that IS your API surface |

## 3. Capture methodology

- **Capture matrices, not traces.** One launch: direct invocations of the
  game's own math across an input matrix (team × shot type × power × aim
  profile), each row = input words + result words + exception flag +
  snapshot-restore check. That corpus is the replay fixture set for the
  sim (tennis: 210 solver + 10 landing + 26 serve + 29 curve rows/launch).
- **Mutating invokes must restore world state**; assert
  `final_matches_snapshot` after every capture run.
- **Live outcome attribution from raw events, never UI snapshots.** Score
  classes expose post-reset points; shut-out games are then unattributable.
  Log `<LastPointWinner>` at the instant `pointResolved` rises and append to
  a per-point winner log; reconstruct games/matches by walking the sequence.
- **Sharded live tournaments** for volume: N game-dir copies (own
  exe/config/state; shared saves by design), shard = index % N of the full
  sorted bot list, seed = base + **global** index (keeps sim replays
  comparable across serial and sharded runs). Budget ~2 GB commit per
  Unity instance; 4 instances ≈ 8.6 GB on a 16 GB box — log free RAM.
- **Artifact attribution**: record UTC `match_started`/`match_ended` per
  result row; collect exports with `shutil.copy2` (mtime preserved);
  game-written filenames embed their creation timestamp → file ⇄ match
  attribution is a set-membership check, never guesswork.

## 4. Engine/genre patterns (seen in both games)

- **Bot brains are dataflow graphs** (named sensor/actuator nodes, ~50 op
  types, all graphs contain feedback cycles, type-mismatch edges are
  normal, some modifiers legitimately empty). Compile them, don't
  interpret per-tick in Python.
- **Mode-first compilation**: `Lowerer::compile(graph, Option<GameSpec>)`;
  mode-owned nodes are hard errors in `compile_pure`. No implicit default
  game mode.
- **Deterministic movers** (`DeterministicMover`: targetDestination +
  pathfinding fields; frozen flag) — physics-level, replayable.
- **Physics state lives on the ball object** (`TennisBall`): position,
  velocity, curve/team, cached predicted bounce(s) + flags. The predictor
  is authoritative for AI positioning — replicate it, don't reinvent.
- **Debug channels** (TimePlot): sink-only nodes, String1=channel name,
  Float1=value; exports = locale-comma JSON (comma between digits =
  decimal point; comma+space = array separator), `simTime` + `series[]`
  with per-tick x/y. Channels from BOTH bots in one file. Ground truth for
  channel-level sim-vs-game diffs.
- **RNG state**: Unity `Random.state` = 4 words; capture/restore for
  determinism probes. Two independent RNG draws per event are common
  (fatigue scatter: lateral then depth).
- **Div/Mod semantics are IEEE** (n/0=+inf, 0/0=nan) — both interpreter
  AND const-fold passes need guards; sanitizing only some inputs hides it.

## 5. Game-behavior traps (recurring)

1. The game **re-serializes loaded graphs on exit** — saves mutate on quit;
   never assume save files are pristine after a run.
2. **Graceful quit flushes exports** (timeplots) when the relevant panel is
   visible; hard kill (`ExitProcess`/taskkill) skips teardown entirely.
   Quit via the poll-file cmd, then wait for exit; taskkill only stragglers.
3. Finished matches **auto-restart after ~1 min** if left alone — quit
   promptly on done.
4. **File locks**: the game holds its trace/log files while running — stop
   processes before deleting/renaming; wait-for-exit before swapping the
   exe (PermissionError race).
5. **Settings files**: UTF-8 **without BOM** or the game rejects them.
6. **Never feed raw inf/NaN into plotted channels** — the game can hang;
   clamp probe outputs (±1e30).
7. Only kill processes whose path is inside the game dir (other windows
   may legitimately run the same exe name).
8. UI quirks post-launch are common (stale scoreboard until match starts)
   — cosmetic; verify against live state fields instead.

## 6. Sim-side discipline

- **Replay in CI**: every captured matrix becomes a test
  (`cargo test --test <game>_solver_parity` prints worst-case deltas).
  Regressions must be loud.
- **Bit-verify constants**: each constant traces to a captured f32 word or
  a committed pinned-context file; re-measure, never eyeball.
- Exclude world-state-dependent rows from matrices instead of faking them.
- Outcome parity is scored from raw per-point events on BOTH sides
  (game: point-winners log; sim: games/points from the tournament bin),
  games-then-points leader comparison, same seed pairs.

## 7. What we have NOT cracked yet (carry forward)

- **Direct remote export invocation** (button-free timeplot export):
  classes located via metadata strings (`Gates\TimePlot.cs`,
  `TimePlotGate.cs`, `UI\TimePlotChartElement.cs`); enumerate their methods
  via the probe's class-dump and wire an `export` parity_cmd. Currently
  moot for volume runs because **graceful quit flushes plots when the
  panel is visible** — but the API route is still the robust end state.
- Getter-item sensor ABI (per-version label tables) — pin from a live
  capture before trusting sensor reads.
- Marathon/timeout semantics (100k-tick matches) not fully modeled.