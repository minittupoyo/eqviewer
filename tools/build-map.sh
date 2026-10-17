#!/usr/bin/env bash
# 地図データを生成する: tools/build-map.sh <作業ディレクトリ(空)>
# 出典: 気象庁「地震情報／細分区域」「市町村等(地震津波関係)」/ Natural Earth 50m
set -euo pipefail
W=${1:?work dir}
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/public/map"
# 国内の形(jp-*.geojson)は画面では読まず、タイル化の元データとして置く
SRC="$ROOT/tools/map-src"
mkdir -p "$W" "$OUT" "$SRC"
cd "$W"

curl -sLo city.zip https://www.data.jma.go.jp/developer/gis/20241128_AreaInformationCity_quake_GIS.zip
curl -sLo area.zip https://www.data.jma.go.jp/developer/gis/20240520_AreaForecastLocalE_GIS.zip
curl -sLo ne.zip https://naturalearth.s3.amazonaws.com/50m_cultural/ne_50m_admin_0_countries.zip
mkdir -p city area ne && unzip -oq area.zip -d area && unzip -oq city.zip -d city && unzip -oq ne.zip -d ne
# 気象庁のファイル名はShift_JISなので固定名にする
[ -e city/src.shp ] || for e in shp dbf shx; do mv -- city/*."$e" "city/src.$e"; done

# 国内: 市町村を許容誤差25mで簡略化(ズーム10でも1px=約125mなので見た目は元データと同等)。
# 県境は「簡略化済みの市町村」をdissolveして作る。別々に簡略化すると海岸線・県境が層ごとにずれる。
bunx mapshaper city/src.shp \
  -each 'pref=regioncode.slice(0,2)' \
  -filter-fields name,regioncode,pref \
  -simplify dp interval=25 keep-shapes \
  -o "$SRC/jp-city.geojson" format=geojson precision=0.00001 \
  -dissolve pref \
  -o "$SRC/jp-pref.geojson" format=geojson precision=0.00001

# 震度速報の地域(地震情報/細分区域)。市町村と同じ許容誤差で簡略化する。
[ -e area/src.shp ] || for e in shp dbf shx; do mv -- area/*."$e" "area/src.$e"; done
bunx mapshaper area/src.shp \
  -filter-fields code,name \
  -simplify dp interval=25 keep-shapes \
  -o "$SRC/jp-area.geojson" format=geojson precision=0.00001

# 世界: 日本は国内の市町村データで描くので除く(海岸線が二重にずれるのを防ぐ)
bunx mapshaper ne/ne_50m_admin_0_countries.shp \
  -filter 'ISO_A3 != "JPN"' \
  -filter-fields NAME,ISO_A3 \
  -simplify 15% keep-shapes \
  -o "$OUT/world.geojson" format=geojson precision=0.01

# 画面が読む軽量インデックスと事前ベクタタイルを作る
cd "$ROOT"
python3 tools/build-map-index.py
bun tools/build-tiles.ts
