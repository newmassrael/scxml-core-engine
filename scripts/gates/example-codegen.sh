#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Example SCXML codegen smoke.
#
# The emscripten-backed doom_wasm / visualizer builds are too heavy for a
# push-time gate, but codegen failures (the kind that broke the
# `urn:sce:extensions` -> `http://sce.dev/ext` namespace migration) surface
# well before link time. Run sce-codegen over each example SCXML in a
# scratch output dir and fail if any produce no artifacts.
#
# No CI counterpart, which is why the registry carries an explicit local
# trigger with the reason next to it rather than leaving the gate silently
# always-on or silently never-on.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

CODEGEN="$(sce_gate_codegen)"

GEN_OUT="$(mktemp -d)"
sce_gate_on_exit "rm -rf '$GEN_OUT'"

failures=0
for scxml in examples/doom_wasm/scxml/*.scxml examples/smart_light/smart_light.scxml; do
    [[ -f "$scxml" ]] || continue
    if ! "$CODEGEN" generate "$scxml" \
            --language cpp \
            --output-dir "$GEN_OUT/" >/dev/null 2>&1; then
        printf '  FAIL: %s\n' "$scxml" >&2
        failures=$((failures + 1))
    fi
done
(( failures == 0 )) || sce_gate_fail "$failures example(s) failed codegen"
