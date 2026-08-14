#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Shared helpers for the gate scripts under `scripts/gates/`.
#
# Every gate is a standalone script: `scripts/gate <slug>` runs one, and so
# does executing the file directly. That is the property the previous
# arrangement lacked — the gates lived inline in the pre-push hook, so the
# only way to exercise one was to attempt a push, and the "run this before
# pushing" recipes ended up written down in prose elsewhere instead of being
# runnable.
#
# Sourcing this file puts the caller at the repository root, so a gate body
# can use repo-relative paths without restating the `cd`.

set -euo pipefail

SCE_REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$SCE_REPO_ROOT"

# Name of the running gate, used in failure messages. Derived from the
# script filename so a gate cannot report a slug it is not.
SCE_GATE_SLUG="$(basename "${BASH_SOURCE[1]:-unknown}" .sh)"

# Fail the gate with a message that names the slug. The slug — not a stage
# number — is the stable identifier: it survives reordering, so a failure
# message, a doc reference and a memory note all keep pointing at the same
# gate after the run order changes.
sce_gate_fail() {
    printf '\nERROR gate[%s]: %s\n' "$SCE_GATE_SLUG" "$1" >&2
    exit 1
}

# Exit status for "this check could not run", kept distinct from 1 ("what the
# gate judges is wrong"). The two are different verdicts about different
# parties: exit 1 says the author's tree is bad, exit 3 says the gate's own
# inputs are missing.
#
# Collapsing them is not a cosmetic loss. `gate_registry_contract` drives
# `ledger-citations.sh` over a fixture holding a real citation and asserts the
# gate accepts it; on a machine without the rev-pinned `mnemosyne-cli` the gate
# refused — correctly, and saying so — and the assertion reported "the staged
# gate rejected a real citation". A verdict about the author's text for a fault
# in the checker's own inputs. Measured on the build machine 2026-08-12: two
# tests red, nothing wrong with the tree.
#
# `tools/mnemosyne-adoption/migrate_citations.py` reached the same conclusion
# first and named the constant `EXIT_CANNOT_RUN`; the value is 3 here so the
# two halves of one gate agree.
sce_gate_cannot_run() {
    printf '\nERROR gate[%s]: cannot run — %s\n' "$SCE_GATE_SLUG" "$1" >&2
    exit 3
}

# Progress line within a gate that runs several commands.
sce_gate_step() {
    printf '  [%s] %s\n' "$SCE_GATE_SLUG" "$1" >&2
}

# Register a cleanup command to run when the gate exits, for any reason.
# Gates that start a server or a scratch directory use this instead of
# their own `trap`, because a second `trap ... EXIT` silently replaces the
# first — which is how a background HTTP server outlived its gate before
# the hook composed its cleanups into one function.
SCE_GATE_CLEANUPS=()
sce_gate_on_exit() { SCE_GATE_CLEANUPS+=("$1"); }
_sce_gate_run_cleanups() {
    local c
    for c in "${SCE_GATE_CLEANUPS[@]:-}"; do
        [[ -n "$c" ]] && eval "$c" || true
    done
    # An EXIT trap's final status becomes the script's exit status; a
    # cleanup that legitimately returns non-zero must not turn a passing
    # gate into a failing one.
    return 0
}
trap _sce_gate_run_cleanups EXIT

# Resolve the in-tree sce-codegen the way every other in-repo consumer
# does, rather than letting a gate reach for whatever is on PATH. A gate
# that measures "some installed binary against this tree's templates" is
# measuring a combination that never shipped.
sce_gate_codegen() {
    source "$SCE_REPO_ROOT/scripts/lib/sce_codegen.sh"
    sce_codegen_require "$SCE_REPO_ROOT"
}

# Start the W3C BasicHTTP fixture server on localhost:8080 for the caller,
# and stop it when the gate exits.
#
# Several conformance arms need it — the Rust workspace suite, the Go suite,
# and any other backend running the W3C SCXML C.2 fixtures (test_201, _509,
# _513, _518-520, _532, _534, _567), which issue real HTTP POSTs and fail on
# connection-refused. Each CI job starts its own copy; a gate that mirrors one
# of those jobs has to do the same, and doing it here rather than in each gate
# is what keeps the settle window, the failure message and the cleanup from
# being written three times and drifting.
#
# The C++ suite is the opposite case and must NOT have this: a live 8080 makes
# its ctest run report 13 tests Not Run. That asymmetry is the reason this is a
# call a gate makes rather than something the runner does for everyone.
sce_gate_http_fixture_server() {
    command -v node >/dev/null 2>&1 \
        || sce_gate_fail "node.js required for the W3C HTTP fixture server (apt install nodejs)"

    local log
    log="$(mktemp)"
    sce_gate_on_exit "rm -f '$log'"
    node "$SCE_REPO_ROOT/tests/w3c/standalone_http_server.js" 8080 /test >"$log" 2>&1 &
    local pid=$!
    sce_gate_on_exit "kill $pid 2>/dev/null"
    # Match the CI workflows' settle window before issuing requests.
    sleep 1
    if ! kill -0 "$pid" 2>/dev/null; then
        cat "$log" >&2
        sce_gate_fail "W3C HTTP fixture server failed to start (port 8080 already in use?)"
    fi
    sce_gate_step "W3C HTTP fixture server up on localhost:8080"
}

