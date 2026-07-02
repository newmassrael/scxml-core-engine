cmake_minimum_required(VERSION 3.22.1)
project(sce_android_jni C CXX)

set(CMAKE_C_STANDARD 11)
set(CMAKE_CXX_STANDARD 17)

# ---------------------------------------------------------------------------
# Source paths (relative to this CMakeLists.txt)
# ---------------------------------------------------------------------------
set(LUA_SRC_DIR "${CMAKE_SOURCE_DIR}/../../../../third_party/lua/src")
set(LUA_JNI_DIR "${CMAKE_SOURCE_DIR}/../../../../lua/src/main/cpp")
set(QUICKJS_SRC_DIR "${CMAKE_SOURCE_DIR}/../../../../third_party/quickjs")
set(QUICKJS_JNI_DIR "${CMAKE_SOURCE_DIR}/../../../../quickjs/src/main/cpp")

# ---------------------------------------------------------------------------
# Lua 5.4 + JNI bridge
# ---------------------------------------------------------------------------
file(GLOB LUA_SOURCES "${LUA_SRC_DIR}/*.c")
list(FILTER LUA_SOURCES EXCLUDE REGEX "lua\\.c$")
list(FILTER LUA_SOURCES EXCLUDE REGEX "luac\\.c$")

add_library(sce_lua_jni SHARED
    ${LUA_SOURCES}
    ${LUA_JNI_DIR}/lua_jni.cpp
)

target_include_directories(sce_lua_jni PRIVATE
    ${LUA_SRC_DIR}
)

# Android (bionic): -lm for math, -llog for __android_log_print
# No -ldl (bionic embeds dlopen), no -lpthread (bionic embeds pthread)
target_link_libraries(sce_lua_jni PRIVATE m log)
target_compile_definitions(sce_lua_jni PRIVATE LUA_USE_LINUX)

set_source_files_properties(${LUA_SOURCES} PROPERTIES
    COMPILE_FLAGS "-w"
)

# ---------------------------------------------------------------------------
# QuickJS + JNI bridge
# ---------------------------------------------------------------------------
set(QUICKJS_SOURCES
    ${QUICKJS_SRC_DIR}/quickjs.c
    ${QUICKJS_SRC_DIR}/cutils.c
    ${QUICKJS_SRC_DIR}/libregexp.c
    ${QUICKJS_SRC_DIR}/libunicode.c
    ${QUICKJS_SRC_DIR}/xsum.c
)

add_library(sce_quickjs_jni SHARED
    ${QUICKJS_SOURCES}
    ${QUICKJS_JNI_DIR}/quickjs_jni.cpp
)

target_include_directories(sce_quickjs_jni PRIVATE
    ${QUICKJS_SRC_DIR}
)

target_compile_definitions(sce_quickjs_jni PRIVATE _GNU_SOURCE)
target_link_libraries(sce_quickjs_jni PRIVATE m log)

set_source_files_properties(${QUICKJS_SOURCES} PROPERTIES
    COMPILE_FLAGS "-w"
)
