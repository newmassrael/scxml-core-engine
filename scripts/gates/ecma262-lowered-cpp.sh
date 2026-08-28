#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: ecma262-lowered-cpp.yml
#
# The gate `docs/SCE_LUA_TRANSLATION_SEAM.md` ended without: a C++ artifact
# generated with `--script-engine lua`, COMPILED with `-DSCE_SCRIPT_ENGINE=lua`,
# and RUN.
#
# Three comments in `tests/CMakeLists.txt` used to state the absence from the
# other side — "no gate configures -DSCE_SCRIPT_ENGINE=lua" — and each named a
# consequence: `LuaDOMBinding`, `LuaEngine`'s event-data reading and the
# refusing half of the language seam were compiled by every build and run by
# none. This is the gate that configures it.
#
# WHICH SIDE OF THE SEAM IT MEASURES: build-time lowering.
# `tests/ecmascript/lua_engine_divergences.json` measures the RUN-TIME rewriter,
# and that file is nonetheless the population — every entry is an expression the
# rewriter answers differently from ECMA-262, and a lowered artifact hands its
# engine Lua so the rewriter is never reached. The binary carries the same
# document generated BOTH ways for exactly that reason: the lowered one must
# answer the language, the source-passing one must not, and the second is what
# keeps the first from being a suite that measures nothing.
#
# WHY A TREE OF ITS OWN: `SCE_SCRIPT_ENGINE` is a CMake cache option and the
# definition it sets is PUBLIC on `sce_scripting`, so the selection is a property
# of the whole tree. A developer's `build/` is the quickjs one and must stay
# that way; this configures a throwaway `build_lua` beside it. Measured
# 2026-08-29 on the build machine: 7s to configure, ~50s to build the one
# target from cold, under 1s to run.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

# The generated artifact is the subject, so the generator is resolved the way
# every other in-tree consumer resolves it rather than from PATH.
CODEGEN="$(sce_gate_codegen)"

# The fixture is expanded from the two shared JSON tables at configure time, so
# there is no degraded mode: without python3 there is nothing to compile.
#
# Deliberately NOT `sce_gate_requires_tool`, which offers a SKIP. A gate whose
# whole subject is an artifact it cannot generate has nothing to say when the
# generator is missing, and `hook_ci_parity`'s skip-capable rule is right to
# demand that a lane claim the check ran — the honest answer here is to refuse
# rather than to make the lane promise a tool it does not need to install.
command -v python3 >/dev/null 2>&1 \
    || sce_gate_fail "python3 is not on PATH; tools/generate_lowered_ecma262_fixture.py expands the fixture this gate compiles, so there is nothing to measure without it"

# Named `build_lua`, not a bare mktemp name: the top-level CMakeLists refuses
# any build directory whose leaf is not `build` or `build_*`, so the scratch
# tree has to satisfy that rule too.
SCRATCH="$(mktemp -d)"
BUILD_DIR="$SCRATCH/build_lua"
sce_gate_on_exit "rm -rf '$SCRATCH'"

GENERATOR=()
if command -v ninja >/dev/null 2>&1; then
    GENERATOR=(-G Ninja)
else
    sce_gate_step "ninja not installed; using the default generator (same build type and selection)"
fi

# `BUILD_TESTS` stays ON — the target under test is a test target — and only
# that one target is built, so the rest of the suite is registered and not
# compiled. Examples are off because nothing here reads them.
#
# The engine is passed explicitly rather than relied on: `sce/CMakeLists.txt`
# validates the value and aborts on `lua` without `SCE_ENABLE_LUA`, so a
# configure that succeeds has already asserted the selection this gate is named
# for. The test source additionally refuses to compile without
# `SCE_SCRIPT_ENGINE_LUA`, so the two locks are independent.
sce_gate_step "configuring a SCE_SCRIPT_ENGINE=lua tree"
cmake -S "$SCE_REPO_ROOT" -B "$BUILD_DIR" \
      ${GENERATOR+"${GENERATOR[@]}"} \
      -DCMAKE_BUILD_TYPE=Debug \
      -DSCE_SCRIPT_ENGINE=lua \
      -DBUILD_EXAMPLES=OFF \
      -DSCE_CODEGEN="$CODEGEN" \
      >"$SCRATCH/configure.log" 2>&1 \
    || { tail -40 "$SCRATCH/configure.log" >&2; sce_gate_fail "SCE_SCRIPT_ENGINE=lua configure"; }

