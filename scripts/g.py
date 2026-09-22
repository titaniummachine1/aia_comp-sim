import re, glob
PATS = [r"ball_in_strike_range", r"racket_center", r"STRIKE_HEIGHT", r"RACKET_DIST", r"RacketDist", r"perfect_ticks", r"zone_ticks", r"HitTier"]
rx = re.compile("|".join(PATS))
for f in glob.glob("src/**/*.rs", recursive=True):
    for i, line in enumerate(open(f, encoding="utf-8", errors="replace"), 1):
        if rx.search(line):
            print(f"{f}:{i}: {line.rstrip()}")
