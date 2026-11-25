# 3.20 is the oldest CMake whose add_custom_command(DEPFILE) every generator
# accepts and whose CMP0116 hands Ninja a depfile in the build directory's terms.
# A custom command's DEPFILE names its targets as sce-codegen wrote them,
# absolute, and Ninja compares them with the command's outputs, which it holds
# relative to the build directory; with CMP0116 unset the two never match
# ("expected depfile ... to mention 'x_sm.h', got '/abs/x_sm.h'" under
# `ninja -d explain`) and every build re-runs the command. The generated
# headers' freshness contract (spec 6.2.6) rides on that depfile.
#
# Measured 2026-09-30 with Kitware's release tarballs and this tree: 3.19.8 and
# 3.16.9 stop the configure under Unix Makefiles ("Option DEPFILE not supported
# by Unix Makefiles"), and under Ninja every depfile edge re-runs on every build.
# 3.20.6 and 3.28.3 configure, build, and then have nothing left to do. Below
# 3.18 the zenoh-cpp package cannot load either: its config file defines an
# ALIAS of a non-GLOBAL imported target.
#
# The version below also sets CMP0116 NEW here and in every directory that says
# the same. A function keeps the policy of the scope it was DEFINED in, so the
# modules in cmake/ that a consumer includes under a lower minimum still set it
# for themselves.
cmake_minimum_required(VERSION 3.20)

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

# Check if lld is available. `-fuse-ld` is the GCC and Clang driver's option, so
# the search is theirs: MSVC links with its own link.exe, which reads the option
# as one it does not know, and the LLVM a Windows machine carries would be found
# here and handed to it.
if(NOT MSVC)
    find_program(LLD_LINKER NAMES ld.lld lld)
endif()

if(MSVC)
    message(STATUS "Linking with the MSVC linker")
elseif(LLD_LINKER)
    # Use lld for all linking
    set(CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} -fuse-ld=lld")
    set(CMAKE_SHARED_LINKER_FLAGS "${CMAKE_SHARED_LINKER_FLAGS} -fuse-ld=lld")
    set(CMAKE_MODULE_LINKER_FLAGS "${CMAKE_MODULE_LINKER_FLAGS} -fuse-ld=lld")
    message(STATUS "✅ Using lld linker (35% faster than GNU ld)")
else()
    message(STATUS "⚠️  lld not found, using default linker (consider: sudo apt install lld)")
endif()

# =============================================================================
# Debug information layout - split DWARF
# =============================================================================
# Debug info is what this build directory is made of, and the linker copies it
# into every executable that links a translation unit. Measured 2026-08-12 over
# the 134 test executables: 4.20 GB of .debug* sections against 0.16 GB of
# .text. One binary, state_machine_tests, is 106.9 MB of which 100.4 MB is
# debug info that also sits in every other binary linking the same headers.
#
# -gsplit-dwarf leaves .debug_info, .debug_loclists and .debug_str in a .dwo
# file beside the object instead. The .dwo exists once per translation unit,
# so the per-binary duplication is what disappears. Measured on three
# representative targets, same tree, same targets, on a 32-core / 125 GB Linux
# build machine with gcc 13.3 and lld 18:
#
#   executable   106.9 MB -> 72.4 MB   (-32%)
#   build tree      1.2 GB -> 800 MB   (-33%, .dwo files included)
#   relink            98 ms -> 74 ms
#
# The remaining 66 MB per binary is .debug_gnu_pubnames (26.0) +
# .debug_gnu_pubtypes (20.3) + .debug_addr (11.1) + .debug_line (8.5). gcc
# turns those pubname tables on WITH -gsplit-dwarf and -gno-pubnames does not
# decline them (measured: byte-identical output), so no *compile* flag declines
# them. -Wl,--gdb-index consumes them instead, at link time — see the block
# below, which is where the -32% figure stops being the ceiling.
#
# Applied through add_compile_options rather than CMAKE_CXX_FLAGS on purpose:
# flags set before project() are also given to CMake's own compiler test, and
# that is exactly where an earlier --gdb-index attempt died. Here the flag
# reaches real targets only.
#
# Debugging is unaffected as long as the .dwo files stay beside their objects
# in the build tree, which is where the compiler writes them. gdb reads them
# through the skeleton unit; a build tree moved without its .dwo files loses
# variable and type detail, so this is a build-tree setting, not a shipping one.
option(SCE_SPLIT_DWARF "Keep DWARF in .dwo files instead of copying it into every binary" ON)

