# Interactive demos (macOS and Windows)

Live: https://ap5911443-jpg.github.io/waitplay-website/ (chooser), `/mac/` and `/windows/` (GitHub Pages, branch `gh-pages`).

Browser simulations of the desktop apps for people who do not have the other platform: tray/menu-bar menu, the floating widget (the
macOS glass bar, the Windows notch), the entertainment window, Settings, Account, light/dark mode and a scripted agent story (prompt,
second agent, needs input, finish, bridge down). The widget, Settings, Account and entertainment-header pages are the app's own files
(`crates/waitplay-app/assets/`), run unchanged inside iframes with `window.ipc` wired to a small JS host that plays the role of the
Rust app. The glass is CSS blur; the Instagram/YouTube/X content is placeholder.

The host also reproduces the companion's window rules (`companion/logic.rs`): the entertainment window is shown while the terminal
that started the agent is in front, steps aside (after a short grace) when the terminal is minimized or another app is in front, comes
back when the terminal is back and the agent is still working, and stays away after the user closed it for that prompt. The Windows
demo also hides the notch while a fullscreen app is in front.

Build and publish (each page is one self-contained file):

```
node demo/build.mjs    # -> demo/dist/index.html, mac/index.html, windows/index.html
# publish: push demo/dist as the root of the gh-pages branch (plus an empty .nojekyll)
```
