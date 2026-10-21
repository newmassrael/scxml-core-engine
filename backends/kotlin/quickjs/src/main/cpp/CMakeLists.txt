cmake_minimum_required(VERSION 3.16)
project(sce_quickjs_jni C CXX)

set(CMAKE_C_STANDARD 11)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

# QuickJS source directory (passed from Gradle)
if(NOT DEFINED QUICKJS_SRC_DIR)
    message(FATAL_ERROR "QUICKJS_SRC_DIR must be set to third_party/quickjs")
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
    ${SCE_JNI_INCLUDE_DIRS}
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
