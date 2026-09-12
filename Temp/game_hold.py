import io, os, sys
sys.path.insert(0, r"C:\gitProjects\AIA_tennis\modhost")
from parse_timeplots import parse_timeplot_json

p = os.path.join(os.path.expanduser("~"), "AppData", "LocalLow", "Unicorn One",
                 "AIComp", "Saves", "Tennis", "Timeplots",
                 "timeplot_2026-09-12_12-40-45.json")
d = parse_timeplot_json(io.open(p, encoding="utf-8").read())
g = {s["name"]: s.get("y", []) for s in d["series"]}
y = g["v44_BallY"]
print("held height (ticks 25-80):", sorted(set(round(v, 2) for v in y[25:81])))
print("toss window ticks 78-104:")
print(" ".join(f"{i}:{y[i]:.2f}" for i in range(78, min(105, len(y)))))
