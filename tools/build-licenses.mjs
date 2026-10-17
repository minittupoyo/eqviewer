// 同梱するソフトウェア(Rustのクレート・npmパッケージ)のライセンス一覧を生成する。
// 使い方: bun run tools/build-licenses.mjs   出力: src/lib/licenses.generated.json
// Rust: Windows向けビルドの通常の依存(`cargo tree -e normal`)。npm: package.json の dependencies を再帰的にたどる。
// ライセンス全文は各パッケージ同梱のLICENSE/COPYINGファイルから集め、同じ全文はまとめる。
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const ROOT = path.resolve(import.meta.dirname, "..");
const TARGET = "x86_64-pc-windows-msvc";
const LICENSE_FILE = /^(licen[sc]e|copying|notice|unlicense)([-_.].*)?$/i;

const texts = [];
const textIndex = new Map();
function textId(dir) {
  const files = fs.existsSync(dir)
    ? fs.readdirSync(dir).filter((f) => LICENSE_FILE.test(f) && fs.statSync(path.join(dir, f)).isFile())
    : [];
  if (!files.length) return [];
  return files.sort().map((f) => {
    const t = fs.readFileSync(path.join(dir, f), "utf8").replace(/\r\n/g, "\n").trim();
    if (!textIndex.has(t)) textIndex.set(t, texts.push(t) - 1);
    return textIndex.get(t);
  });
}
const uniq = (a) => [...new Set(a)];

// --- Rust
const cargo = (args) => execFileSync("cargo", args, { cwd: path.join(ROOT, "src-tauri"), maxBuffer: 1 << 28, encoding: "utf8" });
const used = new Set(
  cargo(["tree", "-e", "normal", "--target", TARGET, "--prefix", "none", "--format", "{p}"])
    .split("\n")
    .map((l) => l.replace(/ \(.*$/, "").replace(/ \(\*\)$/, "").trim())
    .filter(Boolean)
    .map((l) => l.split(" ").slice(0, 2).join(" ")),
);
const meta = JSON.parse(cargo(["metadata", "--format-version", "1", "--filter-platform", TARGET]));
const packages = [];
for (const p of meta.packages) {
  if (!used.has(`${p.name} v${p.version}`) || p.name === "eqviewer") continue;
  packages.push({
    kind: "rust",
    name: p.name,
    version: p.version,
    license: p.license ?? "(不明)",
    repo: p.repository ?? p.homepage ?? null,
    texts: textId(path.dirname(p.manifest_path)),
  });
}

// --- npm
const seen = new Set();
function walk(dir, name, shallow = false) {
  // node_modules を親方向にたどって解決する
  for (let d = dir; ; d = path.dirname(d)) {
    const pj = path.join(d, "node_modules", name, "package.json");
    if (fs.existsSync(pj)) return visit(path.dirname(pj), shallow);
    if (d === path.dirname(d)) return;
  }
}
function visit(pkgDir, shallow = false) {
  const j = JSON.parse(fs.readFileSync(path.join(pkgDir, "package.json"), "utf8"));
  const key = `${j.name}@${j.version}`;
  if (seen.has(key)) return;
  seen.add(key);
  const repo = typeof j.repository === "string" ? j.repository : j.repository?.url;
  packages.push({
    kind: "npm",
    name: j.name,
    version: j.version,
    license: typeof j.license === "string" ? j.license : (j.license?.type ?? "(不明)"),
    repo: repo ? repo.replace(/^git\+/, "").replace(/\.git$/, "") : (j.homepage ?? null),
    texts: textId(pkgDir),
  });
  if (shallow) return;
  for (const dep of Object.keys({ ...j.dependencies, ...j.optionalDependencies })) walk(pkgDir, dep);
}
// ビルド時だけ使う道具は、成果物に取り込まれる部分(CSS・コピーしたコンポーネント)の分だけ載せ、依存はたどらない。
const BUILD_ONLY_SHALLOW = new Set(["tailwindcss", "tw-animate-css", "shadcn-vue"]);
// 実行時には含まれない(Viteの組み込みプラグイン)
const EXCLUDED = new Set(["@tailwindcss/vite"]);
const root = JSON.parse(fs.readFileSync(path.join(ROOT, "package.json"), "utf8"));
for (const dep of Object.keys(root.dependencies)) {
  if (EXCLUDED.has(dep)) continue;
  walk(ROOT, dep, BUILD_ONLY_SHALLOW.has(dep));
}

// 同じリポジトリの兄弟パッケージにだけ全文が同梱されている場合は、それを引く(例: alloc-no-stdlib と alloc-stdlib)
const byRepo = new Map();
for (const p of packages) if (p.repo && p.texts.length) byRepo.set(p.repo, p.texts);
for (const p of packages) if (!p.texts.length && p.repo && byRepo.has(p.repo)) p.texts = byRepo.get(p.repo);

packages.sort((a, b) => a.kind.localeCompare(b.kind) || a.name.localeCompare(b.name));
const out = { packages, texts };
fs.writeFileSync(path.join(ROOT, "src/lib/licenses.generated.json"), JSON.stringify(out));
const noText = packages.filter((p) => !p.texts.length);
console.log(`${packages.length} packages (rust ${packages.filter((p) => p.kind === "rust").length}), ${texts.length} texts, no license file: ${noText.length}`);
for (const p of noText) console.log("  no text:", p.kind, p.name, p.version, p.license);
