// Builds the two self-contained demos (macOS and Windows) plus a small chooser page.
//   template.html + the app's own UI pages + the real site logos  ->  dist/mac/index.html, dist/windows/index.html, dist/index.html
// Run: node demo/build.mjs
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..");
const assets = join(root, "crates", "waitplay-app", "assets");
const read = (p) => readFileSync(p, "utf8");

// The app's own pages, per platform. The account and header pages are shared.
const pagesFor = {
  mac: { widget: "mac_widget.html", settings: "settings.html", account: "account.html", header: "header.html" },
  win: { widget: "win_widget.html", settings: "win_settings.html", account: "account.html", header: "header.html" },
};

// Logo geometry lives in the app's Rust source (Simple Icons, CC0): take it from there so the demo never drifts.
const sites = read(join(root, "crates", "waitplay-app", "src", "sites.rs"));
const konst = (name) => {
  const m = sites.match(new RegExp(`const ${name}: &str = "([^"]+)";`));
  if (!m) throw new Error(`sites.rs has no ${name}`);
  return m[1];
};
const svg = (inner) => `<svg xmlns='http://www.w3.org/2000/svg' width='18' height='18' viewBox='0 0 24 24' aria-hidden='true'>${inner}</svg>`;
const logos = {
  instagram: svg(
    `<defs><linearGradient id='wp-ig' x1='0.1' y1='1' x2='0.9' y2='0'><stop offset='0' stop-color='#FEDA75'/><stop offset='.25' stop-color='#FA7E1E'/><stop offset='.5' stop-color='#D62976'/><stop offset='.75' stop-color='#962FBF'/><stop offset='1' stop-color='#4F5BD5'/></linearGradient></defs><path fill='url(#wp-ig)' d='${konst("INSTAGRAM")}'/>`
  ),
  youtube: svg(`<path fill='#FFFFFF' d='${konst("YOUTUBE_PLAY")}'/><path fill='#FF0000' fill-rule='nonzero' d='${konst("YOUTUBE")}'/>`),
  x: svg(`<path fill='currentColor' d='${konst("X")}'/>`),
};

// Safe inside a <script>: no early </script>, no <!-- state change, no JS-illegal line separators.
const js = (v) => JSON.stringify(v).replace(/<\/script/gi, "<\\/script").replace(/<!--/g, "<\\!--").replace(/\u2028/g, "\\u2028").replace(/\u2029/g, "\\u2029");
const template = read(join(here, "template.html"));

function build(plat, dir, title) {
  const pages = Object.fromEntries(Object.entries(pagesFor[plat]).map(([k, f]) => [k, read(join(assets, f))]));
  const out = template.replace("/*TITLE*/", () => title).replace("/*PLAT*/", () => js(plat)).replace("/*PAGES*/", () => js(pages)).replace("/*LOGOS*/", () => js(logos));
  mkdirSync(join(here, "dist", dir), { recursive: true });
  writeFileSync(join(here, "dist", dir, "index.html"), out);
  console.log(`demo/dist/${dir}/index.html: ${(out.length / 1024).toFixed(0)} KiB`);
}
build("mac", "mac", "WaitPlay for macOS: interactive demo");
build("win", "windows", "WaitPlay for Windows: interactive demo");

const chooser = `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>WaitPlay: interactive demos</title>
<style>*{box-sizing:border-box}body{margin:0;min-height:100vh;display:grid;place-items:center;background:#0c0c12;color:#e8e8ee;font:15px -apple-system,"Segoe UI",system-ui,sans-serif}
main{max-width:760px;padding:32px;text-align:center}h1{font-size:28px;margin:0 0 8px;letter-spacing:-.4px}p{opacity:.7;line-height:1.55;margin:0 0 28px}
.g{display:grid;grid-template-columns:1fr 1fr;gap:16px}a{display:block;padding:26px 20px;border-radius:18px;border:1px solid rgba(255,255,255,.12);background:rgba(255,255,255,.06);color:inherit;text-decoration:none;transition:background .15s,transform .15s}
a:hover{background:rgba(255,255,255,.12);transform:translateY(-2px)}a b{display:block;font-size:19px;margin-bottom:6px}a span{opacity:.65;font-size:13.5px;line-height:1.5}
@media(max-width:600px){.g{grid-template-columns:1fr}}</style></head><body><main><h1>WaitPlay: interactive demos</h1>
<p>Browser simulations of the desktop apps. The widget, Settings, Account and entertainment-header pages are the apps' real UI pages; everything around them is a stand-in.</p>
<div class="g"><a href="mac/"><b>WaitPlay on a Mac</b><span>Liquid Glass widget in a corner, menu bar item, Dock.</span></a><a href="windows/"><b>WaitPlay on Windows</b><span>The black notch at the top centre, tray icon, taskbar, fullscreen behaviour.</span></a></div></main></body></html>`;
writeFileSync(join(here, "dist", "index.html"), chooser);
console.log("demo/dist/index.html (chooser)");