if(SCE_SPLIT_DWARF AND CMAKE_BUILD_TYPE MATCHES "Debug|RelWithDebInfo")
    include(CheckCXXCompilerFlag)
    check_cxx_compiler_flag("-gsplit-dwarf" SCE_HAS_SPLIT_DWARF)
    if(SCE_HAS_SPLIT_DWARF)
        add_compile_options($<$<COMPILE_LANGUAGE:C,CXX>:-gsplit-dwarf>)
        message(STATUS "✅ Split DWARF enabled (debug info stays in .dwo, ~32% smaller binaries)")
    else()
        message(STATUS "⚠️  -gsplit-dwarf not supported by this compiler; debug info stays in the binaries")
    endif()
endif()

# =============================================================================
# Debug symbol lookup - .gdb_index instead of pubname tables
# =============================================================================
# What split DWARF leaves behind is dominated by two lookup tables gcc emits
# for the debugger's benefit: .debug_gnu_pubnames and .debug_gnu_pubtypes,
# 47.2 MB of the 67.6 MB of debug sections on a representative test binary.
# They are an index, not information — every fact they point at also lives in
# the .dwo files.
#
# --gdb-index has the linker consume both and emit .gdb_index in their place,
# which is the format gdb actually prefers. Measured 2026-08-13 by relinking
# tests/action_tests three times each, same objects, gcc 13.3 + lld 18.1.3:
#
#   executable      74.6 MB -> 38.7 MB   (-48%)
#   link              149 ms -> 122 ms   (-18%: 47 MB of tables cost more to
#                                         write than an 11 MB index)
#   .debug_gnu_pub*  47.2 MB -> 0, replaced by .gdb_index 11.3 MB
#
# Debugging improves rather than degrades: `info line` and `info functions`
# both resolve through the index, and `addr2sce --pc` is unaffected either way
# because it reads .symtab rather than DWARF (its own --help says so).
#
# Guarded on lld because bfd ld rejects the option; the guard is the reason
# this sits inside the LLD_LINKER branch's reach rather than beside the
# compile flags. add_link_options, not CMAKE_EXE_LINKER_FLAGS, for the same
# reason the split-DWARF block gives: CMake's compiler test never sees it.
#
# `scripts/gates/cpp-suite.sh` asserts the emitted binaries actually carry
# .gdb_index and no pubnames. A flag that stops reaching the link is exactly
# the kind of change that leaves every test green.
if(LLD_LINKER AND CMAKE_BUILD_TYPE MATCHES "Debug|RelWithDebInfo")
    add_link_options(-Wl,--gdb-index)
    message(STATUS "✅ .gdb_index enabled (pubname tables consumed at link, ~48% smaller binaries)")
endif()

# =============================================================================
# Build paths kept out of what the build writes
# =============================================================================
# The compiler is given every source by its absolute path, so __FILE__ (which
# SCE::source_location records for every log call,
# sce/include/common/SourceLocation.h), an assert's message and the debug info
# carried the directory this tree was checked out and configured in: two
# builds of one source in two checkouts differed in their bytes, and no cache
# could hand one's objects to the other (reproducible-builds.org, "build
# path"). Each source is named by its path under the source tree instead.
#
# The flags name no path of the checkout where the compiler allows it, for the
# reason bx gives its remote compiles the same ones (bx RFC 0012,
# `reexec::path_flags`, measured by its tools/measure/canonical-paths.sh): the
# flags are part of the compile command, and a command that names the checkout
# makes a compiler cache that keys on it one per checkout path.
#   - GCC resolves a relative prefix where it compiles (`-fcanon-prefix-map`,
#     GCC 13), so the way up from the build tree to the source tree (`..` for
#     `build/`) maps the source tree to `.`: a source is `./sce/src/...`, a file
#     the build generates `./build/...`, the compilation directory `./build`.
#     A Ninja build compiles in the build tree, which is what makes the way up
#     the same for every compile; a build tree outside the source tree has no
#     way up that names no path, and keeps its paths.
#   - Clang maps a prefix as text, and CMake names sources by absolute path, so
#     the source tree is named and mapped to nothing: a source is
#     `sce/src/...`, and a build tree inside the source tree, the compilation
#     directory among it, is `build/...`. The emscripten trees take this, emcc
#     being Clang.
#   - MSVC has no prefix map, and its builds keep the paths.
#
# A debugger is pointed at the source tree for the sources (`directory`). What
# reads a path at run time derives none from __FILE__: the W3C runner finds its
# report script in the tree it runs in (tests/w3c/W3CTestRunner.cpp).
file(RELATIVE_PATH _SCE_WAY_UP "${CMAKE_BINARY_DIR}" "${CMAKE_SOURCE_DIR}")
if(_SCE_WAY_UP MATCHES "^\\.\\.(/\\.\\.)*$")
    set(_SCE_BUILD_INSIDE_SOURCE TRUE)
