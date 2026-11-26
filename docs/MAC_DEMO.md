# macOS interactive demo

Live: https://ap5911443-jpg.github.io/waitplay-website/ (GitHub Pages, branch `gh-pages`).

A browser simulation of the macOS app for people without a Mac: menu bar menu, floating widget, the entertainment window, Settings,
Account, light/dark mode, and a scripted agent story (prompt, second agent, needs input, finish, bridge down). The widget, Settings,
Account and entertainment-header pages are the app's own files (`crates/waitplay-app/assets/{mac_widget,settings,account,header}.html`),
run unchanged inside iframes with `window.ipc` wired to a small JS host that plays the role of the Rust app. The glass is CSS blur
(the app uses `NSVisualEffectView`); the Instagram/YouTube/X content is placeholder.

Build and publish (the page is one self-contained file):

```
node demo/mac/build.mjs          # -> demo/mac/dist/index.html
# publish: push dist/index.html (plus an empty .nojekyll) as the root of the gh-pages branch
```
