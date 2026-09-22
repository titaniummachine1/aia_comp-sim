"""Measure the game's racket geometry + hit-tier rule from native timeplots.

Answers:
  1. Is RacketDist the 3D distance ball<->racket? (definition check)
  2. Is the racket at a FIXED height (1.25 m) or does it move up/down with the ball?
  3. Is the swing radius different vertically vs horizontally?
     (decompose the contact distance into (dx,dz) ground vs dy vertical)
  4. What is v44_AimTier (the game's own hit grading) and how is it produced?

Usage: python scripts/measure_racket_geometry.py [n_files]
"""
import glob
import json
import math
import os
import re
import sys

TP = os.path.join(os.environ["USERPROFILE"], "AppData", "LocalLow",
                  "Unicorn One", "AIComp", "Saves", "Tennis", "Timeplots")


def load(path):
    with open(path, encoding="utf-8") as f:
        raw = f.read()
    raw = re.sub(r"(?<=\d),(?=\d)", ".", raw)
    return json.loads(raw)["series"]


def inst_map(series):
    out = {}
    for s in series:
        nm = s.get("name", "?")
        if nm not in out:
            out[nm] = s["y"]
    return out


def main():
    n_files = int(sys.argv[1]) if len(sys.argv) > 1 else 6
    files = sorted(glob.glob(os.path.join(TP, "*.json")), key=os.path.getmtime)[-n_files:]
    need = ["v44_RacketX", "v44_RacketY", "v44_RacketZ",
            "v44_BallX", "v44_BallY", "v44_BallZ", "v44_RacketDist",
            "v44_SelfX", "v44_SelfZ", "v44_AimTier", "v44_LateExpect",
            "v44_InSwingRange", "v44_SwingHeld", "v44_ChargePct"]

    racket_y = []
    dist_err = []
    decomp = []          # (ground_dist, vert_dist, racket_dist)
    tier_by_zone = {}    # tier -> list of (zone_ticks, racket_dist)
    late_expect = []
    racket_off = []
    nfiles_ok = 0

    for path in files:
        inst = inst_map(load(path))
        missing = [k for k in need if k not in inst]
        if missing:
            print("skip", os.path.basename(path), "missing", missing)
            continue
        nfiles_ok += 1
        n = min(len(inst[k]) for k in need)
        RX, RY, RZ = inst["v44_RacketX"], inst["v44_RacketY"], inst["v44_RacketZ"]
        BX, BY, BZ = inst["v44_BallX"], inst["v44_BallY"], inst["v44_BallZ"]
        RD = inst["v44_RacketDist"]
        SX, SZ = inst["v44_SelfX"], inst["v44_SelfZ"]
        TIER = inst["v44_AimTier"]
        LEXP = inst["v44_LateExpect"]
        RNG = inst["v44_InSwingRange"]

        zone = 0
        for i in range(n):
            dx = BX[i] - RX[i]
            dy = BY[i] - RY[i]
            dz = BZ[i] - RZ[i]
            d3 = math.sqrt(dx * dx + dy * dy + dz * dz)
            if RD[i] < 500:
                dist_err.append(abs(d3 - RD[i]))
            racket_y.append(RY[i])
            ground = math.sqrt(dx * dx + dz * dz)
            decomp.append((ground, abs(dy), RD[i]))
            racket_off.append((RX[i] - SX[i], RZ[i] - SZ[i]))
            zone = zone + 1 if RNG[i] > 0.5 else 0
            t = int(round(TIER[i]))
            tier_by_zone.setdefault(t, []).append((zone, RD[i]))
            if abs(LEXP[i]) > 1e-6:
                late_expect.append(LEXP[i])

    def stats(v):
        if not v:
            return "n=0"
        s = sorted(v)
        return ("n=%d med=%.3f p05=%.3f p95=%.3f min=%.3f max=%.3f"
                % (len(s), s[len(s) // 2], s[int(len(s) * 0.05)],
                   s[int(len(s) * 0.95)], s[0], s[-1]))

    print("files used:", nfiles_ok)
    print("\n1. RacketDist definition: |3D ball-racket| - RacketDist  ->", stats(dist_err))
    print("2. RacketY (height):", stats(racket_y))
    print("3. racket offset from player: dx=%s dz=%s"
          % (stats([a for a, _ in racket_off]), stats([b for _, b in racket_off])))
    print("\n4. contact decomposition (ground_dist, |vert|) at InSwingRange ticks:")
    inr = [(g, v, r) for (g, v, r) in decomp if r <= 2.6]
    if inr:
        gs = sorted(g for g, _, _ in inr)
        vs = sorted(v for _, v, _ in inr)
        print("   ground dist: med=%.2f p95=%.2f max=%.2f" % (gs[len(gs) // 2], gs[int(len(gs) * 0.95)], gs[-1]))
        print("   vert   dist: med=%.2f p95=%.2f max=%.2f" % (vs[len(vs) // 2], vs[int(len(vs) * 0.95)], vs[-1]))
        # how much of the reach is vertical at the extremes?
        high = [(g, v) for (g, v, r) in inr if v > 1.0]
        print("   in-range ticks with |vert|>1.0 m: %d/%d" % (len(high), len(inr)))
    print("\n5. v44_AimTier values -> (zone_ticks, racket_dist):")
    for t in sorted(tier_by_zone):
        rows = tier_by_zone[t]
        zt = sorted(z for z, _ in rows)
        rd = sorted(r for _, r in rows)
        print("   tier=%d n=%d zone med=%d max=%d | rd med=%.2f min=%.2f max=%.2f"
              % (t, len(rows), zt[len(zt) // 2], zt[-1],
                 rd[len(rd) // 2], rd[0], rd[-1]))
    print("\n6. v44_LateExpect non-zero samples:", stats(late_expect))
    if late_expect:
        s = sorted(set(round(v, 2) for v in late_expect))
        print("   distinct values (first 20):", s[:20])


if __name__ == "__main__":
    main()
