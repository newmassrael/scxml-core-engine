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
#   - Kotlin: backends/kotlin/forge-runtime/src/cborPinTest, compiled into jvmTest
#     with the codecs when SCE_CBOR_PIN_INPUTS is set (build.gradle.kts);
#   - C++, C11, Go, Python and Swift: tests/forge/cbor_pin/drivers/<backend>, each
#     built here against `sce-codegen generate --with-imports` of the codecs and the
#     backend's forge runtime, in a directory of this run's own outside the tree.
#
# Every driver writes one line a case into the run's directory, and `pin.py merge`
# folds the seven into tests/forge/cbor_pin/captured.json: the bytes once where the
# backends agree, each backend's where they do not (named under `disagreements`).
# A case captured.json already pins is held, not rewritten: a capture whose bytes
# for it moved on any backend fails and leaves the file as it was, which is how
# every later step of ADR 0101 shows it moved no pinned byte. So a run before the
# pin exists writes it, and every run after checks it.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"
SCE_REPO_ROOT="$REPO_ROOT"
export SCE_REPO_ROOT
source scripts/lib/sce_codegen.sh
source scripts/lib/sce_build_jobs.sh
source scripts/lib/swift_pin.sh

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
mapfile -t DOCUMENTS < <(python3 "$PIN/pin.py" codecs)
python3 "$PIN/pin.py" inputs "$INPUTS"

CODEGEN="$(sce_codegen_require "$REPO_ROOT")"

# The pinned codecs and every document they import, for one backend, below $2.
generate() {
    local language="$1" out="$2"
    shift 2
    mkdir -p "$out"
    "$CODEGEN" generate --with-imports "${DOCUMENTS[@]}" --language "$language" --output-dir "$out" "$@" >/dev/null
}

step "Rust"
SCE_CBOR_PIN_INPUTS="$INPUTS" SCE_CBOR_PIN_OUTPUT="$CAPTURE/rust.txt" \
    cargo test --jobs "$JOBS" -p sce-forge-conformance --features cbor-pin --test forge_cbor_pin

step "C++"
generate cpp "$WORK/cpp/generated"
"${CXX:-c++}" -std=c++17 -O2 -Wall -Wextra \
    -I "$WORK/cpp/generated" -I backends/cpp/forge-runtime/include \
    "$PIN/drivers/cpp/capture.cpp" -o "$WORK/cpp/capture"
"$WORK/cpp/capture" "$INPUTS" "$CAPTURE/cpp.txt"

step "C11"
generate c11 "$WORK/c11/generated"
"${CC:-cc}" -std=c11 -O2 -Wall -Wextra \
    -I "$WORK/c11/generated" -I backends/c/forge-runtime/include -I backends/c/runtime/include \
    "$PIN/drivers/c11/capture.c" -o "$WORK/c11/capture"
"$WORK/c11/capture" "$INPUTS" "$CAPTURE/c11.txt"

step "Go"
# A module of the run's own, whose generated packages are imported as
# sce.dev/cborpin/gen/<document>, over the forge runtime's module in the tree.
generate go "$WORK/go/gen" --go-module-prefix sce.dev/cborpin/gen
cat >"$WORK/go/go.mod" <<EOF
module sce.dev/cborpin

go 1.22

require github.com/newmassrael/sce-forge-runtime v0.0.0

replace github.com/newmassrael/sce-forge-runtime => $REPO_ROOT/backends/go/forge-runtime
EOF
cp "$PIN/drivers/go/main.go" "$WORK/go/main.go"
(cd "$WORK/go" && GOFLAGS=-mod=mod go run . "$INPUTS" "$CAPTURE/go.txt")

step "Kotlin"
SCE_CBOR_PIN_INPUTS="$INPUTS" SCE_CBOR_PIN_OUTPUT="$CAPTURE/kotlin.txt" \
    SCE_CBOR_PIN_DOCUMENTS="$(printf '%s\n' "${DOCUMENTS[@]}")" \
    scripts/gradlew-on-ci-jdk.sh :sce-forge-runtime-kotlin:jvmTest \
    --tests 'com.sce.forge.cborpin.CborPinCaptureTest' --console=plain

step "Python"
generate python "$WORK/python/generated"
python3 "$PIN/drivers/python/capture.py" "$WORK/python/generated" "$INPUTS" "$CAPTURE/python.txt"

step "Swift"
# The toolchain bx.toml pins, where provision installs it; a `swift` on PATH
# otherwise, as on a machine that installed the same release by hand.
SWIFT="$HOME/.local/share/swift/$(sce_swift_pin_version)/bin/swift"
if [ ! -x "$SWIFT" ]; then
    SWIFT="$(command -v swift)" || {
        echo "capture-cbor-pin: no Swift toolchain at $SWIFT nor on PATH" >&2
        exit 1
    }
fi
SWIFT_PACKAGE="$WORK/swift"
SWIFT_SOURCES="$SWIFT_PACKAGE/Sources/CborPinCapture"
generate swift "$WORK/swift-generated"
mkdir -p "$SWIFT_SOURCES"
find "$WORK/swift-generated" -name '*.swift' -exec cp {} "$SWIFT_SOURCES/" \;
cp "$PIN/drivers/swift/main.swift" "$SWIFT_SOURCES/main.swift"
cat >"$SWIFT_PACKAGE/Package.swift" <<EOF
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
"$SWIFT" build --package-path "$SWIFT_PACKAGE" -c release --jobs "$JOBS"
"$SWIFT_PACKAGE/.build/release/CborPinCapture" "$INPUTS" "$CAPTURE/swift.txt"

step "pin"
python3 "$PIN/pin.py" merge "$CAPTURE"
