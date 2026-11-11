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
#   clock the link-time clock a scheduling machine reads by default
#         (`sce/clock.h`'s `_sce_clock_now_ms`)
#
# SCE_C_RUNTIME_REQUIREMENTS is every requirement the generator can state
# (`RuntimeRequirement` in sce-build/src/generator.rs;
# `a_c11_machine_includes_what_it_states_it_requires` holds the two to each
# other), set whatever this configuration builds. Each tier below registers
# the requirement it implements where it is built
# (`SCE_C_RUNTIME_REQUIREMENT_<requirement>`), so a requirement whose tier
# this configuration does not build has no library, and a build knows which
# machines it cannot link here from the machines rather than from a list of
# them. `sce_c_runtime_libraries` reads the two, and `sce_c_link_runtime`
# links a target with what the generated machines it compiles require.
set_property(GLOBAL PROPERTY SCE_C_RUNTIME_REQUIREMENTS lua dom http clock)

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

# sce_c_link_runtime(<target> [REQUIRES <requirement>...] [PROVIDES <requirement>...]
#                    [EXCLUDES <requirement>...])
#
#   Link <target>, a program that compiles generated C11 machines, with the
#   header tier and with the library of this runtime that implements each
#   requirement those machines state: no target names `lua54`,
#   `sce_c_scripting` or `sce_c_runtime_posix` for a machine it compiles, so
#   which machines need which library is the generator's answer and never a
#   list kept beside them.
#
#   The machines are read from the target's own sources. A file a plan
#   declared carries what its generation requires (`SCE_GENERATED_REQUIRES:<path>`,
#   cmake/SCECodegenPlan.cmake), named by its path or by the generator
#   expression `$<TARGET_PROPERTY:<holder>,<property>>` a deferred route hands
#   its caller, which is read from the holder here. Because a deferred route
#   declares its generations when the directory ends, the link is made then
#   too, after them.
#
#   REQUIRES  What the target's own hand-written code needs of the runtime
#             beside its machines (a runner that times itself requires `clock`).
#   PROVIDES  What the target implements itself, and so does not link from
#             the runtime: a test that is the platform for its machine (its
#             own BasicHTTP client, a manual clock) and whose link is the proof
#             that the machine reaches no other.
#   EXCLUDES  What the target's machines must not require: a program whose
#             point is that its machine needs no script engine refuses the
#             configure the day the generator states one, rather than linking
#             it and passing.
#
#   A requirement this configuration does not build is refused: the target
#   exists, and could not be linked. A caller that would rather leave such a
#   program out asks `sce_c_runtime_libraries` before it creates it.
function(sce_c_link_runtime TARGET)
    cmake_parse_arguments(PARSE_ARGV 1 _LINK "" "" "REQUIRES;PROVIDES;EXCLUDES")
    if(_LINK_UNPARSED_ARGUMENTS)
        message(FATAL_ERROR
            "sce_c_link_runtime(${TARGET}): unexpected argument(s): "
            "${_LINK_UNPARSED_ARGUMENTS}")
    endif()
    if(NOT TARGET "${TARGET}")
        message(FATAL_ERROR "sce_c_link_runtime(${TARGET}): no such target")
    endif()
    get_property(_ID GLOBAL PROPERTY _SCE_C_LINK_RUNTIME_COUNT)
    if("${_ID}" STREQUAL "")
        set(_ID 0)
    endif()
    math(EXPR _ID "${_ID} + 1")
    set_property(GLOBAL PROPERTY _SCE_C_LINK_RUNTIME_COUNT "${_ID}")
    set_property(GLOBAL PROPERTY "_SCE_C_LINK_RUNTIME_${_ID}_TARGET" "${TARGET}")
    foreach(_KEPT REQUIRES PROVIDES EXCLUDES)
        set_property(GLOBAL PROPERTY "_SCE_C_LINK_RUNTIME_${_ID}_${_KEPT}" "${_LINK_${_KEPT}}")
    endforeach()
    # The id is spelled into the deferred call now: a deferred call's
    # arguments are otherwise read when it runs.
    cmake_language(EVAL CODE "cmake_language(DEFER CALL _sce_c_link_runtime_now ${_ID})")
