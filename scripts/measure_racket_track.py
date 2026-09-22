"""How does the game's racket follow the ball vertically? (NBA finding: RacketY
is NOT constant - it moves 0.65..2.78 m.) Derive the rule so the sim can stop
using a fixed STRIKE_HEIGHT.

Prints
  * RacketY vs BallY: correlation + the tracking band
  * rest height (rackset Y when the ball is far) vs chase height (ball near)
  * the vertical clamp band (min/max racket Y that ever occurs)
  * RACKET_SIDE sign rule (which side of the player the racket sits)
"""
import glob
import json
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


def corr(xs, ys):
    n = len(xs)
    if n < 2:
        return 0.0
    mx = sum(xs) / n
    my = sum(ys) / n
    sxy = sum((a - mx) * (b - my) for a, b in zip(xs, ys))
    sxx = sum((a - mx) ** 2 for a in xs)
    syy = sum((b - my) ** 2 for b in ys)
    if sxx <= 0 or syy <= 0:
        return 0.0
    return sxy / (sxx * syy) ** 0.5


def main():
    n_files = int(sys.argv[1]) if len(sys.argv) > 1 else 6
    files = sorted(glob.glob(os.path.join(TP, "*.json")), key=os.path.getmtime)[-n_files:]
    need = ["v44_RacketX", "v44_RacketY", "v44_RacketZ", "v44_BallX", "v44_BallY",
            "v44_BallZ", "v44_RacketDist", "v44_SelfX", "v44_SelfZ", "v44_SwingHeld"]

    near_ry, near_by, near_rd = [], [], []
    far_ry, far_rd = [], []
    side_pos, side_neg = 0, 0
    for path in files:
        inst = inst_map(load(path))
        if any(k not in inst for k in need):
            print("skip", os.path.basename(path))
            continue
        n = min(len(inst[k]) for k in need)
        RY, BY, RD = inst["v44_RacketY"], inst["v44_BallY"], inst["v44_RacketDist"]
        RZ, SZ = inst["v44_RacketZ"], inst["v44_SelfZ"]
        BZ = inst["v44_BallZ"]
        for i in range(n):
            if RD[i] < 500:
                if RD[i] <= 2.6:
                    near_ry.append(RY[i])
                    near_by.append(BY[i])
                    near_rd.append(RD[i])
                elif RD[i] > 6.0:
                    far_ry.append(RY[i])
                    far_rd.append(RD[i])
            if (RZ[i] - SZ[i]) * (BZ[i] - SZ[i]) > 0:
                side_pos += 1
            else:
                side_neg += 1

    def st(v, name):
        if not v:
            print(name, "n=0")
            return
        s = sorted(v)
        print("%s n=%d med=%.3f p05=%.3f p95=%.3f min=%.3f max=%.3f"
              % (name, len(s), s[len(s) // 2], s[int(len(s) * 0.05)],
                 s[int(len(s) * 0.95)], s[0], s[-1]))

    print("files:", len(files))
    st(near_ry, "racketY when IN RANGE (d<=2.6):")
    st(near_by, "ballY  when IN RANGE            :")
    st(far_ry, "racketY when ball FAR (d>6)     :")
    print("corr(racketY, ballY) in range = %.3f" % corr(near_ry, near_by))
    # tracking error
    err = sorted(abs(a - b) for a, b in zip(near_ry, near_by))
    if err:
        print("|racketY - ballY| in range: med=%.3f p95=%.3f max=%.3f"
              % (err[len(err) // 2], err[int(len(err) * 0.95)], err[-1]))
    # clamp hypothesis: racketY == clamp(ballY, lo, hi)?
    lo = min(near_ry) if near_ry else 0
    hi = max(near_ry) if near_ry else 0
    cl = [abs(a - min(max(b, lo), hi)) for a, b in zip(near_ry, near_by)]
    if cl:
        cl.sort()
        print("clamp test clamp(ballY,[%.2f,%.2f]): med=%.3f p95=%.3f"
              % (lo, hi, cl[len(cl) // 2], cl[int(len(cl) * 0.95)]))
    print("RACKET_SIDE sign: same-sign(bz-sz)=%d opposite=%d" % (side_pos, side_neg))


if __name__ == "__main__":
    main()
