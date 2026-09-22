"""Grep the titanium tree (and NOTES) for the deviation-law constants."""
import os, io, re, sys

ROOT = r"C:\gitProjects\AIA_tennis\titanium"
PATS = ["bias", "deviation", "off_z", "offz", "early", "late", "1.85", "1.05",
        "2.6", "z_off", "Z_OFF", "aim_z", "Dev", "grade", "GRADE"]
EXTS = (".py", ".md", ".rs", ".hpp", ".h", ".cpp", ".json")

for dirpath, dirnames, filenames in os.walk(ROOT):
    dirnames[:] = [d for d in dirnames if d not in (".git", "target", "__pycache__")]
    for fn in filenames:
        if not fn.lower().endswith(EXTS):
            continue
        p = os.path.join(dirpath, fn)
        try:
            with io.open(p, encoding="utf-8", errors="replace") as fh:
                for i, line in enumerate(fh, 1):
                    low = line.lower()
                    if any(pat.lower() in low for pat in PATS):
                        rel = os.path.relpath(p, ROOT)
                        print(f"{rel}:{i}: {line.rstrip()[:200]}")
        except Exception as e:
            print("ERR", p, e)
