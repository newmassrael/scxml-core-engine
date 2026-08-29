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
# WHICH SIDE OF THE SEAM IT MEASURES: build-time lowering, and it is that
# path's CONTRACT rather than a measurement of it.
# `tests/ecmascript/lua_engine_divergences.json` says per entry which routes
# into the Lua engine still answer a case differently (`diverges_on`), and this
# gate holds the `build-time-lowering` one in BOTH directions — a case the
# lowered artifact gets wrong without being declared is red, and so is a case
# declared and answered correctly. The second direction is what lets that list
# empty.
#
# The POPULATION is the shared table (`tests/ecmascript/ecma262_semantics.json`)
# in full, not the divergence list. It used to be the list, which meant the
# table's other cases were never asked through a lowered artifact at all: a
# path's divergences cannot be enumerated by a list built from a different
# path's failures. The binary carries the same document generated BOTH ways for
# the reason the control always exists: the lowered one is the subject, the
# source-passing one is what keeps its green from being a suite that measures
# nothing.
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
for stem in ecma262_lowered ecma262_source ecma262_default; do
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
default_sm="$BUILD_DIR/tests/lowered_ecma262/generated/ecma262_default_sm.inl"
lowered_pairs="$(grep -c 'ScriptSource::lua(' "$lowered_sm" 2>/dev/null || true)"
source_pairs="$(grep -c 'ScriptSource::lua(' "$source_sm" 2>/dev/null || true)"
default_pairs="$(grep -c 'ScriptSource::lua(' "$default_sm" 2>/dev/null || true)"
lowered_pairs="${lowered_pairs:-0}"; source_pairs="${source_pairs:-0}"; default_pairs="${default_pairs:-0}"
if (( lowered_pairs == 0 )); then
    sce_gate_fail "the lowered artifact carries no ScriptSource::lua(...) call — it was generated without --script-engine lua, so both machines in this binary hand the engine ECMAScript and the control below compares one against itself"
fi
if (( source_pairs != 0 )); then
    sce_gate_fail "the control artifact carries $source_pairs ScriptSource::lua(...) call(s) — it was generated WITH --script-engine lua, so it is a second copy of the subject rather than a control"
fi
# The DERIVED default, read off the artifact rather than off the CMake source.
# `sce_add_state_machine` gives a `-DSCE_SCRIPT_ENGINE=lua` tree `--script-engine
# lua` when the caller names nothing, because an artifact built here can only run
# on the engine this tree compiled in. That is a claim about what the build
# PRODUCES, so it is asked of the product — and asked here as well as in the
# suite, because "it was emitted lowered" and "it answers like the lowered one"
# are two different readings and two artifacts can agree by both being wrong.
if (( default_pairs != lowered_pairs )); then
    sce_gate_fail "the artifact generated with NO SCRIPT_ENGINE_LANGUAGE carries $default_pairs ScriptSource::lua(...) call(s) against the explicitly-lowered one's $lowered_pairs — this tree selected a Lua engine and then emitted a machine for a different language, so sce_add_state_machine did not derive the target"
fi
sce_gate_step "lowered artifact: $lowered_pairs ScriptSource::lua(...) pair(s); derived default: $default_pairs; control: none"

sce_gate_step "running the lowered artifact"
LOG="$SCRATCH/ctest.log"
JUNIT="$SCRATCH/ctest.xml"
# `-V` rather than `--output-on-failure`: the suite prints a census line naming
# what it measured, and a number that only exists on a red run is a number
# nobody can cite from a green one.
ctest --test-dir "$BUILD_DIR" -R '^LoweredEcma262$' \
      --verbose --no-tests=error --output-junit "$JUNIT" 2>&1 | tee "$LOG"
ctest_status="${PIPESTATUS[0]}"
(( ctest_status == 0 )) || sce_gate_fail "the lowered C++ artifact did not answer ECMA-262 through the build-time lowering"