else()
    set(_SCE_BUILD_INSIDE_SOURCE FALSE)
endif()
include(CheckCCompilerFlag)
include(CheckCXXCompilerFlag)
if(CMAKE_C_COMPILER_ID STREQUAL "GNU" AND CMAKE_CXX_COMPILER_ID STREQUAL "GNU")
    check_c_compiler_flag("-fcanon-prefix-map" SCE_C_HAS_CANON_PREFIX_MAP)
    check_cxx_compiler_flag("-fcanon-prefix-map" SCE_CXX_HAS_CANON_PREFIX_MAP)
    if(SCE_C_HAS_CANON_PREFIX_MAP AND SCE_CXX_HAS_CANON_PREFIX_MAP
       AND _SCE_BUILD_INSIDE_SOURCE AND CMAKE_GENERATOR MATCHES "Ninja")
        add_compile_options(
            "$<$<COMPILE_LANGUAGE:C,CXX>:-fcanon-prefix-map>"
            "$<$<COMPILE_LANGUAGE:C,CXX>:-ffile-prefix-map=${_SCE_WAY_UP}=.>")
    else()
        message(STATUS "Build paths: kept (GCC without -fcanon-prefix-map, a build tree outside "
                       "the source tree, or a generator that compiles outside the build tree)")
    endif()
elseif(CMAKE_C_COMPILER_ID MATCHES "Clang" AND CMAKE_CXX_COMPILER_ID MATCHES "Clang")
    check_c_compiler_flag("-ffile-prefix-map=/a=/b" SCE_C_HAS_FILE_PREFIX_MAP)
    check_cxx_compiler_flag("-ffile-prefix-map=/a=/b" SCE_CXX_HAS_FILE_PREFIX_MAP)
    if(SCE_C_HAS_FILE_PREFIX_MAP AND SCE_CXX_HAS_FILE_PREFIX_MAP)
        add_compile_options("$<$<COMPILE_LANGUAGE:C,CXX>:-ffile-prefix-map=${CMAKE_SOURCE_DIR}/=>")
    endif()
endif()

# Sanitizer options for Debug builds.
# ThreadSanitizer is a host-native build: `cmake -DENABLE_TSAN=ON` (see
# scripts/build_tsan.sh). No container is required — TSAN is a compiler
# instrumentation and the tests skip the HTTP paths that are incompatible
# with it via SCE::Test::Utils::isThreadSanitizerBuild().
option(ENABLE_TSAN "Enable ThreadSanitizer instead of AddressSanitizer (Debug only)" OFF)

