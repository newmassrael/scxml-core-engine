#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Capture what every backend's CBOR encode writes for the pinned values, and pin it
# (docs/adr/0101, Landing order step 0 and section 8.3). Declared as the bx action
# `capture-cbor-pin`; run it from the repository root:
#
#     scripts/capture_cbor_pin.sh
#
# tests/forge/cbor_pin/pin.py states the values (cases.json) and writes each in
# deterministic CBOR. Each of the seven backends' drivers decodes those bytes with
# the codec the generator writes for it and encodes the value it read again:
#
#   - Rust: backends/rust/forge-conformance/tests/forge_cbor_pin.rs, built for the
#     crate's `cbor-pin` feature, whose build.rs generates the codecs;
#   - Kotlin: backends/kotlin/forge-runtime/src/cborPinTest, the compilation
#     `cborPin` of that module, run by its task `cborPinCapture`, both configured
#     only when SCE_CBOR_PIN_INPUTS is set (build.gradle.kts);
#   - C++, C11, Go, Python and Swift: tests/forge/cbor_pin/drivers/<backend>, each
#     built here against `sce-codegen generate --with-imports` of the codecs and the
#     backend's forge runtime, in a directory of this run's own outside the tree.
#
# Every backend runs, whichever failed before it, so one run names every backend
# that failed, each by name with its status; a run with any failure stops before
# the pin and leaves it as it was. Otherwise every driver has written one line a
# case into the run's directory, and `pin.py merge` folds the seven into
# tests/forge/cbor_pin/captured.json: the bytes once where the backends agree, each
# backend's where they do not (named under `disagreements`). A case captured.json
# already pins is held, not rewritten: a capture whose bytes for it moved on any
# backend fails and leaves the file as it was, which is how every later step of
# ADR 0101 shows it moved no pinned byte. So a run before the pin exists writes it,
# and every run after checks it.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"
SCE_REPO_ROOT="$REPO_ROOT"
export SCE_REPO_ROOT
source scripts/lib/sce_codegen.sh
source scripts/lib/sce_build_jobs.sh
source scripts/lib/swift_pin.sh
source scripts/lib/go_toolchain.sh

PIN="tests/forge/cbor_pin"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
INPUTS="$WORK/inputs.txt"
CAPTURE="$WORK/capture"
mkdir -p "$CAPTURE"
JOBS="$(sce_build_jobs_value)"

step() {
    printf '==> capture-cbor-pin: %s\n' "$*"
}

step "the cases are values of their documents"
python3 "$PIN/pin.py" check
# Absolute paths: the Kotlin build reads them from the environment as they are.
mapfile -t DOCUMENTS < <(python3 "$PIN/pin.py" codecs | sed "s|^|$REPO_ROOT/|")
python3 "$PIN/pin.py" inputs "$INPUTS"

CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

# The pinned codecs and every document they import, for one backend, below $2.
generate() {
    local language="$1" out="$2"
    shift 2
    mkdir -p "$out"
    "$CODEGEN" generate --with-imports "${DOCUMENTS[@]}" --language "$language" --output-dir "$out" "$@" >/dev/null
}

capture_rust() {
    SCE_CBOR_PIN_INPUTS="$INPUTS" SCE_CBOR_PIN_OUTPUT="$CAPTURE/rust.txt" \
        cargo test --jobs "$JOBS" -p sce-forge-conformance --features cbor-pin --test forge_cbor_pin
}

capture_cpp() {
    generate cpp "$WORK/cpp/generated"
    "${CXX:-c++}" -std=c++17 -O2 -Wall -Wextra \
        -I "$WORK/cpp/generated" -I backends/cpp/forge-runtime/include \
        "$PIN/drivers/cpp/capture.cpp" -o "$WORK/cpp/capture"
    "$WORK/cpp/capture" "$INPUTS" "$CAPTURE/cpp.txt"
}

capture_c11() {
    generate c11 "$WORK/c11/generated"
    "${CC:-cc}" -std=c11 -O2 -Wall -Wextra \
        -I "$WORK/c11/generated" -I backends/c/forge-runtime/include -I backends/c/runtime/include \
        "$PIN/drivers/c11/capture.c" -o "$WORK/c11/capture"
    "$WORK/c11/capture" "$INPUTS" "$CAPTURE/c11.txt"
}

