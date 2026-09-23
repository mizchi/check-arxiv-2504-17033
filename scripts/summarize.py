#!/usr/bin/env python3
"""Turn results/*.csv from `compare` into a markdown table (time relative to the faster Dijkstra)."""
import csv, sys
from collections import OrderedDict

rows = list(csv.DictReader(open(sys.argv[1])))
graphs = OrderedDict()
for r in rows:
    graphs.setdefault((r["graph"], int(r["n"]), int(r["m"])), {})[r["algorithm"]] = r
algos = []
for r in rows:
    if r["algorithm"] not in algos and not r["algorithm"].startswith("dijkstra"):
        algos.append(r["algorithm"])
print("| graph | n | m | best Dijkstra (ms) | " + " | ".join(algos) + " |")
print("|---|---:|---:|---:|" + "---:|" * len(algos))
for (g, n, m), a in graphs.items():
    base = min(float(a[k]["median_ms"]) for k in a if k.startswith("dijkstra"))
    cells = []
    for al in algos:
        if al not in a:
            cells.append("—")
            continue
        r = a[al]
        t = float(r["median_ms"])
        s = f"{t/base:.2f}×"
        import re
        mm = re.search(r"pre=([0-9.]+)ms", r.get("notes", ""))
        if mm and al.startswith("chd"):
            s += f" ({(t - float(mm.group(1)))/base:.2f}×)"
        if r["check"] != "ok":
            s += f" **{r['check']}**"
        cells.append(s)
    lg = n.bit_length() - 1
    print(f"| {g} | 2^{lg} | {m:,} | {base:.1f} | " + " | ".join(cells) + " |")
