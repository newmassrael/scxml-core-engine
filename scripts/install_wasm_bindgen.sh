#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Install the wasm-bindgen command-line tool at the version Cargo.lock holds.
#
# `wasm32-scenarios` writes the JavaScript glue of the engine's wasm32 module
# with it, and the tool refuses a module built by another version of the
# `wasm-bindgen` crate, so the tool's version is the crate's, which Cargo.lock
# fixes. The hosted lane reads that version from Cargo.lock and runs `cargo
# install` itself; this script is the same install for the build machines.
#
# The unit is the VERSION: `WASM_BINDGEN_REV` below is the one home of it on the
# build machines, and the install path derives from it. Cargo.lock is not a file
# bx can read a version out of in one line, so the version is written here and
# the gate registry's self-test holds it to Cargo.lock's
# (`the-fleet-runs-the-wasm-bindgen-cargo-lock-holds`): a crate update that
# moves Cargo.lock without this line fails there, in the commit that made it.
#
# Idempotent: a tool at the version that answers `--version` is left alone.
#
# The build machines get it from `[platform.linux.tools.wasm-bindgen]` in
# bx.toml, which runs this script with WASM_BINDGEN_ROOT set to bx's store entry
# and CARGO_HOME to a scratch directory. The gate (`wasm32-scenarios`) looks for
# the tool at the path derived here when the one on PATH is another version.
set -euo pipefail

# The version: Cargo.lock's `wasm-bindgen` package.
WASM_BINDGEN_REV="0.2.117"

WASM_BINDGEN_ROOT="${WASM_BINDGEN_ROOT:-$HOME/.local/share/wasm-bindgen/$WASM_BINDGEN_REV}"
TOOL="$WASM_BINDGEN_ROOT/bin/wasm-bindgen"

if [[ -x "$TOOL" ]] && [[ "$("$TOOL" --version 2>/dev/null)" == "wasm-bindgen $WASM_BINDGEN_REV" ]]; then
    echo "wasm-bindgen $WASM_BINDGEN_REV is already installed at $TOOL"
    exit 0
fi

command -v cargo >/dev/null 2>&1 || {
    echo "install_wasm_bindgen: cargo is not installed" >&2
    exit 2
}

echo "installing wasm-bindgen $WASM_BINDGEN_REV in $WASM_BINDGEN_ROOT"
cargo install wasm-bindgen-cli --version "$WASM_BINDGEN_REV" --locked --root "$WASM_BINDGEN_ROOT"

# The postcondition, asserted rather than assumed.
[[ "$("$TOOL" --version)" == "wasm-bindgen $WASM_BINDGEN_REV" ]] || {
    echo "install_wasm_bindgen: $TOOL does not report wasm-bindgen $WASM_BINDGEN_REV" >&2
    exit 1
}
echo "wasm-bindgen $WASM_BINDGEN_REV installed at $TOOL"
