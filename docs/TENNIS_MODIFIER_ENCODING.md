# Tennis node `modifier` encoding (label vs numeric index)

**Status: pinned by game evidence, 2026-09-13. This is a handoff doc — read it
before touching tennis sensor emission, graph loading, or the VM lowerer.**

## The rule

Some tennis node kinds store their dropdown selection as the **Unity option
text**, not as a numeric index. For those kinds the JSON `modifier` field must
be the exact label string, e.g. `"Ball Position"`.

The authoritative list is the vendor library's `DROPDOWN_MODIFIER_AS_LABEL`
(`AIA_tennis/AIGamePyLibrary/AIGamePyLibrary/data.py`):

```
Keypress, Color, Country, SurvivalEmote, SurvivalState,
TennisAutoSwing, TennisGetTransform, TennisGetBool,
TennisGetFloat, TennisGetVector3
```

Everything else (`Operation`, `CompareFloats`, `Vector3Split`,
`RelativePosition`, `Vector3Make`, ...) is **not** a dropdown and keeps its
literal modifier (an operator id, port name, etc.).

`data.py`'s own comment states the reason plainly: *"Label-as-modifier nodes
(Unity matches option text, not a numeric index)."* The same file normalizes
numeric input to `str(idx)` **except** for nodes in that set, which it
normalizes to `options[idx]` (the text).

## Why this mattered (the v57 "can't hit the ball once" bug)

`aia_graphc` emitted `Op::TennisGet { kind, index, label }` as
`em.node(node_kind, index.to_string())` — the **numeric index** — for every
`TennisGet*` node, even though the IR already carried the Unity label.

Consequence in the real game: the dropdown never matched, so the sensor gate
read its default and every ball sensor returned dead values
(`Ball Position == (0,0,0)`, `Ball Velocity == (0,0,0)`). The graph still ran,
the controller still moved, but the bot could not compute a contact point →
"titanium gets obliterated in a real game, can't hit the ball once".

Consequence in the simulator: none. `aia_comp-sim`'s resolver
(`src/graph/dropdowns.rs`, `resolve`) *tolerantly* accepted the numeric index
and mapped `"0"` → `"Ball Position"`. The sim therefore masked the bug: the
compiled bot swept the sim while being blind in the game. **A sim that accepts
more encodings than the game is a false-confidence machine.**

*(Resolved 2026-09-13: `resolve_for_version` — the version-aware entry point the
loader actually calls — no longer performs that index upgrade on label-matched
nodes, and the version-unknown load path applies the same rule via
`is_dead_label_modifier`. The tolerant `resolve` still maps index -> label for
kinds that are not label-matched, but no tennis sensor path reaches it with a
bare index any more.)*

## Evidence

Node census over real saves
(`%USERPROFILE%/AppData/LocalLow/Unicorn One/AIComp/Saves/Tennis`):

| Save | Producer | `TennisGetVector3` modifier |
|---|---|---|
| `titanium54.txt` | AIGamePyLibrary (`AddNode(..., "Ball Position")`) | `"Ball Position"` |
| `Adam.txt`, `Apex.txt`, `underdog.txt` | AIGamePyLibrary | label text |
| `PerfectController_baseline.txt` | game-authored | label text |
| `diagbot.txt`, `titanium57.txt` | graphc (pre-fix) | `"0"` |

Live/dead channel check via the game's own Timeplots export
(`modhost/parse_timeplots.py`): in graphc-built runs the ball channels are
frozen at 0 while `Self`/`Opponent` channels move; in the label-modifier
titanium54 run the ball channels carry real trajectories
(BallX 1203 distinct values, BallY 0.31–3.74 in the 2026-09-11 export).

Non-dropdown nodes are numeric in **both** producers (`Vector3Split` modifier,
`Operation` op ids), which is what makes the split look "fine" while the
sensor beside it is dead — the giveaway is that only `TennisGet*` /
`TennisAutoSwing` differ.

## What was changed