if(CMAKE_BUILD_TYPE STREQUAL "Debug")
    if(CMAKE_CXX_COMPILER_ID MATCHES "GNU|Clang")
        if(ENABLE_TSAN)
            message(STATUS "Enabling ThreadSanitizer for Debug build")
            message(STATUS "  Tip: export TSAN_OPTIONS=ignore_noninstrumented_modules=1 to quiet system-library reports")

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

    # C++ exceptions are caught under Emscripten only where the code that
    # catches was COMPILED with catching on: the default compiles every `catch`
    # away and a throw aborts the program. The engine reports a failed
    # expression by throwing and catches it where the specification raises
    # error.execution, so catching is one setting of the whole build, for
    # every C++ object and every link. It was a link option of each test
    # program before, which left the engine's own catches compiled away and
    # turned the first such error into an abort of the W3C run.
    add_compile_options($<$<COMPILE_LANGUAGE:CXX>:-sDISABLE_EXCEPTION_CATCHING=0>)
    add_link_options(-sDISABLE_EXCEPTION_CATCHING=0)

    # Whether this tree has threads, which decides the engine's execution model
    # (sce/include/common/ThreadSupport.h): `-pthread` defines
    # __EMSCRIPTEN_PTHREADS__ and the engine is threaded, as natively; without
    # it the engine is host-driven. It is one setting of the whole tree because
    # it is part of every object's ABI (shared memory and atomics): a program
    # linked without `-pthread` against archives compiled with it mixes two
    # ABIs, which the browser module did while the model was chosen by the
    # target and no thread was ever started.
    #
    #   ON  (build_wasm): the Node programs, every test suite among them.
    #   OFF (build_wasm_web): the browser module, which runs without
    #       cross-origin isolation and so without SharedArrayBuffer.
    option(SCE_WASM_THREADS "Build the emscripten tree with pthreads (the threaded engine)" ON)
    if(SCE_WASM_THREADS)
        add_compile_options(-pthread)
        add_link_options(-pthread)
    endif()
    message(STATUS "WASM: threads ${SCE_WASM_THREADS}")
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

# The COSE operations of the C++ reference host, on libsodium where it is found
# (docs/adr/0107, D7): option SCE_HOST_COSE, target sce_host_cose.
include(cmake/SCEHostCose.cmake)

# The network transport of the C++ reference host's HTTP operation, on libcurl
# where it is found, behind a destination policy (docs/adr/0109, D4): option
# SCE_HOST_HTTP_CURL, target sce_host_http_curl.
include(cmake/SCEHostHttp.cmake)

# Position independent code generation (for shared libraries)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

# Disable cpp-httplib zstd support
# Reason: Current project uses only small HTTP messages, compression unnecessary
# Avoids cpp-httplib CMake zstd integration issues
set(HTTPLIB_USE_ZSTD_IF_AVAILABLE OFF CACHE BOOL "Disable zstd compression support" FORCE)

# Compilation options
if(MSVC)
    add_compile_options(/W4 /WX)  # /WX: treat warnings as errors
    # The sources are UTF-8, which cl reads in the system's code page unless told:
    # a comment's em dash is then a warning (C4819, an error under /WX) on a
    # machine set to another one, and a string literal's bytes change with it.
    # /bigobj: a translation unit of generated machines and a test framework
    # passes the 2^16 sections an object holds by default (C1128).
    add_compile_options(/utf-8 /bigobj)
    # The C library functions every platform has (getenv, fopen, strdup) are
    # deprecated by the Microsoft C runtime in favour of its own *_s forms,
    # which only it has (C4996). The portable names are the ones this tree calls.
    add_compile_definitions(_CRT_SECURE_NO_WARNINGS _CRT_NONSTDC_NO_WARNINGS)
else()
    add_compile_options(-Wall -Wextra -pedantic -Werror)  # -Werror: treat warnings as errors
endif()

# Modular Architecture - Add subdirectories
add_subdirectory(backends/cpp/forge-runtime)  # sce_forge_runtime: headers generated C++ includes
add_subdirectory(sce)                 # Unified SCE library with integrated parsing

# The reference adapters of the Mesh https binding's port (docs/adr/0107, D2):
# options SCE_MESH_HTTPS_CURL and SCE_MESH_HTTPS_WINHTTP, targets
# sce_mesh_https_curl and sce_mesh_https_winhttp. After `sce`, which declares
# SCE_ENABLE_MESH, and after the warning options above, which they compile with.
include(cmake/SCEMeshHttps.cmake)
add_subdirectory(backends/c/runtime)       # C11 heapless runtime (RFC §5.J.1)

# Tools - TXML conversion (code generation is handled by sce-codegen Rust binary)
# TXML converter (Native builds only - requires filesystem operations)
if(NOT EMSCRIPTEN)
    add_subdirectory(tools/txml_converter)  # TXML to SCXML converter tool
endif()