# Resolve the main CMake tree into SCE_MAIN_BUILD_DIR, configuring it when it
# is not ready, and refuse a tree configured for a build type CI never builds.
#
# Three gates judge this one directory — the C++ conformance suite, the C11 arm
# and the rest of the ctest suite — and each carried its own copy of this
# block. The copies had already diverged: `w3c-cpp` keys readiness on the
# GENERATOR's own output file after measuring that a CI-restored cache carries
# `CMakeCache.txt` without `build.ninja`, while `w3c-c11` still keyed it on the
# cache alone and would send ninja into a directory it cannot build in. One
# reader means one answer, and it means the next fix lands for all three.
#
# Sets a variable rather than echoing one: `sce_gate_fail` exits, and an exit
# inside `$( )` ends the subshell while the caller carries on with an empty
# value — a gate that judges the empty string is worse than one that stops.
sce_main_build_dir() {
    SCE_MAIN_BUILD_DIR="${SCE_W3C_BUILD_DIR:-build}"

    # The lane builds RelWithDebInfo under Ninja. A gate that judged a
    # differently configured binary would be reporting on something CI never
    # sees — the defect `forge-cpp` carried until its build type was pinned. An
    # existing tree is reused rather than reconfigured, because a developer's
    # build directory is theirs; the mismatch is reported with the command that
    # fixes it. The cache is read for the build type it records, which is a
    # claim about the tree being RIGHT rather than about it being READY.
    local configured
    if [[ -f "$SCE_MAIN_BUILD_DIR/CMakeCache.txt" ]]; then
        configured="$(sed -n 's/^CMAKE_BUILD_TYPE:STRING=//p' "$SCE_MAIN_BUILD_DIR/CMakeCache.txt")"
        if [[ "$configured" != "RelWithDebInfo" ]]; then
            sce_gate_fail "$SCE_MAIN_BUILD_DIR is configured CMAKE_BUILD_TYPE=${configured:-<unset>}; the lane builds RelWithDebInfo, so this tree would judge a different binary. Reconfigure with: cmake -B $SCE_MAIN_BUILD_DIR -DCMAKE_BUILD_TYPE=RelWithDebInfo -G Ninja"
        fi
    fi

    # The file the cache's OWN generator produces, not "any build file". This
    # tree carried a stale `Makefile` from an earlier Make-generator configure
    # next to a cache that names Ninja, so "either one is present" answered yes
    # for a directory ninja could not build in.
    # `|| true` because a tree that does not exist yet is the normal first run,
    # not an error — and without it this line ENDS the gate. An assignment
    # takes the exit status of its command substitution, `sed` exits 2 on a
    # missing file, and `set -e` at the top of this file turns that into a
    # silent death: no `sce_gate_fail`, no reason, just a non-zero gate.
    # Measured 2026-08-12 on the first CI run of the cpp-suite lane, which is
    # the only one that can start with no `build/` at all — the others' lanes
    # restore a CMake cache first, so all three carried this and only one hit
    # it.
    local generator generated
    generator="$(sed -n 's/^CMAKE_GENERATOR:INTERNAL=//p' "$SCE_MAIN_BUILD_DIR/CMakeCache.txt" 2>/dev/null || true)"
    case "$generator" in
        Ninja*) generated="$SCE_MAIN_BUILD_DIR/build.ninja" ;;
        "")     generated="" ;;
        *)      generated="$SCE_MAIN_BUILD_DIR/Makefile" ;;
    esac
    if [[ -z "$generated" || ! -f "$generated" ]]; then
        sce_gate_step "configuring $SCE_MAIN_BUILD_DIR (RelWithDebInfo, mirroring the lane)"
        local GENERATOR=()
        command -v ninja >/dev/null 2>&1 && GENERATOR=(-G Ninja)
        # `SCE_ENABLE_MESH=ON` because `cpp-suite` asserts a floor of 140
        # registered non-c11 cases and the mesh tests are most of them. The
        # option defaults OFF, so the configure written here could not reach
        # the floor the gate it serves demands: the gate only ever passed by
        # INHERITING a `build/` somebody had configured by hand, and reported
        # 59-of-287 the first time it had to make its own.
        # Measured 2026-08-14, after this tree's `build/` was deleted. The
        # same shape explains a build machine that had never been hand-
        # configured registering the same 59.
        # A gate whose verdict depends on how a directory was once configured
        # by a person is not a gate; the configure and the floor have to be
        # written next to each other.
        cmake -B "$SCE_MAIN_BUILD_DIR" -DCMAKE_BUILD_TYPE=RelWithDebInfo \
              -DBUILD_TESTS=ON -DSCE_ENABLE_MESH=ON \
              ${GENERATOR+"${GENERATOR[@]}"} -Wno-dev >/dev/null \
            || sce_gate_fail "cmake configure"
    fi
}

