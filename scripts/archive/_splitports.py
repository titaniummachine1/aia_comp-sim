"""One-off: compare Vector3Split port ids across library fork/upstream + saves."""
from __future__ import annotations

import io
import os
import re
import sys
from collections import Counter

sys.path.insert(0, os.path.dirname(__file__))
import _dump_save as d  # noqa: E402


def scan_ports(text: str, label: str) -> None:
    m = re.search(r'"Vector3Split":\s*\[(.*?)\]', text, re.S)
    print(f"{label}: {m.group(1).strip() if m else 'NOT FOUND'}")
    for name in ("X", "Y", "Z"):
        pat = '"id": "' + name + '"'
        print(f"   port id {name!r}: {bool(re.search(re.escape(pat), text))}")


def main() -> None:
    base = r"c:\gitProjects\AIA_tennis"
    for rel in ("AIGamePyLibrary", "AIGamePyLibrary_upstream"):
        p = os.path.join(base, rel, "AIGamePyLibrary", "data.py")
        if os.path.exists(p):
            scan_ports(io.open(p, encoding="utf-8").read(), rel)

    saves = os.path.expanduser(
        "~/AppData/LocalLow/Unicorn One/AIComp/Saves/Tennis")
    print("\n--- saves ---")
    for f in sorted(os.listdir(saves)):
        if not f.endswith(".txt"):
            continue
        p = os.path.join(saves, f)
        try:
            s = d.load(p)
        except Exception as e:  # noqa: BLE001
            print(f"{f}: ERR {e}")
            continue
        ids = Counter()
        split_ports = None
        for n in s.get("serializableNodes", []):
            for pt in n["serializablePorts"]:
                ids[pt["id"]] += 1
            if n["id"] == "Vector3Split" and split_ports is None:
                split_ports = [pt["id"] for pt in n["serializablePorts"]]
        print(f"{f:26s} Vector3Split ports={split_ports} "
              f"has X={'X' in ids} Y={'Y' in ids} Z={'Z' in ids}")


if __name__ == "__main__":
    main()
