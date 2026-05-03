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
if(NOT BUILD_DIR_NAME MATCHES "^(build|build_.*)$")
    message(FATAL_ERROR
        "
❌ ERROR: Please use standard build directory name!
"
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
"
        "Current directory: '${BUILD_DIR_NAME}'
"
        "Allowed names: build, build_* (e.g., build_release, build_wasm, build_tsan)
"
        "
"
        "Correct usage:
"
        "  mkdir build              (standard build)
"
        "  mkdir build_release      (release build)
"
        "  mkdir build_wasm         (WASM build)
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

# =============================================================================
# Optional Event Dispatchers
# =============================================================================
# Platform-specific event dispatcher implementations
# - Qt: Requires Qt5/Qt6 Core module
# - GLib: Requires glib-2.0

include(cmake/dispatchers/qt.cmake)
include(cmake/dispatchers/glib.cmake)

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
add_subdirectory(sce-c-runtime)       # C11 heapless runtime (RFC §5.J.1, watching-zenoh)

# Tools - TXML conversion (code generation is handled by sce-codegen Rust binary)
# TXML converter (Native builds only - requires filesystem operations)
if(NOT EMSCRIPTEN)
    add_subdirectory(tools/txml_converter)  # TXML to SCXML converter tool
endif()

# Test build option (enabled by default)
option(BUILD_TESTS "Build the tests" ON)
if(BUILD_TESTS)
    enable_testing()
    add_subdirectory(tests)
    # RFC §5.J.1 — C11 backend conformance harness, sibling of the
    # C++ tests/ tree. Lives at the repo root next to sce-c-runtime
    # so the C-only translation units stay isolated from C++ link.
    add_subdirectory(sce-c-tests)
endif()

# Example build option (enabled by default)
option(BUILD_EXAMPLES "Build the examples" ON)
if(BUILD_EXAMPLES)
    add_subdirectory(examples/cmake_function)
    add_subdirectory(examples/traffic_light)
    add_subdirectory(examples/smart_light)
    add_subdirectory(examples/timer)
    add_subdirectory(examples/async_dispatcher)
    add_subdirectory(examples/traffic_light_auto)
endif()

# Python bindings option (disabled by default, excluded from WASM builds)
option(BUILD_PYTHON_BINDINGS "Build Python bindings via pybind11" OFF)
if(BUILD_PYTHON_BINDINGS AND NOT EMSCRIPTEN)
    find_package(Python3 REQUIRED COMPONENTS Interpreter Development)
    add_subdirectory(sce-python)
endif()

# =============================================================================
# Thread Safety Option
# =============================================================================
# Enable thread-safe event queues for cross-thread event submission
# - When enabled: Adds mutex protection to EventQueueManager operations
# - When disabled: Zero overhead (default for single-threaded usage)
# - Zero Duplication: Single implementation in EventQueueManager applies to all engines
option(SCE_THREAD_SAFE "Enable thread-safe event queues" OFF)

if(SCE_THREAD_SAFE)
    message(STATUS "SCE: Thread-safe event queues enabled")
    target_compile_definitions(sce_base PUBLIC SCE_THREAD_SAFE)
else()
    message(STATUS "SCE: Thread-safe event queues disabled (zero overhead)")
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

# =============================================================================
# Installation Configuration
# =============================================================================
# Tier-scoped export sets (ARCHITECTURE.md §4-Tier):
#   SCE_CoreTargets       — sce_core + sce_base        (pure static AOT)
#   SCE_ScriptingTargets  — sce_scripting + qjs/lua54  (static hybrid AOT)
#   SCE_RuntimeTargets    — sce_runtime                (full interpreter)
# Each set lands in its own SCE_XxxTargets.cmake. SCEConfig.cmake includes
# them in dependency order; consumers link whichever tier they need, and the
# unused tiers are simply not linked — "you don't pay for what you don't use".
# qjs / lua54 register themselves into SCE_ScriptingTargets from their own
# CMakeLists (sce/CMakeLists.txt and third_party/lua/CMakeLists.txt).

# Define installation directories
set(SCE_INSTALL_CMAKEDIR "${CMAKE_INSTALL_LIBDIR}/cmake/SCE")
set(SCE_INSTALL_CODEGENDIR "${CMAKE_INSTALL_DATADIR}/sce/codegen")

# Install components mirror the 4-tier export sets so `cmake --install
# --component <Tier>` ships exactly the artifacts for that tier:
#   Core        — sce_core + sce_base + public headers + SCEConfig entry
#   Scripting   — sce_scripting + qjs + lua54 (+ lua headers)
#   Runtime     — sce_runtime + txml-converter
#   Codegen — SCECodegen/SCEClangFormat + sce-codegen binary + templates
# Codegen stays separate because it is independent of tier selection —
# any consumer (base/scripting/runtime) that uses sce_add_state_machine()
# relies on the same codegen binary + Jinja2 templates.

# Core tier: sce_core (header-only) + sce_base (pure static AOT utilities)
install(TARGETS sce_core sce_base
    EXPORT SCE_CoreTargets
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR} COMPONENT Core
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR} COMPONENT Core
    ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR} COMPONENT Core
    INCLUDES DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}
)

