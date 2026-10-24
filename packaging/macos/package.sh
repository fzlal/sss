#!/usr/bin/env bash
# Build the universal macOS .app, .dmg and .tar.gz.
#
# Usage: packaging/macos/package.sh VERSION BINARY OUT_DIR
# BINARY must be a universal (x86_64 + arm64) executable.

set -euo pipefail

if [[ $# -ne 3 ]]; then
    echo "usage: $0 VERSION BINARY OUT_DIR" >&2
    exit 2
fi

version="${1#v}"
binary="$2"
out="$3"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

die() {
    echo "error: $*" >&2
    exit 1
}

[[ "$version" =~ ^[0-9A-Za-z.+~-]+$ ]] || die "invalid version: $version"
[[ -f "$binary" ]] || die "binary not found: $binary"
[[ -x "$binary" ]] || die "binary is not executable: $binary"

mkdir -p "$out"
out="$(cd "$out" && pwd)"
binary="$(cd "$(dirname "$binary")" && pwd)/$(basename "$binary")"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

app="$tmp/Super Slime Soccer.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources/licenses"
install -m755 "$binary" "$app/Contents/MacOS/slime-soccer"

sed "s/@VERSION@/${version}/g" "$root/packaging/macos/Info.plist" \
    > "$app/Contents/Info.plist"
plutil -lint "$app/Contents/Info.plist" >/dev/null

iconset="$tmp/AppIcon.iconset"
mkdir -p "$iconset"
for size in 16 32 128 256 512; do
    double=$((size * 2))
    sips -z "$size" "$size" \
        "$root/packaging/icons/slime-soccer-1024.png" \
        --out "$iconset/icon_${size}x${size}.png" >/dev/null
    sips -z "$double" "$double" \
        "$root/packaging/icons/slime-soccer-1024.png" \
        --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$iconset" -o "$app/Contents/Resources/AppIcon.icns"

for doc in README.md LICENSE ASSETS.md THIRD_PARTY_NOTICES.md; do
    cp "$root/$doc" "$app/Contents/Resources/$doc"
done
cp "$root/assets/fonts/OFL-PressStart2P.txt" \
    "$app/Contents/Resources/licenses/OFL-PressStart2P.txt"
cp "$root/assets/fonts/OFL-VT323.txt" \
    "$app/Contents/Resources/licenses/OFL-VT323.txt"
cp "$root/assets/fonts/OFL-IBMPlexSansArabic.txt" \
    "$app/Contents/Resources/licenses/OFL-IBMPlexSansArabic.txt"

# Ad-hoc signing is enough for local execution, including Apple Silicon. Set
# CODESIGN_IDENTITY to a Developer ID identity for a distributable signed build.
identity="${CODESIGN_IDENTITY:--}"
codesign --force --sign "$identity" "$app/Contents/MacOS/slime-soccer"
codesign --force --sign "$identity" "$app"

lipo -archs "$app/Contents/MacOS/slime-soccer"

tar_file="$out/slime-soccer-${version}-macos-universal.tar.gz"
COPYFILE_DISABLE=1 tar -czf "$tar_file" -C "$tmp" "Super Slime Soccer.app"
echo "built $tar_file"

dmg_root="$tmp/dmg"
mkdir -p "$dmg_root"
cp -R "$app" "$dmg_root/"
ln -s /Applications "$dmg_root/Applications"

dmg_file="$out/slime-soccer-${version}-macos-universal.dmg"
rm -f "$dmg_file"
hdiutil create -quiet -fs HFS+ -format UDZO -imagekey zlib-level=9 \
    -volname "Super Slime Soccer" \
    -srcfolder "$dmg_root" \
    "$dmg_file"
echo "built $dmg_file"

echo "macOS packages in ${out}"