# Test build option (enabled by default)
option(BUILD_TESTS "Build the tests" ON)
if(BUILD_TESTS)
    enable_testing()

    # §scxml-C-2-3: where the W3C BasicHTTP fixture listener answers.
    #
    # The PATH is read out of `tests/w3c/basic_http_test_endpoint.h` instead of
    # being restated here, because that header is what the compiled runners
    # include. Restating it would make CMake a second author of one fact, and
    # the two would drift the first time either moved.
    #
    # The PORT has no default (the header says why). The one listener this tree
    # starts for its tests, the C11 suite's Node fixture, binds the port
    # SCE_W3C_HTTP_FIXTURE_PORT names, and when it names none (the default) one
    # the kernel assigns, which the fixture writes into a port file its runners
    # are pointed at (backends/c/tests/CMakeLists.txt). No run of this tree
    # needs a fixed port free, so two checkouts, or another account's program on
    # the host, cannot collide with it. The in-process listeners of the C++
    # runners read SCE_W3C_HTTP_PORT from the run's environment, and bind one
    # the kernel assigns without it.
    #
    # SCE_W3C_HTTP_FIXTURE_PORT replaces the cache variable SCE_W3C_HTTP_PORT,
    # which held the header's old default: a tree configured before keeps that
    # value in its cache, and a renamed variable is what keeps the fixed port
    # from surviving there. The old entry is dropped, saying so.
    if(DEFINED CACHE{SCE_W3C_HTTP_PORT})
        message(STATUS
            "SCE_W3C_HTTP_PORT (${SCE_W3C_HTTP_PORT}) is no longer a cache variable: "
            "the C11 fixture listener takes a kernel-assigned port unless "
            "-DSCE_W3C_HTTP_FIXTURE_PORT=N names one. Dropping the old entry.")
        unset(SCE_W3C_HTTP_PORT CACHE)
    endif()
    set(SCE_W3C_HTTP_ENDPOINT_HEADER
        "${CMAKE_CURRENT_SOURCE_DIR}/tests/w3c/basic_http_test_endpoint.h")
    file(STRINGS "${SCE_W3C_HTTP_ENDPOINT_HEADER}" _sce_w3c_http_path_line
        REGEX "^#define[ \t]+SCE_W3C_HTTP_TEST_PATH[ \t]")
    string(REGEX MATCH "\"[^\"]+\"" _sce_w3c_http_default_path "${_sce_w3c_http_path_line}")
    string(REPLACE "\"" "" _sce_w3c_http_default_path "${_sce_w3c_http_default_path}")
    if(NOT _sce_w3c_http_default_path)
        message(FATAL_ERROR
            "${SCE_W3C_HTTP_ENDPOINT_HEADER} no longer declares "
            "SCE_W3C_HTTP_TEST_PATH. That header owns the BasicHTTP fixture "
            "endpoint for every channel; configuring on a guessed path would "
            "bind one address and publish another.")
    endif()
    set(SCE_W3C_HTTP_FIXTURE_PORT "" CACHE STRING
        "TCP port the C11 suite's W3C BasicHTTP fixture listener binds. Empty \
(the default) takes one the kernel assigns and hands it to the runners through \
a port file, so no run needs a fixed port free.")
    if(NOT SCE_W3C_HTTP_FIXTURE_PORT STREQUAL ""
       AND NOT SCE_W3C_HTTP_FIXTURE_PORT MATCHES "^[1-9][0-9]*$")
        message(FATAL_ERROR
            "SCE_W3C_HTTP_FIXTURE_PORT=\"${SCE_W3C_HTTP_FIXTURE_PORT}\" is not a TCP "
            "port; leave it empty for one the kernel assigns.")
    endif()
    set(SCE_W3C_HTTP_PATH "${_sce_w3c_http_default_path}" CACHE STRING
        "Path the W3C BasicHTTP fixture listener answers on. Read from \
tests/w3c/basic_http_test_endpoint.h; the documents address it through the \
published location, so this rarely needs overriding.")

    add_subdirectory(tests)
    # RFC §5.J.1 — C11 backend conformance harness, sibling of the
    # C++ tests/ tree. Lives at the repo root next to backends/c/runtime
    # so the C-only translation units stay isolated from C++ link.
    #
    # Not declared under Emscripten. What this project ships as WebAssembly is
    # the C++ engine (the visualizer), and the emscripten tree builds and runs
    # that engine's suites under Node (`cpp-suite-wasm`). The C11 harness holds
    # the generated C a host or a microcontroller compiles; no product of it is
    # WebAssembly, so a build of it with emcc would check a target nothing
    # ships. Every target the emscripten tree declares is built and run by a
    # gate, so a target with no WebAssembly purpose is not declared there.
    if(NOT EMSCRIPTEN)
        add_subdirectory(backends/c/tests)
    endif()