# Mesh transport (opt-in via SCE_ENABLE_MESH). sce_mesh_common is a
# C++-first static library per mesh_cpp_first_policy; it ships with the
# Core component so `--component Core` on a mesh-enabled build still gets
# everything required to link AOT mesh machines. tinycbor is vendored
# like lua54/qjs and must be installed alongside because sce_mesh_common
# PRIVATE-links it — a static archive consumer still resolves CBOR
# symbols at link time.
if(SCE_ENABLE_MESH)
    install(TARGETS sce_mesh_common tinycbor
        EXPORT SCE_CoreTargets
        LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR} COMPONENT Core
        ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR} COMPONENT Core
        INCLUDES DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}
    )
    install(DIRECTORY sce/include/mesh
        DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}
        COMPONENT Core
        FILES_MATCHING PATTERN "*.h"
    )
endif()

# Scripting tier: sce_scripting (qjs/lua54 register themselves here)
if(SCE_HAS_SCRIPTING)
    install(TARGETS sce_scripting
        EXPORT SCE_ScriptingTargets
        LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR} COMPONENT Scripting
        RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR} COMPONENT Scripting
        ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR} COMPONENT Scripting
    )
endif()

# Runtime tier: sce_runtime (full interpreter, PUBLIC-links httplib)
install(TARGETS sce_runtime
    EXPORT SCE_RuntimeTargets
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR} COMPONENT Runtime
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR} COMPONENT Runtime
    ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR} COMPONENT Runtime
)

# Install txml-converter (Native builds only)
if(NOT EMSCRIPTEN)
    install(TARGETS txml-converter
        RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR} COMPONENT Runtime
    )
endif()

# Install headers partitioned by tier. Each subdirectory below is installed
# with the component that owns the code it declares, so a consumer requesting
# `--component Core` does not end up with Runtime-only headers on disk.
#
# Top-level *.h files (types.h, SCXMLTypes.h, GuardUtils.h, SCXMLEngine.h,
# ReadySCXMLEngine.h) stay with Core because lower-tier headers already
# include them (core/EventMetadata.h → SCXMLTypes.h, common/*.h → types.h)
# and shipping them conditionally would break Core-only compilation.

# Core: sce_core (header-only) + sce_base + AOT-side helpers (wrappers,
# dispatchers interface/impls). Mesh headers are gated separately because
# sce_mesh_common only exists when SCE_ENABLE_MESH=ON; shipping the
# headers unconditionally would leave stale mesh/*.h on disk without the
# archive that defines their symbols.
foreach(_sce_core_dir core static common backends wrappers dispatchers)
    install(DIRECTORY sce/include/${_sce_core_dir}
        DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}
        COMPONENT Core
        FILES_MATCHING PATTERN "*.h"
    )
