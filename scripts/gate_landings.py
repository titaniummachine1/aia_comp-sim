"""Gate dataset v2: request (AimX/Z @ strike) vs LANDING (first y=0.31
descending crossing after strike), titanium54's own strikes only
(RacketDist < 1.2 at strike => self within contact).
Game-side only: no seeds needed.
Usage: python scripts/gate_landings.py
"""
from __future__ import annotations

import io
import json
import math
import os
import sys

sys.path.insert(0, r'C:\gitProjects\AIA_tennis\modhost')
from parse_timeplots import parse_timeplot_json

CAP = (r'C:\gitProjects\AIA_tennis\modhost\captures\timeplots-20260910'
       r'\timeplot_2026-09-10_15-27-03.json')
DT = 0.019
FLOOR = 0.31


def series(data, name):
    for s in data.get('series', []):
        if s.get('name') == name:
            return s.get('y', [])
    return []


def landing_after(y, x, z, i0):
    """First descending crossing of FLOOR after tick i0. Returns (tick,x,z) or None.
    Stops at the next teleport (ball reset = point over) so a late bounce of a
    dead point is never attributed to an early strike."""
    for i in range(i0 + 1, min(len(y) - 1, i0 + 600)):
        if math.dist((x[i], y[i], z[i]), (x[i-1], y[i-1], z[i-1])) > 3.0:
            return None
        if y[i] > FLOOR >= y[i + 1] and y[i] - y[i + 1] < 3.0:
            f = (y[i] - FLOOR) / max(1e-6, (y[i] - y[i + 1]))
            return i, x[i] + f * (x[i + 1] - x[i]), z[i] + f * (z[i + 1] - z[i])
    return None


def main() -> None:
    cap = sys.argv[1] if len(sys.argv) > 1 else CAP
    data = parse_timeplot_json(io.open(cap, encoding='utf-8').read())
    print(f'== {cap} ==')
    get = lambda n: series(data, n)
    bx, by, bz = get('v44_BallX'), get('v44_BallY'), get('v44_BallZ')
    ax, az = get('v44_AimX'), get('v44_AimZ')
    rd = get('v44_RacketDist')
    chg = get('v44_ChargePct')
    shot = get('v44_ShotType')
    adj = get('v49_AimZAdj')
    mode = get('v44_Mode')
    n = len(bx)
    pv = None
    mine = opp = 0
    print('tick req -> landing (residual) [shot q adj mode]')
    for i in range(1, n - 1):
        v = ((bx[i] - bx[i - 1]) / DT, (by[i] - by[i - 1]) / DT, (bz[i] - bz[i - 1]) / DT)
        if pv is not None:
            dv = math.dist(v, pv)
            # skip teleports/resets (either step jumps > 3 m: the jump itself
            # AND the settle tick after it, whose dv is jump-sized but bogus)
            teleport = math.dist((bx[i], by[i], bz[i]), (bx[i-1], by[i-1], bz[i-1])) > 3.0
            prev_teleport = math.dist((bx[i-1], by[i-1], bz[i-1]), (bx[i-2], by[i-2], bz[i-2])) > 3.0
            # skip balls parked at the reset slot (0, 0.46, 0)
            parked = math.dist((bx[i], by[i], bz[i]), (0.0, 0.46, 0.0)) < 0.05
            if dv > 8.0 and not teleport and not prev_teleport and not parked and rd[i - 1] < 20.0:
                own = rd[i - 1] < 1.2
                land = landing_after(by, bx, bz, i)
                tag = 'MINE' if own else 'opp '
                if own:
                    mine += 1
                else:
                    opp += 1
                if own and land is not None:
                    li, lx, lz = land
                    print(f'{i:5} {tag} req=({ax[i-1]:6.2f},{az[i-1]:6.2f}) '
                          f'land=({lx:6.2f},{lz:6.2f}) '
                          f'res=({lx-ax[i-1]:+.2f},{lz-az[i-1]:+.2f}) '
                          f'shot={shot[i-1]:.0f} q={chg[i-1]:.2f} adj={adj[i-1]:+.1f} mode={mode[i-1]:.0f}')
        pv = v
    print(f'own strikes={mine} opp strikes={opp}')


if __name__ == '__main__':
    main()
