# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# §scxml-C-2-3: the shell's reader of the W3C BasicHTTP fixture endpoint.
#
# The endpoint is owned by `tests/w3c/basic_http_test_endpoint.h` — a C header
# because the C11 AOT runners must include it. This file does not restate the
# path; it READS it from that header, so a shell caller, a CMake tree and a
# compiled runner cannot come to disagree about where the listener answers.
#
# Usage:
#   REPO_ROOT="$(git rev-parse --show-toplevel)"
#   source "$REPO_ROOT/scripts/lib/sce_http_endpoint.sh"
#   port="$(sce_http_endpoint_port "$REPO_ROOT")"
#
# The port has no default (the header says why): it is the one SCE_W3C_HTTP_PORT
# names, and a caller that names none starts its listener on port 0 and reads
# the port it bound from the listener's `[READY] port=<n>` line.

# The header that owns the endpoint, relative to the repository root.
SCE_HTTP_ENDPOINT_HEADER_REL="tests/w3c/basic_http_test_endpoint.h"

# Print the port SCE_W3C_HTTP_PORT names.
#
# Fails loudly rather than printing a guess, both when the variable is not a
# port and when it names none: a gate that fell back to a number would start a
# listener on, or check, a port this run was never given, and another program
# on the host may hold it.
sce_http_endpoint_port() {
    local from_env="${SCE_W3C_HTTP_PORT:-}"

    if [[ -z "$from_env" ]]; then
        printf 'SCE_W3C_HTTP_PORT names no port. The W3C BasicHTTP fixture endpoint has no default port: start the listener on port 0 and read the port its [READY] port=<n> line names.\n' >&2
        return 1
    fi
    if [[ ! "$from_env" =~ ^[0-9]+$ ]] || ((from_env < 1 || from_env > 65535)); then
        printf 'SCE_W3C_HTTP_PORT="%s" is not a TCP port.\n' "$from_env" >&2
        return 1
    fi
    printf '%s\n' "$from_env"
}

# Print the path the fixture listener answers on, read from the same header.
sce_http_endpoint_path() {
    local root="${1:-$(git rev-parse --show-toplevel)}"
    local header="$root/$SCE_HTTP_ENDPOINT_HEADER_REL"
    local path

    if [[ ! -f "$header" ]]; then
        printf 'the BasicHTTP fixture endpoint header is missing: %s\n' "$header" >&2
        return 1
    fi

    path="$(sed -n 's/^#define SCE_W3C_HTTP_TEST_PATH[[:space:]]\{1,\}"\(.*\)".*/\1/p' \
        "$header" | head -1)"
    if [[ -z "$path" ]]; then
        printf '%s declares no SCE_W3C_HTTP_TEST_PATH — the endpoint owner moved or was renamed\n' \
            "$header" >&2
        return 1
    fi
    printf '%s\n' "$path"
}
