#!/usr/bin/env bash
# Builds every Linux package from already compiled binaries.
#
#   packaging/linux/build-packages.sh <version> <dir with copal, copal-lite, copal-cli> <out dir>
#
# Produces, for both editions:
#   copal_<v>_amd64.deb            copal-lite_<v>_amd64.deb         (needs dpkg-deb)
#   copal-<v>-1.x86_64.rpm         copal-lite-<v>-1.x86_64.rpm      (needs rpmbuild)
#   Copal-<v>-x86_64.AppImage      Copal-Lite-<v>-x86_64.AppImage   (needs $APPIMAGETOOL)
#   copal-<v>-linux-x86_64.tar.gz                                    (all binaries + desktop files)
#   copal-bin.PKGBUILD             copal-lite-bin.PKGBUILD          (AUR, from the tar.gz)
#
# The full package contains Copal, Copal Lite and copal-cli. The lite package contains only
# Copal Lite and copal-cli, for small systems. They conflict, install one of them.
set -euo pipefail

VERSION=$1
BIN=$(realpath "$2")
OUT=$(realpath -m "$3")
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../.." && pwd)
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
mkdir -p "$OUT"
DATE=$(date -u +%Y-%m-%d)
ID=io.github.hakiguru.Copal
SUMMARY="Free slicer for resin (MSLA) 3D printers"

# Installs files under a prefix. $1 = destination root, $2 = full|lite
stage() {
    local dest=$1 edition=$2 prefix=$1/usr
    install -Dm755 "$BIN/copal-cli" "$prefix/bin/copal-cli"
    install -Dm755 "$BIN/copal-lite" "$prefix/bin/copal-lite"
    install -Dm644 "$HERE/$ID.Lite.desktop" "$prefix/share/applications/$ID.Lite.desktop"
    install -Dm644 "$ROOT/assets/logo-256.png" "$prefix/share/icons/hicolor/256x256/apps/$ID.Lite.png"
    if [ "$edition" = full ]; then
        install -Dm755 "$BIN/copal" "$prefix/bin/copal"
        install -Dm644 "$HERE/$ID.desktop" "$prefix/share/applications/$ID.desktop"
    fi
    # The project icon and file type are shared by both editions.
    install -Dm644 "$ROOT/assets/logo-256.png" "$prefix/share/icons/hicolor/256x256/apps/$ID.png"
    install -Dm644 "$ROOT/assets/logo-512.png" "$prefix/share/icons/hicolor/512x512/apps/$ID.png"
    install -Dm644 "$HERE/$ID.mime.xml" "$prefix/share/mime/packages/$ID.xml"
    sed -e "s/@VERSION@/$VERSION/" -e "s/@DATE@/$DATE/" "$HERE/$ID.metainfo.xml" \
        | install -Dm644 /dev/stdin "$prefix/share/metainfo/$ID.metainfo.xml"
    install -Dm644 "$ROOT/LICENSE" "$prefix/share/licenses/$( [ "$edition" = full ] && echo copal || echo copal-lite )/LICENSE"
}

# ---------------------------------------------------------------------------------- .deb
build_deb() {
    local name=$1 edition=$2 conflicts=$3 root="$WORK/deb-$1"
    stage "$root" "$edition"
    mkdir -p "$root/DEBIAN"
    local size
    size=$(du -sk "$root/usr" | cut -f1)
    cat > "$root/DEBIAN/control" <<EOF
Package: $name
Version: $VERSION
Architecture: amd64
Maintainer: hakiguru <262621558+hakiguru@users.noreply.github.com>
Installed-Size: $size
Depends: libc6 (>= 2.35), libgl1, libegl1, libxkbcommon0
Recommends: libxkbcommon-x11-0, libwayland-client0, libwayland-egl1, libx11-6, libxcursor1, libxrandr2, libxi6
Conflicts: $conflicts
Replaces: $conflicts
Section: graphics
Priority: optional
Homepage: https://github.com/hakiguru/Copal
Description: $SUMMARY
 Copal prepares 3D models for MSLA / LCD resin printers: place a model, add
 supports, slice it and export a file your printer understands.
 No ads, no paywalls, no accounts.
EOF
    dpkg-deb --root-owner-group --build "$root" "$OUT/${name}_${VERSION}_amd64.deb" >/dev/null
    echo "built ${name}_${VERSION}_amd64.deb"
}

