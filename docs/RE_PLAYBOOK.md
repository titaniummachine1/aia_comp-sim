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
6. **Get the release changelog.** The author's own patch notes are the highest
   leverage artifact in the whole process: they name the behaviour changes, so
   version drift can be triaged instead of blindly diffed. (v0.15's
   "interpolate ball position between frames" and "fixed ball time-to-ground
   caching" told us in one line where to look.) Ask the user for it if it is
   not in the repo.
7. Re-targeting an existing game to a **new version** has its own recipe —
   see §8. It is usually cheap because the probe resolves by name.

## 1b. Where things live (tennis, as the worked example)

| Thing | Path |
|---|---|
| Sim repo (Rust VM + world + reference interpreter) | `C:\gitProjects\aia_comp-sim` |
| Compiler (Python AST front end + `graphc-rs` backend) | `C:\gitProjects\aia_graphc` |
| Mod + probe + drivers + captures | `C:\gitProjects\AIA_tennis\modhost` |
| Pristine game installs (per version, never modified) | `C:\gitProjects\AIA_tennis\v0.015f`, `v0.14 tennis`, … |
| **Modified** game host dirs (one per version, exe swapped) | `modhost\v0.14`, `modhost\v0.15f`, `modhost\v0.14_w2..w4` |
| Probe C source (version-agnostic launcher + metadata header) | `modhost\paritymod-src\{bootstrap.c,metadata_probe.h,api_indices.h}` |
| Built probe exes (never commit game binaries) | `modhost\paritymod\Aialanders-paritymod*.exe` |
| Bot saves (loadable by name) | `%USERPROFILE%\AppData\LocalLow\Unicorn One\AIComp\Saves\Tennis\` |
| Game TimePlot exports | `…\Saves\Tennis\Timeplots\` |


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

- **Direct remote export invocation — DONE (2026-09-12).** The native
  `TimePlot.ExportToJson` method (singleton via `get_Instance`) was located from
  the metadata strings and wired into the mod: it now flushes on `restart_match`
  (before `ResetMatch`), on `quit`, on an on-demand `{"cmd":"export_timeplot"}`,
  and on interactive window close (`WM_CLOSE`/`WM_ENDSESSION` via a
  `UnityWndClass` subclass; build links `user32`). Graceful quit remains the
  zero-effort path.
- **Getter-item sensor ABI** (per-version label tables) — pin from a live
  capture before trusting sensor reads. 18 tennis getters are still UNCERTAIN.
- **Marathon/timeout semantics** (100k-tick matches) not fully modeled.
- **v0.15 real behaviour changes not yet modeled**: (a) the ball is now
  **interpolated between frames** so fast balls can still be hit — contact /
  `Ball In Swing Range` detection probably needs a swept test; (b) the
  **ball time-to-ground cache** was fixed (three cache fields deleted), which
  touches the predictor and may explain the v0.14 landing residual.
- **Unknown per-version drift** in anything not diffed yet: run the §8 ladder
  and §8.7 triage before assuming a new version behaves like the last one.


## 8. Re-targeting a game VERSION (worked recipe: v0.14 → v0.15f)

Full case study: `docs/HANDOFF.md` §18. This section is the generalised,
repeatable version — the thing to follow for v0.16, and for any future game
whose probe already boots once.

### 8.1 Why it is usually cheap

The probe resolves IL2CPP metadata **by managed class/method/field name** at
runtime. So class renames are rare and offsets/RVAs don't matter. On
v0.14 → v0.15f, **3167 methods, 1943 fields, 109 classes all resolved
unchanged**, and both bot graphs loaded with byte-identical structure
(titanium54 5383 nodes / 7932 conns). Exactly **two** things broke, and both
were *our* assumptions, not the game's naming:

- one **instruction-level hook** whose signature included an embedded address;
- one **named-field list** containing fields the new build deleted.

Expect the same shape on the next version: it's usually a handful of small,
mechanical assumptions, each with a loud failure signature.

### 8.2 The recipe

1. **Host dir, pristine backup.**
   `robocopy <pristine install> modhost\v<ver> /E`, then copy
   `Aialanders.exe` → `Aialanders-original.exe` (the pristine launcher must be
   preserved: `modctl.py` refuses to launch without it).
2. **Build the probe with the SAME sources and flags**, distinct output name
   (`build_paritymod_v015.ps1` → `Aialanders-paritymod-reset-v015.exe`).
   Do **not** touch the working exe for the previous version — you want to be
   able to go back.
   ```
   gcc -O1 -DV014_METADATA -DV014_NATURAL_TRACE -DPARITYMOD \
       -fno-stack-protector -fno-builtin -fno-strict-aliasing \
       -I $src -c bootstrap.c -o out.obj
   gcc -nostdlib -mwindows -e start -o Aialanders-paritymod-reset-vNNN.exe \
       out.obj -lkernel32 -luser32 -lgcc
   ```
   (Needs w64devkit on PATH, e.g. `%TEMP%\opencode\w64devkit\bin`.)
3. **Drive it purely by env**, no driver edits:
   `MODHOST_GAMEDIR=…\modhost\v<ver>` and
   `MODHOST_PARITY_EXE=…\paritymod-reset-vNNN.exe`, then
   `python modctl.py launch --home X --away Y --seed N --points P`.
   Long runs go through a scheduled task — the agent shell kills children.
4. **Read the probe's own report** (`<gamedir>\probe-startup.jsonl`) with the
   diagnostic ladder in §8.3. The probe is designed to fail loudly with a
   `reason` string; you almost never need a debugger.
5. **Only then** re-mine fixtures and triage drift (§8.7).

### 8.3 The diagnostic ladder (read the trace in this order)

Every row is `{"kind": ...}`. A healthy run passes these gates in order; the
first one missing or failing is your bug.

| Gate | Row to look for | Healthy | Failure means |
|---|---|---|---|
| 1 | `bootstrap`, `resolver_observed` | present | probe never started / UnityPlayer load failed |
| 2 | `metadata_module`, `class_catalog`, `metadata_image` | many rows | metadata access (API index order) wrong |
| 3 | `metadata_complete`, `method_info_layout` | present | discovery aborted early |
| 4 | `owned_spawn` (one per bot) | `exception:false` | the save failed to load as a graph |
| 5 | `natural_hook_status` | `installed:true`, `reason:"installed_dynamic_method_entry"` | **hook install failed** → §8.4 |
| 6 | `trace_initial` | `seed_invoke_ok:true`, `state_read_ok:true` | **snapshot read failed** → §8.5 |
| 7 | `trace_start` | `queue_invoke_ok:true` | match never queued |
| 8 | `<gamedir>\parity_state.json` | `tick>0`, rising `callbacks` | ticks not advancing |
| 9 | `natural_trace_status` | `status:"complete"`, `ticks>0` | trace ended early (read `reason`) |
| 10 | TimePlot in `…\Saves\…\Timeplots\` | full-size file (hundreds of KB) | quit didn't flush (panel visible? graceful quit?) |

Two practical notes:
- The `reason` strings are the whole point — `stamina_hook_install_failed`,
  `seed_or_initial_snapshot_failed`, `preamble_mismatch`,
  `guard_outside_gameassembly`, `stamina_class_missing`,
  `method_target_outside_gameassembly`. Grep them, don't guess.
- A **stub** TimePlot (~4 KB) vs a real one (hundreds of KB) instantly tells
  you whether the match actually ran. File size is a free verdict.

### 8.4 Failure class A — instruction-level hooks (byte-pinned signatures)

Symptom: `natural_hook_status.installed:false`, `reason:"preamble_mismatch"`,
then `natural_trace_status.reason:"stamina_hook_install_failed"`.

Some methods are called from **native** code (not through
`il2cpp_runtime_invoke`), so the only way to observe them is a small entry
detour. That means validating the target's entry bytes — and the trap is
pinning **address bytes** as if they were opcodes.

The v0.15f case: the first 16 bytes of `Stamina.OnSimulationTick` are

```
40 53                  push rbx
48 83 EC 40            sub rsp, 0x40
80 3D <disp32> <imm8>  cmp byte ptr [rip+disp32], imm8   <- guard read
48 8B D9               mov rbx, rcx
```

`<disp32>` is a RIP-relative address of a **static field**. It moves between
builds (v0.14 `0E D7 7A 06` → v0.15f `5E E4 7C 06`); everything else is byte
identical. Lessons:

- **Match instruction SHAPE, not addresses.** Compare the opcode/ModRM/imm
  bytes and skip displacement/immediate-address fields (here: validate bytes
  0..7 and 12..15; skip 8..11).
- The relocation is **already done from the OBSERVED bytes**, so validating a
  pinned address adds nothing but build-coupling. Replace it with a
  *structural* check that is still strict enough to catch a bad match — e.g.
  "the re-encoded guard address must land inside the module image".
- Keep the failure path loud (`reason` + observed bytes in the row) so the next
  version reports immediately instead of silently mis-hooking.

Generalisation: **whenever you pin a byte signature, ask which bytes are
semantics and which are layout.** Layout bytes (displacements, absolute
addresses, jump targets, vtable slots, RVAs) must be masked or relocated.

### 8.5 Failure class B — named fields that moved or vanished

Symptom: `trace_initial.state_read_ok:false`, then
`natural_trace_status.reason:"seed_or_initial_snapshot_failed"`.

The snapshot writer walks **named field lists**. A field that no longer exists
makes the read fail, and one failure aborts the whole snapshot — so the trace
never starts, which is a very misleading place to land.

Fixes, in order of preference:

1. **Explicit version-optional allowlist** (what we did):
   ```c
   static int metadata_trace_field_optional(const char* name) { /* … */ }
   /* in the field-group writer: */
   else { write_text("null");
          if(!metadata_trace_field_optional(specs[i].name)) ok=0; }
   ```
   An absent field *listed as optional* is a version difference; a typo or a
   genuine read error still fails loudly. Keep the list short and comment it
   with which version removed what.
2. Only if the field is *semantically needed*: branch on version (detect via a
   field present in both builds, or a version string) and read the replacement
   field/property.

Anti-pattern: making the group never fail. That trades a loud version error
for silent nonsense downstream.

### 8.6 The trace-diff technique (find class B in minutes, not hours)

Don't eyeball a 14 KB JSON row. Snapshot the **same gate row from both
versions** and diff it programmatically:

```python
# 1. grab the first trace_initial row from each version's probe-startup.jsonl
#    (iterate lines, stop at the first match — the file is huge)
# 2. compare the EMITTED VALUE SETS per group
#    manager_fields / score.fields / ball.fields / players.*.fields / …
# 3. then diff the game's own metadata rows for the suspect names
```

Two traps make this necessary:

- **Name sets can look identical even when fields vanished.** The writer emits
  every *listed* name, so a removed field appears as a key with value `null`,
  not as a missing key. Diff **values**, not just keys.
- **The emitted dict reflects your list, not the game's struct.** For the true
  story you need the game's own metadata.

Second half of the technique: **search the trace for the field's metadata row**
(`class_catalog` / `metadata_field` rows carry `owner`/`name`/`type`/`flags`/
`offset`). For the v0.15f case this gave decisive evidence: the three fields
were present in v0.14 as `System.Single` at 0x140/0x144/0x14c and **absent
entirely** in v0.15f, with the neighbouring boolean shifted 0x148 → 0x140 — the
classic "field deleted, struct shrank" signature.

Invariants worth checking while diffing:
- A **type change** (`System.Single` → `System.Nullable<…>`/`System.Double`)
  usually means the value semantics changed → parity-relevant, go look.
- A **removed cache/prediction field** is a strong hint that caching logic was
  reworked (here the changelog said so outright).
- Identical *unsupported* rows across versions (`IntPtr`,
  `CancellationTokenSource`, `unknown_value_type`) are harmless noise — they are
  tolerated, not failures. Don't chase them. (We nearly did: 8 identical
  unsupported reads in both builds, yet only v0.15 failed — because the real
  cause was elsewhere.)

### 8.7 Changelog-first drift triage

Once the probe runs, re-mine and **triage against the changelog** before
touching the sim:

1. Map each changelog line to a sim surface and classify it: *real behaviour
   change* vs *QoL/UI only*. (v0.15: ball frame interpolation and the
   time-to-ground cache fix are real; the searchable node menu is not.)
2. Re-mine (`capture_fixtures.py` → `emit_rust_fixtures.py`) and run the replay
   test; it prints worst-case deltas per case.
3. Assign every drift row a class: presentation-only / constant change /
   dropdown-table change / genuinely new behaviour. **No unexplained rows** —
   an unexplained row means the version is not yet understood.
4. Version-parameterise the sim only for the classes that need it
   (`GameSpec::tennis_v014()` ↔ a v0.15 sibling); keep the old fixtures pinned
   so the previous version stays verifiable.
5. Stop if the drift is presentation-only. Do **not** fork a solver for a
   version whose physics did not move.

### 8.8 Brand-new GAME (not just a version) — deltas from the above

- Everything in §1 first. The probe's **launcher** half (`bootstrap.c`) is
  game-agnostic; only the **metadata** half (`metadata_probe.h`) is
  game-specific.
- Re-derive `api_indices.h` (the IL2CPP export **index order**) from a
  bootstrap-only capture of that game before trusting the metadata build — the
  index order is the one thing that is *not* name-resolved.
- Find the game's "start a deterministic match" entry points by name (seed RNG
  → spawn save → queue match) and make them the `trace_initial` / `trace_start`
  gates, so failures stay as legible as they are here.
- Only then start a game-specific notes doc; keep the generic parts here.