endif()

# Example build option (enabled by default)
#
# Not declared under Emscripten: each example is a native program that embeds
# the engine in a host process (its own threads, timers and terminal), which is
# what it demonstrates. The emscripten tree declares only what a gate builds
# and runs there (see backends/c/tests above).
option(BUILD_EXAMPLES "Build the examples" ON)
if(BUILD_EXAMPLES AND NOT EMSCRIPTEN)
    add_subdirectory(examples/cmake_function)
    add_subdirectory(examples/traffic_light)
    add_subdirectory(examples/smart_light)
    add_subdirectory(examples/timer)
    add_subdirectory(examples/async_dispatcher)
    add_subdirectory(examples/traffic_light_auto)
    add_subdirectory(examples/ai_loop)
    add_subdirectory(examples/model_harness)
endif()

# Python bindings option (disabled by default, excluded from WASM builds)
option(BUILD_PYTHON_BINDINGS "Build Python bindings via pybind11" OFF)
if(BUILD_PYTHON_BINDINGS AND NOT EMSCRIPTEN)
    find_package(Python3 REQUIRED COMPONENTS Interpreter Development)
    add_subdirectory(backends/python/bindings)
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
# Automatic deployment: Runs during build if web/visualizer/ doesn't exist
# Manual deployment: cmake --build build --target deploy-visualizer-local

# Automatic deployment strategy:
# EMSCRIPTEN builds always deploy WASM files after visualizer target builds
if(NOT EXISTS "${CMAKE_SOURCE_DIR}/web/visualizer")
    if(EMSCRIPTEN)
        message(STATUS "Visualizer directory not found, will create during EMSCRIPTEN build")
    else()
        message(STATUS "Visualizer deployment skipped: web/visualizer/ source directory not found")
    endif()
else()
    if(EMSCRIPTEN)
        message(STATUS "WASM files will be automatically deployed to web/visualizer/ after build")
    endif()
endif()

# Deployment target: Copy WASM build outputs to web/visualizer/ source directory
# web/visualizer/ contains both source files (tracked in git) and build outputs (gitignored)
set(VISUALIZER_DIR "${CMAKE_SOURCE_DIR}/web/visualizer")
set(VISUALIZER_WASM_FILE "${VISUALIZER_DIR}/visualizer.wasm")
set(VISUALIZER_JS_FILE "${VISUALIZER_DIR}/visualizer.js")

if(EMSCRIPTEN)
    # EMSCRIPTEN: Build WASM and copy directly to web/visualizer/
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
        COMMENT "Deploying WASM files to web/visualizer/"
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
#                           + sce_forge_runtime        (headers generated C++ includes)
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
#   Core        — sce_core + sce_base + sce_forge_runtime + public headers
#                 + SCEConfig entry
#   Scripting   — sce_scripting + qjs + lua54 (+ lua headers)
#   Runtime     — sce_runtime + txml-converter
#   Codegen — SCECodegen/SCEClangFormat + sce-codegen binary + templates
# Codegen stays separate because it is independent of tier selection —
# any consumer (base/scripting/runtime) that uses sce_add_state_machine()
# relies on the same codegen binary + Jinja2 templates.

# Core tier: sce_core (header-only) + sce_base (pure static AOT utilities)
# + sce_forge_runtime (header-only), which sce_core links INTERFACE and so
# must share its export set: the C++ the generator writes includes
# <sce/forge/algorithm.h>, <sce/forge/optional.h> and <sce/forge/span.h>, and
# a consumer reaches them by linking the tier it already links.
install(TARGETS sce_core sce_base sce_forge_runtime
    EXPORT SCE_CoreTargets
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR} COMPONENT Core
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR} COMPONENT Core
    ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR} COMPONENT Core
    INCLUDES DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}
)

# The Forge C++ runtime's headers, all of them and in every configuration:
# generated statechart code names them without the mesh, and the mesh headers
# (EventQueueBridge.h, ShmChannel.h) name its queue. They land under
# include/sce/forge, below the `${CMAKE_INSTALL_INCLUDEDIR}` that
# sce_forge_runtime's install interface exposes.
install(DIRECTORY backends/cpp/forge-runtime/include/sce
    DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}
    COMPONENT Core
    FILES_MATCHING PATTERN "*.h"
)

