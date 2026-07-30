# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# backends/c/runtime — C11 backend 4-tier layout (RFC §5.J.1, watching-zenoh
# consumer; ARCHITECTURE.md §"C11 backend layering").
#
# Mirrors the cpp 4-tier shape (sce_core / sce_base / sce_scripting /
# sce_runtime) adapted to the C11 backend reality: generated code carries
# the runtime engine itself (jinja2-inline-only contract — no shared
# runtime-engine .c), so the "tier 4" slot holds platform-specific impls
# (POSIX / RTOS / lwIP) instead of a runtime-engine library.
#
# Layer mapping:
#
#   sce_c_runtime         — INTERFACE, header-only contracts (clock,
#                           dom, http_client, lua bindings). The
#                           single-source-of-truth for the C11 backend's
#                           public API surface; no .c sources, no
#                           external link — bare-metal consumers can
#                           link only this tier and supply their own
#                           platform impl.
#
#   sce_c_base            — STATIC, freestanding-C helpers (DOM parser).
#                           No POSIX, no Lua. Linkable on bare-metal
#                           with only `libc` (string/stdio/stdlib).
#
#   sce_c_scripting       — STATIC, optional. Lua-bound bridges over
#                           sce_c_base + the platform impl. Gated by
#                           SCE_ENABLE_LUA (the same flag that controls
#                           the cpp `sce_scripting` tier).
#
#   sce_c_runtime_posix   — STATIC, optional. POSIX reference impl of
#                           `clock.h` and `http_client.h`. Default ON
#                           for host-compatible builds; opt out by
#                           setting SCE_C_RUNTIME_POSIX=OFF for bare-
#                           metal consumers that supply their own
#                           platform impl (e.g. watching-zenoh's
#                           `sce_link_runtime_lwip`).

# ── Tier 1: sce_c_runtime (INTERFACE — public headers) ──────────
add_library(sce_c_runtime INTERFACE)
target_include_directories(sce_c_runtime
    INTERFACE
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
)

# ── Tier 2: sce_c_base (STATIC — freestanding helpers) ──────────
add_library(sce_c_base STATIC base/dom.c)
target_link_libraries(sce_c_base PUBLIC sce_c_runtime)
set_target_properties(sce_c_base PROPERTIES
    C_STANDARD 11
    C_STANDARD_REQUIRED ON
    C_EXTENSIONS OFF
)

# ── Tier 3: sce_c_scripting (STATIC, optional — Lua bridges) ────
# SCE_ENABLE_LUA mirrors the same option used by the cpp `sce_scripting`
# tier (see ARCHITECTURE.md). When OFF, fixtures with NEEDS_LUA are
# expected to be excluded by the consumer's CMakeLists.
if(SCE_ENABLE_LUA)
    add_library(sce_c_scripting STATIC
        scripting/lua_dom_binding.c
        scripting/http_lua_binding.c
    )
    target_link_libraries(sce_c_scripting PUBLIC sce_c_runtime sce_c_base lua54)
    set_target_properties(sce_c_scripting PROPERTIES
        C_STANDARD 11
        C_STANDARD_REQUIRED ON
        C_EXTENSIONS OFF
    )
endif()

# ── Tier 4: sce_c_runtime_posix (STATIC, optional — platform impl) ──
# Default ON: the SCE-shipped POSIX reference implementation of
# `clock.h` (§scxml-6.2 delayed-send clock) and `http_client.h`
# (§scxml-C-2 BasicHTTP fixtures). Bare-metal consumers (e.g.
# watching-zenoh per `consumer_watching_zenoh.md`) opt out and supply
# their own `sce_c_runtime_<target>` providing the same symbols.
option(SCE_C_RUNTIME_POSIX "Build the POSIX reference impl for backends/c/runtime" ON)
if(SCE_C_RUNTIME_POSIX)
    add_library(sce_c_runtime_posix STATIC
        posix/clock.c
        posix/http_client.c
    )
    target_link_libraries(sce_c_runtime_posix PUBLIC sce_c_runtime)
    set_target_properties(sce_c_runtime_posix PROPERTIES
        C_STANDARD 11
        C_STANDARD_REQUIRED ON
        C_EXTENSIONS OFF
    )
endif()
