"""強震モニタの凡例画像(リアルタイム震度)から、色→震度の変換表を生成する。

使い方: uv run --with pillow python tools/build-kmoni-scale.py <凡例gif> 
凡例は縦のカラーバーで、上端の目盛り(y=17.5)が震度7.0、下端の目盛り(y=297.5)が-3.0、
1目盛り(震度1.0)が28px。バー中央付近(x=6)の色を1pxごとに読む。
出力: src-tauri/data/kmoni_jma_scale.bin (RGB x 279行、上=震度大)
      src/lib/kmoniColors.ts (地図用の色の補間点)
凡例の入手先: https://www.kmoni.bosai.go.jp/data/map_img/ScaleImg/nied_jma_s_w_scale.gif
"""
import sys
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
im = Image.open(sys.argv[1]).convert("RGB")
X, TOP, BOTTOM = 6, 19, 296  # 枠線を除いたバーの範囲
rows = [im.getpixel((X, y)) for y in range(TOP, BOTTOM + 1)]
(ROOT / "src-tauri/data/kmoni_jma_scale.bin").write_bytes(bytes(c for rgb in rows for c in rgb))

def intensity(y):
    return 7 - (y - 17.5) / 28

stops = []
for v in range(-3, 8):
    y = min(max(round(17.5 + (7 - v) * 28), TOP), BOTTOM)
    r, g, b = im.getpixel((X, y))
    stops.append((v, f"#{r:02x}{g:02x}{b:02x}"))
body = ",\n".join(f'  [{v}, "{c}"]' for v, c in stops)
(ROOT / "src/lib/kmoniColors.ts").write_text(
    "// 自動生成: tools/build-kmoni-scale.py\n"
    "/** リアルタイム震度(-3〜7)の配色。[震度, 色] */\n"
    f"export const KMONI_COLOR_STOPS: [number, string][] = [\n{body},\n];\n"
)
print(len(rows), "rows;", intensity(TOP), "to", intensity(BOTTOM))