# Mesh transport (opt-in via SCE_ENABLE_MESH). sce_mesh_common is a
# C++-first static library per mesh_cpp_first_policy; it ships with the
# Core component so `--component Core` on a mesh-enabled build still gets
# everything required to link AOT mesh machines. Its CBOR is the generated
# envelope codec over the header-only forge runtime, compiled into the
# archive, so no CBOR library is installed beside it.
if(SCE_ENABLE_MESH)
    install(TARGETS sce_mesh_common
        EXPORT SCE_CoreTargets
        LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR} COMPONENT Core
        ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR} COMPONENT Core
        INCLUDES DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}
    )
    # The forge runtime's queue these headers name is installed with the rest
    # of that runtime, above, for every configuration.
    # The https adapters' headers (transports/https) stay out: their targets
    # are not installed (cmake/SCEMeshHttps.cmake), and a header without the
    # archive that defines it would be a declaration nothing links.
    install(DIRECTORY sce/include/mesh
        DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}
        COMPONENT Core
        FILES_MATCHING PATTERN "*.h"
        PATTERN "https" EXCLUDE
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
# `test` is the test profile's runtime (docs/SCE_TEST_PROFILE.md §12), header-only:
# a generated test component reaches it, so it ships with the code that does.
foreach(_sce_core_dir core static common backends wrappers dispatchers test)
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
        PATTERN "static"      EXCLUDE
        PATTERN "test"        EXCLUDE
        PATTERN "wrappers"    EXCLUDE
)

# Scripting: JSEngine / LuaEngine / ScriptEngineProvider, and the
# EventRaiser registry both engines route a script's raise through. The
# registry's code is compiled into sce_scripting and scripting/JSEngine.h
# includes its interface, so its headers ship with this component even
# though they live under events/.
install(DIRECTORY sce/include/scripting
    DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}
    COMPONENT Scripting
    FILES_MATCHING PATTERN "*.h"
)
set(SCE_SCRIPTING_EVENT_HEADERS
    IEventRaiserRegistry.h
    EventRaiserRegistry.h
    EventRaiserService.h
)
foreach(_sce_header IN LISTS SCE_SCRIPTING_EVENT_HEADERS)
    install(FILES sce/include/events/${_sce_header}
        DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}/events
        COMPONENT Scripting
    )
endforeach()

# Runtime: Interpreter/state-machine/parser/events headers, less the
# events/ headers the Scripting component ships.
set(_sce_runtime_header_excludes)
foreach(_sce_header IN LISTS SCE_SCRIPTING_EVENT_HEADERS)
    list(APPEND _sce_runtime_header_excludes PATTERN "${_sce_header}" EXCLUDE)
endforeach()
foreach(_sce_runtime_dir model runtime actions events parsing factory)
    install(DIRECTORY sce/include/${_sce_runtime_dir}
        DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}
        COMPONENT Runtime
        FILES_MATCHING PATTERN "*.h"
        ${_sce_runtime_header_excludes}
    )
endforeach()

# Codegen-component assets (cmake utilities + templates) are declared in
# sce/sce_codegen_assets.cmake — the single source of truth shared with
# scripts/package_embed.sh. Adding a file to the Codegen component means
# editing only the SSOT; both install() rules and the embed packager
# pick it up automatically.
include(${CMAKE_CURRENT_SOURCE_DIR}/sce/sce_codegen_assets.cmake)

# License-text distribution is declared in sce/sce_licenses.cmake — the
# single source of truth shared with scripts/package_embed.sh. AGPL-3.0
# section 4 (SCE itself) and MIT section 1 (every vendored third_party dep with a
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

