"""Search the solver variant space against all 210 fixture rows.

Variants tried (independently + combined):
  cap   : apply the 46 m/s launch-speed cap by raising t (current) vs no cap
  lobmin: Lob horizontal speed floor = charged speed  (game: max(dist/t, speed))
  ramp  : other-family charge ramp 0.189 (=0.42*0.45, the doc) vs 0.2025 (=0.45*0.45)
  xnet  : add +0.05 s for the crossed-net profile (Slice only?)
Prints, per variant combo, how many rows are within 0.25 m/s.
"""
import json, struct, math, itertools

FIX = "tests/fixtures/tennis-v014/shot_solver.jsonl"
G = 28.0
FLOOR_Y = 0.31
MAX_SPEED = 46.0
CPM = 0.85
TABLE = {0: (24.0, 0.08), 1: (16.0, 0.04), 2: (28.0, 0.02),
         3: (12.0, 6.0), 4: (7.6, 0.27), 5: (24.0, 0.08), 6: (24.0, 0.08)}
FULL = {0, 2}
MIN_TIMES = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]


def w(x):
    return struct.unpack("<f", struct.pack("<I", x))[0]


def ws(v):
    return [w(i) for i in v] if isinstance(v, list) else []


rows = []
for line in open(FIX, encoding="utf-8"):
    line = line.strip()
    if not line:
        continue
    r = json.loads(line)
    d, inp = r["dimensions"], r["input"]
    call = next((c for c in r["calls"] if c["operation"] == "ComputeShotVelocity"), None)
    if call is None or call.get("invoke_ok") is not True:
        continue
    game = ws(call["result_f32_words"])
    if len(game) != 3:
        continue
    q = ws(d.get("power_f32_words"))
    rows.append(dict(case=r["case_id"], profile=d.get("profile"),
                     arg=d.get("shot_type"), q=(q[0] if q else 0.0),
                     from3=ws(inp["from_f32_words"]),
                     aim=ws(inp["aim_target_f32_words"]), game=game))


def solve(r, cap, lobmin, ramp, xnet):
    from3, aim3, arg, q = r["from3"], r["aim"], r["arg"], r["q"]
    base, lift = TABLE[arg]
    fx, fz = from3[0], from3[2]
    tx, tz = aim3[0], aim3[2]
    dx, dz = tx - fx, tz - fz
    dist = math.hypot(dx, dz)
    high = 1.3 if arg in FULL else 0.85 + ramp
    speed = (CPM + (high - CPM) * q) * base
    if dist < 1e-3:
        return None
    dirx, dirz = dx / dist, dz / dist
    if arg == 1:
        t = 0.5 + 0.12 * q + lift * 0.08
        if xnet and r["profile"] == "crossed-net":
            t += 0.05
    elif arg == 3:
        t = max(0.82 + 0.14 * q + lift * 0.14, 0.88 + 0.08 * q)
    elif arg == 4:
        t = 0.63 + 0.13 * q + lift * 0.13
    else:
        t = max(MIN_TIMES[arg], dist / max(speed, 1e-4))
    # horizontal magnitude (Lob has a charged-speed floor)
    hmag = dist / t
    if lobmin and arg == 3:
        hmag = max(hmag, speed)
    if cap:
        t_cap = max(2.4, t + 0.35)
        it = 0
        while True:
            vx, vz = dirx * hmag, dirz * hmag
            vy = (FLOOR_Y - from3[1]) / t + 0.5 * G * t
            tot = math.sqrt(vx * vx + vz * vz + vy * vy)
            if tot <= MAX_SPEED or it >= 12 or t >= t_cap:
                break
            t += 0.05
            hmag = dist / t
            if lobmin and arg == 3:
                hmag = max(hmag, speed)
            it += 1
    return (dirx * hmag, (FLOOR_Y - from3[1]) / t + 0.5 * G * t, dirz * hmag)


best = []
for cap, lobmin, ramp, xnet in itertools.product([False, True], [False, True],
                                                 [0.189, 0.2025], [False, True]):
    within = 0
    tot = 0
    maxerr = 0.0
    bad = []
    for r in rows:
        v = solve(r, cap, lobmin, ramp, xnet)
        if v is None:
            continue
        tot += 1
        g = r["game"]
        e = max(abs(v[0] - g[0]), abs(v[1] - g[1]), abs(v[2] - g[2]))
        maxerr = max(maxerr, e)
        if e <= 0.25:
            within += 1
        else:
            bad.append((r["case"], r["profile"], r["arg"], r["q"], round(e, 2)))
    best.append((within, maxerr, cap, lobmin, ramp, xnet, bad))

best.sort(key=lambda x: -x[0])
for within, maxerr, cap, lobmin, ramp, xnet, bad in best:
    print(f"within={within:3d}/210 maxerr={maxerr:8.3f} cap={int(cap)} "
          f"lobmin={int(lobmin)} ramp={ramp} xnet={int(xnet)} "
          f"({len(bad)} bad) {bad[:6]}")
