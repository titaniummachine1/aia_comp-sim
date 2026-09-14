"""Weakness analysis over data/tennis/titanium_ladder.jsonl (read-only).

Reconstructs games from point_winners (4pts + 2-lead) and assigns serve per
game (seed 7 draws first_server=AWAY — verified empirically: default run is
tick-identical to AIA_FIRST_SERVER=away). Assumes no mid-game serve-clock
forfeits (serve_forfeits is not in the JSON; flagged if pace suggests it).
"""
import io
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
IN = os.path.join(HERE, "..", "..", "data", "tennis", "titanium_ladder.jsonl")
HOME = "titanium58"
FIRST_SERVER = 1  # away (seed 7, verified)


def games_of(pw):
    """Yield (server, game_winner) per game. Server alternates from AWAY."""
    games = []
    a = b = 0
    n = 0
    for w in pw:
        if w == 0:
            a += 1
        else:
            b += 1
        if (a >= 4 or b >= 4) and abs(a - b) >= 2:
            server = FIRST_SERVER ^ (n % 2)
            games.append((server, 0 if a > b else 1))
            a = b = 0
            n += 1
    return games, (a, b)


def cls(name):
    if name in ("pusher", "slicer", "alternator", "cross_court", "open_court"):
        return "styles"
    if name.startswith(("graphc_", "helperdemo", "underdog")):
        return "graphc-bots"
    if name.startswith("sim_") or name.startswith("simop_") or name.startswith("sim_v"):
        return "sim-lineage"
    if name == "stock":
        return "stock"
    if name.startswith("titanium"):
        return "titanium-lineage"
    if name in ("Adam", "Apex", "Zudan6", "nqvxf22", "martico2432v7", "bat",
                "LeBlock_James", "Unlucky", "Safe_Corner_v0.3",
                "safe_corner_v02", "safe-corner-league-e12",
                "_Court-Weaver_8a4b6c1584e7"):
        return "champs"
    return "other"


with io.open(IN, encoding="utf-8") as fh:
    rows = [json.loads(l) for l in fh if l.strip()]

print(f"rows={len(rows)}")
tw = sum(1 for r in rows if r.get("winner") == HOME)
print(f"match record: {tw}W-{len(rows) - tw}L\n")

# --- per-class ---
bycls = {}
for r in rows:
    bycls.setdefault(cls(r["away"]), []).append(r)
for c, rs in sorted(bycls.items()):
    w = sum(1 for r in rs if r.get("winner") == HOME)
    tp = sum(sum(1 for x in r.get("point_winners", []) if x == 0) for r in rs)
    ap = sum(len(r.get("point_winners", [])) for r in rs)
    print(f"{c:16} {w}W-{len(rs) - w}L  points {tp}/{ap} ({100.0 * tp / max(ap, 1):.0f}%)")
    for r in sorted(rs, key=lambda x: x["away"]):
        pw = r.get("point_winners", [])
        t = sum(1 for x in pw if x == 0)
        print(f"    {'W' if r.get('winner') == HOME else 'L'} {r['away']:30} "
              f"sets={r.get('sets')} pts {t}/{len(pw)} ticks={r.get('ticks')}")

# --- serve split (all rows, first_server=AWAY) ---
hold_opps = hold_own = brk_opps = brk_own = 0  # games served/won
serve_games = ret_games = 0
for r in rows:
    for n, (srv, gw) in enumerate(games_of(r.get("point_winners", []))[0]):
        if srv == 0:  # titanium serves
            serve_games += 1
            hold_own += gw == 0
        else:
            ret_games += 1
            brk_own += gw == 0
print(f"\ntitanium serving: held {hold_own}/{serve_games} "
      f"({100.0 * hold_own / max(serve_games, 1):.1f}%)")
print(f"titanium returning: broke {brk_own}/{ret_games} "
      f"({100.0 * brk_own / max(ret_games, 1):.1f}%)")

# --- fade: point-win% by match stage ---
for lo, hi in [(0, 8), (8, 24), (24, 10 ** 9)]:
    t = a = 0
    for r in rows:
        for x in r.get("point_winners", [])[lo:hi]:
            if x == 0:
                t += 1
            else:
                a += 1
    print(f"points {lo + 1}-{hi if hi < 10 ** 9 else 'end'}: titanium {t}/{t + a} "
          f"({100.0 * t / max(t + a, 1):.1f}%)")

# --- serve factor ---
f = sum(sum(r.get("faults", [0, 0])) for r in rows)
df = sum(sum(r.get("double_faults", [0, 0])) for r in rows)
ac = sum(sum(r.get("aces", [0, 0])) for r in rows)
npts = sum(len(r.get("point_winners", [])) for r in rows)
print(f"\n{npts} points: faults={f} double_faults={df} aces={ac}")
shut = [r["away"] for r in rows
        if r.get("winner") != HOME and not any(x == 0 for x in r.get("point_winners", []))]
print(f"shutouts (0 pts): {len(shut)}: {shut}")
close = [(r["away"], r.get("sets")) for r in rows if r.get("sets") in ([2, 1], [1, 2])]
print(f"3-setters ({len(close)}): {close}")
