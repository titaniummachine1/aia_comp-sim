"""One-off: channel names + variance scan across every TimePlot export."""
from __future__ import annotations

import glob
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
import _diag_timeplot as d  # noqa: E402

DIR = os.path.expanduser(
    "~/AppData/LocalLow/Unicorn One/AIComp/Saves/Tennis/Timeplots")


def main() -> None:
    files = sorted(glob.glob(os.path.join(DIR, "*.json")),
                   key=os.path.getmtime)
    for p in files:
        try:
            data = d.load(p)
        except Exception as e:  # noqa: BLE001
            print(f"{os.path.basename(p)}: parse-error {e}")
            continue
        series = data.get("series", [])
        print(f"\n== {os.path.basename(p)} simTime={data.get('simTime')} "
              f"ch={len(series)}")
        for s in series:
            y = s.get("y") or s.get("x") or []
            if not y:
                print(f"   {s.get('name'):16s} EMPTY")
                continue
            distinct = len({round(v, 4) for v in y})
            print(f"   {s.get('name'):16s} n={len(y):5d} min={min(y):11.3f} "
                  f"max={max(y):11.3f} distinct={distinct}")


if __name__ == "__main__":
    main()
