#!/usr/bin/env python3
"""市町村・震度速報地域の外接矩形だけを持つ軽量インデックスを作る。

画面側は震度の対応づけと表示範囲の計算にしか形を使わないので、
形の本体(tools/map-src/jp-city/jp-area.geojson)は事前タイル化してあるので、本体を読まずにこのインデックスで済ませる。
"""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "tools" / "map-src"
MAP = ROOT / "public" / "map"


def bbox(coords):
    xs, ys = [], []

    def visit(c):
        if isinstance(c[0], (int, float)):
            xs.append(c[0])
            ys.append(c[1])
        else:
            for x in c:
                visit(x)

    visit(coords)
    return min(xs), min(ys), max(xs), max(ys)


for name in ("jp-city", "jp-area"):
    src = json.loads((SRC / f"{name}.geojson").read_text())
    feats = []
    for f in src["features"]:
        w, s, e, n = bbox(f["geometry"]["coordinates"])
        feats.append(
            {
                "type": "Feature",
                "properties": f["properties"],
                "geometry": {"type": "Polygon", "coordinates": [[[w, s], [e, s], [e, n], [w, n], [w, s]]]},
            }
        )
    out = MAP / f"{name}-index.geojson"
    out.write_text(json.dumps({"type": "FeatureCollection", "features": feats}, ensure_ascii=False, separators=(",", ":")))
    print(out.name, out.stat().st_size)
