"""Sim tournament: titanium54 vs every bot in Saves\\Tennis, headless.

Runs the tennis_tournament bin per opponent, collects JSON summaries into
aia_comp-sim/data/tennis/sim_tournament.jsonl. Bot graphs that crash or
hang are recorded as failures, not skipped silently.
"""
from __future__ import annotations

import io
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
BIN = ["cargo", "run", "-q", "--bin", "tennis_tournament", "--"]
SAVES = os.path.join(
    os.path.expanduser("~"),
    "AppData", "LocalLow", "Unicorn One", "AIComp", "Saves", "Tennis",
)
OUT = os.path.join(ROOT, "data", "tennis", "sim_tournament.jsonl")

HOME = sys.argv[1] if len(sys.argv) > 1 else "titanium54"
SEED = sys.argv[2] if len(sys.argv) > 2 else "7"
POINTS = sys.argv[3] if len(sys.argv) > 3 else "4"

os.makedirs(os.path.dirname(OUT), exist_ok=True)
bots = sorted(
    f[:-4] for f in os.listdir(SAVES) if f.endswith(".txt")
)
print(f"{len(bots)} bots; home={HOME} seed={SEED} points={POINTS}")

results = 0
failures = 0
with io.open(OUT, "a", encoding="utf-8") as out:
    for bot in bots:
        rec = {"home": HOME, "away": bot, "seed": SEED}
        try:
            proc = subprocess.run(
                BIN + ["--home", HOME, "--away", bot,
                       "--seed", SEED, "--points", POINTS,
                       "--max-ticks", "60000"],
                capture_output=True, text=True, timeout=120,
            )
            line = proc.stdout.strip().splitlines()[-1] if proc.stdout.strip() else ""
            if proc.returncode == 0 and line.startswith("{"):
                rec.update(json.loads(line))
                results += 1
            else:
                err_lines = [
                    l for l in proc.stderr.strip().splitlines()
                    if "RUST_BACKTRACE" not in l and l.strip()
                ]
                rec["error"] = (err_lines[-1] if err_lines else "?")[:300]
                failures += 1
        except subprocess.TimeoutExpired:
            rec["error"] = "timeout"
            failures += 1
        except Exception as e:
            rec["error"] = f"{type(e).__name__}: {e}"
            failures += 1
        out.write(json.dumps(rec, separators=(",", ":")) + "\n")
        out.flush()
        status = rec.get("winner") or rec.get("error", "?")
        print(f"  {bot:28} {status}")

print(f"done: {results} results, {failures} failures -> {OUT}")
