cmake_minimum_required(VERSION 3.16)
project(sce_quickjs_jni C CXX)

set(CMAKE_C_STANDARD 11)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

# QuickJS source directory (passed from Gradle)
if(NOT DEFINED QUICKJS_SRC_DIR)
    message(FATAL_ERROR "QUICKJS_SRC_DIR must be set to third_party/quickjs")
endif()

# Find JNI headers
find_package(JNI REQUIRED)

# QuickJS core sources only (no libc, no CLI tools — sandboxed by default)
set(QUICKJS_SOURCES
    ${QUICKJS_SRC_DIR}/quickjs.c
    ${QUICKJS_SRC_DIR}/cutils.c
    ${QUICKJS_SRC_DIR}/libregexp.c
    ${QUICKJS_SRC_DIR}/libunicode.c
)

# Build shared library: quickjs + JNI bridge
add_library(sce_quickjs_jni SHARED
    ${QUICKJS_SOURCES}
    quickjs_jni.cpp
)

target_include_directories(sce_quickjs_jni PRIVATE
    ${QUICKJS_SRC_DIR}
    ${JNI_INCLUDE_DIRS}
)

# QuickJS requires _GNU_SOURCE on Linux for certain features
if(UNIX AND NOT APPLE)
    target_compile_definitions(sce_quickjs_jni PRIVATE _GNU_SOURCE)
    target_link_libraries(sce_quickjs_jni PRIVATE m dl pthread)
elseif(APPLE)
    target_link_libraries(sce_quickjs_jni PRIVATE m)
endif()

# Suppress warnings in QuickJS third-party sources
set_source_files_properties(${QUICKJS_SOURCES} PROPERTIES
    COMPILE_FLAGS "-w"
)
