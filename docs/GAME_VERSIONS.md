# Game versions & simulator parity scope

## What this simulator targets

**Tennis: v0.14.** Every constant, solver structure and fixture in
`src/tennis/` is pinned against the v0.14 `GameAssembly.dll`
(word-exact recovered helpers + live-captured fixture matrices from
`modhost/captures/`). The `GameSpec::tennis_v014()` variant is the parity
target; `tennis_builder()` covers graphs written by AIGamePyLibrary's
v0.012 builder order.

**The simulator is NOT yet adjusted for v0.15f (free) or Patreon builds.**
Do not trust tennis parity numbers against those versions until the capture
pipeline has been re-run against them.

## Version taxonomy

| Suffix | Meaning |
|---|---|
| `v0.15f` | Latest **free** release ("f" = free) |
| Patreon builds | Carry extra features (native timeplots export etc.) |
| `v0.12` … `v0.14` | Older releases; the sim's pinned corpus is v0.12-era
  builder labels with v0.14 runtime parity |

## Trying original game versions

Zipped copies of past releases live in the release posts / Downloads
(e.g. `Tennis_v0_12.zip` … `Tennis_v0_15f.zip`). Anyone can run the
original game from those zips directly — no simulator needed. The same
applies to the soccer game and any future game artifacts: keep the zips,
run the originals, and use the simulator only as the offline copy.

## Re-targeting a new version

1. Point `modhost/` at the new `GameAssembly.dll` (metadata-driven — the
   probe resolves by managed names; RVAs are provenance labels).
2. `python modhost/capture_fixtures.py --home <bot> --away <bot> --seed N`
3. `python modhost/emit_rust_fixtures.py <run>`
4. `cargo test --test tennis_v014_solver_parity` — the replay reports
   exactly which solver behavior drifted.
