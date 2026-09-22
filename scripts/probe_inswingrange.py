"""What is InSwingRange actually keyed on? Compare it to SwingHeld/ChargePct."""
import json, re, os, glob

TP = os.path.join(os.environ["USERPROFILE"], "AppData", "LocalLow",
                  "Unicorn One", "AIComp", "Saves", "Tennis", "Timeplots")

def load_series(path):
    with open(path, encoding="utf-8") as f:
        raw = re.sub(r"(?<=\d),(?=\d)", ".", f.read())
    inst = {}
    for s in json.loads(raw)["series"]:
        nm = re.sub(r"^v\d+_", "", s.get("name", "?"))
        inst.setdefault(nm, s["y"])
    return inst

files = sorted(glob.glob(os.path.join(TP, "*.json")), key=os.path.getmtime)
seed_files = [f for f in files if "seed" in os.path.basename(f)][-2:]
for path in seed_files:
    try:
        inst = load_series(path)
    except Exception as e:
        print("skip", e); continue
    if "InSwingRange" not in inst:
        continue
    print("==", os.path.basename(path))
    keys = [k for k in ["InSwingRange", "SwingHeld", "ChargePct", "RacketDist",
                        "BallY", "BallX", "SelfX"] if k in inst]
    n = min(len(inst[k]) for k in keys)
    runs = []
    i = 0
    while i < n:
        if inst["InSwingRange"][i] >= 0.5:
            j = i
            while j < n and inst["InSwingRange"][j] >= 0.5:
                j += 1
            runs.append((i, j - 1)); i = j
        else:
            i += 1
    same = sum(1 for t in range(n) if (inst["InSwingRange"][t] >= 0.5) ==
               (inst["SwingHeld"][t] >= 0.5))
    print(f"  runs={len(runs)}  InSwingRange==SwingHeld: {same}/{n} "
          f"({100.0*same/n:.1f}%)")
    print("  run lengths:", [b - a + 1 for a, b in runs][:40])
    for (a, b) in runs[:6]:
        print(f"   [{a}-{b}] len={b-a+1}")
        for k in keys:
            print(f"      {k:13s} {[round(inst[k][t],2) for t in range(a, min(b+1,a+6))]}")
