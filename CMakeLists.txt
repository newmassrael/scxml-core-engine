cmake_minimum_required(VERSION 3.14)

# Set policy for Boost before project()
if(POLICY CMP0167)
    cmake_policy(SET CMP0167 NEW)
endif()

project(SCXMLCoreEngine VERSION 1.0.0 LANGUAGES C CXX)

# =============================================================================
# Build Directory Validation
# =============================================================================

# Prevent in-source builds
if(CMAKE_SOURCE_DIR STREQUAL CMAKE_BINARY_DIR)
    message(FATAL_ERROR 
        "
❌ ERROR: In-source builds are not allowed!
"
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
"
        "Please create a separate build directory:
"
        "  mkdir build
"
        "  cd build
"
        "  cmake ..
"
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
")
endif()

# Validate build directory name
get_filename_component(BUILD_DIR_NAME "${CMAKE_BINARY_DIR}" NAME)
if(NOT BUILD_DIR_NAME MATCHES "^(build.*|Build|BUILD|Debug|Release|_build)$")
    message(FATAL_ERROR
        "
❌ ERROR: Please use standard build directory name!
"
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
"
        "Current directory: '${BUILD_DIR_NAME}'
"
        "Allowed names: build, build_*, Build, BUILD, Debug, Release, _build
"
        "
"
        "Correct usage:
"
        "  mkdir build         (standard build)
"
        "  mkdir build_wasm    (WASM build)
"
        "  cd build
"
        "  cmake ..
"
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
")
endif()

message(STATUS "Building in correct directory: ${BUILD_DIR_NAME}")

# Set default build type to Debug if not specified
if(NOT CMAKE_BUILD_TYPE)
    set(CMAKE_BUILD_TYPE "Debug" CACHE STRING "Choose the type of build (Debug or Release)" FORCE)
endif()

# C++20 standard
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# =============================================================================
# ccache Configuration - Cache compilation results
# =============================================================================
# ccache dramatically speeds up rebuilds by caching compilation results
# Benchmark results (full rebuild after clean):
#   - Without ccache: ~120 seconds (clean build)
#   - With ccache:    ~15 seconds (90% faster)

find_program(CCACHE_PROGRAM ccache)
if(CCACHE_PROGRAM)
    set(CMAKE_C_COMPILER_LAUNCHER "${CCACHE_PROGRAM}")
    set(CMAKE_CXX_COMPILER_LAUNCHER "${CCACHE_PROGRAM}")
    message(STATUS "Using ccache for faster rebuilds")
else()
    message(STATUS "ccache not found, consider installing: sudo apt install ccache")
endif()

# =============================================================================
# Linker Configuration - Use lld for faster linking
# =============================================================================
# lld (LLVM linker) provides 30-50% faster linking compared to GNU ld
# Benchmark results (621MB binary):
#   - GNU ld:  45.89 seconds
#   - lld:     29.58 seconds (35.5% improvement)
#
# This is especially important for w3c_test_cli which links 240+ test runners

# Check if lld is available
find_program(LLD_LINKER NAMES ld.lld lld)

if(LLD_LINKER)
    # Use lld for all linking
    set(CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} -fuse-ld=lld")
    set(CMAKE_SHARED_LINKER_FLAGS "${CMAKE_SHARED_LINKER_FLAGS} -fuse-ld=lld")
    set(CMAKE_MODULE_LINKER_FLAGS "${CMAKE_MODULE_LINKER_FLAGS} -fuse-ld=lld")
    message(STATUS "✅ Using lld linker (35% faster than GNU ld)")
else()
    message(STATUS "⚠️  lld not found, using default linker (consider: sudo apt install lld)")
endif()

# Sanitizer options for Debug builds
# Docker TSAN environment auto-detection
if(DEFINED ENV{IN_DOCKER_TSAN})
    set(ENABLE_TSAN ON CACHE BOOL "Auto-enabled in Docker TSAN environment" FORCE)
    message(STATUS "Docker TSAN environment detected - TSAN auto-enabled")
endif()

option(ENABLE_TSAN "Enable ThreadSanitizer instead of AddressSanitizer (Debug only)" OFF)

if(CMAKE_BUILD_TYPE STREQUAL "Debug")
    if(CMAKE_CXX_COMPILER_ID MATCHES "GNU|Clang")
        if(ENABLE_TSAN)
            message(STATUS "Enabling ThreadSanitizer for Debug build")
            message(STATUS "  Note: Use docker_tsan/ environment for TSAN with system library workarounds")

            add_compile_options(-fsanitize=thread -fno-omit-frame-pointer)
            add_link_options(-fsanitize=thread)
        elseif(NOT EMSCRIPTEN)
            # ASan disabled for WASM: QuickJS JS_CallInternal exceeds WebAssembly local variable limit
            message(STATUS "Enabling AddressSanitizer for Debug build")
            add_compile_options(-fsanitize=address -fno-omit-frame-pointer)
            add_link_options(-fsanitize=address)
        else()
            message(STATUS "WASM Debug build: ASan disabled (QuickJS compatibility)")
        endif()
    endif()
endif()

# =============================================================================
# WASM Memory Configuration (Single Source of Truth)
# =============================================================================
if(EMSCRIPTEN)
    # Central WASM memory configuration shared by sce/ and tests/
    set(WASM_INITIAL_MEMORY 4160749568 CACHE STRING "WASM initial heap size in bytes (4GB - 128MB)")
    message(STATUS "WASM: INITIAL_MEMORY=${WASM_INITIAL_MEMORY} (${WASM_INITIAL_MEMORY} bytes = 3.875 GB)")
endif()

# Include GNUInstallDirs for installation paths
include(GNUInstallDirs)

# Include SCE code generation utilities
include(cmake/SCECodegen.cmake)

# Position independent code generation (for shared libraries)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

# Disable cpp-httplib zstd support
# Reason: Current project uses only small HTTP messages, compression unnecessary
# Avoids cpp-httplib CMake zstd integration issues
set(HTTPLIB_USE_ZSTD_IF_AVAILABLE OFF CACHE BOOL "Disable zstd compression support" FORCE)

# Compilation options
if(MSVC)
    add_compile_options(/W4 /WX)  # /WX: treat warnings as errors
else()
    add_compile_options(-Wall -Wextra -pedantic -Werror)  # -Werror: treat warnings as errors
endif()

find_package(PkgConfig REQUIRED)

# Modular Architecture - Add subdirectories
add_subdirectory(sce)                 # Unified SCE library with integrated parsing

# Tools - code generation and TXML conversion
add_subdirectory(tools/codegen)       # Code generation tool

# TXML converter (Native builds only - requires filesystem operations)
if(NOT EMSCRIPTEN)
    add_subdirectory(tools/txml_converter)  # TXML to SCXML converter tool
endif()

# Test build option (enabled by default)
option(BUILD_TESTS "Build the tests" ON)
if(BUILD_TESTS)
    enable_testing()
    add_subdirectory(tests)
endif()

# Example build option (disabled by default)
option(BUILD_EXAMPLES "Build the examples" OFF)
if(BUILD_EXAMPLES)
    add_subdirectory(examples/cmake_function)
    add_subdirectory(examples/traffic_light)
endif()

# =============================================================================
# Local Visualizer Deployment
# =============================================================================
# Deploy visualizer to project root matching GitHub Pages structure
# Automatic deployment: Runs during build if visualizer/ doesn't exist
# Manual deployment: cmake --build build --target deploy-visualizer-local

# Automatic deployment strategy:
# EMSCRIPTEN builds always deploy WASM files after visualizer target builds
if(NOT EXISTS "${CMAKE_SOURCE_DIR}/visualizer")
    if(EMSCRIPTEN)
        message(STATUS "Visualizer directory not found, will create during EMSCRIPTEN build")
    else()
        message(STATUS "Visualizer deployment skipped: visualizer/ source directory not found")
    endif()
else()
    if(EMSCRIPTEN)
        message(STATUS "WASM files will be automatically deployed to visualizer/ after build")
    endif()
endif()

# Deployment target: Copy WASM build outputs to visualizer/ source directory
# visualizer/ contains both source files (tracked in git) and build outputs (gitignored)
set(VISUALIZER_DIR "${CMAKE_SOURCE_DIR}/visualizer")
set(VISUALIZER_WASM_FILE "${VISUALIZER_DIR}/visualizer.wasm")
set(VISUALIZER_JS_FILE "${VISUALIZER_DIR}/visualizer.js")

if(EMSCRIPTEN)
    # EMSCRIPTEN: Build WASM and copy directly to visualizer/
    # Manual deployment target (always available)
    add_custom_target(deploy-visualizer-local
        COMMAND ${CMAKE_COMMAND} -E copy_if_different
                ${CMAKE_CURRENT_BINARY_DIR}/tests/visualizer.wasm
                ${VISUALIZER_WASM_FILE}
        COMMAND ${CMAKE_COMMAND} -E copy_if_different
                ${CMAKE_CURRENT_BINARY_DIR}/tests/visualizer.js
                ${VISUALIZER_JS_FILE}
        COMMAND ${CMAKE_COMMAND} -E create_symlink
                visualizer.html
                ${VISUALIZER_DIR}/index.html
        DEPENDS visualizer
        WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
        COMMENT "Deploying WASM files to visualizer/"
        VERBATIM
    )
else()
    # Non-EMSCRIPTEN: WASM files should be pre-built or obtained from CI
    add_custom_target(deploy-visualizer-local
        COMMAND ${CMAKE_COMMAND} -E echo "Non-EMSCRIPTEN build: WASM files should be pre-built or obtained from CI"
        COMMAND ${CMAKE_COMMAND} -E create_symlink
                visualizer.html
                ${VISUALIZER_DIR}/index.html
        WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
        COMMENT "Visualizer source files in place, WASM pre-built or from CI"
        VERBATIM
    )
endif()

# Installation configuration (using GNUInstallDirs variables)
install(TARGETS sce_unified
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
    ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR}
    INCLUDES DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}
)

