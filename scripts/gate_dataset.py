"""Game-side strike dataset from titanium54 native timeplots.
Step 1: inspect channels, detect strikes, dump (request, state) at strikes.
Usage: python scripts/gate_dataset.py
Reads modhost captures/timeplots-20260910/timeplot_2026-09-10_15-27-03.json
"""
from __future__ import annotations

import io
import json
import math
import os
import re
import sys

sys.path.insert(0, r'C:\gitProjects\AIA_tennis\modhost')
from parse_timeplots import parse_timeplot_json

CAP = (r'C:\gitProjects\AIA_tennis\modhost\captures\timeplots-20260910'
       r'\timeplot_2026-09-10_15-27-03.json')
DT = 0.019


def series(data, name):
    for s in data.get('series', []):
        if s.get('name') == name:
            return s.get('y', [])
    return []


def main() -> None:
    data = parse_timeplot_json(io.open(CAP, encoding='utf-8').read())
    get = lambda n: series(data, n)
    bx, by, bz = get('v44_BallX'), get('v44_BallY'), get('v44_BallZ')
    ax, az = get('v44_AimX'), get('v44_AimZ')
    n = len(bx)
    print(f'ticks={n}')
    for ch in ('v44_AimReq', 'v49_AimZAdj', 'v44_AimTier', 'v44_ShotType', 'v44_ChargePct',
               'v44_BotVersion', 'v44_Mode', 'v44_SwingHeld', 'v44_Bounced', 'v44_Incoming'):
        y = get(ch)
        if y:
            print(f'{ch:16} min={min(y):.3f} max={max(y):.3f} last={y[-1]:.3f}')
    # vel via diff; strike = |dvel| > 8
    print('--- strikes (|dvel|>8) ---')
    pv = None
    for i in range(1, n):
        v = ((bx[i] - bx[i-1]) / DT, (by[i] - by[i-1]) / DT, (bz[i] - bz[i-1]) / DT)
        if pv is not None:
            dv = math.dist(v, pv)
            if dv > 8.0:
                print(f'tick {i:5} from=({bx[i-1]:.2f},{by[i-1]:.2f},{bz[i-1]:.2f}) '
                      f'vel=({v[0]:.1f},{v[1]:.1f},{v[2]:.1f}) dv={dv:.1f} '
                      f'req=({ax[i-1]:.2f},{az[i-1]:.2f}) '
                      f'shot={get("v44_ShotType")[i-1]:.0f} q={get("v44_ChargePct")[i-1]:.2f} '
                      f'adj={get("v49_AimZAdj")[i-1]:.3f} tier={get("v44_AimTier")[i-1]:.0f}')
        pv = v


if __name__ == '__main__':
    main()
