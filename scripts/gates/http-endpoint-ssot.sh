#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: http-endpoint-ssot.yml
#
# §scxml-C-2-3: nobody re-spells the W3C BasicHTTP fixture endpoint.
#
# `tests/w3c/basic_http_test_endpoint.h` owns where the fixture listener
# answers. It is a C header so every channel can read it: the C11 AOT runners
# include it directly, the C++ harness wraps it, CMake reads its path, and the
# gates and CI go through `scripts/lib/sce_http_endpoint.sh`.
#
# That ownership is a property of the tree, not of one commit, and this is what
# keeps it. Twelve C runners each carried "http://localhost:8080/test" and two
# gates plus a CI job carried the port again -- one fact spelled in five places,
# and the shape only became visible when a second checkout could not run the
# BasicHTTP suites while the first held the port.
#
# The endpoint has NO DEFAULT PORT any more (the owner says why): a listener
# binds the port it is told or one the kernel assigns, and the port it bound is
# handed to whoever posts to it. A fixed default, even one spelled once, made
# every lane need that one port free on a host it shares with other programs.
#
# WHAT THIS REFUSES:
#   - the owner declaring a default port again (SCE_W3C_HTTP_DEFAULT_PORT);
#   - a path mirror that disagrees with the owner's path;
#   - the port every fixture listener used to share, HISTORICAL_PORT below,
#     written literally anywhere outside this gate's allowlist: a harness that
#     spells it again depends on that port being free.
#
# WHAT IT DELIBERATELY ALLOWS, and why each is a different fact:
#
#   - `sce/include/events/HttpEventReceiver.h` -- the PRODUCT's default listen
#     port. It is not the test fixture, and collapsing the two would make it
#     impossible to say which listener a port belongs to.
#   - `sce-build/tests/fixtures/codegen_smoke/*.scxml` -- documents whose POINT
#     is to carry a literal address through the generator.
#   - `tests/w3c_template_parity/fixtures/**` -- likewise.
#   - Comments and documentation prose. A sentence explaining the endpoint is
#     not a second author of it. The scanner strips comments before looking,
#     which this repository has learned to do the hard way: a scanner that
#     reads comments reports the explanation of a rule as a violation of it.
#
# THE FLOOR: a scan that finds no violations is only good news if it looked at
# something. This asserts that the search turned up a file known to spell the
# number for a different reason, and that the owner still declares the path,
# so a renamed header or a broken glob fails loudly instead of reporting a
# clean tree it never read.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
source "$SCE_REPO_ROOT/scripts/lib/sce_http_endpoint.sh"

OWNER="tests/w3c/basic_http_test_endpoint.h"
READER="scripts/lib/sce_http_endpoint.sh"

# The port every fixture listener used to bind, kept here only to refuse it.
HISTORICAL_PORT=8080

# The file the floor below needs the search to find: the product's listener
# default, a different fact (allowed below) spelled with the same number.
FLOOR_WITNESS="sce/include/events/HttpEventReceiver.h"

# ── The owner still owns it, and declares no port ──────────────────
[[ -f "$OWNER" ]] \
    || sce_gate_fail "the endpoint owner is missing: $OWNER"

if grep -qE '^#define[[:space:]]+SCE_W3C_HTTP_DEFAULT_PORT' "$OWNER"; then
    sce_gate_fail "$OWNER declares SCE_W3C_HTTP_DEFAULT_PORT again. The fixture endpoint has no default port: a listener binds the port it is told or one the kernel assigns, and hands on the port it bound."
fi
sce_gate_step "endpoint owner declares no default port"

# ── The path every harness answers on or posts to, pinned to the owner ──
# The Kotlin harness and the Python conftest bind ports of their own (their
# listeners run beside the machines they serve), and the Rust and Go harnesses
# are told the port their out-of-process listener bound, so no harness spells a
# port. The PATH is still spelled in each. The Rust and Go files are embedded by
# `sce-build` into the standalone suites it emits, where no repository sits
# above them, so they CANNOT read the owner (an earlier cut had the Rust one
# `include_str!` the header and the emitted suite failed to compile); the rule
# is not "may spell it" but "must spell the SAME thing". Nothing compared the
# Kotlin spellings to the owner's either: a path edited in the owner left a
# Kotlin listener answering on the old one while the document asked for the new.
ENDPOINT_PATH="$(sce_http_endpoint_path "$SCE_REPO_ROOT")" \
    || sce_gate_fail "$OWNER no longer declares a readable test path"
