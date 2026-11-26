#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The Swift toolchain this repository builds and tests the Swift backend with, read
# from the one place that names it: the Swift tool of `bx.toml`'s platform
# (`[platform.linux.tools.swift]`; `[tools.swift]` of the shorthand until 2026-11-04),
# which is what `provision` installs on the build machines.
#
# Everything else asks here: `scripts/install_swift_toolchain.sh` installs it, the gates
# take the release they require from here, and the mutation harness finds the toolchain
# by the same release. The hosted lanes (`swift-backend.yml`, the Swift rounds of
# `mutation-rounds.yml`) run in the swift image (tools/ci-images/swift/Containerfile),
# which names the release and its checksum itself (ADR 0145, D3); the gates' check of
# `swift --version` against this pin is what keeps that second copy from falling
# behind the build machines' without a red that names both.
#
# Sourced; defines functions only. `SCE_REPO_ROOT` is the repository root.

# The release, `6.4.0`.
sce_swift_pin_version() {
    awk '/^\[/ { in_swift = ($0 == "[platform.linux.tools.swift]"); next }
         in_swift && /^version = / { gsub(/[" ]/, "", $3); print $3; exit }' \
        "${SCE_REPO_ROOT}/bx.toml"
}

# The SHA-256 of the release's tarball for Ubuntu release `$1` (`24.04`), taken from
# the install command's `ubuntu<release>) sum=<digest>` arm. Prints nothing for a
# release the pin has no digest for, which the caller refuses by name: a tarball
# downloaded with no digest to check it against is not installed.
sce_swift_pin_digest() {
    local found
    found="$(grep -o "ubuntu$1) sum=[0-9a-f]\{64\}" "${SCE_REPO_ROOT}/bx.toml")" || return
    found="${found%%$'\n'*}"
    printf '%s\n' "${found##*sum=}"
}

# What `swift --version` prints in parentheses for release `$1`: `swift-6.4-RELEASE`
# for `6.4.0`, `swift-6.4.1-RELEASE` for a patch release.
sce_swift_pin_tag() {
    printf 'swift-%s-RELEASE\n' "${1%.0}"
}
