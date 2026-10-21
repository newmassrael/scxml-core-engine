cmake_minimum_required(VERSION 3.16)
project(sce_lua_jni C CXX)

set(CMAKE_C_STANDARD 11)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

# Lua 5.4 source directory (passed from Gradle)
if(NOT DEFINED LUA_SRC_DIR)
    message(FATAL_ERROR "LUA_SRC_DIR must be set to third_party/lua/src")
endif()

# JNI headers of the JDK that builds and loads this library (passed from
# Gradle, gradle/sce-jni-headers.gradle.kts). Not find_package(JNI): without
# COMPONENTS it also requires AWT, which a headless JDK does not ship, and
# its COMPONENTS are ignored before CMake 3.24. Only the headers are used;
# the library neither links libjvm nor uses AWT.
if(NOT DEFINED SCE_JNI_INCLUDE_DIRS)
    message(FATAL_ERROR
        "SCE_JNI_INCLUDE_DIRS must be set to <jdk>/include;<jdk>/include/<platform> "
        "of the JDK that loads this library")
endif()
foreach(sce_jni_header jni.h jni_md.h)
    set(sce_jni_header_found FALSE)
    foreach(sce_jni_dir IN LISTS SCE_JNI_INCLUDE_DIRS)
        if(EXISTS "${sce_jni_dir}/${sce_jni_header}")
            set(sce_jni_header_found TRUE)
        endif()
    endforeach()
    if(NOT sce_jni_header_found)
        message(FATAL_ERROR
            "${sce_jni_header} is in none of SCE_JNI_INCLUDE_DIRS "
            "(${SCE_JNI_INCLUDE_DIRS}); they must be the include directories "
            "of a full JDK, not a JRE")
    endif()
endforeach()

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
    ${SCE_JNI_INCLUDE_DIRS}
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
# without the frontend used to answer the shared ECMA-262 table from
# `EcmaScriptToLuaTransformer` alone — a SECOND set of answers under one
# engine name, only one of which `tests/ecmascript/kotlin_lua_divergences.json`
# describes. That rewriter is retired, so such a library now answers NOTHING:
# every ECMAScript expression reaches `refusedToLower`.
#
# ⚠ This comment named `theFrontendIsLinked` for four rounds before anything
# carried that name — a promise nothing kept. The test exists now, in
# `ScriptLanguageSeamTest`, and it is a RUN-TIME check rather than a build-time
# one: it asks the engine for `1 + 1` and requires an answer. The retirement is
# what makes that exact, because the frontend is now the only half that could
# have produced it.
if(NOT DEFINED SCE_REPO_ROOT)
    message(FATAL_ERROR
        "SCE_REPO_ROOT must be set. This library links sce-build's lowering "
        "surface through cmake/SCEBuildLowering.cmake, which is located from "
        "the repository root rather than relative to this file.")
endif()
include(${SCE_REPO_ROOT}/cmake/SCEBuildLowering.cmake)
target_link_libraries(sce_lua_jni PRIVATE SCE::Lowering)