1. **`aia_graphc/src/lib.rs`** — the `Op::TennisGet` emit arm now writes the
   Unity label (the IR's `label`). The index is *not* a fallback: a sensor whose
   `label` is empty is a **hard compile error** (`...has no Unity label...`),
   because an index is not an encoding the game can resolve.
   `TennisAutoSwing` already wrote its label and additionally validates the mode
   against the three real options.
2. **`aia_comp-sim/src/graph/dropdowns.rs`** — `label_matched` (the vendor's
   `DROPDOWN_MODIFIER_AS_LABEL` set), `is_dropdown_index`, and
   `is_dead_label_modifier` are the single source of truth for the rule.
   `resolve_for_version` no longer upgrades an index to option text on a
   label-matched node, so the sim cannot mask this bug class again.
3. **`aia_comp-sim/src/graph/load.rs`** — the version-unknown load path
   (`normalize_modifier_for(.., None)`) applies the same predicate. It used to
   call the tolerant `resolve`, which mapped `"0"` → `"Ball Position"` on *any*
   load route.
4. **`aia_comp-sim/src/graph_vm/lower.rs`** — `ApiSlotTable::intern` now
   separates two situations that used to be one:

   | modifier under a strict ABI | game behaviour | sim behaviour |
   |---|---|---|
   | bare index (`"0"`) | dropdown never matches → **dead** sensor | intern `UNKNOWN_ID`, VM reads `Null`, warn on stderr |
   | real option text the capture does not pin (`"Ball Speed"`) | **live** sensor | `panic!` — refuse loudly |

   The second row matters: the sim's own sensor catalog is builder-ordered
   (`data.DROPDOWN_OPTIONS`), so for an unpinned label it cannot know the
   runtime index. Faking a dead sensor there would make a bot look *worse* than
   it is — the same class of lie as the original bug, inverted.

Verified end-to-end: recompiling `titanium/entry.py` with the rebuilt backend
emits all 15 of its `TennisGet*`/`TennisAutoSwing` nodes as Unity option text
(`Ball Position`, `Predicted Bounce`, `Self`, `Prefer Charge`, ...), with the
same vocabulary as the known-good in-game `titanium54.txt` save.

## Which sim mode can simulate what

This trips people up, so it is stated explicitly:

- `GameSpec::tennis_builder()` (`TennisV012`) — **the mode for real bots.** Its
  label tables are the builder's order (`data.DROPDOWN_OPTIONS`), which is what
  graph-JSON indices mean. Every tennis bot test in the repo
  (`tennis_interpreter_certification`, `interpreter_semantics_battery`,
  `compiler_corpus_parity`, `pass_bisect`, ...) uses this spec. New
  label-emitting saves load correctly here, and — after change 2 — an
  index-modifier save reads **dead** here too, which is the whole point.
- `TennisV014` / `TennisV015` — a **pinned partial ABI** (`tennis_v014::*`
  tables pin only ~31 of 106 runtime items). Anything unpinned refuses loudly.
  Do not use these to evaluate a full bot; the label positions are not captured.

So "can the simulator be trusted?" has a precise answer: *in builder mode it
reproduces the game's sensor liveness for both encodings, and it refuses to
guess where it cannot.*

## Regression tests pinning all of this

| Test | Repo | Pins |
|---|---|---|
| `tennis_sensors_emit_unity_labels_not_indices` | `aia_graphc/src/lib.rs` | emitter writes `label`, never `index`; every `TennisGet*` kind + `TennisAutoSwing`, on `v0.14` and `v15f` |
| `tennis_sensor_without_label_is_a_loud_error` | `aia_graphc/src/lib.rs` | label-less sensor does not compile |
| `numeric_modifier_on_label_matched_sensor_is_dead` | `aia_comp-sim` `graph/dropdowns.rs` | `"0"` stays `"0"` and is never admitted, on all three tennis versions |
| `label_matched_is_the_vendor_label_modifier_set` | `aia_comp-sim` `graph/dropdowns.rs` | the strict set equals `DROPDOWN_MODIFIER_AS_LABEL`, no more, no less |
| `numeric_modifier_on_ordinary_node_is_untouched` | `aia_comp-sim` `graph/dropdowns.rs` | `Vector3Split` / `Operation` indices still pass through |
| `numeric_tennis_modifier_interns_dead_and_unpinned_label_refuses` | `aia_comp-sim` `graph_vm/lower.rs` | dead-slot vs. loud-refusal split |

## Verification recipe (do this after any emitter or loader change)

```powershell
# 1. rebuild the compiler backend
cd c:\gitProjects\aia_graphc
cargo build --release --bin graphc-rs

# 2. compile the bot
python -m graphc titanium\entry.py -o "$env:TEMP\ti_check.txt"

# 3. assert every TennisGet*/TennisAutoSwing modifier is a known label
python -c "import json,io,collections; d=json.load(io.open(r'$env:TEMP\ti_check.txt'));
c=collections.Counter((n['id'],n['modifier']) for n in d['serializableNodes']
  if n['id'].startswith('TennisGet') or n['id']=='TennisAutoSwing');
[print(k,v) for k,v in sorted(c.items())]"
```

Any output where the modifier is a bare integer (`'0'`, `'27'`, ...) on a
`TennisGet*`/`TennisAutoSwing` node is the bug.

Measured on the fixed emitter (titanium, target `v15f`, 266 nodes): 15 tennis
sensor nodes, **0 numeric modifiers**, all Unity option text, and the label set
matches the known-good `titanium54.txt` in-game save (`Ball Position`,
`Predicted Bounce`, `Serve Stance`, `Ball Incoming`, `Self`, `Opponent`,
`Prefer Charge`, ...; 11 of the 15 labels are literally shared with that bot).

## Related

- `docs/GRAPH_COMPILER.md` — compiler contract and cost model.
- `AIA_tennis/AIGamePyLibrary/AIGamePyLibrary/data.py` — `DROPDOWN_OPTIONS`,
  `DROPDOWN_MODIFIER_AS_LABEL`, `DROPDOWN_ALIASES` (the ground truth).
- `AIA_tennis/modhost/paritymod-src/getter_items_v014.h` — v0.14/v0.15 getter
  item capture the probe pinned.
