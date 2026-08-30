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
# The rule and the measurement that produced it live in
# `scripts/lib/sce_build_jobs.sh`, which is where they had to move once the
# readers stopped being only gates: `smoke_embed_consumer.sh` and
# `check_clang_format.sh` are reached by a gate and by the pre-commit hook but
# are standalone scripts, and sourcing THIS file would `cd` them to the
# repository root and make them report as a gate.
#
# ⚠ THAT RULE IS A SECOND SPELLING of the one the build wrapper uses when it
# sizes a remote run (cores minus the current run queue, never below one). The
# gates cannot simply call that wrapper: several of them link against artifacts
# an earlier gate left at a local path, so they cannot leave this machine at
# all.
#
# ⚠ Sourced HERE, inside the function, not at file scope: `gate_registry_contract`
# materialises a SUBSET of the tree into a temp dir and runs the gates there, so
# a top-level source of a file that subset does not carry kills every gate
# before it starts. `sce_gate_codegen` and `sce_gate_http_fixture_server` source
# their own libraries the same way, and those siblings are the contract.
sce_build_jobs() {
    source "$SCE_REPO_ROOT/scripts/lib/sce_build_jobs.sh"
    sce_build_jobs_value
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

# Start the W3C BasicHTTP fixture server for the caller, and stop it when the
# gate exits.
#
# Several conformance arms need it — the Rust workspace suite, the Go suite,
# and any other backend running the W3C SCXML C.2 fixtures (test_201, _509,
# _513, _518-520, _532, _534, _567), which issue real HTTP POSTs and fail on
# connection-refused. Each CI job starts its own copy; a gate that mirrors one
# of those jobs has to do the same, and doing it here rather than in each gate
# is what keeps the settle window, the failure message and the cleanup from
# being written three times and drifting.
#
# WHERE the listener answers is not decided here. It is read from
# `tests/w3c/basic_http_test_endpoint.h` via `sce_http_endpoint_port`, the same
# header the compiled runners include, so the address this binds and the
# 'location' those runners publish cannot come apart. The gate also EXPORTS the
# value, because the suites it starts the server for run runners that read it —
# a gate that bound one port and let its children address another would fail
# every fixture for a reason that has nothing to do with the backend.
#
# The C++ suite is the opposite case and must NOT have this: a live listener on
# the endpoint makes its ctest run report 13 tests Not Run. That asymmetry is
# the reason this is a call a gate makes rather than something the runner does
# for everyone.
#
# Answers into SCE_GATE_HTTP_FIXTURE_PID rather than onto stdout, because the
# obvious `pid="$(_sce_gate_http_fixture_start)"` would put `sce_gate_fail`
# inside a command substitution — where its `exit` ends the subshell and the
# caller reads on with an empty pid. A helper whose refusal cannot stop its
# caller is worse than no helper.
SCE_GATE_HTTP_FIXTURE_PID=""
SCE_GATE_HTTP_FIXTURE_CLEANUP_ARMED=""

# Stop whatever is up when the gate exits — registered once, and DEFERRED.
#
# The command is `eval`ed at exit, so it reads the variable then rather than
# baking a number in at registration. Both properties are load-bearing for the
# scoped form below, which starts and stops a server per round: a per-round
# registration would leave the gate exiting with a list of kills for pids that
# finished rounds ago, and a pid the kernel has since handed to somebody else is
# a gate that kills an unrelated process on its way out.
_sce_gate_http_fixture_arm_cleanup() {
    [[ -z "$SCE_GATE_HTTP_FIXTURE_CLEANUP_ARMED" ]] || return 0
    SCE_GATE_HTTP_FIXTURE_CLEANUP_ARMED=1
    sce_gate_on_exit '[[ -z "$SCE_GATE_HTTP_FIXTURE_PID" ]] || kill "$SCE_GATE_HTTP_FIXTURE_PID" 2>/dev/null'
}

_sce_gate_http_fixture_start() {
    command -v node >/dev/null 2>&1 \
        || sce_gate_fail "node.js required for the W3C HTTP fixture server (apt install nodejs)"

    # One listener per gate, whichever entry point asked for it. Mixing the two
    # forms would leave the gate-wide server orphaned the moment a scoped round
    # cleared the variable, and an orphan on the endpoint is what the next suite
    # fails to bind against. Refused rather than reconciled: a gate wanting both
    # is asking for something neither form means.
    [[ -z "$SCE_GATE_HTTP_FIXTURE_PID" ]] \
        || sce_gate_fail "a W3C HTTP fixture server is already up for this gate (pid $SCE_GATE_HTTP_FIXTURE_PID). Use sce_gate_http_fixture_server for a gate-wide one or sce_gate_with_http_fixture_server per command, not both."

    local port path log
    # Sourced HERE, not at file scope: `gate_registry_contract` materialises a
    # SUBSET of the tree into a temp dir and runs the gates there, so a
    # top-level source of a file that subset does not carry kills every gate
    # before it starts. `sce_gate_codegen_require` sources its own library the
    # same way, and that sibling is the contract.
    source "$SCE_REPO_ROOT/scripts/lib/sce_http_endpoint.sh"
    port="$(sce_http_endpoint_port "$SCE_REPO_ROOT")" \
        || sce_gate_fail "could not resolve the W3C BasicHTTP fixture endpoint port"
    path="$(sce_http_endpoint_path "$SCE_REPO_ROOT")" \
        || sce_gate_fail "could not resolve the W3C BasicHTTP fixture endpoint path"
    export SCE_W3C_HTTP_PORT="$port"

    log="$(mktemp)"
    sce_gate_on_exit "rm -f '$log'"
    node "$SCE_REPO_ROOT/tests/w3c/standalone_http_server.js" "$port" "$path" >"$log" 2>&1 &
    SCE_GATE_HTTP_FIXTURE_PID=$!
    _sce_gate_http_fixture_arm_cleanup

    # Wait for the listener to ACCEPT, not for a timer to expire.
    #
    # This was `sleep 1` followed by `kill -0`, which asks whether the process
    # exists — a question node answers yes to long before it has bound the
    # port. On an idle machine the second is plenty and the gate looked
    # correct; under load it is not, and the gate then announces "server up"
    # and hands the round a socket nobody is listening on. Measured through
    # `the_gate_starts_the_declared_service_for_that_round_and_no_other`:
    # green locally and on a doc-only push, RED on the two pushes that ran a
    # full workflow set beside it. A fixed settle window is a guess about
    # someone else's machine.
    #
    # Probed through NODE, not through bash's `/dev/tcp`, and the difference is
    # a measured one. The server binds `listen(port, 'localhost')`; Node 17+
    # resolves that verbatim, so on a host whose `localhost` answers `::1`
    # first it binds IPv6 ONLY. A `/dev/tcp/127.0.0.1` probe then never
    # connects — green on this machine, where IPv4 comes first, and a 30s
    # timeout on every CI runner, which is exactly what it did. Asking node to
    # connect to the same NAME puts the probe on the same resolver and the same
    # stack as the listener, so the two can no longer disagree about where
    # "localhost" is. node is already required above, so this adds nothing.
    #
    # The liveness check stays inside the loop so a server that dies during
    # startup still fails with its log rather than at the deadline.
    local deadline=$(( SECONDS + 30 ))
    until node -e '
        const net = require("net");
        const s = net.connect(Number(process.argv[1]), "localhost");
        s.on("connect", () => { s.end(); process.exit(0); });
        s.on("error", () => process.exit(1));
    ' "$port" 2>/dev/null; do
        if ! kill -0 "$SCE_GATE_HTTP_FIXTURE_PID" 2>/dev/null; then
            cat "$log" >&2
            sce_gate_fail "W3C HTTP fixture server failed to start (port $port already in use?)"
        fi
        if (( SECONDS >= deadline )); then
            cat "$log" >&2
            sce_gate_fail "W3C HTTP fixture server never accepted on localhost:${port} within 30s"
        fi
        sleep 0.2
    done
    sce_gate_step "W3C HTTP fixture server up on localhost:${port}${path}"
}

# The whole gate needs it: start once, stop at exit.
sce_gate_http_fixture_server() {
    _sce_gate_http_fixture_start
}

# ONE command needs it: start, run, stop, and answer with the command's own
# status.
#
# The scoped form exists because holding the port is not free. `mutation-rounds`
# runs several rounds in one invocation and only some of them want a listener on
# the endpoint; the rest drive ctest, where the C11 BasicHTTP entries bring up
# their own copy through the `w3c_c_http_server` CMake fixture (backends/c/
# tests/CMakeLists.txt) and the C++ W3C runner binds the port itself. A server
# left up across the whole gate would make those fail to bind — the same
# 13-cases-Not-Run shape `sce_gate_requires_free_http_port` refuses for,
# arrived at from the other direction.
#
# The exit cleanup armed by the start covers the path this function's own stop
# cannot: a gate interrupted mid-round (Ctrl-C, a cancelled CI job) never
# reaches the line below, and a fixture server that outlives its gate is what
# the next suite trips over. Clearing the pid after stopping is what keeps the
# two from both firing.
sce_gate_with_http_fixture_server() {
    _sce_gate_http_fixture_start
    local pid="$SCE_GATE_HTTP_FIXTURE_PID"

    local rc=0
    "$@" || rc=$?

    # `wait` before returning, so the port is released before the caller's next
    # round asks for it. `kill` only delivers the signal.
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
    SCE_GATE_HTTP_FIXTURE_PID=""
    return "$rc"
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

# Refuse to run when something already holds the fixture endpoint's port.
#
# The C++ conformance suite reports 13 of its cases Not Run when the fixture
# server is live, which is a smaller suite reported as a passing one. The case
# floor would catch the count, but not name the cause; this does.
#
# The port comes from the endpoint owner rather than being spelled here, so a
# tree configured onto a different one checks the port it will actually use.
# This is what lets a second checkout run this suite while the first holds the
# default — the collision the old literal made unavoidable.
sce_gate_requires_free_http_port() {
    local port
    # Sourced here for the reason its sibling above is.
    source "$SCE_REPO_ROOT/scripts/lib/sce_http_endpoint.sh"
    port="$(sce_http_endpoint_port "$SCE_REPO_ROOT")" \
        || sce_gate_fail "could not resolve the W3C BasicHTTP fixture endpoint port"
    if command -v ss >/dev/null 2>&1 && ss -ltn 2>/dev/null | grep -q ":${port} "; then
        sce_gate_fail "something is listening on localhost:${port}. The W3C HTTP fixture server makes this suite report 13 cases Not Run, so the run would be a smaller suite reported as a passing one. Stop it and retry, or point this tree at another port with SCE_W3C_HTTP_PORT."
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

# The major version a JDK home reports. `$1` empty means the `java` on PATH.
#
# Java 8 spells itself `1.8.0_x` and everything since spells `<major>.x`, so the
# leading `1.` is dropped before the first component is read.
sce_java_major() {
    local home="${1:-}" out version
    out="$("${home:+$home/bin/}java" -version 2>&1 | head -1)" || return 1
    version="${out#*\"}"
    version="${version%%\"*}"
    case "$version" in
    1.*) version="${version#1.}" ;;
    esac
    printf '%s' "${version%%.*}"
}

# ── The JDK Gradle will ACTUALLY run on ───────────────────────────
#
# TWO gates drive Gradle — `w3c-kotlin` and `ecma262-lowered-kotlin` — and both
# need the same guarantee, so it is written once. A second copy would be a
# second answer to "which JVM is this", free to fall behind the first exactly
# when a machine's `JAVA_HOME` is the problem.
#
# "Is there a java" and "which java does Gradle use" are different questions:
# Gradle honours `JAVA_HOME` over the `java` on PATH, and nothing keeps the two
# in step.
#
# Measured 2026-08-24 on a build machine whose `/etc/environment` carried
# `JAVA_HOME=…java-8-openjdk…` for an unrelated toolchain: `java --version` said
# 17, `update-alternatives` said 17, and Gradle compiled the build scripts on 8
# — failing on a `ByteArrayOutputStream.toString(Charset)` overload that Java 10
# added. Nothing in that error named a JDK, and the gate had no way to say "you
# are on the wrong one".
#
# CI never meets this because `actions/setup-java` exports `JAVA_HOME`. So the
# floor is READ from the version CI pins, and this makes the same guarantee
# locally rather than inheriting whatever the machine happens to export.
#
# An adequate `JAVA_HOME` is left alone — overriding a deliberate choice is not
# a gate's business. `SCE_JAVA_HOME` names one explicitly for a layout the
# search below does not know.
#
# `$1` is the workflow file whose `java-version:` pin is the floor.
sce_gate_require_jdk() {
    local pin_file="$1" floor now candidate

    # Deliberately not skip-capable. A lane obtains its JDK through
    # `actions/setup-java` rather than a package install, and these gates are
    # selected only when the Kotlin backend changed — a skip there would be
    # silence about the exact change that asked for the check.
    command -v java >/dev/null 2>&1 \
        || sce_gate_fail "java is not on PATH, and this gate was selected because the Kotlin backend changed. Install a JDK 17+ (apt install openjdk-17-jdk) — skipping here would report green on an unverified backend."

    floor="$(sed -n "s/^[[:space:]]*java-version:[[:space:]]*['\"]\{0,1\}\([0-9]\{1,\}\).*/\1/p" \
        "$pin_file" | head -1)"
    [ -n "$floor" ] \
        || sce_gate_fail "no \`java-version:\` pin found in $pin_file — this gate derives its JDK floor from the version CI installs, and a missing pin would let it run on any JVM."

    if [ -z "${JAVA_HOME:-}" ] || [ "$(sce_java_major "${JAVA_HOME:-}" || echo 0)" -lt "$floor" ]; then
        # The pinned major first, then anything at or above the floor: a
        # machine that carries several JDKs should land on the one CI uses,
        # not merely on one that compiles.
        for candidate in ${SCE_JAVA_HOME:+"$SCE_JAVA_HOME"} \
            "/usr/lib/jvm/java-$floor-openjdk-"* /usr/lib/jvm/java-*-openjdk-*; do
            # `javac`, not `java`: a JRE runs the tests but cannot compile
            # them, and one of the fleet's "JDK 21" directories turned out to
            # be exactly that — a JRE whose `bin/` holds four files and no
            # compiler.
            [ -x "$candidate/bin/javac" ] || continue
            [ "$(sce_java_major "$candidate" || echo 0)" -ge "$floor" ] || continue
            export JAVA_HOME="$candidate"
            break
        done
    fi

    now="$(sce_java_major "${JAVA_HOME:-}" || echo 0)"
    [ "$now" -ge "$floor" ] \
        || sce_gate_fail "Gradle would run on JDK $now, and this suite needs $floor+ (the version ${pin_file##*/} installs). JAVA_HOME=${JAVA_HOME:-<unset>}. Install a JDK $floor, or point SCE_JAVA_HOME at one — running on an older JVM fails inside a build script with a message that names no JDK."
    sce_gate_step "Gradle will run on JDK $now (floor $floor from ${pin_file##*/})"
}