# Install the sce-codegen binary this build used.
#
# `SCE_CODEGEN` is already resolved — `cmake/SCECodegen.cmake` runs the
# ecosystem locator near the top of this file, and that locator has both
# checked the binary against its own sources and put the profiles in the
# order that keeps a stale one from outranking a fresh one.
#
# A second `find_program` used to stand here with the profiles reversed,
# which meant `cmake --install` could ship a release binary this build
# never used while the build itself ran on debug. Nothing reported the
# difference: both steps succeeded, and the installed generator was simply
# not the one anything had verified.
install(PROGRAMS "${SCE_CODEGEN}"
    DESTINATION ${CMAKE_INSTALL_BINDIR}
    COMPONENT Codegen
)

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
# Derived from the resolved binary rather than named, so a build-tree
# consumer is pointed at the generator this build actually used. Named, it
# said `target/release` — a directory nothing in this repository produces
# any more, so `find_package(SCE)` against a build tree handed consumers a
# path that does not exist.
get_filename_component(CODEGEN_BIN_DIR "${SCE_CODEGEN}" DIRECTORY)
set(SCE_HTTPLIB_FIND_MODULES_DIR "${CMAKE_CURRENT_SOURCE_DIR}/third_party/cpp-httplib/cmake")
# The lowering staticlib sce_scripting links (cmake/SCEBuildLowering.cmake): where cargo put it
# in this build; empty when sce_scripting does not link it (no scripting tier).
if(TARGET SCE::Lowering)
    get_target_property(LOWERING_LIB_PATH SCE::Lowering IMPORTED_LOCATION)
else()
    set(LOWERING_LIB_PATH "")
endif()
configure_package_config_file(
    "${CMAKE_CURRENT_SOURCE_DIR}/cmake/SCEConfig.cmake.in"
    "${CMAKE_CURRENT_BINARY_DIR}/SCEConfig.cmake"
    INSTALL_DESTINATION "${CMAKE_CURRENT_BINARY_DIR}"
    PATH_VARS INCLUDE_INSTALL_DIR CODEGEN_INSTALL_DIR CODEGEN_BIN_DIR LOWERING_LIB_PATH
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
# can include() it from the same directory as SCEConfig.cmake, with every
# module it includes from its own directory beside it: the list the install
# ships (sce/sce_codegen_assets.cmake), so the two cannot name different files.
foreach(_sce_codegen_cmake IN LISTS SCE_CODEGEN_CMAKE_FILES)
    get_filename_component(_sce_codegen_cmake_name "${_sce_codegen_cmake}" NAME)
    configure_file(
        "${CMAKE_CURRENT_SOURCE_DIR}/${_sce_codegen_cmake}"
        "${CMAKE_CURRENT_BINARY_DIR}/${_sce_codegen_cmake_name}"
        COPYONLY
    )
endforeach()

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
# The lowering staticlib is a build product, not a target install(TARGETS) can export: it is
# installed as a file with the Scripting tier, and the installed config names it as the
# imported target SCE::Lowering that sce_scripting's exported link interface asks for.
if(TARGET SCE::Lowering)
    get_target_property(_sce_lowering_built SCE::Lowering IMPORTED_LOCATION)
    get_filename_component(_sce_lowering_file "${_sce_lowering_built}" NAME)
    set(LOWERING_LIB_PATH "${CMAKE_INSTALL_LIBDIR}/${_sce_lowering_file}")
    install(FILES "${_sce_lowering_built}"
        DESTINATION ${CMAKE_INSTALL_LIBDIR}
        COMPONENT Scripting
    )
else()
    set(LOWERING_LIB_PATH "")
endif()
configure_package_config_file(
    "${CMAKE_CURRENT_SOURCE_DIR}/cmake/SCEConfig.cmake.in"
    "${CMAKE_CURRENT_BINARY_DIR}/_install/SCEConfig.cmake"
    INSTALL_DESTINATION ${SCE_INSTALL_CMAKEDIR}
    PATH_VARS INCLUDE_INSTALL_DIR CODEGEN_INSTALL_DIR CODEGEN_BIN_DIR LOWERING_LIB_PATH
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
# SCECodegen.cmake and the modules it includes (SCECodegenPlan.cmake,
# SCEFindCodegen.cmake, SCEClangFormat.cmake) — file list in
# sce/sce_codegen_assets.cmake (SSOT shared with package_embed.sh).
foreach(_sce_codegen_cmake IN LISTS SCE_CODEGEN_CMAKE_FILES)
    install(FILES "${CMAKE_CURRENT_SOURCE_DIR}/${_sce_codegen_cmake}"
        DESTINATION ${SCE_INSTALL_CMAKEDIR}
        COMPONENT Codegen
    )
endforeach()
