#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Install the pinned Swift toolchain where the build machines keep it:
# `~/.local/share/swift/<version>/`, with `bin/swift` a link into the unpacked
# toolchain. The version and the tarball's digest are read from `bx.toml`
# (`scripts/lib/swift_pin.sh`), so a hosted lane installs the compiler the build
# machines have and not one its own workflow names.
#
# The download is checked against the pinned digest and never against a checksum the
# same server publishes beside it. A release the pin has no digest for (another Ubuntu
# release, another distribution) fails by name rather than downloading something
# unchecked.
#
# Idempotent: an installed toolchain of the pinned release is left alone, which is what
# lets a lane restore it from a cache and call this anyway.
#
# Usage: scripts/install_swift_toolchain.sh

set -euo pipefail

SCE_REPO_ROOT="$(git rev-parse --show-toplevel)"
source "$SCE_REPO_ROOT/scripts/lib/swift_pin.sh"

version="$(sce_swift_pin_version)"
[ -n "$version" ] || { echo "error: bx.toml has no [tools.swift] version" >&2; exit 1; }
tag="$(sce_swift_pin_tag "$version")"
prefix="$HOME/.local/share/swift/$version"

if [ -x "$prefix/bin/swift" ] && "$prefix/bin/swift" --version 2>&1 | grep -qF "$tag"; then
    echo "Swift $version is installed at $prefix"
    exit 0
fi

# shellcheck disable=SC1091
. /etc/os-release
release="$VERSION_ID"
[ "$ID" = ubuntu ] \
    || { echo "error: no Swift $version tarball is pinned for $ID $VERSION_ID" >&2; exit 1; }
digest="$(sce_swift_pin_digest "$release")"
[ -n "$digest" ] \
    || { echo "error: bx.toml pins no Swift $version digest for Ubuntu $release" >&2; exit 1; }

dir="ubuntu$release"
file="swift-$version-RELEASE-$dir.tar.gz"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

curl -fsSL --retry 3 --retry-all-errors -o "$work/$file" \
    "https://download.swift.org/swift-$version-release/${dir//./}/swift-$version-RELEASE/$file"
echo "$digest  $work/$file" | sha256sum -c --quiet

mkdir -p "$prefix/bin"
tar -xzf "$work/$file" -C "$prefix" --strip-components=1

# Only `swift` gets a link, as on the build machines (`prefix_link` of `[tools.swift]`):
# `swiftc` and the rest sit beside the file it points to, which is where the gate and
# the mutation harness look.
ln -sfn "$prefix/usr/bin/swift" "$prefix/bin/swift"
"$prefix/bin/swift" --version 2>&1 | grep -qF "$tag" \
    || { echo "error: the unpacked toolchain is not $tag" >&2; exit 1; }
echo "Swift $version installed at $prefix"
