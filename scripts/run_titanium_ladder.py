"""Titanium strength ladder: full best-of-3 matches (first to 2 sets) vs everyone.

Runs the tennis_tournament binary per opponent with 4 parallel workers,
collects JSON summaries into data/tennis/titanium_ladder.jsonl (resume-safe:
already-recorded home+away+seed rows are skipped).

Usage:
  python scripts/run_titanium_ladder.py [HOME] [SEED]
  defaults: HOME=titanium58 SEED=7

Full match = --points 10000 (cap only; the world ends the match at 2 sets)
+ --max-ticks 200000 (stall cap).
"""
from __future__ import annotations

import concurrent.futures
import io
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
# NOTE: invoke via `cargo run` (not the exe directly): the debug exe alone
# exits 0xC0000135 outside cargo's env; cargo adds ~1-2 s freshness-check
# overhead per match, negligible next to full best-of-3 runtimes.
CMD = ["cargo", "run", "-q", "--bin", "tennis_tournament", "--"]
SAVES = os.path.join(
    os.path.expanduser("~"),
    "AppData", "LocalLow", "Unicorn One", "AIComp", "Saves", "Tennis",
)
OUT = os.path.join(ROOT, "data", "tennis", "titanium_ladder.jsonl")

HOME = sys.argv[1] if len(sys.argv) > 1 else "titanium63"
SEED = sys.argv[2] if len(sys.argv) > 2 else "7"
# Optional 3rd arg: comma-separated opponent subset (smoke tests).
ONLY = sys.argv[3].split(",") if len(sys.argv) > 3 else None

# World-model version. v015 (default) = the live game: the v0.15 free and
# Patreon builds share the world model, whose only delta from v0.14 is the
# swept (frame-interpolated) ball contact. Rows produced before 2026-09-14
# were v014 physics and are NOT comparable — that default is what made the
# sim report "titanium66 beats Unlucky 2-1" while the real game bagelled it
# 6-0. Set AIA_LADDER_VERSION=v014 only to reproduce those old rows.
GAME_VERSION = os.environ.get("AIA_LADDER_VERSION", "v015")

# Compiler bots split walk/aim via t.aim() — the sim's legacy aim latch
# IGNORES that wire (strike aims fall back to deep-middle = arena suicide).
# Every compiler-bot run MUST set the separate-aim latch model. Measured
# 2026-09-14: without it titanium loses to stock 0-2; with it 2-1.
os.environ.setdefault("AIA_AIM_MODEL", "separate")
POINTS = "10000"
MAX_TICKS = "200000"
WORKERS = 4
TIMEOUT = 600

# Excluded: perf probes, diagnostic stubs, ours (home), exact-duplicate names.
EXCLUDE_PREFIXES = ("stress_", "loop_p", "probe_", "diagbot", "controller",
                    "ignore_ball", "sim_probe", "semantics_battery")
EXCLUDE_EXACT = {"titanium.txt", "titanium_recv.txt", "titaniumpy.txt",
                 f"{HOME}.txt"}


def opponents():
    bots = []
    for f in os.listdir(SAVES):
        if not f.endswith(".txt"):
            continue
        if f in EXCLUDE_EXACT:
            continue
        if f.startswith(EXCLUDE_PREFIXES):
            continue
        if " (" in f:  # "X (1).txt" duplicates
            continue
        name = f[:-4]
        if name == "LeBlock James":  # dup of LeBlock_James
            continue
        bots.append(name)
    bots.append("stock")  # no save file: engine fallback returner (baseline)
    return sorted(bots)


def done_keys():
    keys = set()
    if os.path.exists(OUT):
        with io.open(OUT, encoding="utf-8") as fh:
            for line in fh:
                line = line.strip()
                if not line:
                    continue
                try:
                    r = json.loads(line)
                    # Version is part of the identity: a v014 row must NOT
                    # satisfy a v015 request (old rows have no field → v014).
                    keys.add((r.get("home"), r.get("away"), str(r.get("seed")),
                              r.get("game_version", "v014")))
                except ValueError:
                    pass
    return keys


def play(bot):
    cmd = CMD + ["--home", HOME, "--away", bot,
                 "--seed", SEED, "--points", POINTS, "--max-ticks", MAX_TICKS,
                 "--game-version", GAME_VERSION]
    rec = {"home": HOME, "away": bot, "seed": SEED}
    try:
        proc = subprocess.run(cmd, capture_output=True, text=True, timeout=TIMEOUT)
        line = proc.stdout.strip().splitlines()[-1] if proc.stdout.strip() else ""
        if proc.returncode == 0 and line.startswith("{"):
            rec.update(json.loads(line))
        else:
            err = [l for l in proc.stderr.strip().splitlines()
                   if "RUST_BACKTRACE" not in l and l.strip()]
            rec["error"] = (err[-1] if err else "?")[:300]
    except subprocess.TimeoutExpired:
        rec["error"] = "timeout"
    except Exception as e:  # noqa: BLE001
        rec["error"] = f"{type(e).__name__}: {e}"
    return rec


def main():
    bots = opponents()
    if ONLY:
        bots = [b for b in bots if b in ONLY]
    done = done_keys()
    todo = [b for b in bots if (HOME, b, SEED, GAME_VERSION) not in done]
    print(f"{len(bots)} opponents; home={HOME} seed={SEED} workers={WORKERS}; "
          f"world={GAME_VERSION}; "
          f"{len(todo)} to play ({len(done)} cached)")
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    wins = losses = 0
    with io.open(OUT, "a", encoding="utf-8") as out:
        with concurrent.futures.ThreadPoolExecutor(max_workers=WORKERS) as pool:
            future_to_bot = {pool.submit(play, b): b for b in todo}
            for i, fut in enumerate(concurrent.futures.as_completed(future_to_bot)):
                rec = fut.result()
                out.write(json.dumps(rec, separators=(",", ":")) + "\n")
                out.flush()
                bot = future_to_bot[fut]
                if rec.get("winner") == HOME:
                    wins += 1
                elif rec.get("winner"):
                    losses += 1
                status = rec.get("winner") or rec.get("error", "?")
                sets = rec.get("sets", "?")
                print(f"[{i + 1}/{len(todo)}] {bot:32} {status} sets={sets} "
                      f"(W{wins}-L{losses})", flush=True)
    print(f"done: +{wins}W +{losses}L this run -> {OUT}")


if __name__ == "__main__":
    main()
