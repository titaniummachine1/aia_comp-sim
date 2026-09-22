"""One-off: per-export ball-channel health + channel prefix, sorted by mtime."""
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
        names = [s.get("name", "?") for s in series]
        prefixes = sorted({n.split(".")[0].split("_")[0] for n in names})
        ball = [s for s in series
                if "ball" in s.get("name", "").lower()]
        info = []
        for s in ball:
            y = s.get("y") or []
            if y:
                info.append(f"{s['name']}={len(set(round(v, 3) for v in y))}d")
        print(f"{os.path.basename(p):38s} t={data.get('simTime'):>9} "
              f"ch={len(series):3d} prefixes={','.join(prefixes)[:28]:28s} "
              f"ball[{len(ball)}] {' '.join(info)[:90]}")


if __name__ == "__main__":
    main()
