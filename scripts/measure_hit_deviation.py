"""Measure the early/late hit Z-deviation from the game's own timeplots.

Law documented in the titanium builds (v39-v42) and user-confirmed:
  landing_z = aim_z + pull,  pull in Z ONLY (nothing else changes)
    PERFECT (first tick in the 1.0 m window):  pull = 0
    EARLY   (rushed big swing, 2.6 m ring):    pull = -self_x_sgn * 6.0
    LATE    (receding ball at release):        pull = +self_x_sgn * (6.5..12)

This script re-derives the numbers straight from the game so the sim's
constants are measured, not assumed. It also checks the user's claim that
the deviation is Z-only (off_x must be ~0 while off_z is large).

Usage: python scripts/measure_hit_deviation.py [--n 8]
"""
import argparse
import glob
import json
import os
import re
import statistics as st

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
        nm = re.sub(r"^v\d+_", "", s.get("name", "?"))  # v44_/v49_ prefixes
        if nm not in out:
            out[nm] = s["y"]
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--n", type=int, default=8)
    args = ap.parse_args()

    files = sorted(glob.glob(os.path.join(TP, "*.json")), key=os.path.getmtime)[-args.n:]
    print("files:")
    for f in files:
        print("  ", os.path.basename(f))

    rows = []
    for path in files:
        S = load(path)
        inst = inst_map(S)
        need = ["AimX", "AimZ", "ChargePct", "SwingHeld", "RacketDist",
                "InSwingRange", "SelfX", "BallX", "BallY", "BallZ"]
        missing = [k for k in need if k not in inst]
        if missing:
            print("skip", os.path.basename(path), "missing", missing)
            continue
        n = min(len(inst[k]) for k in need)
        AIMX, AIMZ = inst["AimX"], inst["AimZ"]
        CHG, HELD, RD = inst["ChargePct"], inst["SwingHeld"], inst["RacketDist"]
        RNG = inst["InSwingRange"]
        SX = inst["SelfX"]
        BX, BY, BZ = inst["BallX"], inst["BallY"], inst["BallZ"]

        for i in range(1, n - 2):
            if not (HELD[i - 1] > 0.5 and HELD[i] < 0.5):
                continue
            chg = CHG[i - 1]
            aimx, aimz = AIMX[i], AIMZ[i]
            if abs(aimx) < 900 and abs(aimx) <= 7.3 and chg > 0.6:
                continue  # serve (aim inside the service-box x-range)
            # consecutive ticks in range just before release (the timing canon)
            zone = 0
            j = i - 1
            while j >= 0 and RNG[j] > 0.5:
                zone += 1
                j -= 1
            # first bounce after release
            k, k0 = i + 1, i + 1
            while k < n - 2:
                if BY[k] < BY[k - 1] and BY[k] <= BY[k + 1] and BY[k] < 1.2:
                    break
                k += 1
                if k - k0 > 300:
                    break
            if k >= n - 2 or k - k0 > 300:
                continue
            sgn = 1.0 if SX[i] >= 0 else -1.0
            rows.append({
                "file": os.path.basename(path)[:31],
                "i": i, "chg": chg, "rd": RD[i], "zone": zone,
                "aim": (aimx, aimz), "land": (BX[k], BZ[k]),
                "off_x": BX[k] - aimx, "off_z": BZ[k] - aimz, "sgn": sgn,
            })

    print(f"\nreleases with a measured first bounce: {len(rows)}")
    if not rows:
        return

    def group(name, pred):
        g = [r for r in rows if pred(r)]
        if not g:
            print(f"{name}: n=0")
            return
        oz = sorted(r["off_z"] for r in g)
        ox = sorted(abs(r["off_x"]) for r in g)
        # law check: signed by the player's X side
        law = sum(1 for r in g if abs(r["off_z"]) > 0.5)
        early_law = sum(1 for r in g if r["off_z"] * (-r["sgn"]) > 0.5)
        late_law = sum(1 for r in g if r["off_z"] * r["sgn"] > 0.5)
        print(f"{name}: n={len(g)}  off_z med={st.median(oz):+6.2f} "
              f"min={oz[0]:+6.2f} max={oz[-1]:+6.2f}  |off_z|>0.5: {law}  "
              f"early-law(-sgn): {early_law}  late-law(+sgn): {late_law}  "
              f"|off_x| med={st.median(ox):.2f} max={ox[-1]:.2f}")

    group("PERFECT  rd<=1.05 zone==1", lambda r: r["rd"] <= 1.05 and r["zone"] == 1)
    group("PERFECT-ish rd<=1.05     ", lambda r: r["rd"] <= 1.05)
    group("GOOD     1.05<rd<=1.85   ", lambda r: 1.05 < r["rd"] <= 1.85)
    group("WIDE     1.85<rd<=2.6    ", lambda r: 1.85 < r["rd"] <= 2.6)
    group("OUT      rd>2.6          ", lambda r: r["rd"] > 2.6)
    group("ZONE>=2 (late-ish)       ", lambda r: r["zone"] >= 2)
    group("ZONE==1 (first tick)     ", lambda r: r["zone"] == 1)

    print("\nper-release detail (first 30 with |off_z|>0.5):")
    big = [r for r in rows if abs(r["off_z"]) > 0.5][:30]
    for r in big:
        print(f"  {r['file']} i={r['i']:5d} sgn={r['sgn']:+.0f} chg={r['chg']:.2f} "
              f"rd={r['rd']:.2f} zone={r['zone']} aim=({r['aim'][0]:6.2f},{r['aim'][1]:5.2f}) "
              f"land=({r['land'][0]:6.2f},{r['land'][1]:5.2f}) "
              f"off=({r['off_x']:+5.2f},{r['off_z']:+5.2f})")


if __name__ == "__main__":
    main()