endfunction()

# _sce_c_link_runtime_now(<id>)
#
#   The link `sce_c_link_runtime` deferred: every requirement of every
#   generated file the target compiles, and the target's own REQUIRES, less
#   its PROVIDES, each linked with the library that implements it.
function(_sce_c_link_runtime_now ID)
    get_property(_TARGET GLOBAL PROPERTY "_SCE_C_LINK_RUNTIME_${ID}_TARGET")
    foreach(_KEPT REQUIRES PROVIDES EXCLUDES)
        get_property(_${_KEPT} GLOBAL PROPERTY "_SCE_C_LINK_RUNTIME_${ID}_${_KEPT}")
    endforeach()
    get_property(_SOURCES TARGET "${_TARGET}" PROPERTY SOURCES)
    set(_NEEDED ${_REQUIRES})
    foreach(_ENTRY IN LISTS _SOURCES)
        if("${_ENTRY}" MATCHES "^\\$<TARGET_PROPERTY:([^,>]+),([^>]+)>$")
            get_property(_FILES TARGET "${CMAKE_MATCH_1}" PROPERTY "${CMAKE_MATCH_2}")
        else()
            set(_FILES "${_ENTRY}")
        endif()
        foreach(_FILE IN LISTS _FILES)
            get_property(_FILE_NEEDS GLOBAL PROPERTY "SCE_GENERATED_REQUIRES:${_FILE}")
            list(APPEND _NEEDED ${_FILE_NEEDS})
        endforeach()
    endforeach()
    if(_NEEDED)
        list(REMOVE_DUPLICATES _NEEDED)
    endif()
    foreach(_BARRED IN LISTS _EXCLUDES)
        if("${_BARRED}" IN_LIST _NEEDED)
            message(FATAL_ERROR
                "${_TARGET} compiles a generated machine that requires `${_BARRED}`, "
                "which the target excludes: the machine it holds to needing none now "
                "needs one")
        endif()
    endforeach()
    foreach(_OWN IN LISTS _PROVIDES)
        list(REMOVE_ITEM _NEEDED "${_OWN}")
    endforeach()
    sce_c_runtime_libraries(_LIBRARIES _UNMET ${_NEEDED})
    if(NOT "${_UNMET}" STREQUAL "")
        message(FATAL_ERROR
            "${_TARGET} compiles generated machines that require ${_UNMET}, which this "
            "configuration does not build: create the target only where the tier that "
            "implements it is built")
    endif()
    target_link_libraries(${_TARGET} PRIVATE sce_c_runtime ${_LIBRARIES})
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
# `clock.h` (§scxml-6.2 delayed-send clock), `http_client.h`
# (§scxml-C-2 BasicHTTP fixtures) and `directory.h` (the log directory the
# runner program of a generated test suite makes, docs/SCE_TTCN3_LANE.md §4).
# Bare-metal consumers opt out and supply their own `sce_c_runtime_<target>`
# providing the same symbols. No generated machine states `directory.h`: the
# runner program, which is hand-written, links this library itself.
option(SCE_C_RUNTIME_POSIX "Build the POSIX reference impl for backends/c/runtime" ON)
if(SCE_C_RUNTIME_POSIX)
    add_library(sce_c_runtime_posix STATIC
        posix/clock.c
        posix/directory.c
        posix/http_client.c
    )
    target_link_libraries(sce_c_runtime_posix PUBLIC sce_c_runtime)
    set_target_properties(sce_c_runtime_posix PROPERTIES
        C_STANDARD 11
        C_STANDARD_REQUIRED ON
        C_EXTENSIONS OFF
    )
    # The POSIX implementations of the BasicHTTP client a machine that sends
    # over BasicHTTP links, and of the clock a scheduling machine reads. A
    # bare-metal consumer's platform tier registers its own.
    set_property(GLOBAL PROPERTY SCE_C_RUNTIME_REQUIREMENT_http sce_c_runtime_posix)
    set_property(GLOBAL PROPERTY SCE_C_RUNTIME_REQUIREMENT_clock sce_c_runtime_posix)
endif()
