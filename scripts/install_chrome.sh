#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Install the pinned headless Chrome `wasm32-scenarios` replays the scenarios in.
#
# The gate replays every scenario of the engine's wasm32 module in Chrome, the
# web target's own host (backends/rust/scenarios/wasm/run_chrome.mjs, which
# starts the binary CHROME_BIN names). Until 2026-11-08 that was whatever Chrome
# the machine carried: the hosted image's, of the month. A verdict that moves
# with the browser is not one the tree decides, and no build machine carried a
# Chrome at all, so an update to main that reached the gate could not have been
# measured there. So the browser is pinned here, as Chrome for Testing: the
# builds Google publishes for automation, by exact version, at fixed URLs.
#
# The unit is the VERSION: `CHROME_REV` below is the one home of the pin, and
# the install path derives from it. Raise it in a commit of its own, with the
# gate's run on the new version in front of the person raising it.
#
# Chrome for Testing publishes no digest of its archives, so the download is
# checked by its origin (HTTPS from Google's storage) and by the postcondition
# below, which runs the binary and reads its version.
#
# What it needs and does not install: curl and unzip, and the shared libraries
# Chrome links against. Those are system packages whose names differ between
# Ubuntu 22.04 and 24.04 (24.04 renamed several for its 64-bit time_t, as
# `libasound2t64`), so they cannot be one package list of a tool; the
# postcondition is what finds a machine without them, because the binary does
# not start there, and bx then installs no tool on that machine and places no
# unit of the gate on it. On Ubuntu 24.04:
#
#   sudo apt-get install -y libnss3 libgbm1 libxkbcommon0 libxcomposite1 \
#       libxdamage1 libxrandr2 libxfixes3 libdrm2 libpango-1.0-0 libcairo2 \
#       libasound2t64 libatk1.0-0t64 libatk-bridge2.0-0t64 libcups2t64 \
#       libatspi2.0-0t64 libgtk-3-0t64
#
# and on 22.04 the same names without the `t64`.
#
# The build machines get it from `[platform.linux.tools.chrome]` in bx.toml,
# which runs this script with CHROME_ROOT set to bx's store entry. The gate looks
# for the binary at the path derived here.
set -euo pipefail

# The pin: a Chrome for Testing release, and the SHA-256 of its linux64 zip.
# Chrome for Testing publishes no digest, so the one written here was measured
# when the pin was set (2026-11-08, the Stable channel's release that day); a
# download that does not match it is refused before it is unpacked.
CHROME_REV="157.0.8103.38"
CHROME_ZIP_SHA256="8cdeb344e9b2cefe59f4aea8d1d93d5d93054c03cde217665c16fc1f62e72a66"

CHROME_ROOT="${CHROME_ROOT:-$HOME/.local/share/chrome-for-testing/$CHROME_REV}"
CHROME="$CHROME_ROOT/chrome-linux64/chrome"

if [[ -x "$CHROME" ]] && "$CHROME" --version 2>/dev/null | grep -F " $CHROME_REV" >/dev/null; then
    echo "Chrome for Testing $CHROME_REV is already installed at $CHROME"
    exit 0
fi

for tool in curl unzip; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "install_chrome: '$tool' is not installed; see the header of this script" >&2
        exit 2
    }
done

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
url="https://storage.googleapis.com/chrome-for-testing-public/$CHROME_REV/linux64/chrome-linux64.zip"
echo "installing Chrome for Testing $CHROME_REV in $CHROME_ROOT"
curl -fsSL --retry 3 --retry-all-errors -o "$WORK/chrome-linux64.zip" "$url"
echo "$CHROME_ZIP_SHA256  $WORK/chrome-linux64.zip" | sha256sum -c --quiet - || {
    echo "install_chrome: the download of $CHROME_REV does not match its pinned SHA-256" >&2
    exit 1
}
mkdir -p "$CHROME_ROOT"
unzip -q -o "$WORK/chrome-linux64.zip" -d "$CHROME_ROOT"

# The postcondition, asserted rather than assumed: the binary starts on this
# machine, which needs its shared libraries, and names the pinned release.
"$CHROME" --version | grep -F " $CHROME_REV" >/dev/null || {
    echo "install_chrome: $CHROME does not start here or does not report $CHROME_REV;" >&2
    echo "install_chrome: its shared libraries are listed in the header of this script" >&2
    exit 1
}
echo "Chrome for Testing $CHROME_REV installed at $CHROME"