# `--no-tests=error` covers the zero case and the exit status covers a failing
# one. What neither covers is the case being SKIPPED while its fixture passes:
# `LoweredEcma262` declares `sce_build_is_current` as a required fixture, so a
# run of this regex is two ctest entries, and a run where only the fixture
# executed would exit 0 over a suite that never asked anything.
#
# ⚠ This is asked of ctest's JUNIT XML, not of its summary line, and the reason
# is measured rather than stylistic. The summary line's WORDING differs between
# ctest versions: CI's prints `100% tests passed, 0 tests failed out of 2` and
# the build machine's prints `100% tests passed out of 2`. A gate keyed on
# either spelling is green on one machine and red on the other over the same
# artifact — which is exactly what happened, in both directions, in two
# successive rounds. A tool's human-readable summary is not a machine contract;
# its `--output-junit` is.
python3 - "$JUNIT" <<'PY' || sce_gate_fail "ctest's JUnit report does not show LoweredEcma262 as a case that ran and passed"
import sys
import xml.etree.ElementTree as ET

path = sys.argv[1]
try:
    root = ET.parse(path).getroot()
except (OSError, ET.ParseError) as exc:
    print(f"cannot read ctest's JUnit report at {path}: {exc}", file=sys.stderr)
    raise SystemExit(1)

cases = root.iter("testcase")
subject = None
bad = []
for case in cases:
    name = case.get("name", "")
    # A case is a failure when it says so, and it is ALSO a failure when it was
    # skipped: a skip keeps ctest's exit status at 0 while asking nothing, which
    # is the one hole the exit status leaves.
    for tag in ("failure", "error", "skipped"):
        if case.find(tag) is not None:
            bad.append(f"{name}: {tag}")
    if name == "LoweredEcma262":
        subject = case

if subject is None:
    print("ctest's JUnit report carries no case named LoweredEcma262 — the run "
          "covered its fixture and not the suite", file=sys.stderr)
    raise SystemExit(1)
if bad:
    print("ctest reported these cases as not-passing: " + ", ".join(bad), file=sys.stderr)
    raise SystemExit(1)
PY

# The suite's own census, lifted out of the verbose log so a green run states
# what it measured. `docs/SCE_LUA_TRANSLATION_SEAM.md` re-derives its numbers
# from this line rather than from a paragraph someone typed.
census="$(sed -n 's/.*\(LoweredEcma262 census: .*\)/\1/p' "$LOG" | head -1)"
[[ -n "$census" ]] \
    || sce_gate_fail "the suite printed no census line, so this run cannot say how large a population it asked"
sce_gate_step "$census"

# The probe controls, held HERE and not only by the suite's own EXPECTs.
#
# Every condition verdict in that suite is gated on a probe: `agrees` refuses to
# read one whose probe says the engine could not evaluate the expression. A
# probe stuck on one answer therefore decides the whole measurement — stuck on
# "refused" makes every condition case a divergence, stuck on "evaluated" makes
# a genuine §scxml-5.9.1 refusal read as the answer `false`. The suite asserts
# it distinguishes; what the suite cannot assert is that it still ASKS. Delete
# the two `assertProbeDistinguishes` calls and every test in the file still
# passes over a probe nobody controls.
#
# So the readings are census fields, and a green run has to carry them. The
# refusal side must still read the sentinel — the fixture probes a member of an
# absent object, which raises in ECMA-262 and in Lua alike — and the evaluable
# side must read anything else, because the fixture probes a literal.
for artifact in lowered source; do
    refused="$(sed -n "s/.*[[:space:]]${artifact}-control-refused=\([^[:space:]]*\).*/\1/p" <<<"$census")"
    evaluable="$(sed -n "s/.*[[:space:]]${artifact}-control-evaluable=\([^[:space:]]*\).*/\1/p" <<<"$census")"
    [[ -n "$refused" && -n "$evaluable" ]] \
        || sce_gate_fail "the census names no probe control for the $artifact artifact — the suite stopped reporting whether the probe every condition verdict rests on can tell a refusal from an answer"
    [[ "$refused" == "<unevaluated>" ]] \
        || sce_gate_fail "the $artifact artifact's refusal control read '$refused', not the unevaluated sentinel — the probe is not reporting §scxml-5.9.1 refusals, so a guard the engine would not parse reads as the answer false"
    [[ "$evaluable" != "<unevaluated>" ]] \
        || sce_gate_fail "the $artifact artifact's evaluable control read the unevaluated sentinel over a literal — the probe is stuck on refusal, so every condition case is a divergence by construction"
done
sce_gate_step "the refusal probe reported both outcomes on both artifacts, on this run"

sce_gate_step "build-time lowering diverges exactly where the divergence list declares it to, and the source-passing control does not"
