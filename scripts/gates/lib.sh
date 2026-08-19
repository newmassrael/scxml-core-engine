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

# How many parallel jobs a build inside a gate may ask this machine for.
#
# NOT `$(nproc)`, which is what seven gate scripts asked for and which means
# "every core, whatever else is on the box". These gates do not run on a
# dedicated runner: this workstation carries sixteen concurrent sessions and
# six repositories at once, and a push here runs twenty-seven gates back to
# back.
#
# MEASURED 2026-08-19. One push gate reached `cargo test --workspace` while the
# machine was otherwise idle and produced eight concurrent rustc processes plus
# a linker at 335% CPU, driving 600MB/s of reads and 400MB/s of writes through
# the build cache; forty processes sat blocked on disk and CPU idle fell to
# 43%. Earlier the same day an uncapped build in a sibling repository pushed
# 13GB into swap and took the box to load 29 while 82% of its CPU sat idle —
# every bit of that load was processes waiting on the paging disk. Asking for
# all thirty-two cores does not make a gate finish sooner once the disk is the
# thing that is short.
#
# ⚠ THIS IS A SECOND SPELLING of the rule the build wrapper uses when it sizes
# a remote run (cores minus the current run queue, never below one), and it is
# kept to four lines for exactly that reason — a longer one would drift from
# the original and nothing would notice. The gates cannot simply call that
# wrapper: several of them link against artifacts an earlier gate left at a
# local path, so they cannot leave this machine at all.
#
# ⚠ Linux load counts uninterruptible sleep, so a box thrashing its disk reads
# as busier than its idle CPU suggests. That error is in the safe direction
# here: a machine already waiting on a disk is precisely the one that should
# not be handed thirty-two more compile jobs.
#
# `$SCE_BUILD_JOBS` overrides, for a runner that really does own its cores.
sce_build_jobs() {
    if [ -n "${SCE_BUILD_JOBS:-}" ]; then
        printf '%s' "$SCE_BUILD_JOBS"
        return 0
    fi
    local cores queued jobs
    cores="$(nproc)"
    queued="$(awk '{printf "%d", ($1 == int($1)) ? $1 : int($1) + 1}' /proc/loadavg)"
    jobs=$(( cores - queued ))
    if [ "$jobs" -lt 1 ]; then jobs=1; fi
    printf '%s' "$jobs"
}

# Progress line within a gate that runs several commands.
sce_gate_step() {
    printf '  [%s] %s\n' "$SCE_GATE_SLUG" "$1" >&2
}

# Build a configured tree, and when the build fails, SAY WHY.
#
# NOT `cmake --build … >/dev/null`, which is what five gates asked for and
# which is a diagnosis-proof redirection: ninja prints the compiler's own
# output on STDOUT, so a build that dies leaves the gate log holding nothing
# but the gate's five-word failure line. MEASURED 2026-08-19 on the build
# machine: a GCC internal compiler error inside a mesh test translation unit
# took three separate probes to name, because `cpp-suite` reported "main tree
# build" and the error itself had gone to /dev/null. The failure was real, the
# gate was right to stop, and the log could not say what had happened.
#
# Quiet on success for the reason the redirection was there in the first place
# — a passing gate's log is read by someone looking for the ONE failure in it,
# and a full compile log buries that. The output is captured either way and the
# tail is emitted only when the build fails, which is the shape `scripts/mutate`
# already uses (a quiet build plus a verbose replay) and the shape this file's
# ctest callers use (`tee` + `PIPESTATUS[0]`).
#
# Usage: sce_gate_build <build-dir> [extra cmake --build args...]
#        || sce_gate_fail "<what was being built>"
# `--parallel` is supplied here so the job-count rule stays in one place.
sce_gate_build() {
    local dir="$1"
    shift
    local log
    log="$(mktemp)"
    sce_gate_on_exit "rm -f '$log'"

    if cmake --build "$dir" "$@" --parallel "$(sce_build_jobs)" >"$log" 2>&1; then
        return 0
    fi

    # stderr, not stdout: the caller's `sce_gate_fail` writes there too, so the
    # cause and the verdict land in the same stream in the order they happened.
    #
    # From ninja's own `FAILED:` marker rather than from the end of the log.
    # A tail is the obvious choice and it was the wrong one: this tree's build
    # emits a JSON manifest line per generated file, and ninja keeps running its
    # other edges after one fails, so the last sixty lines were sixty manifests
    # and the compiler's message sat above the window. Measured 2026-08-19, the
    # first use of this helper in anger — a `sce-codegen` crash whose one line
    # of evidence ("Illegal instruction") had to be grepped out by hand.
    local first_failed
    first_failed="$(grep -n '^FAILED:' "$log" | head -1 | cut -d: -f1)"
    if [[ -n "$first_failed" ]]; then
        printf '  [%s] build failed; from the first FAILED edge:\n' "$SCE_GATE_SLUG" >&2
        sed -n "${first_failed},\$p" "$log" | head -80 >&2
    else
        # No marker: the failure was cmake's own (a bad argument, a missing
        # generator), and those speak at the end.
        printf '  [%s] build failed; last 60 lines of its output:\n' "$SCE_GATE_SLUG" >&2
        tail -60 "$log" >&2
    fi
    return 1
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

    # The generator this tree is built with, resolved before anything asks
    # CMake for anything.
    #
    # `cmake/SCEFindCodegen.cmake` searches `target/{debug,release}` and PATH
    # and stops with a FATAL_ERROR when it finds nothing, so a configure run in
    # a checkout that has no `target/` cannot succeed — and a configure is
    # exactly what this function does next. It was left to each caller to
    # remember: six workflows carry a "Build sce-codegen" step ahead of the gate
    # they run, and `mutation-rounds.yml` did not. Its ctest path was reached
    # for the first time in 34 commits on 2026-08-17 and died at
    # `SCEFindCodegen.cmake:55` before a round ran.
    #
    # Unconditional rather than inside the configure branch below, because a
    # ready tree needs the binary too: CI restores `build/CMakeCache.txt` from a
    # cache, the cache names a `SCE_CODEGEN` path from an earlier run, and every
    # generator call then fails with the empty error that module's header
    # describes. `sce_codegen_require` returns in one process when the tree
    # already holds a current binary, so the ready case pays nothing for this.
    sce_gate_codegen >/dev/null \
        || sce_gate_fail "could not provide sce-codegen, which configuring $SCE_MAIN_BUILD_DIR requires"

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
