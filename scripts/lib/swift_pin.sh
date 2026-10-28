#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The Swift toolchain this repository builds and tests the Swift backend with, read
# from the one place that names it: the `[tools.swift]` table of `bx.toml`, which is
# what `provision` installs on the build machines.
#
# Everything else asks here. The hosted lanes (`swift-backend.yml`, the Swift rounds of
# `mutation-rounds.yml`) install it with `scripts/install_swift_toolchain.sh`, the
# gate takes its version from here, and the mutation harness finds the toolchain by the
# same layout. A second copy of the version or of the tarball's digest in a workflow
# would be a second answer to "which compiler is this", free to fall behind the build
# machines' exactly when a bump moves one and not the other; so there is none.
#
# Sourced; defines functions only. `SCE_REPO_ROOT` is the repository root.

# The release, `6.4.0`.
sce_swift_pin_version() {
    awk '/^\[/ { in_swift = ($0 == "[tools.swift]"); next }
         in_swift && /^version = / { gsub(/[" ]/, "", $3); print $3; exit }' \
        "${SCE_REPO_ROOT}/bx.toml"
}

# The SHA-256 of the release's tarball for Ubuntu release `$1` (`24.04`), taken from
# the install command's `ubuntu<release>) sum=<digest>` arm. Prints nothing for a
# release the pin has no digest for, which the caller refuses by name: a tarball
# downloaded with no digest to check it against is not installed.
sce_swift_pin_digest() {
    grep -o "ubuntu$1) sum=[0-9a-f]\{64\}" "${SCE_REPO_ROOT}/bx.toml" | head -1 | sed 's/.*sum=//'
}

# What `swift --version` prints in parentheses for release `$1`: `swift-6.4-RELEASE`
# for `6.4.0`, `swift-6.4.1-RELEASE` for a patch release.
sce_swift_pin_tag() {
    printf 'swift-%s-RELEASE\n' "${1%.0}"
}
