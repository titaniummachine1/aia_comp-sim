"""Static corpus analysis over all mined tennis bot graphs.

Answers, without the game:
- which node types and node->node EDGES real bots actually use (the test
  corpus must cover exactly these)
- feedback cycles (need latch semantics), empty/odd modifiers, type-mismatch
  wirings (Float->Bool etc.), function call shapes, conditional patterns
- per-bot comprehension under the current sim (which graphs load, which
  nodes are still unsupported)

Output: data/tennis/graph_corpus_report.json
"""
from __future__ import annotations

import collections
import io
import json
import os
import sys

SAVES = os.path.join(
    os.path.expanduser("~"),
    "AppData", "LocalLow", "Unicorn One", "AIComp", "Saves", "Tennis",
)
OUT = os.path.join(
    os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
    "data", "tennis", "graph_corpus_report.json",
)

NUMERIC_PORTS = ("Float", "Bool", "Vector31", "Transform1", "Any")

def port_kind(pid: str) -> str:
    if pid.startswith("Vector"): return "vec"
    if pid.startswith("Transform"): return "transform"
    if pid.startswith("Bool"): return "bool"
    if pid.startswith("Float"): return "float"
    if pid.startswith("String"): return "string"
    if pid.startswith("Color"): return "color"
    if pid.startswith("Country"): return "country"
    if pid.startswith("Any"): return "any"
    return "other:" + pid[:6]

def main() -> None:
    graphs = sorted(f for f in os.listdir(SAVES) if f.endswith(".txt"))
    node_types = collections.Counter()
    edge_pairs = collections.Counter()
    cycles = collections.Counter()
    anomalies = collections.Counter()
    per_bot = []
    type_mismatch_edges = 0
    empty_modifier_ops = 0

    for name in graphs:
        try:
            d = json.load(io.open(os.path.join(SAVES, name), encoding="utf-8"))
        except Exception as e:
            per_bot.append({"bot": name[:-4], "parse_error": str(e)[:120]})
            continue
        nodes = {n.get("sID"): n for n in d.get("serializableNodes", [])}
        ports = {}
        for n in d.get("serializableNodes", []):
            for p in n.get("serializablePorts", []):
                ports[p.get("sID")] = (n.get("sID"), p.get("id"), p.get("polarity"))
        bot_nodes = collections.Counter()
        bot_edges = collections.Counter()
        feed = collections.defaultdict(set)  # sid -> source sids

        for n in d.get("serializableNodes", []):
            bot_nodes[n.get("id")] += 1
            node_types[n.get("id")] += 1
            mod = n.get("modifier")
            if n.get("id") in ("Operation", "AbsFloat") and (mod is None or str(mod).strip() == ""):
                empty_modifier_ops += 1
                anomalies[f"{name}: empty Operation modifier"] += 1
        for c in d.get("serializableConnections", []):
            a = ports.get(c.get("port0SID"))
            b = ports.get(c.get("port1SID"))
            if not a or not b:
                continue
            (sa, pa, pol_a), (sb, pb, pol_b) = a, b
            # source = polarity 1 (out) side
            if pol_a == 1 and pol_b == 0:
                src, dst = (sa, pa), (sb, pb)
            elif pol_b == 1 and pol_a == 0:
                src, dst = (sb, pb), (sa, pa)
            else:
                continue
            src_node = nodes.get(src[0], {})
            dst_node = nodes.get(dst[0], {})
            pair = (src_node.get("id"), dst_node.get("id"))
            bot_edges[pair] += 1
            edge_pairs[pair] += 1
            feed[dst[0]].add(src[0])
            ka, kb = port_kind(src[1]), port_kind(dst[1])
            if ka != kb and ka != "any" and kb != "any" and not (
                {ka, kb} <= {"float", "bool"}  # scalar coercion pairs are expected
            ):
                type_mismatch_edges += 1
                anomalies[f"{name}: {ka}->{kb} {pair[0]}->{pair[1]}"] += 1

        # feedback cycles via DFS
        def has_path(u, target, seen):
            if u == target:
                return True
            seen.add(u)
            return any(v not in seen and has_path(v, target, seen) for v in feed.get(u, ()))
        ccount = 0
        for sid in list(feed):
            if has_path(sid, sid, set()):
                ccount += 1
        cycles[name[:-4]] = ccount
        per_bot.append({
            "bot": name[:-4],
            "nodes": len(nodes),
            "feedback_cycles": ccount,
            "distinct_edges": len(bot_edges),
        })

    report = {
        "graphs": len(graphs),
        "node_types": dict(node_types.most_common()),
        "top_edge_pairs": [
            {"from": a, "to": b, "count": c}
            for (a, b), c in edge_pairs.most_common(40)
        ],
        "distinct_edge_pairs": len(edge_pairs),
        "feedback_cycles": cycles,
        "anomalies": dict(anomalies.most_common(30)),
        "type_mismatch_edges": type_mismatch_edges,
        "empty_modifier_ops": empty_modifier_ops,
        "per_bot": per_bot,
    }
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    io.open(OUT, "w", encoding="utf-8").write(json.dumps(report, indent=1))
    print("graphs:", len(graphs))
    print("distinct node types:", len(node_types))
    print("distinct edge pairs:", len(edge_pairs))
    print("graphs with feedback cycles:", sum(1 for v in cycles.values() if v))
    print("type-mismatch edges:", type_mismatch_edges)
    print("empty Operation modifiers:", empty_modifier_ops)
    print("-> ", OUT)


if __name__ == "__main__":
    main()
