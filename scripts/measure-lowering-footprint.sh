#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The NET of the swap: what a C++ consumer takes on if the frontend
# becomes callable at run time, MINUS what leaves with the rewriter it
# replaces.
#
# `docs/SCE_LUA_TRANSLATION_SEAM.md` priced this as an ADDITION — the
# cdylib's reachable lowering code, +214 KB — and that is the wrong
# shape. `EcmaScriptToLuaTransformer` is the run-time adapter the
# frontend would displace, so a decision reads a net, not a gross. The
# document has already retired one number for exactly this reason: "159
# of 382" was a real figure about the wrong subject.
#
# TWO HALVES, MEASURED THE SAME WAY WHEREVER POSSIBLE.
#
#   IN  — build `sce-build` as a cdylib twice, once with the `ffi`
#         feature (which exports the four lowering entry points plus the
#         scope handle) and once without. The difference is what the
#         linker kept because the C surface reaches it.
#
#         ⚠ The probe hands the lowered string BACK. The first version of
#         this measurement used wrappers that returned NULL, which lets
#         the linker drop the emitter the measurement exists to weigh.
#
#   OUT — the rewriter translation unit's own sections. A link-level
#         difference is not available on this half: `LuaEngine` holds an
#         `EcmaScriptToLuaTransformer` member, so a tree without the TU
#         does not link, and the honest measurement is the object's
#         allocatable sections. Comdat template instantiations are
#         EXCLUDED: the linker folds them with copies from other TUs, so
#         they are not what this TU alone takes away.
#
# ⚠ The two halves are not the same profile — the cdylib is a cargo
# release build and the object comes from the configured C++ tree
# (RelWithDebInfo). Both are -O2; the debug sections are excluded here
# and never reach a shipped image. Treat the NET as a magnitude, not as a
# byte-exact figure, and re-run rather than citing it.
#
# WHY BOTH HALVES ARE PAID BY THE SAME POPULATION, which is what makes a
# net meaningful at all: `src/scripting/EcmaScriptToLuaTransformer.cpp`
# is listed UNCONDITIONALLY in `sce/sce_base_sources.cmake` — not behind
# `$<$<BOOL:${SCE_ENABLE_LUA}>:...>` the way `LuaEngine.cpp` is — so it is
# compiled by every C++ configure in the tree. That is the same
# population the ledger names for the link.

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

BUILD_DIR="${SCE_BUILD_DIR:-build}"
OBJ="$BUILD_DIR/sce/CMakeFiles/sce_base.dir/src/scripting/EcmaScriptToLuaTransformer.cpp.o"

# Ask CARGO where it put the artifact rather than spelling a profile
# directory. `codegen_binary_resolution` turned a lane red over exactly
# that once: a second copy of the search is a second copy of the profile
# ORDER, which is what decides whether a stale binary outranks a fresh
# one.
cdylib_path() {
    cargo rustc -p sce-build --lib --release --crate-type cdylib "$@" \
        --message-format=json 2>/dev/null \
        | python3 -c 'import sys, json
for line in sys.stdin:
    try:
        m = json.loads(line)
    except ValueError:
        continue
    for f in m.get("filenames") or []:
        if f.endswith(".so"):
            print(f)'
}

echo "measure-lowering-footprint: building the cdylib without the C surface" >&2
bare="$(cdylib_path | tail -1)"
[ -n "$bare" ] || { echo "cargo reported no .so for the bare build" >&2; exit 1; }
bare_bytes="$(stat -c %s "$bare")"

echo "measure-lowering-footprint: building it with --features ffi" >&2
probed="$(cdylib_path --features ffi | tail -1)"
[ -n "$probed" ] || { echo "cargo reported no .so for the probed build" >&2; exit 1; }
probed_bytes="$(stat -c %s "$probed")"

exported="$(nm -D --defined-only "$probed" | grep -c ' T sce_' || true)"

if [ ! -f "$OBJ" ]; then
    echo "measure-lowering-footprint: no rewriter object at $OBJ" >&2
    echo "  Configure and build the C++ tree first, or set SCE_BUILD_DIR." >&2
    echo "  The OUT half cannot be measured without it, and a NET printed" >&2
    echo "  from the IN half alone would be the very defect this script" >&2
    echo "  exists to remove." >&2
    exit 1
fi

python3 - "$bare_bytes" "$probed_bytes" "$exported" "$OBJ" <<'PY'
import re, subprocess, sys

bare, probed, exported, obj = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3]), sys.argv[4]

DEBUG = (".debug", ".comment", ".note", ".group")
own = comdat = 0
for line in subprocess.run(["size", "-A", obj], capture_output=True, text=True).stdout.splitlines():
    m = re.match(r"^(\.\S+)\s+(\d+)\s+\d+\s*$", line.strip())
    if not m:
        continue
    name, size = m.group(1), int(m.group(2))
    if name.startswith(DEBUG):
        continue
    # A comdat template instantiation is folded with copies from every
    # other TU, so it is not what this TU alone would take away.
    if re.match(r"^\.(text|rodata|data\.rel\.ro)\._Z", name):
        comdat += size
    else:
        own += size

gained = probed - bare
net = gained - own
print()
print(f"LoweringFootprint census: exported_symbols={exported} "
      f"cdylib_bare={bare} cdylib_probed={probed} "
      f"in_bytes={gained} out_bytes={own} out_comdat={comdat} net_bytes={net}")
print()
print(f"  IN   the linker keeps {gained:>8} B ({gained/1024:7.1f} KB) once the C surface reaches it")
print(f"  OUT  the rewriter TU  {own:>8} B ({own/1024:7.1f} KB) leaves with it")
print(f"       (+{comdat} B of comdat excluded — folded with other TUs)")
print(f"  NET  {net:>8} B ({net/1024:7.1f} KB)"
      f"  {'added' if net > 0 else 'saved'} by the swap")
print()
if net > 0:
    print(f"  Pricing the link as an ADDITION overstates it by {own/gained*100:.0f}%.")
else:
    print("  The swap is a NET SAVING: the rewriter is larger than the surface.")
PY
