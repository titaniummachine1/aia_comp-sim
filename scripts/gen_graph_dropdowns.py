"""Generate src/graph/dropdowns.rs from AIGamePyLibrary DROPDOWN_OPTIONS.

Reads every AIGamePyLibrary fork it can find (2026 worldcupteams fork carries
the RacingV2 tables the older tennis copy lacks) and merges them, newest keys
winning. RacingV2 tables are included because racing compiles are accepted
(ABI-only â€” no parity simulator).
"""
from __future__ import annotations

import importlib.util
import json
import os
import sys
from pathlib import Path

#: AIGamePyLibrary forks, best-first. `GRAPHC_PYLIB` env still wins.
PYLIB_CANDIDATES = [
    os.environ.get("GRAPHC_PYLIB"),
    r"C:\gitProjects\worldcup\worldcupteams\AIGamePyLibrary",
    r"C:\gitProjects\AIA_tennis\AIGamePyLibrary",
    r"C:\gitProjects\AIA_tennis\AIGamePyLibrary_upstream",
]


def _load_options() -> dict:
    merged: dict = {}
    for i, cand in enumerate(c for c in PYLIB_CANDIDATES if c):
        path = os.path.join(cand, "AIGamePyLibrary", "data.py")
        if not os.path.exists(path):
            print(f"gen_graph_dropdowns: no data.py in {cand}, skipped")
            continue
        spec = importlib.util.spec_from_file_location(
            f"_aigamepylib_data_{i}", path)
        mod = importlib.util.module_from_spec(spec)
        try:
            spec.loader.exec_module(mod)
        except Exception as e:  # noqa: BLE001
            print(f"gen_graph_dropdowns: skip {path}: {e}")
            continue
        for k, v in getattr(mod, "DROPDOWN_OPTIONS", {}).items():
            merged[k] = tuple(v)
    if not merged:
        raise SystemExit("gen_graph_dropdowns: no AIGamePyLibrary found")
    return merged


DROPDOWN_OPTIONS = _load_options()

# Ground-truth override: labels captured at runtime (AIA_Comp_Libry.racing.dropmap)
# always win over fork guesses. See aia_graphc/docs/DECOMP_PLAYBOOK.md.
_GT_MAP = {
    "RacingV2GetFloat": "RacingV2GetFloat",
    "RacingV2GetBool": "RacingV2GetBoolGate",
    "RacingV2GetCar": "RacingV2GetCarGate",
    "RacingV2GetWaypoint": "RacingV2GetWaypointGate",
    "RacingV2Waypoint": "RacingV2WaypointGate",
    "GetCarPart": "GetCarPartGate",
}
_GT: dict = {}

def _load_ground_truth() -> None:
    p = Path(r"c:\gitProjects\aia_graphc\graphc\api\AIA_Comp_Libry\racing\dropmap.py")
    if not p.exists():
        print("gen_graph_dropdowns: no dropmap.py - keeping fork tables")
        return
    spec = importlib.util.spec_from_file_location("_racing_dropmap", p)
    mod = importlib.util.module_from_spec(spec)
    try:
        spec.loader.exec_module(mod)
    except Exception as e:  # noqa: BLE001
        print(f"gen_graph_dropdowns: dropmap load failed: {e}")
        return
    dd = getattr(mod, "DROPDOWNS", {})
    for out_key, gt_key in _GT_MAP.items():
        if gt_key in dd:
            DROPDOWN_OPTIONS[out_key] = tuple(dd[gt_key])
            print(f"gen_graph_dropdowns: ground truth override {out_key} "
                  f"({len(dd[gt_key])} labels)")
    for name in ("GRAPH_OPCODES", "PART_TYPES", "VALUE_KINDS"):
        vals = getattr(mod, name, None)
        if vals:
            _GT[name] = list(vals)

_load_ground_truth()

OUT = Path(__file__).resolve().parents[1] / "src" / "graph" / "dropdowns.rs"

KEYS = [
    ("SoccerGetBool", "SOCCER_GET_BOOL"),
    ("SoccerGetFloat", "SOCCER_GET_FLOAT"),
    ("SoccerGetTransform", "SOCCER_GET_TRANSFORM"),
    ("SoccerGetVector3", "SOCCER_GET_VECTOR3"),
    ("RacingV2GetFloat", "RACING_V2_GET_FLOAT"),
    ("RacingV2GetBool", "RACING_V2_GET_BOOL"),
    ("RacingV2GetCar", "RACING_V2_GET_CAR"),
    ("RacingV2GetWaypoint", "RACING_V2_GET_WAYPOINT"),
    ("RacingV2Waypoint", "RACING_V2_WAYPOINT"),
    ("GetCarPart", "RACING_GET_CAR_PART"),
]


def main() -> None:
    lines = [
        "//! Auto-generated SoccerGet* + RacingV2Get* dropdown labels (index -> label).",
        "//! Source: AIGamePyLibrary.data.DROPDOWN_OPTIONS (2026 fork for RacingV2)",
        "//! Regenerate: python scripts/gen_graph_dropdowns.py",
        "",
    ]
    for src, rust in KEYS:
        try:
            opts = DROPDOWN_OPTIONS[src]
        except KeyError:
            print(f"gen_graph_dropdowns: {src!r} not in any fork â€” skipped")
            continue
        lines.append(f"pub const {rust}: &[&str] = &[")
        for o in opts:
            lines.append(f"    {json.dumps(o)},")
        lines.append("];")
        lines.append("")

    for name in ("GRAPH_OPCODES", "PART_TYPES", "VALUE_KINDS"):
        vals = _GT.get(name)
        if vals:
            lines.append(f"pub const {name}: &[&str] = &[")
            for v in vals:
                lines.append(f"    {json.dumps(v)},")
            lines.append("];")
            lines.append("")
    lines += [
        "pub fn resolve(node_id: &str, modifier: &str) -> &str {",
        "    let opts: &[&str] = match node_id {",
        '        "SoccerGetBool" => SOCCER_GET_BOOL,',
        '        "SoccerGetFloat" => SOCCER_GET_FLOAT,',
        '        "SoccerGetTransform" => SOCCER_GET_TRANSFORM,',
        '        "SoccerGetVector3" => SOCCER_GET_VECTOR3,',
        '        "RacingV2GetFloat" => RACING_V2_GET_FLOAT,',
        '        "RacingV2GetBool" => RACING_V2_GET_BOOL,',
        '        "RacingV2GetCar" => RACING_V2_GET_CAR,',
        '        "RacingV2GetWaypoint" => RACING_V2_GET_WAYPOINT,',
        '        "RacingV2Waypoint" => RACING_V2_WAYPOINT,
        "GetCarPart" => RACING_GET_CAR_PART,',
        "        _ => return modifier,",
        "    };",
        "    if let Ok(i) = modifier.parse::<usize>() {",
        "        if let Some(label) = opts.get(i) {",
        "            return label;",
        "        }",
        "    }",
        "    modifier",
        "}",
        "",
    ]
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text("\n".join(lines), encoding="utf-8")
    print(f"wrote {OUT}")


if __name__ == "__main__":
    main()
