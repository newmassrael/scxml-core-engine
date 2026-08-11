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
