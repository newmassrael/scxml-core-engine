# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# §scxml-C-2-3: the shell's reader of the W3C BasicHTTP fixture endpoint.
#
# The endpoint is owned by `tests/w3c/basic_http_test_endpoint.h` — a C header
# because the C11 AOT runners must include it. This file does not restate the
# port; it READS it from that header, so the number exists in exactly one place
# and a shell caller, a CMake tree and a compiled runner cannot come to disagree
# about where the listener answers.
#
# Usage:
#   REPO_ROOT="$(git rev-parse --show-toplevel)"
#   source "$REPO_ROOT/scripts/lib/sce_http_endpoint.sh"
#   port="$(sce_http_endpoint_port "$REPO_ROOT")"
#
# A caller that already has SCE_W3C_HTTP_PORT set in its environment gets that
# value back: the variable is what moves the endpoint, and the header default is
# only what applies when nothing has moved it.

# The header that owns the endpoint, relative to the repository root.
SCE_HTTP_ENDPOINT_HEADER_REL="tests/w3c/basic_http_test_endpoint.h"

# Print the port the fixture listener should bind and the runners will address.
#
# Fails loudly rather than printing a guess. A gate that silently fell back to a
# number would start a listener on a port the run was told not to use, and the
# collision would be reported as a test failure in whichever tree lost it.
sce_http_endpoint_port() {
    local root="${1:-$(git rev-parse --show-toplevel)}"
    local header="$root/$SCE_HTTP_ENDPOINT_HEADER_REL"
    local from_env="${SCE_W3C_HTTP_PORT:-}"
    local default

    if [[ -n "$from_env" ]]; then
        if [[ ! "$from_env" =~ ^[0-9]+$ ]] || ((from_env < 1 || from_env > 65535)); then
            printf 'SCE_W3C_HTTP_PORT="%s" is not a TCP port.\n' "$from_env" >&2
            return 1
        fi
        printf '%s\n' "$from_env"
        return 0
    fi

    if [[ ! -f "$header" ]]; then
        printf 'the BasicHTTP fixture endpoint header is missing: %s\n' "$header" >&2
        return 1
    fi

    default="$(sed -n 's/^#define SCE_W3C_HTTP_DEFAULT_PORT[[:space:]]\{1,\}\([0-9]\{1,\}\).*/\1/p' \
        "$header" | head -1)"
    if [[ -z "$default" ]]; then
        printf '%s declares no SCE_W3C_HTTP_DEFAULT_PORT — the endpoint owner moved or was renamed\n' \
            "$header" >&2
        return 1
    fi
    printf '%s\n' "$default"
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
