# Game versions & simulator parity scope

## What this simulator targets

**Tennis: v0.14.** Every constant, solver structure and fixture in
`src/tennis/` is pinned against the v0.14 `GameAssembly.dll`
(word-exact recovered helpers + live-captured fixture matrices from
`modhost/captures/`). The `GameSpec::tennis_v014()` variant is the parity
target; `tennis_builder()` covers graphs written by AIGamePyLibrary's
v0.012 builder order.

**The capture probe now runs on v0.15f too (2026-09-12).** The ParityMod
bootstrap is metadata-driven, so it is largely version-agnostic, but two
version-coupled assumptions had to be relaxed for v0.15f (see
`docs/HANDOFF.md` §18):

1. the `Stamina.OnSimulationTick` entry preamble was matched byte-for-byte,
   including a RIP-relative displacement to a static field. That address moves
   between builds, so the matcher now checks the **instruction shape** and
   relocates the observed displacement (plus a "guard must point inside
   GameAssembly" sanity check).
2. three named ball fields were **removed** in v0.15f when the ball
   time-to-ground caching bug was fixed (`cachedPredictedBounceTime`,
   `cachedPredictedSecondBounceTime`, `predictedBounceAge`). The field-group
   reader now treats an absent *version-optional* field as a version
   difference (emits `null`, does not fail the snapshot).

**Parity numbers from v0.15f are still NOT trustworthy:** the sim's solver
fixtures are v0.14-mined, and v0.15 changed real behaviour (ball position is
now interpolated between frames so fast balls can still be hit; the
time-to-ground cache was fixed). Re-mine and triage before quoting v0.15
parity — the four steps below.

## Version taxonomy

| Suffix | Meaning |
|---|---|
| `v0.15f` | Latest **free** release ("f" = free); mod host at `modhost/v0.15f/` |
| Patreon builds | Carry extra features (native timeplots export etc.) |
| `v0.12` … `v0.14` | Older releases; the sim's pinned corpus is v0.12-era
  builder labels with v0.14 runtime parity |

## Re-targeting a new version

1. Point `modhost/` at the new `GameAssembly.dll` (metadata-driven — the
   probe resolves by managed names; RVAs are provenance labels).
2. `python modhost/capture_fixtures.py --home <bot> --away <bot> --seed N`
3. `python modhost/emit_rust_fixtures.py <run> [--out=<versioned fixture dir>]`
4. `cargo test --test tennis_v014_solver_parity` — the replay reports
   exactly which solver behavior drifted (it reads `tests/fixtures/tennis-v014`
   and, when present, `tennis-v015`; a missing/empty per-version `shot_solver.jsonl`
   makes that version skip cleanly).

**v0.15f status (2026-09-12, HANDOFF §21):** the event fixture now runs
end-to-end on v0.15f (two fixes: the event snapshot/restore/match groups tolerate
version-removed fields, and the shot fixture's `v014_shot_ball_fields` marks the
three removed ball cache fields optional). Captured: **shot 210/210**, curve
29/29, serve_direct 26, simulate 13, nth_landing 10. **The v0.15 shot solver is
identical to v0.14** — `tennis_v014_solver_parity` reports 200/210 (max_err
23.6888) for BOTH, so the solver fixtures need no v0.15 re-pin. Build the v0.15
event probe with `build_paritymod_event_v015.ps1`.

Start the mod host for a new version the same way v0.14/v0.15f were done:
copy the pristine install to `modhost/v<ver>/`, keep the untouched launcher as
`Aialanders-original.exe`, build with `build_paritymod_v015.ps1` (or the v0.14
script), then drive it with `MODHOST_GAMEDIR` + `MODHOST_PARITY_EXE`.