# Install txml-converter (Native builds only)
if(NOT EMSCRIPTEN)
    install(TARGETS txml-converter
        RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
    )
endif()

install(DIRECTORY
        sce/include/
    DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}
    FILES_MATCHING PATTERN "*.h"
)

# Install Python code generator
install(PROGRAMS tools/codegen/codegen.py
    DESTINATION ${CMAKE_INSTALL_BINDIR}
    RENAME scxml-codegen
)

install(FILES tools/codegen/scxml_parser.py
    DESTINATION ${CMAKE_INSTALL_BINDIR}
)

install(DIRECTORY tools/codegen/templates
    DESTINATION ${CMAKE_INSTALL_BINDIR}
    FILES_MATCHING PATTERN "*.jinja2"
)

# Package configuration
include(CMakePackageConfigHelpers)
write_basic_package_version_file(
    "${CMAKE_CURRENT_BINARY_DIR}/StateMachineConfigVersion.cmake"
    VERSION ${PROJECT_VERSION}
    COMPATIBILITY SameMajorVersion
)

# Generate StateMachineConfig.cmake file
configure_file(
    ${CMAKE_CURRENT_SOURCE_DIR}/cmake/StateMachineConfig.cmake.in
    ${CMAKE_CURRENT_BINARY_DIR}/StateMachineConfig.cmake
    @ONLY
)

# Package installation
install(
    FILES
        "${CMAKE_CURRENT_BINARY_DIR}/StateMachineConfig.cmake"
        "${CMAKE_CURRENT_BINARY_DIR}/StateMachineConfigVersion.cmake"
        "${CMAKE_CURRENT_SOURCE_DIR}/cmake/SCECodegen.cmake"
    DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/StateMachine
)
