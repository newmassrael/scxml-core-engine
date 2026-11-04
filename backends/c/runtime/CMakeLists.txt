# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# backends/c/runtime — C11 backend 4-tier layout (RFC §5.J.1, downstream MCU
# consumer; ARCHITECTURE.md §"C11 Backend Layering").
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
#                           platform impl (e.g. an lwIP-based
#                           `sce_link_runtime_lwip`).

# ── What a generated machine requires, and which tier implements it ──
#
# A generated C11 machine states what it needs linked beside it that the
# header-only tier does not provide (`sce-codegen --plan --plan-requires`,
# read into `<PREFIX>_REQUIRES` by cmake/SCECodegenPlan.cmake), and includes
# the interface of each exactly when it states it:
#
#   lua   the Lua 5.4 engine its data model is evaluated in (`lua.h`)
#   dom   the Lua binding of the XML DOM (`sce/lua_dom_binding.h`)
#   http  the BasicHTTP client (`sce/http_client.h`)
#
# SCE_C_RUNTIME_REQUIREMENTS is every requirement the generator can state
# (`RuntimeRequirement` in sce-build/src/generator.rs;
# `a_c11_machine_includes_what_it_states_it_requires` holds the two to each
# other), set whatever this configuration builds. Each tier below registers
# the requirement it implements where it is built
# (`SCE_C_RUNTIME_REQUIREMENT_<requirement>`), so a requirement whose tier
# this configuration does not build has no library, and a build knows which
# machines it cannot link here from the machines rather than from a list of
# them. `sce_c_runtime_libraries` reads the two.
set_property(GLOBAL PROPERTY SCE_C_RUNTIME_REQUIREMENTS lua dom http)

# sce_c_runtime_libraries(<LIBRARIES_VAR> <UNMET_VAR> <requirement>...)
#
#   The libraries of this runtime that implement the requirements a generated
#   machine states, in LIBRARIES_VAR, each once, and in UNMET_VAR each
#   requirement whose tier this configuration does not build: a machine that
#   states one cannot be linked here. A requirement no tier of this runtime
#   implements is refused rather than linked with nothing: the generator
#   states one this runtime was never taught.
function(sce_c_runtime_libraries LIBRARIES_VAR UNMET_VAR)
    get_property(_KNOWN GLOBAL PROPERTY SCE_C_RUNTIME_REQUIREMENTS)
    set(_LIBRARIES "")
    set(_UNMET "")
    foreach(_REQUIREMENT IN LISTS ARGN)
        if(NOT "${_REQUIREMENT}" IN_LIST _KNOWN)
            message(FATAL_ERROR
                "A generated C11 machine requires `${_REQUIREMENT}`, which no tier of "
                "backends/c/runtime implements (it knows: ${_KNOWN}). Name the library "
                "that implements it in SCE_C_RUNTIME_REQUIREMENTS and register it where "
                "that library is built.")
        endif()
        get_property(_LIBRARY GLOBAL PROPERTY "SCE_C_RUNTIME_REQUIREMENT_${_REQUIREMENT}")
        if("${_LIBRARY}" STREQUAL "")
            list(APPEND _UNMET "${_REQUIREMENT}")
        else()
            list(APPEND _LIBRARIES ${_LIBRARY})
        endif()
    endforeach()
    if(_LIBRARIES)
        list(REMOVE_DUPLICATES _LIBRARIES)
    endif()
    set(${LIBRARIES_VAR} "${_LIBRARIES}" PARENT_SCOPE)
    set(${UNMET_VAR} "${_UNMET}" PARENT_SCOPE)
endfunction()

# ── Tier 1: sce_c_runtime (INTERFACE — public headers) ──────────
add_library(sce_c_runtime INTERFACE)
target_include_directories(sce_c_runtime
    INTERFACE
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
)

# ── Tier 2: sce_c_base (STATIC — freestanding helpers) ──────────
# ⚠ The typed `_event.data` lift is NOT here. `sce/event_payload.h` is
# header-only on purpose: the EventSchema native gate links no runtime library
# at all, because a native value path that links nothing is the MCU-clean proof
# the whole lowering exists for, and a lift in this library would have taken
# that proof away from every machine that uses it.
add_library(sce_c_base STATIC base/dom.c)
target_link_libraries(sce_c_base PUBLIC sce_c_runtime)
set_target_properties(sce_c_base PROPERTIES
    C_STANDARD 11
    C_STANDARD_REQUIRED ON
    C_EXTENSIONS OFF
)

# ── Tier 3: sce_c_scripting (STATIC, optional — Lua bridges) ────
# SCE_ENABLE_LUA mirrors the same option used by the cpp `sce_scripting`
# tier (see ARCHITECTURE.md). When OFF, `lua` and `dom` have no library here,
# and a consumer leaves out every machine that states either
# (`sce_c_runtime_libraries` above names them unmet).
if(SCE_ENABLE_LUA)
    add_library(sce_c_scripting STATIC
        scripting/lua_dom_binding.c
    )
    target_link_libraries(sce_c_scripting PUBLIC sce_c_runtime sce_c_base lua54)
    set_target_properties(sce_c_scripting PROPERTIES
        C_STANDARD 11
        C_STANDARD_REQUIRED ON
        C_EXTENSIONS OFF
    )
    # A machine that embeds the Lua engine links it, and one that can hand its
    # data model an XML document links the binding (which links the engine
    # and the freestanding parser in turn).
    set_property(GLOBAL PROPERTY SCE_C_RUNTIME_REQUIREMENT_lua lua54)
    set_property(GLOBAL PROPERTY SCE_C_RUNTIME_REQUIREMENT_dom sce_c_scripting)
endif()

# ── Tier 4: sce_c_runtime_posix (STATIC, optional — platform impl) ──
# Default ON: the SCE-shipped POSIX reference implementation of
# `clock.h` (§scxml-6.2 delayed-send clock) and `http_client.h`
# (§scxml-C-2 BasicHTTP fixtures). Bare-metal consumers opt out and supply
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
    # The POSIX implementation of the BasicHTTP client a machine that sends
    # over BasicHTTP links. A bare-metal consumer's platform tier registers
    # its own.
    set_property(GLOBAL PROPERTY SCE_C_RUNTIME_REQUIREMENT_http sce_c_runtime_posix)
endif()
