#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Regenerate the Swift backend's committed machines and run its three SwiftPM
# packages: the statechart runtime (backends/swift/runtime), the forge runtime
# (backends/swift/forge-runtime) and the generated machines with the tests that drive
# them (backends/swift/tests).
#
# Like `./gradlew test` for Kotlin, it writes: the machines under
# backends/swift/tests/Sources are rewritten from the generator before they are
# compiled, so what is tested is what the templates write today and not what was
# last committed. Run on a build machine (`bx -- scripts/test_swift_backend.sh`), the
# rewritten files stay there, and `bx --action regen` is what brings them back.
#
# Usage (from repo root):
#   scripts/test_swift_backend.sh [package ...]    # runtime, forge-runtime, tests
#
# Requires:
#   sce-codegen (resolved by scripts/lib/sce_codegen.sh, built when missing) and a
#   Swift toolchain (`swift` on PATH, or ~/.local/share/swift/<version>/bin/swift).

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

# `swift` is a link into a toolchain whose own tools (`swiftc`, `swift-build`) sit
# beside the file it points at, and a toolchain installed by hand is not on PATH.
if ! command -v swift > /dev/null 2>&1; then
    swift_link="$(ls "$HOME"/.local/share/swift/*/bin/swift 2> /dev/null | LC_ALL=C sort | tail -n 1)"
    if [ -z "$swift_link" ]; then
        echo "error: no Swift toolchain on PATH or under ~/.local/share/swift" >&2
        exit 1
    fi
    PATH="$(dirname "$(readlink -f "$swift_link")"):$PATH"
    export PATH
fi

if [ "$#" -gt 0 ]; then
    PACKAGES=("$@")
else
    PACKAGES=(runtime forge-runtime tests)
fi

for package in "${PACKAGES[@]}"; do
    if [ "$package" = "tests" ]; then
        scripts/regen_swift_statecharts.sh
        scripts/regen_static_datamodel_swift.sh
    fi
    echo "== swift test: backends/swift/$package"
    (cd "backends/swift/$package" && swift test)
done
