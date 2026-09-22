"""Derive the game's implied flight time t = |xz| / |vxz| for every solver row,
grouped by (case profile, shot arg, power). Prints the per-arg table so the
Lob/Slice time formulas and any speed cap can be read off directly.
"""
import json, os, struct, collections

FIX = os.path.join(r"C:\gitProjects\aia_comp-sim", "tests", "fixtures",
                   "tennis-v014", "shot_solver.jsonl")
ARG = ["Topspin", "Slice", "Flat", "Lob", "Drop", "CurveL", "CurveR"]

def w(x):
    return struct.unpack("<f", struct.pack("<I", x))[0]

def words(v):
    return [w(i) for i in v] if isinstance(v, list) else []

def table():
    # mirror of ShotArg::table() in shot.rs: (base, lift)
    return [(24.0, 0.08), (16.0, 0.04), (28.0, 0.02), (12.0, 6.0), (7.6, 0.27),
            (24.0, 0.08), (24.0, 0.08)]

rows = []
for line in open(FIX, encoding="utf-8"):
    line = line.strip()
    if not line:
        continue
    c = json.loads(line)
    call = next((x for x in c.get("calls", [])
                 if x.get("operation") == "ComputeShotVelocity"), None)
    if not call or call.get("invoke_ok") is not True:
        continue
    g = words(call.get("result_f32_words"))
    if len(g) != 3:
        continue
    inp = c["input"]
    frm = words(inp["from_f32_words"])
    aim = words(inp["aim_target_f32_words"])
    q = words(c["dimensions"]["power_f32_words"])[0]
    arg = c["dimensions"]["shot_type"]
    dx, dz = aim[0] - frm[0], aim[2] - frm[2]
    dist = (dx * dx + dz * dz) ** 0.5
    vxz = (g[0] * g[0] + g[2] * g[2]) ** 0.5
    spd = (vxz * vxz + g[1] * g[1]) ** 0.5
    t = dist / vxz if vxz > 1e-6 else 0.0
    rows.append(dict(case=c["case_id"], arg=arg, q=q, from_x=frm[0], aim_x=aim[0],
                     dist=dist, vxz=vxz, spd=spd, t=t, vy=g[1]))

print(f"rows: {len(rows)}")
print("\n== per-arg summary (only non-fallback, dist>1) ==")
for a in range(7):
    g = [r for r in rows if r["arg"] == a and r["dist"] > 1.0]
    if not g:
        continue
    base, lift = table()[a]
    print(f"\n--- arg {a} {ARG[a]} base={base} lift={lift}  n={len(g)} ---")
    print(f"{'case':<14}{'q':>5}{'dist':>7}{'vxz':>8}{'vy':>8}{'spd':>8}{'t':>7}   cand")
    for r in sorted(g, key=lambda r: (r["case"], r["q"], r["from_x"])):
        q, d, vxz = r["q"], r["dist"], r["vxz"]
        cand = ""
        if a == 1:
            cand = f"slice 0.5+0.12q+lift*0.08={0.5+0.12*q+lift*0.08:.4f}"
        elif a == 3:
            cand = f"lob .82+.14q+.84={0.82+0.14*q+lift*0.14:.4f}"
        elif a == 4:
            cand = f"drop .63+.13q+.35={0.63+0.13*q+lift*0.13:.4f}"
        else:
            cand = f"d/vxz={d/vxz:.4f}"
        print(f"{r['case']:<14}{q:>5.2f}{d:>7.2f}{vxz:>8.3f}{r['vy']:>8.3f}"
              f"{r['spd']:>8.3f}{r['t']:>7.4f}   {cand}")
