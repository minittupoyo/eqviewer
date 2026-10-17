// site/index.html で使う文字を含むスライスだけを、アプリと同じフォント(Inter / Noto Sans JP)から site/fonts/ に取り出す。
// Noto Sans JP は全体で5.5MBあるので、ページの文字に必要なスライスだけを置く。
// 文言を変えたら再実行する: bun tools/build-site-fonts.ts
import { copyFileSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";

const FONTS = ["inter", "noto-sans-jp"];
const OUT = "site/fonts";

const used = new Set<number>();
for (const ch of readFileSync("site/index.html", "utf8")) used.add(ch.codePointAt(0)!);

const inRange = (range: string) =>
  range.split(",").some((r) => {
    const [a, b] = r.trim().replace("U+", "").split("-");
    const lo = parseInt(a, 16);
    const hi = b ? parseInt(b, 16) : lo;
    for (const c of used) if (c >= lo && c <= hi) return true;
    return false;
  });

mkdirSync(OUT, { recursive: true });
let css = "/* tools/build-site-fonts.ts で生成。フォントは SIL Open Font License(同じ場所の LICENSE-*.txt) */\n";
for (const name of FONTS) {
  const dir = `node_modules/@fontsource-variable/${name}`;
  copyFileSync(`${dir}/LICENSE`, `${OUT}/LICENSE-${name}.txt`);
  let kept = 0;
  for (const block of readFileSync(`${dir}/wght.css`, "utf8").match(/@font-face \{[^}]*\}/g)!) {
    const file = block.match(/url\(\.\/files\/([^)]+)\)/)![1];
    const range = block.match(/unicode-range: ([^;]+);/)![1];
    if (!inRange(range)) continue;
    copyFileSync(`${dir}/files/${file}`, `${OUT}/${file}`);
    css += block.replace("./files/", "./") + "\n";
    kept++;
  }
  console.log(name, kept, "slices");
}
writeFileSync(`${OUT}/fonts.css`, css);