endforeach()
install(DIRECTORY sce/include/
    DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}
    COMPONENT Core
    FILES_MATCHING
        PATTERN "*.h"
        PATTERN "actions"     EXCLUDE
        PATTERN "backends"    EXCLUDE
        PATTERN "common"      EXCLUDE
        PATTERN "core"        EXCLUDE
        PATTERN "dispatchers" EXCLUDE
        PATTERN "events"      EXCLUDE
        PATTERN "factory"     EXCLUDE
        PATTERN "mesh"        EXCLUDE
        PATTERN "model"       EXCLUDE
        PATTERN "parsing"     EXCLUDE
        PATTERN "runtime"     EXCLUDE
        PATTERN "scripting"   EXCLUDE
        PATTERN "states"      EXCLUDE
        PATTERN "static"      EXCLUDE
        PATTERN "wrappers"    EXCLUDE
)

# Scripting: JSEngine / LuaEngine / ScriptEngineProvider.
install(DIRECTORY sce/include/scripting
    DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}
    COMPONENT Scripting
    FILES_MATCHING PATTERN "*.h"
)

# Runtime: Interpreter/state-machine/parser/events headers.
foreach(_sce_runtime_dir model runtime actions events states parsing factory)
    install(DIRECTORY sce/include/${_sce_runtime_dir}
        DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}
        COMPONENT Runtime
        FILES_MATCHING PATTERN "*.h"
    )
endforeach()

# Codegen-component assets (cmake utilities + templates) are declared in
# sce/sce_codegen_assets.cmake — the single source of truth shared with
# scripts/package_embed.sh. Adding a file to the Codegen component means
# editing only the SSOT; both install() rules and the embed packager
# pick it up automatically.
include(${CMAKE_CURRENT_SOURCE_DIR}/sce/sce_codegen_assets.cmake)

# License-text distribution is declared in sce/sce_licenses.cmake — the
# single source of truth shared with scripts/package_embed.sh. LGPL-2.1
# §1 (SCE itself) and MIT §1 (every vendored third_party dep with a
# standalone LICENSE file) both require the licence text to ride along
# with the distribution; a flat hardcoded list in either consumer
# drifts the moment a new third_party dep lands. Each file is shipped
# with the tier component that links the code it covers, so
# `--component Core` does not pull Runtime-only licence files.
include(${CMAKE_CURRENT_SOURCE_DIR}/sce/sce_licenses.cmake)

# SCE-self covenant (always Core — SCE itself is the Core baseline).
install(FILES ${SCE_LICENSE_FILES_SCE}
    DESTINATION ${CMAKE_INSTALL_DOCDIR}
    COMPONENT Core
)
# Third-party deps unconditionally linked into sce_base (Core tier).
foreach(_sce_license_file IN LISTS SCE_LICENSE_FILES_THIRD_PARTY_CORE)
    get_filename_component(_sce_license_subdir "${_sce_license_file}" DIRECTORY)
    install(FILES "${CMAKE_CURRENT_SOURCE_DIR}/${_sce_license_file}"
        DESTINATION "${CMAKE_INSTALL_DOCDIR}/${_sce_license_subdir}"
        COMPONENT Core
    )
endforeach()
if(SCE_ENABLE_MESH)
    foreach(_sce_license_file IN LISTS SCE_LICENSE_FILES_THIRD_PARTY_MESH)
        get_filename_component(_sce_license_subdir "${_sce_license_file}" DIRECTORY)
        install(FILES "${CMAKE_CURRENT_SOURCE_DIR}/${_sce_license_file}"
            DESTINATION "${CMAKE_INSTALL_DOCDIR}/${_sce_license_subdir}"
            COMPONENT Core
        )
    endforeach()
endif()
if(SCE_USE_SPDLOG)
    foreach(_sce_license_file IN LISTS SCE_LICENSE_FILES_THIRD_PARTY_SPDLOG)
        get_filename_component(_sce_license_subdir "${_sce_license_file}" DIRECTORY)
        install(FILES "${CMAKE_CURRENT_SOURCE_DIR}/${_sce_license_file}"
            DESTINATION "${CMAKE_INSTALL_DOCDIR}/${_sce_license_subdir}"
            COMPONENT Core
        )
    endforeach()
