#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Install the pinned Emscripten SDK where this repository looks for it.
#
# Four gates build or read WebAssembly made with emscripten: `w3c-wasm` and
# `cpp-suite-wasm` (the two emscripten trees, run under the Node emsdk ships),
# `visualizer-wasm` (the engine WASM the visualizer loads) and
# `visualizer-layout`, which measures what `visualizer-wasm` built. Their hosted
# lanes clone emsdk and install the release named below; this script is the
# same install, written once, for the build machines.
#
# WHY A TOOL OF ITS OWN (2026-11-08). An update to main is judged on the build
# machines with every gate it reaches, `ci_only` included. The emscripten gates
# skip and pass where emsdk is absent (a developer without it still pushes), so
# a landing judged on a machine without it would count three gates as passed
# that ran nothing. Declared as `[platform.linux.tools.emsdk]` in bx.toml, the
# SDK is installed by bx and the units of those gates run only where it is.
#
# The unit is the RELEASE: `EMSDK_REV` below is the one home of the pin on the
# build machines, and the install path derives from it. The hosted lanes name
# the same release in their `emsdk install` steps; the registry's self-test
# holds the two to one value. The release pins the Node emsdk ships, which is
# the Node the WASM suites run under.
#
# Idempotent: an install whose `emcc --version` names the pinned release is
# left alone.
#
# What it needs and does not install: git, python3 and network access to
# GitHub and to the emsdk release storage.
#
# The build machines get it from `[platform.linux.tools.emsdk]` in bx.toml,
# which runs this script with EMSDK_ROOT set to bx's store entry. The gates
# (`sce_gate_activate_emsdk` in scripts/gates/lib.sh) look for
# `emsdk_env.sh` at the path derived here.
set -euo pipefail

# The pin: the release the hosted lanes install.
EMSDK_REV="6.0.13"
EMSDK_URL="https://github.com/emscripten-core/emsdk.git"

EMSDK_ROOT="${EMSDK_ROOT:-$HOME/.local/share/emsdk/$EMSDK_REV}"
EMCC="$EMSDK_ROOT/upstream/emscripten/emcc"

# `grep` reads the whole answer (no `-q`): under `pipefail` an early exit would
# end `emcc` with SIGPIPE and read as an install that is not there.
if [[ -x "$EMCC" ]] && "$EMCC" --version 2>/dev/null | grep -F " $EMSDK_REV " >/dev/null; then
    echo "emsdk $EMSDK_REV is already installed at $EMSDK_ROOT"
    exit 0
fi

for tool in git python3; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "install_emsdk: '$tool' is not installed; see the header of this script" >&2
        exit 2
    }
done

# Cloned beside the root and copied in, because bx hands the install a store
# entry that already exists, and `git clone` refuses a directory that does.
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
git clone --quiet --depth 1 "$EMSDK_URL" "$WORK/emsdk"
mkdir -p "$EMSDK_ROOT"
cp -a "$WORK/emsdk/." "$EMSDK_ROOT/"

echo "installing emsdk $EMSDK_REV in $EMSDK_ROOT"
"$EMSDK_ROOT/emsdk" install "$EMSDK_REV"
"$EMSDK_ROOT/emsdk" activate "$EMSDK_REV"

# FROZEN CACHE. The release ships its system libraries prebuilt in the cache,
# and bx seals a store entry read-only once its install ends. Unfrozen, emcc
# still writes beside them (`cache/symbol_lists` at every link), so on a build
# machine every link failed with EACCES: CMake's own probes failed with it, and
# `visualizer-wasm` stopped at "Could NOT find Threads" (2026-11-20, hkl001).
# Frozen, emcc reads the cache and writes nothing there; a library the release
# did not ship is then a loud error, not a silent build into a sealed tree.
printf '\nFROZEN_CACHE = True\n' >>"$EMSDK_ROOT/.emscripten"

# The postcondition, asserted rather than assumed: the compiler answers with
# the pinned release, and the environment script the gates source is there.
"$EMCC" --version | grep -F " $EMSDK_REV " >/dev/null || {
    echo "install_emsdk: $EMCC does not report release $EMSDK_REV" >&2
    exit 1
}
[[ -f "$EMSDK_ROOT/emsdk_env.sh" ]] || {
    echo "install_emsdk: $EMSDK_ROOT/emsdk_env.sh is missing after activate" >&2
    exit 1
}
# A link with the frozen cache, the step that wrote into it: a program that
# includes pthread.h, as CMake's FindThreads probe does.
printf '#include <pthread.h>\nint main(void) { return 0; }\n' >"$WORK/probe.c"
EM_CONFIG="$EMSDK_ROOT/.emscripten" "$EMCC" "$WORK/probe.c" -o "$WORK/probe.js" || {
    echo "install_emsdk: $EMCC could not link a program from the frozen cache" >&2
    exit 1
}
echo "emsdk $EMSDK_REV installed at $EMSDK_ROOT"
