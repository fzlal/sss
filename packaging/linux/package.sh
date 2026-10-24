#!/usr/bin/env bash
# Build Linux release packages from an already compiled binary.
#
# Usage: packaging/linux/package.sh VERSION ARCH BINARY OUT_DIR
#   VERSION  semantic version without a leading "v"
#   ARCH     x86_64 or aarch64
#   BINARY   path to the release executable
#   OUT_DIR  directory that receives the archives
#
# Set FORMATS to a comma-separated subset of tar,deb,rpm,appimage (default:
# all) to skip tools that are not installed locally.

set -euo pipefail

if [[ $# -ne 4 ]]; then
    echo "usage: $0 VERSION ARCH BINARY OUT_DIR" >&2
    exit 2
fi

version="${1#v}"
arch="$2"
binary="$3"
out="$4"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
formats="${FORMATS:-tar,deb,rpm,appimage}"

die() {
    echo "error: $*" >&2
    exit 1
}

case "$arch" in
    x86_64)
        deb_arch="amd64"
        rpm_arch="x86_64"
        ;;
    aarch64)
        deb_arch="arm64"
        rpm_arch="aarch64"
        ;;
    *)
        die "unsupported architecture: $arch (expected x86_64 or aarch64)"
        ;;
esac

[[ "$version" =~ ^[0-9A-Za-z.+~-]+$ ]] || die "invalid version: $version"
[[ -f "$binary" ]] || die "binary not found: $binary"
[[ -x "$binary" ]] || die "binary is not executable: $binary"

mkdir -p "$out"
out="$(cd "$out" && pwd)"
binary="$(cd "$(dirname "$binary")" && pwd)/$(basename "$binary")"

want() {
    case ",${formats}," in
        *",$1,"*) return 0 ;;
        *) return 1 ;;
    esac
}

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

stage="$tmp/stage"
install -Dm755 "$binary" "$stage/usr/bin/slime-soccer"

for doc in README.md LICENSE ASSETS.md THIRD_PARTY_NOTICES.md; do
    install -Dm644 "$root/$doc" "$stage/usr/share/doc/slime-soccer/$doc"
done
install -Dm644 "$root/packaging/linux/copyright" \
    "$stage/usr/share/doc/slime-soccer/copyright"
install -Dm644 "$root/assets/fonts/OFL-PressStart2P.txt" \
    "$stage/usr/share/doc/slime-soccer/fonts/OFL-PressStart2P.txt"
install -Dm644 "$root/assets/fonts/OFL-VT323.txt" \
    "$stage/usr/share/doc/slime-soccer/fonts/OFL-VT323.txt"
install -Dm644 "$root/assets/fonts/OFL-IBMPlexSansArabic.txt" \
    "$stage/usr/share/doc/slime-soccer/fonts/OFL-IBMPlexSansArabic.txt"

install -Dm644 "$root/packaging/linux/slime-soccer.desktop" \
    "$stage/usr/share/applications/slime-soccer.desktop"
install -Dm644 "$root/packaging/icons/slime-soccer-256.png" \
    "$stage/usr/share/icons/hicolor/256x256/apps/slime-soccer.png"
install -Dm644 "$root/packaging/icons/slime-soccer-512.png" \
    "$stage/usr/share/icons/hicolor/512x512/apps/slime-soccer.png"

if command -v desktop-file-validate >/dev/null 2>&1; then
    desktop-file-validate "$root/packaging/linux/slime-soccer.desktop"
fi

if want tar; then
    tar_file="$out/slime-soccer-${version}-linux-${arch}.tar.gz"
    tar --sort=name --mtime='@0' --owner=0 --group=0 --numeric-owner \
        -cf - -C "$stage" . | gzip -9n > "$tar_file"
    echo "built $tar_file"
fi

