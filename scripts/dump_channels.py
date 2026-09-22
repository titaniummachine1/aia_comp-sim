"""List the channel names present in the newest native game timeplots."""
import glob
import os
import re
import sys

TP = os.path.join(os.environ["USERPROFILE"], "AppData", "LocalLow",
                  "Unicorn One", "AIComp", "Saves", "Tennis", "Timeplots")
files = sorted(glob.glob(os.path.join(TP, "*.json")), key=os.path.getmtime)
n_show = int(sys.argv[1]) if len(sys.argv) > 1 else 3
for path in files[-n_show:]:
    with open(path, encoding="utf-8") as f:
        raw = f.read()
    raw = re.sub(r"(?<=\d),(?=\d)", ".", raw)
    import json
    S = json.loads(raw)["series"]
    names = []
    for s in S:
        nm = s.get("name", "?")
        if nm not in names:
            names.append(nm)
    print(os.path.basename(path), "series:", len(S), "unique:", len(names))
    for nm in names:
        print("   ", nm)