endif()
if(SCE_HAS_SCRIPTING)
    foreach(_sce_license_file IN LISTS SCE_LICENSE_FILES_THIRD_PARTY_SCRIPTING)
        get_filename_component(_sce_license_subdir "${_sce_license_file}" DIRECTORY)
        install(FILES "${CMAKE_CURRENT_SOURCE_DIR}/${_sce_license_file}"
            DESTINATION "${CMAKE_INSTALL_DOCDIR}/${_sce_license_subdir}"
            COMPONENT Scripting
        )
    endforeach()
endif()
if(SCE_ENABLE_HTTP AND NOT EMSCRIPTEN)
    foreach(_sce_license_file IN LISTS SCE_LICENSE_FILES_THIRD_PARTY_RUNTIME)
        get_filename_component(_sce_license_subdir "${_sce_license_file}" DIRECTORY)
        install(FILES "${CMAKE_CURRENT_SOURCE_DIR}/${_sce_license_file}"
            DESTINATION "${CMAKE_INSTALL_DOCDIR}/${_sce_license_subdir}"
            COMPONENT Runtime
        )
    endforeach()
endif()

# Install Jinja2 templates (used by sce-codegen at runtime via SCE_TEMPLATE_DIR)
install(DIRECTORY ${SCE_CODEGEN_TEMPLATE_DIR}
    DESTINATION ${SCE_INSTALL_CODEGENDIR}
    COMPONENT Codegen
    FILES_MATCHING PATTERN "${SCE_CODEGEN_TEMPLATE_PATTERN}"
)

# Install clang-format style file alongside templates so SCEClangFormat.cmake
# can locate it in system-install layout via auto-detection.
install(FILES ${SCE_CODEGEN_STYLE_FILE}
    DESTINATION ${SCE_INSTALL_CODEGENDIR}
    COMPONENT Codegen
)

# Install sce-codegen binary if it was built before cmake configure.
# Build order: cargo build --bin sce-codegen --features cli --release -p sce-build
# Then: cmake -B build && cmake --install build
find_program(_SCE_CODEGEN_BIN sce-codegen
    PATHS "${CMAKE_SOURCE_DIR}/target/release" "${CMAKE_SOURCE_DIR}/target/debug"
    NO_DEFAULT_PATH
)
if(_SCE_CODEGEN_BIN)
    install(PROGRAMS "${_SCE_CODEGEN_BIN}"
        DESTINATION ${CMAKE_INSTALL_BINDIR}
        COMPONENT Codegen
    )
else()
    message(STATUS "SCE: sce-codegen not found in target/ -- skipping binary install")
endif()

# =============================================================================
# Package Configuration for find_package(SCE)
# =============================================================================
# Two consumer paths are supported:
#   1. Install-tree:  cmake --install ...; find_package(SCE)
#   2. Build-tree:    find_package(SCE PATHS /path/to/sce/build NO_DEFAULT_PATH)
# Both generate their own SCEConfig.cmake with correctly resolved paths.
include(CMakePackageConfigHelpers)

# Version file (shared by both trees)
write_basic_package_version_file(
    "${CMAKE_CURRENT_BINARY_DIR}/SCEConfigVersion.cmake"
    VERSION ${PROJECT_VERSION}
    COMPATIBILITY SameMajorVersion
)

