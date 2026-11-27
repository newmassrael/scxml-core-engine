#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Install the pinned headless Firefox `wasm32-scenarios` runs the browser node's kit in.
#
# The kit of a browser node (backends/rust/browser-node/kit) runs in headless browsers of two
# engines, Chromium's and Firefox's (docs/adr/0117, open question 3): Chrome for Testing is
# pinned by scripts/install_chrome.sh, and Firefox here, as a Mozilla release, by exact version,
# from Mozilla's archive. A verdict that moves with the browser is not one the tree decides.
#
# The unit is the VERSION: `FIREFOX_REV` below is the one home of the pin, and the install path
# derives from it. Raise it in a commit of its own, with the gate's run on the new version in
# front of the person raising it.
#
# Mozilla publishes the SHA-256 of every file of a release (its SHA256SUMS); the one written here
# is that file's line for the linux-x86_64 en-US archive of the release, read when the pin was set
# (2026-11-28, the release of 2026-11-24), and a download that does not match it is refused before
# it is unpacked. The postcondition below runs the binary and reads its version.
#
# What it needs and does not install: curl, tar and xz, and the shared libraries Firefox links
# against, system packages a machine carries or not. The postcondition is what finds a machine
# without them, because the binary does not start there, and bx then installs no tool on that
# machine and places no unit of the gate on it. On Ubuntu 24.04, beside what Chrome needs (the
# header of scripts/install_chrome.sh):
#
#   sudo apt-get install -y libdbus-glib-1-2 libx11-xcb1 libxtst6
#
# The build machines get it from `[platform.linux.tools.firefox]` in bx.toml, which runs this
# script with FIREFOX_ROOT set to bx's store entry. The gate looks for the binary at the path
# derived here.
set -euo pipefail

# The pin: a Firefox release, and the SHA-256 Mozilla publishes for its linux-x86_64 archive.
FIREFOX_REV="161.0"
FIREFOX_SHA256="3e9b1e5132d7b53b1ad968c7300281e90e6a0ef57d0656b1516202d686d25733"

FIREFOX_ROOT="${FIREFOX_ROOT:-$HOME/.local/share/firefox/$FIREFOX_REV}"
FIREFOX="$FIREFOX_ROOT/firefox/firefox"

if [[ -x "$FIREFOX" ]] && "$FIREFOX" --version 2>/dev/null | grep -F " $FIREFOX_REV" >/dev/null; then
    echo "Firefox $FIREFOX_REV is already installed at $FIREFOX"
    exit 0
fi

for tool in curl tar xz; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "install_firefox: '$tool' is not installed; see the header of this script" >&2
        exit 2
    }
done

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
url="https://archive.mozilla.org/pub/firefox/releases/$FIREFOX_REV/linux-x86_64/en-US/firefox-$FIREFOX_REV.tar.xz"
echo "installing Firefox $FIREFOX_REV in $FIREFOX_ROOT"
curl -fsSL --retry 3 --retry-all-errors -o "$WORK/firefox.tar.xz" "$url"
echo "$FIREFOX_SHA256  $WORK/firefox.tar.xz" | sha256sum -c --quiet - || {
    echo "install_firefox: the download of $FIREFOX_REV does not match its pinned SHA-256" >&2
    exit 1
}
mkdir -p "$FIREFOX_ROOT"
tar -xJf "$WORK/firefox.tar.xz" -C "$FIREFOX_ROOT"

# The postcondition, asserted rather than assumed: the binary starts on this machine, which needs
# its shared libraries, and names the pinned release.
"$FIREFOX" --version | grep -F " $FIREFOX_REV" >/dev/null || {
    echo "install_firefox: $FIREFOX does not start here or does not report $FIREFOX_REV;" >&2
    echo "install_firefox: its shared libraries are listed in the header of this script" >&2
    exit 1
}
echo "Firefox $FIREFOX_REV installed at $FIREFOX"