capture_go() {
    # The release the Go modules declare, first on PATH with GOTOOLCHAIN=local
    # where it is installed, else the go on PATH, said either way.
    sce_go_select capture-cbor-pin || true
    # A module of the run's own, whose generated packages are imported as
    # sce.dev/cborpin/gen/<document>, over the forge runtime's module in the tree.
    generate go "$WORK/go/gen" --go-module-prefix sce.dev/cborpin/gen
    # The Go release is the forge runtime's own `go` line, read from its go.mod.
    # Go chooses the toolchain by the main module's line: one older than the
    # runtime's ran the machine's go 1.22.2 and stopped at "requires go >=
    # 1.23.0". The runtime's line asks for the release sce_go_select put first
    # on PATH, and where that tool is absent, for the one go fetches.
    local go_release
    go_release="$(sed -n 's/^go \([0-9][0-9.]*\)$/\1/p' "$REPO_ROOT/backends/go/forge-runtime/go.mod")"
    [ -n "$go_release" ] || {
        echo "capture-cbor-pin: no go line in backends/go/forge-runtime/go.mod" >&2
        return 1
    }
    cat >"$WORK/go/go.mod" <<EOF
module sce.dev/cborpin

go $go_release

require github.com/newmassrael/sce-forge-runtime v0.0.0

replace github.com/newmassrael/sce-forge-runtime => $REPO_ROOT/backends/go/forge-runtime
EOF
    cp "$PIN/drivers/go/main.go" "$WORK/go/main.go"
    (cd "$WORK/go" && GOFLAGS=-mod=mod go run . "$INPUTS" "$CAPTURE/go.txt")
}

capture_kotlin() {
    SCE_CBOR_PIN_INPUTS="$INPUTS" SCE_CBOR_PIN_OUTPUT="$CAPTURE/kotlin.txt" \
        SCE_CBOR_PIN_DOCUMENTS="$(printf '%s\n' "${DOCUMENTS[@]}")" \
        scripts/gradlew-on-ci-jdk.sh :sce-forge-runtime-kotlin:cborPinCapture --console=plain
}

capture_python() {
    generate python "$WORK/python/generated"
    python3 "$PIN/drivers/python/capture.py" "$WORK/python/generated" "$INPUTS" "$CAPTURE/python.txt"
}

capture_swift() {
    # The toolchain bx.toml pins, where provision installs it; a `swift` on PATH
    # otherwise, as on a machine that installed the same release by hand.
    local swift="$HOME/.local/share/swift/$(sce_swift_pin_version)/bin/swift"
    if [ ! -x "$swift" ]; then
        swift="$(command -v swift)" || {
            echo "capture-cbor-pin: no Swift toolchain at $swift nor on PATH" >&2
            return 1
        }
    fi
    local package="$WORK/swift"
    local sources="$package/Sources/CborPinCapture"
    generate swift "$WORK/swift-generated"
    mkdir -p "$sources"
    find "$WORK/swift-generated" -name '*.swift' -exec cp {} "$sources/" \;
    cp "$PIN/drivers/swift/main.swift" "$sources/main.swift"
    cat >"$package/Package.swift" <<EOF
// swift-tools-version:6.0
import PackageDescription

let package = Package(
    name: "CborPinCapture",
    platforms: [.macOS(.v15)],
    dependencies: [.package(path: "$REPO_ROOT/backends/swift/forge-runtime")],
    targets: [
        .executableTarget(
            name: "CborPinCapture",
            dependencies: [.product(name: "SceForgeRuntime", package: "forge-runtime")]
        )
    ]
)
EOF
    "$swift" build --package-path "$package" -c release --jobs "$JOBS"
    "$package/.build/release/CborPinCapture" "$INPUTS" "$CAPTURE/swift.txt"
}

# Each backend in a subshell of its own that stops at its first failing command
# (not in a condition, where bash would not stop it), its status kept by name.
FAILED=()
for backend in rust cpp c11 go kotlin python swift; do
    step "$backend"
    set +e
    (
        set -e
        "capture_$backend"
    )
    status=$?
    set -e
    if [ "$status" -ne 0 ]; then
        echo "capture-cbor-pin: the $backend driver failed (status $status); its output is above" >&2
        FAILED+=("$backend")
    fi
done
if [ "${#FAILED[@]}" -ne 0 ]; then
    echo "capture-cbor-pin: ${#FAILED[@]} backend(s) failed: ${FAILED[*]}; the pin is left as it was" >&2
    exit 1
fi

step "pin"
python3 "$PIN/pin.py" merge "$CAPTURE"
