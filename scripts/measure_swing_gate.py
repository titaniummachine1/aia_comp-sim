"""Find the actual InSwingRange gate: threshold on RacketDist (horizontal)."""
import json, re, os, glob, math

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

files = sorted(glob.glob(os.path.join(TP, "*.json")), key=os.path.getmtime)[-6:]
on, off = [], []
for path in files:
    try:
        inst = load_series(path)
    except Exception:
        continue
    NEED = ["RacketX", "RacketZ", "BallX", "BallZ", "RacketDist", "InSwingRange"]
    if any(k not in inst for k in NEED):
        continue
    RX, RZ = inst["RacketX"], inst["RacketZ"]
    BX, BZ = inst["BallX"], inst["BallZ"]
    RD, INR = inst["RacketDist"], inst["InSwingRange"]
    n = min(len(inst[k]) for k in NEED)
    for i in range(n):
        h = math.hypot(BX[i] - RX[i], BZ[i] - RZ[i])
        (on if INR[i] >= 0.5 else off).append((h, RD[i]))

print(f"in-range ticks: {len(on)}  out: {len(off)}")
if on:
    hs = sorted(h for h, _ in on)
    print(f"  in-range horiz: min={hs[0]:.3f} med={hs[len(hs)//2]:.3f} "
          f"p90={hs[int(.9*len(hs))]:.3f} max={hs[-1]:.3f}")
if off:
    hs = sorted(h for h, _ in off)
    print(f"  out-range horiz: min={hs[0]:.3f} med={hs[len(hs)//2]:.3f} max={hs[-1]:.3f}")
# threshold: max "on" and min "off" above 1.0 (ignore in-hand)
on_hi = sorted(h for h, _ in on if h > 1.0)
off_lo = sorted(h for h, _ in off if h > 1.0)
print(f"  above 1.0m -> max on={on_hi[-1] if on_hi else None}, "
      f"min off={off_lo[0] if off_lo else None}")
for lo in (1.5, 2.0, 2.3, 2.5, 2.6, 2.7, 3.0):
    a = sum(1 for h, _ in on if h > lo)
    b = sum(1 for h, _ in off if h > lo)
    print(f"    >{lo:.1f}: on={a} off={b}")
