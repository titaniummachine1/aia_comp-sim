"""Read a game TimePlot export and show where titanium's rally aim points.

The export is the HOME graph (74 channels) written by the game's
`TimePlot:ExportToJson()` into
  %USERPROFILE%\\AppData\\LocalLow\\Unicorn One\\AIComp\\Saves\\Tennis\\Timeplots
Numbers use decimal COMMAS (Windows locale) — normalised on load.

Channels of interest:
  Ball X/Z, Self X/Z, Opponent X/Z   — world geometry (game frame)
  Aim X/Z                            — the aim actually latched for the strike
  T.aim_x / T.aim_z                  — the tick's requested aim
  Move X/Z                           — movement command (post-gate)
  Shot, Serving, Receiving, Bounced, Playable
  Attack score best / saved, Landing forecast X/Z

Usage:
  python scripts/analyze_game_aim.py [timeplot.json] [--top N] [--deadband M]
  (default = newest export)

Reports: for every tick where an aim is present, the distance from the aim
point to the opponent and to the court centre, so a "shoots straight at him"
pattern is measurable instead of anecdotal.
"""
from __future__ import annotations

import glob
import json
import os
import re
import sys


def default_timeplot() -> str:
    tp = os.path.join(os.environ["USERPROFILE"], "AppData", "LocalLow",
                      "Unicorn One", "AIComp", "Saves", "Tennis", "Timeplots")
    files = sorted(glob.glob(os.path.join(tp, "*.json")), key=os.path.getmtime)
    return files[-1]


def load(path: str) -> dict[str, list[float]]:
    raw = open(path, encoding="utf-8", errors="replace").read()
    raw = re.sub(r"(?<=\d),(?=\d)", ".", raw)
    doc = json.loads(raw)
    out = {}
    for s in doc.get("series", []):
        nm = s.get("name")
        if nm is not None:
            out[nm] = s.get("y") or []
    return out


def col(chans: dict, name: str, i: int, default=None):
    ys = chans.get(name)
    if ys is None or i >= len(ys):
        return default
    return ys[i]


def main() -> None:
    args = [a for a in sys.argv[1:]]
    path = None
    deadband = 2.0
    i = 0
    while i < len(args):
        if args[i] == "--deadband" and i + 1 < len(args):
            deadband = float(args[i + 1]); i += 2
        elif args[i] == "--top" and i + 1 < len(args):
            i += 2
        else:
            path = args[i]; i += 1
    if path is None:
        path = default_timeplot()
    chans = load(path)
    n = max(len(v) for v in chans.values())
    print(f"file={os.path.basename(path)} ticks={n} channels={len(chans)}")

    need = ["Ball X", "Ball Z", "Self X", "Self Z", "Opponent X", "Opponent Z",
            "Aim X", "Aim Z", "T.aim_x", "T.aim_z", "Move X", "Move Z"]
    for k in need:
        print(f"  {k:16} {'PRESENT' if k in chans else 'MISSING'}")

    rows = []
    at_opp = 0
    near_center = 0
    rally_ticks = 0
    for t in range(n):
        serving = col(chans, "Serving", t, 0)
        receiving = col(chans, "Receiving", t, 0)
        if not serving and not receiving:
            continue  # not a rally strike tick
        rally_ticks += 1
        ax = col(chans, "Aim X", t)
        az = col(chans, "Aim Z", t)
        ox = col(chans, "Opponent X", t)
        oz = col(chans, "Opponent Z", t)
        bx = col(chans, "Ball X", t)
        bz = col(chans, "Ball Z", t)
        sx = col(chans, "Self X", t)
        sz = col(chans, "Self Z", t)
        if ax is None or ox is None:
            continue
        d_opp = ((ax - ox) ** 2 + (az - oz) ** 2) ** 0.5 if az is not None else None
        d_center = (ax ** 2 + (az or 0.0) ** 2) ** 0.5
        if d_opp is not None and d_opp <= deadband:
            at_opp += 1
        if d_center <= deadband:
            near_center += 1
        rows.append((t, bx, bz, sx, sz, ox, oz, ax, az, d_opp, d_center,
                     col(chans, "Shot", t), col(chans, "Attack score best", t)))

    print(f"\nrally-strike ticks={rally_ticks}")
    if rows:
        print(f"aim within {deadband} m of opponent: {at_opp}/{len(rows)}")
        print(f"aim within {deadband} m of centre:   {near_center}/{len(rows)}")
    print(f"\n{'tick':>6} {'ball x,z':>16} {'self x,z':>16} {'opp x,z':>16} "
          f"{'aim x,z':>16} {'dOpp':>6} {'dCtr':>6} {'shot':>6} {'sbest':>7}")
    for (t, bx, bz, sx, sz, ox, oz, ax, az, do, dc, shot, sb) in rows:
        def f2(v):
            return f"{v:+.2f}" if isinstance(v, (int, float)) else "  ?  "
        print(f"{t:>6} {f2(bx)+','+f2(bz):>16} {f2(sx)+','+f2(sz):>16} "
              f"{f2(ox)+','+f2(oz):>16} {f2(ax)+','+f2(az):>16} "
              f"{do:>6.2f} {dc:>6.2f} {str(shot):>6} {str(sb):>7}")


if __name__ == "__main__":
    main()