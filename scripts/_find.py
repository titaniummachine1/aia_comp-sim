"""Search the docs for a literal token (quote-safe shell bypass)."""
import glob, io, sys, os

needles = sys.argv[1:] or ["210"]
root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for f in glob.glob(os.path.join(root, "docs", "**", "*.md"), recursive=True):
    with io.open(f, encoding="utf-8", errors="replace") as fh:
        for i, line in enumerate(fh, 1):
            if any(n in line for n in needles):
                print(f"{os.path.basename(f)}:{i}: {line.rstrip()}")
