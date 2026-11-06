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

# QuickJS's own `qjs` library, as sce/CMakeLists.txt takes it: its sources,
# definitions and system libraries are the ones the QuickJS release names, so
# a release that renames a source file needs no edit here. The core library
# only: no libc module, no CLI tools, nothing installed.
set(BUILD_SHARED_LIBS OFF CACHE BOOL "" FORCE)
set(QJS_BUILD_LIBC OFF CACHE BOOL "" FORCE)
set(QJS_ENABLE_INSTALL OFF CACHE BOOL "" FORCE)
add_subdirectory(${QUICKJS_SRC_DIR} ${CMAKE_CURRENT_BINARY_DIR}/quickjs EXCLUDE_FROM_ALL)

# Build shared library: quickjs + JNI bridge
add_library(sce_quickjs_jni SHARED
    quickjs_jni.cpp
)

target_include_directories(sce_quickjs_jni PRIVATE
    ${SCE_JNI_INCLUDE_DIRS}
)

target_link_libraries(sce_quickjs_jni PRIVATE qjs)