# ---------------------------------------------------------------------------------- .rpm
build_rpm() {
    local name=$1 edition=$2 conflicts=$3 top="$WORK/rpm-$1"
    mkdir -p "$top"/{BUILD,RPMS,SOURCES,SPECS,SRPMS,stage}
    stage "$top/stage" "$edition"
    local files
    files=$(cd "$top/stage" && find . -type f | sed 's|^\.||' | sort)
    cat > "$top/SPECS/$name.spec" <<EOF
Name:           $name
Version:        $VERSION
Release:        1
Summary:        $SUMMARY
License:        MIT
URL:            https://github.com/hakiguru/Copal
BuildArch:      x86_64
AutoReqProv:    no
%define _build_id_links none
Requires:       glibc >= 2.35, libglvnd-glx, libglvnd-egl, libxkbcommon
Recommends:     libxkbcommon-x11, libwayland-client, libwayland-egl, libX11, libXcursor, libXrandr, libXi
Conflicts:      $conflicts

%description
Copal prepares 3D models for MSLA / LCD resin printers: place a model, add
supports, slice it and export a file your printer understands.
No ads, no paywalls, no accounts.

%install
cp -a $top/stage/. %{buildroot}/

%files
$files
EOF
    rpmbuild --quiet --define "_topdir $top" -bb "$top/SPECS/$name.spec"
    cp "$top"/RPMS/x86_64/*.rpm "$OUT/"
    echo "built $name-$VERSION-1.x86_64.rpm"
}

# ------------------------------------------------------------------------------ AppImage
build_appimage() {
    local label=$1 exe=$2 desktop=$3 edition=$4 appdir="$WORK/AppDir-$1"
    stage "$appdir" "$edition"
    cp "$appdir/usr/share/applications/$desktop.desktop" "$appdir/$desktop.desktop"
    cp "$ROOT/assets/logo-256.png" "$appdir/$desktop.png"
    cat > "$appdir/AppRun" <<EOF
#!/bin/sh
HERE=\$(dirname "\$(readlink -f "\$0")")
exec "\$HERE/usr/bin/$exe" "\$@"
EOF
    chmod +x "$appdir/AppRun"
    ARCH=x86_64 "$APPIMAGETOOL" --appimage-extract-and-run --no-appstream "$appdir" \
        "$OUT/$label-$VERSION-x86_64.AppImage" >/dev/null 2>&1
    echo "built $label-$VERSION-x86_64.AppImage"
}

# -------------------------------------------------------------------------------- tar.gz
build_tarball() {
    local dir="$WORK/copal-$VERSION-linux-x86_64"
    stage "$WORK/tar-stage" full
    mkdir -p "$dir"
    cp -a "$WORK/tar-stage/usr/." "$dir/"
    cp "$ROOT/README.md" "$ROOT/LICENSE" "$dir/"
    tar -C "$WORK" -czf "$OUT/copal-$VERSION-linux-x86_64.tar.gz" "copal-$VERSION-linux-x86_64"
    echo "built copal-$VERSION-linux-x86_64.tar.gz"
}

# ------------------------------------------------------------------------------- PKGBUILD
build_pkgbuilds() {
    local tarball="copal-$VERSION-linux-x86_64.tar.gz" sum
    sum=$(sha256sum "$OUT/$tarball" | cut -d' ' -f1)
    for edition in full lite; do
        local name provides conflicts bins desktops
        if [ "$edition" = full ]; then
            name=copal-bin; provides="'copal' 'copal-lite' 'copal-cli'"; conflicts="'copal' 'copal-lite-bin'"
            bins="copal copal-lite copal-cli"; desktops="$ID.desktop $ID.Lite.desktop"
        else
            name=copal-lite-bin; provides="'copal-lite' 'copal-cli'"; conflicts="'copal' 'copal-bin'"
            bins="copal-lite copal-cli"; desktops="$ID.Lite.desktop"
        fi
        cat > "$OUT/$name.PKGBUILD" <<EOF
# Maintainer: hakiguru <262621558+hakiguru@users.noreply.github.com>
pkgname=$name
pkgver=$VERSION
pkgrel=1
pkgdesc="$SUMMARY"
arch=('x86_64')
url="https://github.com/hakiguru/Copal"
license=('MIT')
depends=('glibc' 'libglvnd' 'libxkbcommon' 'hicolor-icon-theme' 'shared-mime-info')
optdepends=('wayland: Wayland support' 'libxkbcommon-x11: X11 support' 'libxcursor: X11 cursors')
provides=($provides)
conflicts=($conflicts)
source=("$tarball::https://github.com/hakiguru/Copal/releases/download/v\$pkgver/copal-\$pkgver-linux-x86_64.tar.gz")
sha256sums=('$sum')

package() {
    cd "copal-\$pkgver-linux-x86_64"
    for b in $bins; do install -Dm755 "bin/\$b" "\$pkgdir/usr/bin/\$b"; done
    for d in $desktops; do install -Dm644 "share/applications/\$d" "\$pkgdir/usr/share/applications/\$d"; done
    cp -a share/icons share/mime share/metainfo "\$pkgdir/usr/share/"
    install -Dm644 LICENSE "\$pkgdir/usr/share/licenses/\$pkgname/LICENSE"
}
EOF
        echo "built $name.PKGBUILD"
    done
}

build_tarball
build_pkgbuilds
build_deb copal full copal-lite
build_deb copal-lite lite copal
if command -v rpmbuild >/dev/null; then
    build_rpm copal full copal-lite
    build_rpm copal-lite lite copal
else
    echo "rpmbuild not found, skipping .rpm"
fi
if [ -n "${APPIMAGETOOL:-}" ]; then
    build_appimage Copal copal "$ID" full
    build_appimage Copal-Lite copal-lite "$ID.Lite" lite
else
    echo "APPIMAGETOOL not set, skipping AppImage"
fi
