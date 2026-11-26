#!/usr/bin/env bash
# Builds dist/<target>/WaitPlay.app and WaitPlay-<version>-<arch>.dmg from an already built `cargo build --release --target <t>`.
#
#   bash packaging/macos/build_app.sh [target-triple]        (default aarch64-apple-darwin)
#
# Signing is optional so local builds never need an Apple account (they get an ad-hoc signature):
#   APPLE_SIGNING_IDENTITY   "Developer ID Application: Name (TEAMID)"  -> Developer ID signature, Hardened Runtime, secure timestamp
#   APPLE_NOTARY_PROFILE     name of a `notarytool store-credentials` keychain profile -> notarize, staple, validate, Gatekeeper-assess
#   WAITPLAY_NOTARIZE=0      sign with the identity but skip notarization
#   APPLE_KEYCHAIN           keychain file holding the identity and the profile (CI uses a temporary one)
#   WAITPLAY_BIN_DIR         directory with the three built binaries (default target/<triple>/release)
set -euo pipefail

TARGET="${1:-aarch64-apple-darwin}"
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
VERSION="$(grep -m1 '^version' "$ROOT/Cargo.toml" | sed 's/.*"\(.*\)".*/\1/')"
BUILD="${WAITPLAY_BUILD:-$(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null || echo dev)}"
case "$TARGET" in aarch64-*) ARCH=arm64 ;; x86_64-*) ARCH=x86_64 ;; *) ARCH="$TARGET" ;; esac
BIN="${WAITPLAY_BIN_DIR:-$ROOT/target/$TARGET/release}"
OUT="$ROOT/dist/$TARGET"
APP="$OUT/WaitPlay.app"
DMG="$OUT/WaitPlay-$VERSION-$ARCH.dmg"

for b in waitplay waitplay-bridge waitplay-hook; do
  [ -f "$BIN/$b" ] || { echo "missing $BIN/$b: run cargo build --release --workspace --target $TARGET first" >&2; exit 1; }
done

rm -rf "$OUT"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
for b in waitplay waitplay-bridge waitplay-hook; do cp "$BIN/$b" "$APP/Contents/MacOS/$b"; done
sed -e "s/@VERSION@/$VERSION/" -e "s/@BUILD@/$BUILD/" "$HERE/Info.plist.in" > "$APP/Contents/Contents.tmp" && mv "$APP/Contents/Contents.tmp" "$APP/Contents/Info.plist"
printf 'APPL????' > "$APP/Contents/PkgInfo"

ICONSET="$OUT/WaitPlay.iconset"
python3 "$HERE/make_icon.py" "$ICONSET"
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/WaitPlay.icns"
rm -rf "$ICONSET"

IDENTITY="${APPLE_SIGNING_IDENTITY:--}"
KEYCHAIN_ARGS=()
[ -z "${APPLE_KEYCHAIN:-}" ] || KEYCHAIN_ARGS=(--keychain "$APPLE_KEYCHAIN")
if [ "$IDENTITY" = "-" ]; then
  SIGN=(--force --sign -)
else
  case "$IDENTITY" in "Developer ID Application:"*) ;; *) echo "error: APPLE_SIGNING_IDENTITY must be a \"Developer ID Application: ...\" identity" >&2; exit 1 ;; esac
  SIGN=(--force --timestamp --options runtime --entitlements "$HERE/entitlements.plist" --sign "$IDENTITY" ${KEYCHAIN_ARGS[@]+"${KEYCHAIN_ARGS[@]}"})
fi
for b in waitplay-bridge waitplay-hook waitplay; do codesign "${SIGN[@]}" "$APP/Contents/MacOS/$b"; done
codesign "${SIGN[@]}" "$APP"
codesign --verify --deep --strict --verbose=2 "$APP"
if [ "$IDENTITY" != "-" ]; then
  # Hardened Runtime must be on for every executable, or notarization rejects the app
  for f in "$APP" "$APP"/Contents/MacOS/*; do
    codesign -dv "$f" 2>&1 | grep -q "flags=.*runtime" || { echo "error: Hardened Runtime is not enabled on $f" >&2; exit 1; }
  done
fi

STAGE="$OUT/dmg"
mkdir -p "$STAGE"
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
hdiutil create -volname "Vibe Play" -srcfolder "$STAGE" -ov -format UDZO "$DMG"
rm -rf "$STAGE"
[ "$IDENTITY" = "-" ] || codesign --force --timestamp --sign "$IDENTITY" ${KEYCHAIN_ARGS[@]+"${KEYCHAIN_ARGS[@]}"} "$DMG"

if [ "$IDENTITY" = "-" ]; then
  echo "note: ad-hoc signed development build (not Developer ID signed, not notarized): Gatekeeper asks to confirm the first launch"
elif [ "${WAITPLAY_NOTARIZE:-1}" = "0" ]; then
  echo "note: signed with Developer ID but NOT notarized (WAITPLAY_NOTARIZE=0)"
else
  # Credentials live in a notarytool keychain profile (see docs/RELEASE_SIGNING.md); no password ever appears on a command line.
  [ -n "${APPLE_NOTARY_PROFILE:-}" ] || { echo "error: signing with a Developer ID needs APPLE_NOTARY_PROFILE (a notarytool keychain profile); set WAITPLAY_NOTARIZE=0 to skip notarization" >&2; exit 1; }
  NOTARY=(--keychain-profile "$APPLE_NOTARY_PROFILE")
  [ -z "${APPLE_KEYCHAIN:-}" ] || NOTARY+=(--keychain "$APPLE_KEYCHAIN")
  xcrun notarytool submit "$DMG" "${NOTARY[@]}" --wait
  xcrun stapler staple "$DMG"
  xcrun stapler validate "$DMG"
  # the final gate: what Gatekeeper will say on a user's Mac
  spctl --assess --type open --context context:primary-signature --verbose=2 "$DMG"
fi
echo "built: $APP"
echo "built: $DMG"