# httplib transitive-dep detection (used by both trees of SCEConfig.cmake).
# sce_runtime publicly links httplib::httplib when SCE_ENABLE_HTTP is on
# (sce/CMakeLists.txt:501). export() is strict about unresolved deps, so when
# we emit httplibTargets below we must also replay httplib's
# find_dependency(OpenSSL/ZLIB/Brotli/zstd) chain in SCEConfig — cpp-httplib's
# own build-tree httplibConfig.cmake is unusable here because it computes
# PACKAGE_PREFIX_DIR incorrectly for the uninstalled layout. We detect which
# transitive deps are actually in play by scanning the target's link interface
# for marker namespaces, and pass the flags to SCEConfig.cmake.in via
# @SCE_HTTPLIB_NEEDS_*@ substitution.
set(SCE_HTTPLIB_NEEDS_OPENSSL FALSE)
set(SCE_HTTPLIB_NEEDS_ZLIB    FALSE)
set(SCE_HTTPLIB_NEEDS_BROTLI  FALSE)
set(SCE_HTTPLIB_NEEDS_ZSTD    FALSE)
if(SCE_ENABLE_HTTP AND NOT EMSCRIPTEN AND TARGET httplib)
    # Detect by matching the enabled generator-expression form
    # $<$<BOOL:TRUE>:<namespace>::…>. The disabled form has $<BOOL:> (empty),
    # which we must NOT pick up — hence the literal "TRUE>:" prefix.
    get_target_property(_sce_httplib_iface httplib INTERFACE_LINK_LIBRARIES)
    if(_sce_httplib_iface MATCHES "BOOL:TRUE>:OpenSSL::")
        set(SCE_HTTPLIB_NEEDS_OPENSSL TRUE)
    endif()
    if(_sce_httplib_iface MATCHES "BOOL:TRUE>:ZLIB::")
        set(SCE_HTTPLIB_NEEDS_ZLIB TRUE)
    endif()
    if(_sce_httplib_iface MATCHES "BOOL:TRUE>:Brotli::")
        set(SCE_HTTPLIB_NEEDS_BROTLI TRUE)
    endif()
    if(_sce_httplib_iface MATCHES "BOOL:TRUE>:zstd::")
        set(SCE_HTTPLIB_NEEDS_ZSTD TRUE)
    endif()
    unset(_sce_httplib_iface)
endif()

# ---------------------------------------------------------------------------
# Build-tree package config (consumed directly from ${CMAKE_CURRENT_BINARY_DIR})
# Paths are absolute source-tree paths; build tree is not relocatable.
# ---------------------------------------------------------------------------
set(INCLUDE_INSTALL_DIR  "${CMAKE_CURRENT_SOURCE_DIR}/sce/include")
set(CODEGEN_INSTALL_DIR  "${CMAKE_CURRENT_SOURCE_DIR}/tools/codegen")
set(CODEGEN_BIN_DIR      "${CMAKE_CURRENT_SOURCE_DIR}/target/release")
set(SCE_HTTPLIB_FIND_MODULES_DIR "${CMAKE_CURRENT_SOURCE_DIR}/third_party/cpp-httplib/cmake")
configure_package_config_file(
    "${CMAKE_CURRENT_SOURCE_DIR}/cmake/SCEConfig.cmake.in"
    "${CMAKE_CURRENT_BINARY_DIR}/SCEConfig.cmake"
    INSTALL_DESTINATION "${CMAKE_CURRENT_BINARY_DIR}"
    PATH_VARS INCLUDE_INSTALL_DIR CODEGEN_INSTALL_DIR CODEGEN_BIN_DIR
)

# Build-tree target exports (one file per tier, mirrors install(EXPORT) below).
# httplib is a PUBLIC link dep of sce_runtime only, so its build-tree export
# sits alongside SCE_RuntimeTargets and is loaded by SCEConfig only when the
# Runtime tier is present.
export(EXPORT SCE_CoreTargets
    FILE "${CMAKE_CURRENT_BINARY_DIR}/SCE_CoreTargets.cmake"
    NAMESPACE SCE::
)
if(SCE_HAS_SCRIPTING)
    export(EXPORT SCE_ScriptingTargets
        FILE "${CMAKE_CURRENT_BINARY_DIR}/SCE_ScriptingTargets.cmake"
        NAMESPACE SCE::
    )
endif()
export(EXPORT SCE_RuntimeTargets
    FILE "${CMAKE_CURRENT_BINARY_DIR}/SCE_RuntimeTargets.cmake"
    NAMESPACE SCE::
)
if(SCE_ENABLE_HTTP AND NOT EMSCRIPTEN AND TARGET httplib)
    export(EXPORT httplibTargets
        FILE "${CMAKE_CURRENT_BINARY_DIR}/httplibTargets.cmake"
        NAMESPACE httplib::
    )
