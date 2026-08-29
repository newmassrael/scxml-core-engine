cmake_minimum_required(VERSION 3.16)
project(sce_lua_jni C CXX)

set(CMAKE_C_STANDARD 11)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

# Lua 5.4 source directory (passed from Gradle)
if(NOT DEFINED LUA_SRC_DIR)
    message(FATAL_ERROR "LUA_SRC_DIR must be set to third_party/lua/src")
endif()

# Find JNI headers
find_package(JNI REQUIRED)

# Collect Lua 5.4 sources (exclude lua.c and luac.c — those are standalone executables)
file(GLOB LUA_SOURCES "${LUA_SRC_DIR}/*.c")
list(FILTER LUA_SOURCES EXCLUDE REGEX "lua\\.c$")
list(FILTER LUA_SOURCES EXCLUDE REGEX "luac\\.c$")

# Build shared library: lua54 + JNI bridge
add_library(sce_lua_jni SHARED
    ${LUA_SOURCES}
    lua_jni.cpp
    lowering_jni.cpp
)

target_include_directories(sce_lua_jni PRIVATE
    ${LUA_SRC_DIR}
    ${JNI_INCLUDE_DIRS}
)

# Lua needs math library on Linux
if(UNIX AND NOT APPLE)
    target_link_libraries(sce_lua_jni PRIVATE m dl)
    target_compile_definitions(sce_lua_jni PRIVATE LUA_USE_LINUX)
elseif(APPLE)
    target_compile_definitions(sce_lua_jni PRIVATE LUA_USE_MACOSX)
endif()

# Suppress Lua warnings in third-party code
set_source_files_properties(${LUA_SOURCES} PROPERTIES
    COMPILE_FLAGS "-w"
)

# ── sce-build's ECMAScript frontend, linked beside lua54 ──────────────
#
# The same decision the C++ engine took on 2026-08-29 (the `link-beside-lua`
# and `retire-rewriter` rows of `docs/SCE_LUA_TRANSLATION_SEAM.md`), one
# language over: the engine that accepts ECMAScript reaches a PARSER instead
# of a pass that replaces text without knowing where an operand ends.
# `cmake/SCEBuildLowering.cmake` already knows how to build and name that
# staticlib, so this links the same artifact rather than describing it twice.
#
# Not optional, and that is the substance rather than caution. A library built
# without the frontend would answer the shared ECMA-262 table from
# `EcmaScriptToLuaTransformer` alone — a SECOND set of answers under one
# engine name, only one of which `tests/ecmascript/kotlin_lua_divergences.json`
# describes. One artifact, one answer; `theFrontendIsLinked` refuses a build
# that reached here without it.
if(NOT DEFINED SCE_REPO_ROOT)
    message(FATAL_ERROR
        "SCE_REPO_ROOT must be set. This library links sce-build's lowering "
        "surface through cmake/SCEBuildLowering.cmake, which is located from "
        "the repository root rather than relative to this file.")
endif()
include(${SCE_REPO_ROOT}/cmake/SCEBuildLowering.cmake)
target_link_libraries(sce_lua_jni PRIVATE SCE::Lowering)