# The configure is where the selection is decided, so it is read back from the
# cache rather than assumed from the argument. A cache restored by CI could hold
# another value and the argument would then be describing a tree that does not
# exist.
selected="$(sed -n 's/^SCE_SCRIPT_ENGINE:STRING=//p' "$BUILD_DIR/CMakeCache.txt")"
[[ "$selected" == "lua" ]] \
    || sce_gate_fail "the configured tree selected SCE_SCRIPT_ENGINE='$selected', not 'lua' — this gate would be measuring the engine it exists to bypass"

# Both fixture flavours are expanded at configure time; if either is missing the
# target would still build against a stale copy from an earlier run, which in a
# throwaway tree means it would not build at all — but say so here, where the
# cause is legible.
for stem in ecma262_lowered ecma262_source; do
    [[ -f "$BUILD_DIR/tests/lowered_ecma262/$stem.scxml" ]] \
        || sce_gate_fail "the configure wrote no $stem.scxml — tools/generate_lowered_ecma262_fixture.py did not run"
done

sce_gate_step "building the lowered artifact and its source-passing control"
sce_gate_build "$BUILD_DIR" --target lowered_ecma262_test \
    || sce_gate_fail "lowered_ecma262_test build"

# The generated artifact is a verdict of its own and worth naming, because the
# whole point is that its expressions crossed the seam. A count of zero would
# mean the target compiled a machine that hands ECMAScript, and the suite would
# then be running its control twice.
lowered_sm="$BUILD_DIR/tests/lowered_ecma262/generated/ecma262_lowered_sm.inl"
source_sm="$BUILD_DIR/tests/lowered_ecma262/generated/ecma262_source_sm.inl"
lowered_pairs="$(grep -c 'ScriptSource::lua(' "$lowered_sm" 2>/dev/null || true)"
source_pairs="$(grep -c 'ScriptSource::lua(' "$source_sm" 2>/dev/null || true)"
lowered_pairs="${lowered_pairs:-0}"; source_pairs="${source_pairs:-0}"
if (( lowered_pairs == 0 )); then
    sce_gate_fail "the lowered artifact carries no ScriptSource::lua(...) call — it was generated without --script-engine lua, so both machines in this binary hand the engine ECMAScript and the control below compares one against itself"
fi
if (( source_pairs != 0 )); then
    sce_gate_fail "the control artifact carries $source_pairs ScriptSource::lua(...) call(s) — it was generated WITH --script-engine lua, so it is a second copy of the subject rather than a control"
fi
sce_gate_step "lowered artifact: $lowered_pairs ScriptSource::lua(...) pair(s); control: none"

sce_gate_step "running the lowered artifact"
LOG="$SCRATCH/ctest.log"
ctest --test-dir "$BUILD_DIR" -R '^LoweredEcma262$' \
      --output-on-failure --no-tests=error 2>&1 | tee "$LOG"
ctest_status="${PIPESTATUS[0]}"
(( ctest_status == 0 )) || sce_gate_fail "the lowered C++ artifact did not answer ECMA-262 through the build-time lowering"

# `--no-tests=error` covers the zero case, and the exit status covers a failing
# one. What neither covers is the case being SKIPPED while its fixture passes:
# `LoweredEcma262` declares `sce_build_is_current` as a required fixture, so a
# run of this regex is two ctest entries, and a run where only the fixture
# executed would exit 0 over a suite that never asked anything.
#
# So the named case is asserted by name. An earlier version of this line
# compared the total against 1 and failed on a green run for exactly that
# reason — the fixture is a case as far as ctest's totals are concerned.
grep -qE 'LoweredEcma262 [. ]*Passed' "$LOG" \
    || sce_gate_fail "ctest reported no passing case named LoweredEcma262 — the run covered its fixture and not the suite"
grep -qE '100% tests passed out of [0-9]+' "$LOG" \
    || sce_gate_fail "ctest did not report a fully passing run, so something in the selected set failed without changing the exit status"

sce_gate_step "the lowered artifact answered every declared divergence, and its source-passing control did not"
