"""強震モニタの観測点表を生成する。
入力: https://github.com/ingen084/kyoshin-monitor-observation-points (MIT) の intensity-points.json
使い方: python tools/build-kmoni-points.py <intensity-points.json>
出力: src-tauri/data/kmoni_points.csv  (経度,緯度,画像のx,画像のy,色を読むxのずれ,色を読むyのずれ)
画像上の位置を持たない点(point が null)は除く。休止中の点は含めたままにする(画像上では色が付かないだけ)。
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
rows = []
for p in json.load(open(sys.argv[1], encoding="utf-8-sig")):
    pt = p["point"]
    if not pt:
        continue
    c, o, loc = pt["center_point"], pt["offset"], p["location"]
    rows.append(f'{loc["longitude"]},{loc["latitude"]},{c["x"]},{c["y"]},{o["x"]},{o["y"]}')
(ROOT / "src-tauri/data/kmoni_points.csv").write_text("\n".join(rows) + "\n")
print(len(rows), "points")
