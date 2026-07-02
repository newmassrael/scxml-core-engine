// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML C.2 — Lua C API binding for the host-side HTTP helper:
// pushes a JSON document onto the Lua stack as a Lua value (object →
// table, array → array-table 1-based, string → string, number →
// number, true/false → boolean, null → nil).
//
// Used by the C11 BasicHTTPEventProcessor codegen lift to promote the
// standalone Node server's `data` field onto the SM's Lua datamodel
// `_pending_donedata` slot so transitions can dereference
// `_event.data.<param>` (test567, the only fixture in the W3C C.2
// corpus that reads HTTP-response data; the rest only match by event
// name).
//
// Kept separate from `http_client.{c,h}` so the lower-level transport
// stays Lua-free — fixtures whose codegen does not promote response
// data (`needs_script_engine == false`) still link only the
// `http_client.c` translation unit and skip the Lua C API surface.

#ifndef SCE_C_TESTS_SUPPORT_HTTP_LUA_BINDING_H
#define SCE_C_TESTS_SUPPORT_HTTP_LUA_BINDING_H

#include <stdbool.h>
#include <stddef.h>

struct lua_State;

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Parse the JSON document in `[json, json + len)` and push it onto
 * `L` as a single Lua value.
 *
 * Mapping (RFC 8259 → Lua):
 *   object  → table with string-keyed entries
 *   array   → table with 1-based integer-keyed entries
 *   string  → string (with `\uXXXX` decoded as UTF-8, surrogate
 *             pairs combined into supplementary-plane codepoints)
 *   number  → integer (when the JSON literal has no `.`/`e`/`E`)
 *             else double; lua_pushinteger / lua_pushnumber pick
 *   true    → boolean true
 *   false   → boolean false
 *   null    → nil (top-of-stack lua_pushnil)
 *
 * `len` is the exact byte count of the JSON document. The bytes are
 * NOT mutated (unlike `sce_test_http_parse_response`, which unescapes
 * top-level strings in place — the helper here does its own out-of-
 * place decode into the Lua VM).
 *
 * @return true when one Lua value was pushed; false on parse failure
 *         (in which case nothing is pushed and the stack is restored
 *         to its entry depth).
 */
bool sce_test_lua_push_json(struct lua_State *L, const char *json, size_t len);

#ifdef __cplusplus
}
#endif

#endif /* SCE_C_TESTS_SUPPORT_HTTP_LUA_BINDING_H */