PATH_MIRRORS=(
    "backends/kotlin/test-harness/src/main/kotlin/com/sce/http/W3CHttpTestServer.kt|private val path: String = \""
    "backends/kotlin/test-harness/src/main/kotlin/com/sce/w3c/W3CHttpTestBase.kt|W3CHttpTestServer(port = 0, path = \""
    "backends/kotlin/test-harness/src/main/kotlin/com/sce/w3c/W3CHttpTestBase.kt|http://localhost:\$actualPort"
    "backends/python/tests/conftest.py|_HTTP_PATH = \""
    "backends/rust/tests/src/harness.rs|const DEFAULT_ENDPOINT_PATH: &str = \""
    "backends/go/tests/harness/harness.go|const defaultEndpointPath = \""
)

for entry in "${PATH_MIRRORS[@]}"; do
    file="${entry%%|*}"
    before="${entry#*|}"
    [[ -f "$file" ]] \
        || sce_gate_fail "pinned endpoint path mirror is missing: $file"
    # The text between the end of the anchor and the next quote: the path as the
    # file spells it.
    value="$(awk -v anchor="$before" '
        i = index($0, anchor) {
            rest = substr($0, i + length(anchor))
            print substr(rest, 1, index(rest, "\"") - 1)
            exit
        }' "$file")"
    [[ -n "$value" ]] \
        || sce_gate_fail "$file no longer spells the endpoint path after \`$before\`. It cannot read $OWNER, so it must carry a pinned copy of the path."
    [[ "$value" == "$ENDPOINT_PATH" ]] \
        || sce_gate_fail "$file spells the endpoint path as \`$value\` but $OWNER says \`$ENDPOINT_PATH\`. A harness answering on another path answers a request nobody sends."
    sce_gate_step "path mirror pinned: $file ($value)"
done

# ── Everything that is allowed to say the number ───────────────────
# Paths are matched as prefixes against the repo-relative path.
ALLOWED=(
    "$OWNER"
    "$READER"
    "scripts/gates/http-endpoint-ssot.sh"
    "sce/include/events/HttpEventReceiver.h"
    "sce/include/events/IEventReceiver.h"
    "sce/include/events/IHttpClient.h"
    "sce/include/mesh/transports/CustomTcpTransport.h"
    "sce-build/tests/fixtures/codegen_smoke/"
    "tests/w3c_template_parity/fixtures/"
    "examples/doom_wasm/"
    "docs/"
    "web/"
    "SCE_MESH.md"
    # A URL PARSER's unit test: it feeds a literal address in and asserts the
    # port it parsed back out. The number is the test's input, not a claim
    # about where any listener answers.
    "backends/c/tests/unit/http_client_test.c"
    # Malformed-endpoint inputs for the mesh CustomTcp parser ("127.0.0.1:8080abc",
    # "127.0.0.1:8080 "). Their point is that they are NOT valid endpoints.
    "tests/mesh/test_mesh_custom_tcp_runtime.cpp"
    # `bind: "0.0.0.0:8080"` inside driver-class YAML fixtures for the C11
    # WebSocket link. A driver's bind address is a different fact from the W3C
    # BasicHTTP fixture endpoint, and these fixtures assert on the string.
    "sce-build/tests/c11_driver_class_cross_validator.rs"
    "sce-build/tests/c11_websocket_link_driver.rs"
    # `_ioprocessors` descriptor tests: they feed an arbitrary access URI into
    # the builder and assert what `location` reads back. The address is the
    # test's input, and one of them deliberately uses a path that is not the
    # fixture's at all.
    "backends/rust/runtime/src/helpers/io_processors.rs"
    "tests/engine/IOProcessorsTest.cpp"
    # HttpEventBridge request-shaping tests, on `/scxml` rather than the
    # fixture path -- a different listener entirely.
    "tests/events/HttpEventBridgeTest.cpp"
)

is_allowed() {
    local path=$1 allow
    for allow in "${ALLOWED[@]}"; do
        [[ "$path" == "$allow" || "$path" == "$allow"* ]] && return 0
    done
    return 1
}

# ── Scan ───────────────────────────────────────────────────────────
# Only the languages that could actually re-spell the endpoint in code.
#
# CANDIDATES FIRST, then the expensive part. The first cut ran the comment
# stripper over every tracked source file -- 4440 `awk` processes, 14.7s -- and
# a gate that costs that much either runs on a narrow trigger (and misses the
# file nobody thought of) or on a catch-all (and is paid on every push). One
# `git grep` names the handful of files that mention the number at all, and the
# stripper then runs only on those, so the trigger can stay a catch-all: a
# re-spelling anywhere is caught, cheaply.
PORT="$HISTORICAL_PORT"
mapfile -t CANDIDATES < <(git grep -lF "$PORT" -- \
    '*.c' '*.h' '*.cpp' '*.hpp' '*.rs' '*.go' '*.py' '*.kt' '*.js' \
    '*.sh' '*.yml' '*.yaml' '*.cmake' 'CMakeLists.txt' '**/CMakeLists.txt' 2>/dev/null || true)

