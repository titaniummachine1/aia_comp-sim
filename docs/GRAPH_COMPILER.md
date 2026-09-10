# Graph compiler design ("graphc") — source code → AI graph

Goal: write bot behavior in a real language (Python first, Rust optional)
and compile it to the game's node graph — instead of hand-placing hundreds
of nodes (Adam: 371 nodes; Apex: 860; LeBlock_James hand-compiler blocked on
previous-tick registers).

## 1. Verified machine model (2026-09-10, pinned by tests)

| Primitive | Semantics | Proof |
|---|---|---|
| Per-tick evaluation | Graph is a DAG per tick; every node evaluated once | corpus: all 82 graphs have feedback cycles broken at tick boundaries |
| Variables (RAM) | `SetVariable`/`GetVariable`, name = static modifier; **persistent across ticks**, store is same-tick visible to later loads | `runtime_brain_variable_accumulates_across_ticks` (climbs 1,2,3,4) |
| Cycle back-edges | Lowered to previous-tick latches (`LoadVar` latch) | lower.rs cycle lowering + `graph_vm` cycle tests |
| Subroutines | `Function` node: 4 Any inputs (args) + 1 Any output (return); body scoped by `owner_function_sid`; sim IR has `Call`/`Return` | data.py port table; sim opcode enum |
| Conditionals | `Select` / compare ops / `ConditionalSetString` | corpus (53 node types) |
| Arithmetic | Add/Sub/Mul/Div/Mod/Pow/Min/Max/Abs/Clamp/Lerp; Div/Mod are IEEE (n/0=±inf, 0/0=nan) | sim tests + author confirmation |
| Vectors | `ConstructVector3`/`SplitVector3`, Add/Sub/Scale/Dot/Cross — **3 f32 slots per vector = dense packing substrate** | op set |
| No dynamic names | Get/Set variable names are static node modifiers — **no computed addressing** | data.py port schema (single Any port, no name input) |
| No runtime topology change | Node set frozen at load — arrays/allocators must be compile-time pools | graph format |

Consequence: the language is **clocked** — straight-line code per tick +
persistent state across ticks. That is Turing-complete over time (loops =
iterate across ticks, unbounded steps), which is exactly the game's
execution model. Within-tick recursion is impossible unless `Function`
supports nesting (unverified — test next).

## 2. Memory model

- **RAM**: every source variable → one named variable (Set/Get pair),
  assigned by the compiler. Previous-tick registers are free (latches).
- **Arrays**: compile-time pool packed into Vector3 variables via
  mixed-radix: value_i = Mod(Floor(cell / base^k), base) with base ≤ 2^8
  per component (float32 exact to 2^24 → 3 components ≈ 16.7M capacity).
  Address = base_slot + stride·i computed in arithmetic; read = decode,
  write = re-encode + StoreVar. A write must read-modify-write the packed
  vector (one GetVariable + arithmetic + one StoreVar).
- **Allocator (dynamic growth emulation)**: preallocated K-cell pool +
  free-list head (itself a packed variable). alloc/pop/push = a few
  arithmetic ops via Function calls. Indistinguishable from dynamic memory
  up to capacity K; K bounded by node budget, not semantics.
- **Structs**: fixed offsets into the pool; the compiler type-checks slots.

## 3. Control flow

| Source construct | Compilation |
|---|---|
| `if/else` | `Select` nodes (both arms evaluated per tick — cheap, no divergence) |
| `while` | state machine across ticks: condition latched; loop body runs one iteration per tick; unrolled depth 1 |
| `for i in 0..N` | counter variable + state machine (or fully unrolled if N small and tick budget allows) |
| function call | `Function` node instance (4 args, 1 return); per-call-site instantiation by the compiler (no runtime stack) |
| recursion | cross-tick latch emulation only; within-tick = hard error at compile time (until Function nesting is verified) |

## 4. Architecture

```
  .py (decorator DSL)  or  .ten (Rust front-end later)
        │  parse / type-check (scalar f32, bool, vec2/vec3, fixed arrays)
        ▼
 Typed IR (per-tick SSA + persistent state list)
        │  passes: const-fold, dead code, CSE, latch insertion at cycle
        │  back-edges, budget check (nodes, per-tick cost)
        ▼
  graphc backend: assign variable names (cells), pack arrays into
  Vector3 slots, emit RawGraph JSON (nodes + connections + modifiers)
        │
        ├─► tests: load in aia_comp-sim VM, run N ticks, assert traces
        └─► load into game (`Saves\Tennis\<bot>.txt`) for live play
```

- **Front-end first: Python.** Decorator DSL:
  `@bot(tennis)` functions with a restricted, typed expression subset —
  variables, arithmetic, comparisons, if/while, packed lists, and calls to
  the stdlib. The compiler emits the graph through AIGamePyLibrary's
  emitter (proven save format) — so graphs load both in the game and in
  the sim's `load_graph`.
- **Sim is the CI**: every compiled graph runs in `tennis_tournament` /
  unit tests before touching the game — same replay discipline as physics.
- **Rust front-end later**: same IR, `graphc` bin in the sim repo — nice
  for determinism and CI, but Python iteration speed wins for the PoC.

## 5. Budgets (measured anchors)

- Node budget: community bots run 371–860 nodes fine; the game handles the
  full graph set per tick. Compiler must report per-tick op count.
- float32 packing: exact integers ≤ 2^24; mixed-radix base ≤ 256/slot for
  safe headroom; decimals need scaling (charge/aim values are O(1) —
  store scaled ints).
- Variable count: unbounded-ish (named strings), but each Set/Get pair
  costs nodes — packing amortizes.

## 6. First milestone (PoC, next session)

1. Python DSL emitting RawGraph JSON via AIGamePyLibrary: variables,
   arithmetic, `if`→Select, cross-tick `while` state machine.
2. Compile a demo bot: counter latch + packed 8-cell array with
   alloc/write/read — run 100 ticks in the sim VM, dump TimePlot channels,
   assert values.
3. Verify the SAME graph loads and behaves in the game (spawn vs stock).
4. Then extend toward titanium-style behavior (intercept ladder) expressed
   as source code — the usability test that matters.