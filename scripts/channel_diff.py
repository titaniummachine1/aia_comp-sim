"""Distributional sensor/channel parity: sim hch vs game native channels.
Usage: python scripts/channel_diff.py
"""
from __future__ import annotations

import io
import json
import os
import subprocess
import sys
import tempfile

sys.path.insert(0, r'C:\gitProjects\AIA_tennis\modhost')
from parse_timeplots import parse_timeplot_json

SIM_BIN = ["cargo", "run", "-q", "--bin", "tennis_tournament", "--"]
GAME_CAP = (r'C:\gitProjects\AIA_tennis\modhost\captures\timeplots-20260910'
            r'\timeplot_2026-09-10_15-27-03.json')
ROOT = r'C:\gitProjects\aia_comp-sim'
CH = ['v44_InSwingRange', 'v44_MustWait', 'v44_Bounced', 'v44_Incoming',
      'v44_OnSelfSide', 'v44_Chase', 'v44_Mode', 'v44_SwingHeld',
      'v44_RacketDist', 'v44_DNow', 'v44_TIntercept', 'v44_Desperate',
      'v44_LateExpect', 'v44_ChargePct', 'v44_AimX', 'v44_AimZ',
      'v44_SelfX', 'v44_BallX']


def game_stats(cap=None):
    data = parse_timeplot_json(io.open(cap or GAME_CAP, encoding='utf-8').read())
    s = {x.get('name'): x.get('y', []) for x in data.get('series', [])}
    return {c: s.get(c, []) for c in CH}


def sim_stats():
    env = dict(os.environ)
    env.pop("AIA_AIM_MODEL", None)
    env["AIA_TRACE_CHANNELS"] = "1"
    with tempfile.NamedTemporaryFile(suffix='.jsonl', delete=False, mode='w') as tf:
        trace = tf.name
    subprocess.run(SIM_BIN + ["--home", "titanium54", "--away", "aia3",
                              "--seed", "7", "--points", "4",
                              "--trace", trace],
                   capture_output=True, text=True, timeout=300, env=env, cwd=ROOT)
    rows = [json.loads(l) for l in open(trace, encoding='utf-8') if l.strip()]
    os.unlink(trace)
    out = {}
    for c in CH:
        vals = [r.get('hch', {}).get(c) for r in rows]
        out[c] = [v for v in vals if v is not None]
    return out


def summarize(name, vals):
    if not vals:
        return f'{name:18} n=0'
    frac1 = sum(1 for v in vals if v > 0.5) / len(vals)
    mean = sum(vals) / len(vals)
    return f'{name:18} n={len(vals):5} mean={mean:8.3f} frac(>0.5)={frac1:.3f}'


def main() -> None:
    g = game_stats(sys.argv[1] if len(sys.argv) > 1 else None)
    s = sim_stats()
    print(f'{"channel":18} {"GAME":28} SIM')
    for c in CH:
        print(f'{summarize(c, g[c]):18} {summarize("g", g[c])[18:46]} {summarize("s", s[c])[18:]}')


if __name__ == '__main__':
    main()
