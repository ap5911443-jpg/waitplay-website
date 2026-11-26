// Builds the self-contained macOS demo: template.html + the app's own UI pages + the real site logos -> dist/index.html.
// Run: node demo/mac/build.mjs
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");
const assets = join(root, "crates", "waitplay-app", "assets");
const read = (p) => readFileSync(p, "utf8");

const pages = {
  widget: read(join(assets, "mac_widget.html")),
  settings: read(join(assets, "settings.html")),
  account: read(join(assets, "account.html")),
  header: read(join(assets, "header.html")),
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

const js = (v) => JSON.stringify(v).replace(/<\/script/gi, "<\\/script").replace(/<!--/g, "<\\!--").replace(/\u2028/g, "\\u2028").replace(/\u2029/g, "\\u2029");
const out = read(join(here, "template.html")).replace("/*PAGES*/", () => js(pages)).replace("/*LOGOS*/", () => js(logos));
mkdirSync(join(here, "dist"), { recursive: true });
writeFileSync(join(here, "dist", "index.html"), out);
console.log(`demo/mac/dist/index.html: ${(out.length / 1024).toFixed(0)} KiB`);
