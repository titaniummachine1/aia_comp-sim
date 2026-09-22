"""List the series names in the newest N game timeplots."""
import json, re, os, glob, sys

TP = os.path.join(os.environ["USERPROFILE"], "AppData", "LocalLow",
                  "Unicorn One", "AIComp", "Saves", "Tennis", "Timeplots")
k = int(sys.argv[1]) if len(sys.argv) > 1 else 1
files = sorted(glob.glob(os.path.join(TP, "*.json")), key=os.path.getmtime)[-k:]
for path in files:
    with open(path, encoding="utf-8") as f:
        raw = f.read()
    raw = re.sub(r"(?<=\d),(?=\d)", ".", raw)
    S = json.loads(raw)["series"]
    names = [s.get("name") for s in S]
    print(f"=== {os.path.basename(path)}: {len(names)} series ===")
    for i, nm in enumerate(names):
        print(f"  {i:3d} {nm}")