# Remove ctest log temporaries that interrupted runs left behind.
#
# ctest writes `Testing/Temporary/LastTest.log.tmpNNNNN` while a suite runs and
# renames it to `LastTest.log` when the suite finishes. So every `.tmp` file
# still present belongs to a run that was killed — a Ctrl-C, a timeout, a
# machine that went down — and nothing in this tree ever removed one.
#
# Measured 2026-08-12: 40 orphans totalling 774 MB in `build/Testing`, all from
# a single day in April, one of them 652 MB on its own. A completed run's log
# is 1 MB; the 652 MB one had been running at spdlog debug level. The size is
# incidental — the unbounded part is that the count only ever goes up.
#
# Age, not mere existence: a suite running right now owns a fresh `.tmp`, and
# deleting that would corrupt a concurrent gate's log. The threshold is far
# longer than any suite here (the longest measured is 437s), so a live run is
# never a candidate.
sce_prune_ctest_temporaries() {
    local dir="$1/Testing/Temporary"
    [[ -d "$dir" ]] || return 0
    local pruned
    pruned="$(find "$dir" -maxdepth 1 -name 'LastTest.log.tmp*' -mmin +60 -print -delete 2>/dev/null | wc -l)"
    if (( pruned > 0 )); then
        sce_gate_step "pruned $pruned ctest log(s) left by interrupted runs"
    fi
    return 0
}

# Refuse to run when something already holds 8080.
#
# The C++ conformance suite reports 13 of its cases Not Run when the fixture
# server is live, which is a smaller suite reported as a passing one. The case
# floor would catch the count, but not name the cause; this does.
sce_gate_requires_free_http_port() {
    if command -v ss >/dev/null 2>&1 && ss -ltn 2>/dev/null | grep -q ':8080 '; then
        sce_gate_fail "something is listening on localhost:8080. The W3C HTTP fixture server makes this suite report 13 cases Not Run, so the run would be a smaller suite reported as a passing one. Stop it and retry."
    fi
}

# Resolve a tool the gate needs, or decide honestly what its absence
# means.
#
# Two callers want opposite things from a missing toolchain. A developer
# on a machine without clang wants the rest of the suite to keep
# running; a CI lane named for a check wants that check to have actually
# run before it reports green. The switch between them is
# `SCE_REQUIRE_TOOLS`, which is the same variable
# `sce-build/src/toolchain.rs` uses on the Rust side, so both halves of
# the harness answer to one setting.
#
# Without it: skip, saying so on stderr. With it: fail, naming the
# package that supplies the binary.
#
# `sce_gate_requires_tool` is the marker `hook_ci_parity.rs` scans for.
# The pairing it enforces — a lane that runs a skip-capable gate must
# set the variable and install the package — is what makes "CI installs
# it" a checked claim rather than a comment. The comment was already
# there, above the clang-19 install in `embed-vendor-smoke.yml`, saying
# an uninstalled toolchain "would turn this job green without running
# the assertion it exists for". Nothing acted on it.
#
# Returns 1 when the caller should skip, so a call site reads:
#   sce_gate_requires_tool clang++-19 clang-19 || exit 0
sce_gate_requires_tool() {
    local bin="$1" package="$2"
    if command -v "$bin" >/dev/null 2>&1; then
        return 0
    fi
    case "${SCE_REQUIRE_TOOLS:-}" in
        "" | 0 | false)
            echo "SKIP: ${bin} not on PATH (install ${package}, or set SCE_REQUIRE_TOOLS=1 to make this a failure)" >&2
            return 1
            ;;
        *)
            sce_gate_fail "${bin} not on PATH and SCE_REQUIRE_TOOLS is set — install ${package}. A lane that sets this variable is claiming the check ran."
            ;;
    esac
}