if want deb; then
    command -v dpkg-deb >/dev/null 2>&1 || die "dpkg-deb is required for .deb"
    deb_root="$tmp/deb"
    mkdir -p "$deb_root/DEBIAN"
    cp -a "$stage/." "$deb_root/"

    installed_size="$(du -sk "$stage" | cut -f1)"
    cat > "$deb_root/DEBIAN/control" <<EOF
Package: slime-soccer
Version: ${version}
Section: games
Priority: optional
Architecture: ${deb_arch}
Installed-Size: ${installed_size}
Maintainer: Super Slime Soccer contributors <fazel8195@gmail.com>
Depends: libasound2t64 | libasound2, libgl1, libx11-6, libxcursor1, libxi6, libxinerama1, libxrandr2
Description: Arcade slime football, volleyball and basketball
 A cross-platform remake of Super Slime Soccer with soccer, volleyball and
 basketball rules, local and online play, championships and an English and
 Persian interface. The embedded original artwork is not MIT licensed; see
 /usr/share/doc/slime-soccer/THIRD_PARTY_NOTICES.md.
EOF

    (
        cd "$deb_root"
        find usr -type f -print0 | sort -z | xargs -0 md5sum
    ) > "$deb_root/DEBIAN/md5sums"

    deb_file="$out/slime-soccer_${version}_${deb_arch}.deb"
    dpkg-deb --root-owner-group --build "$deb_root" "$deb_file" >/dev/null
    dpkg-deb --info "$deb_file" >/dev/null
    echo "built $deb_file"
fi

if want rpm; then
    command -v rpmbuild >/dev/null 2>&1 || die "rpmbuild is required for .rpm"
    rpm_top="$tmp/rpmbuild"
    mkdir -p "$rpm_top/BUILD" "$rpm_top/BUILDROOT" "$rpm_top/RPMS" \
        "$rpm_top/SOURCES" "$rpm_top/SPECS" "$rpm_top/SRPMS"
    rpmbuild -bb --quiet \
        --define "_topdir $rpm_top" \
        --define "app_version $version" \
        --define "app_arch $rpm_arch" \
        --define "payload $stage" \
        "$root/packaging/linux/slime-soccer.spec"
    rpm_file="$(find "$rpm_top/RPMS" -type f -name '*.rpm' -print -quit)"
    [[ -n "$rpm_file" ]] || die "rpmbuild produced no package"
    rpm_out="$out/slime-soccer-${version}-1.${rpm_arch}.rpm"
    cp "$rpm_file" "$rpm_out"
    rpm -qp --queryformat '%{NAME} %{VERSION}-%{RELEASE} %{ARCH}\n' "$rpm_out"
    echo "built $rpm_out"
fi

if want appimage; then
    command -v curl >/dev/null 2>&1 || die "curl is required for AppImage"
    appdir="$tmp/SlimeSoccer.AppDir"
    install -d "$appdir/usr/bin" "$appdir/usr/share"
    install -m755 "$binary" "$appdir/usr/bin/slime-soccer"
    cp -a "$stage/usr/share/." "$appdir/usr/share/"
    install -m644 "$root/packaging/linux/slime-soccer.desktop" \
        "$appdir/slime-soccer.desktop"
    install -m644 "$root/packaging/icons/slime-soccer-256.png" \
        "$appdir/slime-soccer.png"
    install -m644 "$root/packaging/icons/slime-soccer-256.png" \
        "$appdir/.DirIcon"

    cat > "$appdir/AppRun" <<'EOF'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/usr/bin/slime-soccer" "$@"
EOF
    chmod +x "$appdir/AppRun"

    tool="$tmp/appimagetool"
    curl -fsSL --retry 3 --retry-delay 2 \
        -o "$tool" \
        "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-${arch}.AppImage"
    chmod +x "$tool"

    appimage_file="$out/slime-soccer-${version}-linux-${arch}.AppImage"
    ARCH="$arch" APPIMAGE_EXTRACT_AND_RUN=1 \
        "$tool" --no-appstream "$appdir" "$appimage_file"
    echo "built $appimage_file"
fi

echo "linux packages for ${arch} in ${out}"