endif()

# Stage SCECodegen.cmake into the build tree so find_package(SCE PATHS build)
# can include() it from the same directory as SCEConfig.cmake
configure_file(
    "${CMAKE_CURRENT_SOURCE_DIR}/cmake/SCECodegen.cmake"
    "${CMAKE_CURRENT_BINARY_DIR}/SCECodegen.cmake"
    COPYONLY
)
configure_file(
    "${CMAKE_CURRENT_SOURCE_DIR}/cmake/SCEClangFormat.cmake"
    "${CMAKE_CURRENT_BINARY_DIR}/SCEClangFormat.cmake"
    COPYONLY
)

# ---------------------------------------------------------------------------
# Install-tree package config (staged under _install/ to avoid overwriting
# the build-tree SCEConfig.cmake above)
# ---------------------------------------------------------------------------
set(INCLUDE_INSTALL_DIR  "${CMAKE_INSTALL_INCLUDEDIR}")
set(CODEGEN_INSTALL_DIR  "${SCE_INSTALL_CODEGENDIR}")
set(CODEGEN_BIN_DIR      "${CMAKE_INSTALL_BINDIR}")
# Install-tree takes the find_dependency(httplib) branch, which handles
# transitive module paths itself — no build-tree FindBrotli fallback needed.
set(SCE_HTTPLIB_FIND_MODULES_DIR "")
configure_package_config_file(
    "${CMAKE_CURRENT_SOURCE_DIR}/cmake/SCEConfig.cmake.in"
    "${CMAKE_CURRENT_BINARY_DIR}/_install/SCEConfig.cmake"
    INSTALL_DESTINATION ${SCE_INSTALL_CMAKEDIR}
    PATH_VARS INCLUDE_INSTALL_DIR CODEGEN_INSTALL_DIR CODEGEN_BIN_DIR
)

install(EXPORT SCE_CoreTargets
    FILE SCE_CoreTargets.cmake
    NAMESPACE SCE::
    DESTINATION ${SCE_INSTALL_CMAKEDIR}
    COMPONENT Core
)
if(SCE_HAS_SCRIPTING)
    install(EXPORT SCE_ScriptingTargets
        FILE SCE_ScriptingTargets.cmake
        NAMESPACE SCE::
        DESTINATION ${SCE_INSTALL_CMAKEDIR}
        COMPONENT Scripting
    )
endif()
install(EXPORT SCE_RuntimeTargets
    FILE SCE_RuntimeTargets.cmake
    NAMESPACE SCE::
    DESTINATION ${SCE_INSTALL_CMAKEDIR}
    COMPONENT Runtime
)

# SCEConfig.cmake is the entry point for every tier (it parses COMPONENTS and
# decides which target files to include), so it must ship with the Core
# component so `--component Core` alone yields a usable find_package(SCE).
# SCECodegen / SCEClangFormat are tool-independent utilities and ride with
# the Codegen component alongside the sce-codegen binary + templates.
install(FILES
    "${CMAKE_CURRENT_BINARY_DIR}/_install/SCEConfig.cmake"
    "${CMAKE_CURRENT_BINARY_DIR}/SCEConfigVersion.cmake"
    DESTINATION ${SCE_INSTALL_CMAKEDIR}
    COMPONENT Core
)
# SCECodegen.cmake + SCEClangFormat.cmake — file list in
# sce/sce_codegen_assets.cmake (SSOT shared with package_embed.sh).
foreach(_sce_codegen_cmake IN LISTS SCE_CODEGEN_CMAKE_FILES)
    install(FILES "${CMAKE_CURRENT_SOURCE_DIR}/${_sce_codegen_cmake}"
        DESTINATION ${SCE_INSTALL_CMAKEDIR}
        COMPONENT Codegen
    )
endforeach()