# THE FLOOR, and it is not a file count. The product's listener default spells
# the same number in code, so that file MUST be among the candidates; if it is
# not, the search did not run over this tree and every "clean" verdict below
# would be vacuous. A count would only have told us something was read -- this
# tells us the right thing was.
witness_found=0
for path in "${CANDIDATES[@]}"; do
    [[ "$path" == "$FLOOR_WITNESS" ]] && witness_found=1
done
((witness_found)) \
    || sce_gate_fail "the candidate search did not turn up $FLOOR_WITNESS, which spells $PORT as the product's listener default — the scan did not read this tree, so a clean verdict would mean nothing"

examined=0
violations=0
for path in "${CANDIDATES[@]}"; do
    is_allowed "$path" && continue
    [[ -f "$path" ]] || continue

    examined=$((examined + 1))

    # Strip comments before looking. Block comments SPAN LINES, which a
    # line-at-a-time strip does not see: the first cut of this gate reported
    # three C++ headers whose every hit was inside a `/** ... */` describing
    # the fixture -- the explanation of a rule reported as a violation of it,
    # which is the failure mode this repository already knows about. `awk`
    # carries the in-comment state across lines so that cannot recur.
    #
    # `#` is a comment in shell/python/yaml/cmake and a preprocessor directive
    # in C -- and a `#define` of this port outside the owner is exactly a
    # re-spelling, so C keeps its directives and only `//` and `/* */` go.
    # A `//` PRECEDED BY `:` is a URL scheme, not a comment. Measured on the
    # second cut: `const BasicHTTPAccessURI = "http://localhost:8080/test"` was
    # cut at the `//` in `http://`, so the Go harness's re-spelling of the
    # endpoint vanished from a scan whose whole job is to find exactly that.
    # Only POSIX awk features here (index/substr/length): verified under mawk
    # and busybox awk, since mawk is what /usr/bin/awk resolves to on this
    # fleet and a gate proven on gawk alone has died at push before.
    stripped="$(awk '
        {
            line = $0
            out = ""
            while (length(line) > 0) {
                if (in_block) {
                    i = index(line, "*/")
                    if (i == 0) { line = "" } else { line = substr(line, i + 2); in_block = 0 }
                    continue
                }
                b = index(line, "/*")
                s = index(line, "//")
                while (s > 1 && substr(line, s - 1, 1) == ":") {
                    t = index(substr(line, s + 2), "//")
                    if (t == 0) { s = 0; break }
                    s = s + t + 1
                }
                if (s > 0 && (b == 0 || s < b)) { out = out substr(line, 1, s - 1); line = "" }
                else if (b > 0) { out = out substr(line, 1, b - 1); line = substr(line, b + 2); in_block = 1 }
                else { out = out line; line = "" }
            }
            print out
        }' "$path")"
    case "$path" in
        *.sh|*.py|*.yml|*.yaml|*.cmake|*CMakeLists.txt)
            stripped="$(sed -e 's:#.*::' <<<"$stripped")"
            ;;
    esac

    # A HERESTRING, never `printf | grep -q`. MEASURED on the first cut of this
    # gate: `grep -q` exits at its first match, the writer takes SIGPIPE, and
    # `pipefail` turns the whole pipeline non-zero -- so a file that DID match
    # was read as a file that did not. It only bites files large enough that
    # the writer is still writing when grep leaves, which is why it hid
    # exactly one violation (W3CTestRunner.cpp, ~8k lines) and looked clean.
    # A scanner that under-reports on big files is worse than one that fails.
    if grep -qE "(^|[^0-9])${PORT}([^0-9]|$)" <<<"$stripped"; then
        printf '  %s spells the old fixed fixture port %s\n' "$path" "$PORT" >&2
        violations=$((violations + 1))
    fi
done

if ((violations > 0)); then
    sce_gate_fail "$violations file(s) spell the port the BasicHTTP fixture listener used to share. The endpoint has no default port: bind 0 and read the bound port, or read SCE_W3C_HTTP_PORT / SCE_W3C_HTTP_PORT_FILE as $OWNER does (C/C++), or source $READER (shell/CI). Add a genuinely different fact to this gate's ALLOWED list, with the reason."
fi

sce_gate_step "${#CANDIDATES[@]} candidate(s) named the old fixed port, $examined examined, none depend on it"
