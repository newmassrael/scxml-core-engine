# W3C SCXML Compliance Programs - Native and WASM
#
# The W3C runner (w3c_test_cli) and the interactive suite
# (comprehensive_interactive_test), with the registry and the AOT machines they
# are built from. Included by tests/CMakeLists.txt in every tree: natively, in
# the emscripten tree with threads (the threaded engine, as natively), and in
# the one without threads (build_wasm_web, the host-driven engine the browser
# module runs), which declares nothing else but the browser module. Both
# programs run on both engines:
# - Interpreter Engine: Runtime SCXML parsing and execution
# - AOT Engine: Ahead-of-Time compiled static state machines
#
# Read with tests/ as the current directory (an include, not a subdirectory),
# so every CMAKE_CURRENT_SOURCE_DIR and CMAKE_CURRENT_BINARY_DIR below is the
# tests directory's.
#
# The fixtures w3c_test_cli runs, and the machines it compiles, come from one
# place under every toolchain: the conformance registry, read below, and the
# generator's plan of each fixture, generated into this build tree. An
# Emscripten build is a cross build like any other: `sce-codegen` is a host
# program, found as one (cmake/SCEFindCodegen.cmake), run by `execute_process`
# when the tree is configured and by `add_custom_command` when it is built,
# exactly as natively, and the C++ it writes does not depend on the target.
# The WASM configure used to read the set and the machines out of a native
# build tree that had to be named `build`, inside the source tree, and be
# configured and built first; without one it went on with an empty set.
#
# Output directory for generated static W3C test code (used by w3c_test_cli)
set(STATIC_W3C_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/w3c_static_generated")

# Generate C++ code for W3C tests BEFORE defining executables
include(${CMAKE_SOURCE_DIR}/cmake/SCEStaticW3CTest.cmake)

# Initialize GENERATED_W3C_HEADERS and W3C_AOT_TESTS before any sce_generate_static_w3c_test calls
# W3C Test Registration Automation: W3C_AOT_TESTS is auto-accumulated by sce_generate_static_w3c_test()
# Adding a new test = adding ONE entry to tests/w3c/conformance/fixtures.json,
# which the loop below turns into a call (TestXXX.h + registry auto-generated)
set(GENERATED_W3C_HEADERS "")
set(W3C_AOT_TESTS "")

# ── W3C test registrations, derived from the conformance registry ──
#
# tests/w3c/conformance/fixtures.json is the source of truth for which
# upstream tests this repository runs and which harness each one needs.
# These calls used to be 202 hand-written lines here, and the generator
# read them back out with a regex — CMake was the registry, which meant
# a checkout without CMake could not enumerate the suite at all.
#
# The ids, in registry order, and the harness of each are read through
# `sce_w3c_registry_fixtures` (cmake/SCEW3CRegistry.cmake), the one reading
# of the registry the C11 route (backends/c/tests/CMakeLists.txt) shares, so
# there is no JSON parsing in CMake and no second copy of the set to keep in
# step; the configure depends on the registry, so adding a test needs no
# manual cmake invocation. `sce-build/tests/w3c_registry_drives_cmake.rs`
# pins that what is scheduled here is what the registry declares, and that
# it is read and generated under the conditions w3c_test_cli is declared
# under, so a WASM build runs the set a native one does.
sce_w3c_registry_fixtures(_W3C_ALL_IDS _W3C_HARNESS_OF_)

# Every fixture's plan, asked of the generator in one batch before the loop
# declares the fixtures one by one, each from the answer the batch kept
# (cmake/SCECodegenPlan.cmake): asked one at a time, the plans of the corpus
# were most of what this configure cost.
sce_prefetch_static_w3c_tests("${STATIC_W3C_OUTPUT_DIR}" cpp ${_W3C_ALL_IDS})

foreach(_W3C_ID ${_W3C_ALL_IDS})
    set(_W3C_HARNESS "${_W3C_HARNESS_OF_${_W3C_ID}}")
    if(_W3C_HARNESS STREQUAL "simple")
        sce_generate_static_w3c_test(${_W3C_ID} ${STATIC_W3C_OUTPUT_DIR})
    elseif(_W3C_HARNESS STREQUAL "scheduled" OR _W3C_HARNESS STREQUAL "http")
        string(TOUPPER "${_W3C_HARNESS}" _W3C_TYPE)
        sce_generate_static_w3c_test(${_W3C_ID} ${STATIC_W3C_OUTPUT_DIR} TYPE ${_W3C_TYPE})
    else()
        message(FATAL_ERROR
            "test ${_W3C_ID} names the harness `${_W3C_HARNESS}`, which the C++ route "
            "does not run: map it to the runner it needs here")
    endif()
endforeach()

# ============================================================================
# W3C Test Registration Automation
# ============================================================================
#
# SINGLE SOURCE OF TRUTH: tests/w3c/conformance/fixtures.json. The loop
# above turns it into sce_generate_static_w3c_test() calls, and
# W3C_AOT_TESTS is auto-accumulated from those (Zero Duplication).
#
# WORKFLOW - Adding a new AOT test (ONE step):
#   Add to the registry: {"id": "XXX", "harness": "simple"}
#   Harness: `simple`, `scheduled` (delayed send/invoke) or `http` (BasicHTTP)
#   Everything else is automatic:
#   - TestXXX.h auto-generated from metadata.txt template (if not exists)
#   - W3C_AOT_TESTS auto-accumulated (no manual list to maintain)
#   - W3CTestRunner_TestXXX.cpp auto-generated
#   - W3CTestRegistry.cpp auto-generated
#   - web/visualizer/test-list.js auto-generated
#
# CUSTOM TESTS: For tests needing PASS_STATE override, custom timeouts, etc.,
#   create tests/w3c/aot_tests/TestXXX.h manually. Auto-generation skips existing files.
#
# FILES GENERATED:
# - <build>/tests/w3c/W3CTestRegistry.cpp (from W3CTestRegistry.cpp.in template)
# - tests/w3c/runners/W3CTestRunner_Test*.cpp (CMake auto-generates)
# - tests/w3c/aot_tests/TestXXX.h (CMake auto-generates if not exists)
# ===========================================================================
message(STATUS "W3C AOT Tests auto-accumulated: ${W3C_AOT_TESTS}")

# ============================================================================
# Auto-generate web/visualizer/test-list.js from W3C_AOT_TESTS
# ============================================================================
# Zero Duplication: the conformance registry is the single source, and
# `generate_test_list.py` reads it directly.
#
# It was handed a CMake file instead, the list of ids this tree used to
# accumulate, which the script stopped understanding when the
# registry became the source of truth. It parses its input as JSON, so a
# CMake file fails on the first byte:
#
#   Error extracting tests: Expecting value: line 1 column 1 (char 0)
#
# The output is committed and only rebuilds when an input is newer, so the
# broken call sat latent until something touched the tree hard enough to
# invalidate it, and then failed the whole build.
find_package(Python3 COMPONENTS Interpreter)

if(Python3_FOUND AND NOT EMSCRIPTEN)
    add_custom_command(
        OUTPUT ${CMAKE_SOURCE_DIR}/web/visualizer/test-list.js
        COMMAND ${Python3_EXECUTABLE}
                ${CMAKE_SOURCE_DIR}/tools/generate_test_list.py
                ${SCE_W3C_REGISTRY}
                ${CMAKE_SOURCE_DIR}/web/visualizer/test-list.js
        DEPENDS ${SCE_W3C_REGISTRY}
                ${CMAKE_SOURCE_DIR}/tools/generate_test_list.py
        COMMENT "Generating web/visualizer/test-list.js from the conformance registry"
        VERBATIM
    )

    add_custom_target(generate_test_list ALL
        DEPENDS ${CMAKE_SOURCE_DIR}/web/visualizer/test-list.js
    )

    message(STATUS "Visualizer test list will be auto-generated from W3C_AOT_TESTS")
elseif(NOT Python3_FOUND)
    message(WARNING "Python3 not found - web/visualizer/test-list.js will not be auto-generated")
    message(WARNING "Run manually: python3 tools/generate_test_list.py")
endif()

# Interpreter-Only Tests: Dynamic features requiring runtime SCXML parsing
set(W3C_INTERPRETER_ONLY_TESTS
)

# Generate W3CTestRegistry.cpp from template
#
# With each fixture's harness as the registry names it
# (`_W3C_HARNESS_OF_<id>`, read by `sce_w3c_registry_fixtures` above), so a
# runner asks the generated registry which fixtures need the BasicHTTP endpoint
# instead of keeping a list of its own, and the largest test number a range
# runs to. Every harness the registration loop accepts is a `TestHarness`
# enumerator of the same name (tests/w3c/W3CTestRegistry.h), so a harness the
# header does not know fails to compile rather than reading as another one.
set(W3C_AOT_TEST_REGISTRATIONS "")
set(W3C_HARNESS_REGISTRATIONS "")
set(W3C_HTTP_TEST_IDS "")
set(W3C_LARGEST_TEST_NUMBER 0)
foreach(TEST_NUM ${W3C_AOT_TESTS})
    string(APPEND W3C_AOT_TEST_REGISTRATIONS "        {\"${TEST_NUM}\", TestEngine::AOT},\n")
    string(TOUPPER "${_W3C_HARNESS_OF_${TEST_NUM}}" _W3C_HARNESS_ENUMERATOR)
    string(APPEND W3C_HARNESS_REGISTRATIONS
        "        {\"${TEST_NUM}\", TestHarness::${_W3C_HARNESS_ENUMERATOR}},\n")
    if("${_W3C_HARNESS_OF_${TEST_NUM}}" STREQUAL "http")
        list(APPEND W3C_HTTP_TEST_IDS "${TEST_NUM}")
    endif()
    # A variant (`403a`) runs under its test's number.
    string(REGEX MATCH "^[0-9]+" _W3C_TEST_NUMBER "${TEST_NUM}")
    if(_W3C_TEST_NUMBER GREATER W3C_LARGEST_TEST_NUMBER)
        set(W3C_LARGEST_TEST_NUMBER ${_W3C_TEST_NUMBER})
    endif()
endforeach()

set(W3C_INTERPRETER_TEST_REGISTRATIONS "")
foreach(TEST_NUM ${W3C_INTERPRETER_ONLY_TESTS})
    string(APPEND W3C_INTERPRETER_TEST_REGISTRATIONS "        {\"${TEST_NUM}\", TestEngine::INTERPRETER},\n")
endforeach()

configure_file(
    ${CMAKE_CURRENT_SOURCE_DIR}/w3c/W3CTestRegistry.cpp.in
    ${CMAKE_CURRENT_BINARY_DIR}/w3c/W3CTestRegistry.cpp
    @ONLY
)

# WASM: the same two facts for w3c/wasm_polyfill.js, which decides before
# main() runs whether to start the BasicHTTP endpoint and so cannot ask the
# compiled registry, and where the checkout is. Generated into this build tree
# from the registration above and handed to every target that runs the
# polyfill as the `--pre-js` before it; written only when it changed, so a
# reconfigure relinks nothing.
#
# The checkout is stated rather than guessed. The polyfill read it as the
# working directory's grandparent, which held for one build directory name at
# one depth, and it required its Node modules (the endpoint reader, the fixture
# server) beside the linked program, where only the server was copied: the
# endpoint reader was missing and the server never started. They are read
# where they are kept, in w3c/.
if(EMSCRIPTEN)
    set(W3C_REGISTRY_PRE_JS "${CMAKE_CURRENT_BINARY_DIR}/w3c/w3c_registry_pre.js")
    list(JOIN W3C_HTTP_TEST_IDS "', '" _W3C_HTTP_TEST_IDS_JS)
    sce_codegen_write_input("${W3C_REGISTRY_PRE_JS}"
"// Generated by tests/w3c/W3CPrograms.cmake from tests/w3c/conformance/fixtures.json. Do not edit.
// The fixtures the registry runs under its `http` harness, and the largest test number.
var SCE_W3C_HTTP_TEST_IDS = ['${_W3C_HTTP_TEST_IDS_JS}'];
var SCE_W3C_LARGEST_TEST_NUMBER = ${W3C_LARGEST_TEST_NUMBER};
// The Node package of the harness.
var SCE_W3C_HARNESS_DIR = '${CMAKE_CURRENT_SOURCE_DIR}/w3c';
")
endif()

# Auto-generate AOT test runner files
set(W3C_AOT_RUNNER_SOURCES "")
foreach(TEST_NUM ${W3C_AOT_TESTS})
    set(RUNNER_FILE "${CMAKE_CURRENT_SOURCE_DIR}/w3c/runners/W3CTestRunner_Test${TEST_NUM}.cpp")
    # Kept when unchanged, so a reconfigure recompiles no runner.
    sce_codegen_write_input("${RUNNER_FILE}"
"// Auto-generated runner file for W3C SCXML Test ${TEST_NUM}
#include \"aot_tests/Test${TEST_NUM}.h\"
")
    list(APPEND W3C_AOT_RUNNER_SOURCES "w3c/runners/W3CTestRunner_Test${TEST_NUM}.cpp")
endforeach()

# ============================================================================
# W3C Test CLI Target (Both Native and WASM)
# ============================================================================
# Full W3C Test Runner for all W3C SCXML 1.0 compliance tests
# Built for both Native and WASM, from the one set and the one generation above
set(W3C_TEST_CLI_SOURCES
    w3c/W3CTestCLI.cpp
    w3c/W3CTestRunner.cpp
    w3c/ManualVerification.cpp  # The verdicts of the manual fixtures
    ${CMAKE_CURRENT_BINARY_DIR}/w3c/W3CTestRegistry.cpp  # Auto-generated test registry
    ${W3C_AOT_RUNNER_SOURCES}  # Explicit AOT test runners (from W3C_AOT_TESTS list)
    w3c/impl/TXMLConverter.cpp
    w3c/impl/TestMetadataParser.cpp
    w3c/impl/W3CTestEventDispatcher.cpp
    mocks/MockEventRaiser.cpp
    ${GENERATED_W3C_HEADERS}  # Trigger generation of static test headers for AOT engine
)

# Native-only: HTTP server for BasicHTTP Event I/O Processor tests (W3C SCXML C.2).
# It is a cpp-httplib server on its own threads, and cpp-httplib is not built
# under Emscripten (sce/CMakeLists.txt); a WASM run gets the same endpoint from
# w3c/standalone_http_server.js, which the pre-js below spawns under Node.
if(NOT EMSCRIPTEN)
    list(APPEND W3C_TEST_CLI_SOURCES w3c/W3CHttpTestServer.cpp)
endif()

add_executable(w3c_test_cli ${W3C_TEST_CLI_SOURCES})

target_link_libraries(w3c_test_cli
    PRIVATE
        sce_runtime
)

if(EMSCRIPTEN)
    # WASM: Use pre-js for XMLHttpRequest polyfill, NODEFS mount, and HTTP server auto-management
    set(POLYFILL_PRE_JS "${CMAKE_CURRENT_SOURCE_DIR}/w3c/wasm_polyfill.js")

    # WASM: what the runner adds to the shape apply_sce_test_policies gives
    # every test program (Node, NODERAWFS, the pool, ASYNCIFY, the heap and
    # stack); the fetch client's -sFETCH comes from sce_runtime.
    target_link_options(w3c_test_cli PRIVATE
        -sALLOW_TABLE_GROWTH=1          # Allow function table to grow dynamically (required for QuickJS JS_NewCFunction)
        -O3                             # Optimization required for reasonable ASYNCIFY binary size
        # Each `--pre-js` is one `SHELL:` option with its file: CMake
        # de-duplicates repeated link options, so two bare `--pre-js` flags
        # become one with both files after it, and wasm-ld reads the second
        # file as an input.
        "SHELL:--pre-js ${W3C_REGISTRY_PRE_JS}"  # The registry's BasicHTTP fixtures, read by the polyfill
        "SHELL:--pre-js ${POLYFILL_PRE_JS}"      # The Node host: the BasicHTTP fixture server
        # The polyfill holds main() until the fixture server is ready, through
        # runtime symbols a pre-js has to name to the link.
        "-sEXPORTED_RUNTIME_METHODS=[addRunDependency,removeRunDependency]"
        "-sEXPORTED_FUNCTIONS=['_main','_dom_getElementsByTagName_wrapper','_dom_getAttribute_wrapper']"  # Export main + DOM wrapper functions
    )
    # A pre-js is read at link time, which does not otherwise depend on it.
    set_property(TARGET w3c_test_cli APPEND PROPERTY
        LINK_DEPENDS "${W3C_REGISTRY_PRE_JS}" "${POLYFILL_PRE_JS}")
endif()

# Native: Explicitly link pthread
if(NOT EMSCRIPTEN)
    target_link_libraries(w3c_test_cli PRIVATE pthread)
endif()



target_include_directories(w3c_test_cli
    PRIVATE
        ${CMAKE_SOURCE_DIR}
        ${CMAKE_SOURCE_DIR}/sce/include
        ${CMAKE_SOURCE_DIR}/sce/src
        ${CMAKE_SOURCE_DIR}/tests
        ${CMAKE_SOURCE_DIR}/tests/common  # TestSummaryHelper.h
        ${CMAKE_SOURCE_DIR}/tests/w3c  # AOT test registry headers
        ${CMAKE_CURRENT_BINARY_DIR}/w3c  # Generated W3CTestRegistry.cpp
        ${STATIC_W3C_OUTPUT_DIR}  # Include generated static test headers
)



add_dependencies(w3c_test_cli sce_runtime)

# Applied here rather than through SCE_TEST_TARGETS: the tree without threads
# includes this file and returns before that list is read.
apply_sce_test_policies(w3c_test_cli)

# The brief of an AOT test header states no spec section.
#
# The section a fixture targets has one home, `specnum` in
# resources/<id>/metadata.txt, and a brief restating it is a second answer:
# measured 2026-09-15, 175 of 202 briefs stated one and 82 contradicted
# `specnum`. `sce-codegen check-aot-briefs` reads the brief paragraph only, so
# the citations in a header's body — the other sections a test touches — are
# left alone. It is a build step rather than a configure step so that editing
# a header re-runs it, and w3c_test_cli depends on it, so a brief stating a
# section fails the build that compiles that header.
set(W3C_AOT_HEADERS "")
foreach(TEST_NUM ${W3C_AOT_TESTS})
    list(APPEND W3C_AOT_HEADERS "${CMAKE_CURRENT_SOURCE_DIR}/w3c/aot_tests/Test${TEST_NUM}.h")
endforeach()
set(W3C_AOT_BRIEF_STAMP "${CMAKE_CURRENT_BINARY_DIR}/w3c/aot_briefs.checked")
add_custom_command(
    OUTPUT "${W3C_AOT_BRIEF_STAMP}"
    COMMAND "${SCE_CODEGEN}" check-aot-briefs
            --manifest "${SCE_W3C_REGISTRY}"
            --header-dir "${CMAKE_CURRENT_SOURCE_DIR}/w3c/aot_tests"
    COMMAND "${CMAKE_COMMAND}" -E touch "${W3C_AOT_BRIEF_STAMP}"
    DEPENDS "${SCE_W3C_REGISTRY}" "${SCE_CODEGEN}" ${W3C_AOT_HEADERS}
    COMMENT "Checking that no AOT test header brief states its fixture's spec section"
    VERBATIM
)
add_custom_target(w3c_aot_brief_check DEPENDS "${W3C_AOT_BRIEF_STAMP}")
add_dependencies(w3c_test_cli w3c_aot_brief_check)

# ============================================================================
# Comprehensive Interactive Debugging Verification Test
# ============================================================================
# Automated time-travel debugging verification for ALL W3C SCXML tests
# Verifies step forward, step back, and reset operations maintain perfect
# state consistency across all W3C SCXML 3.13 microstep semantics
add_executable(comprehensive_interactive_test
    w3c/ComprehensiveInteractiveTest.cpp
    w3c/InteractiveRestoreFidelityTest.cpp
    w3c/InteractiveTestRunner.cpp
    w3c/impl/SnapshotComparator.cpp
    w3c/impl/TXMLConverter.cpp
    w3c/impl/TestMetadataParser.cpp
    # The registry the W3C runner asks, for the fixtures that need the
    # BasicHTTP endpoint (left out here: their timing is the network's).
    ${CMAKE_CURRENT_BINARY_DIR}/w3c/W3CTestRegistry.cpp
)

target_link_libraries(comprehensive_interactive_test
    PRIVATE
        sce_runtime
        GTest::gtest
        spdlog::spdlog
)

target_include_directories(comprehensive_interactive_test
    PRIVATE
        ${CMAKE_SOURCE_DIR}
        ${CMAKE_SOURCE_DIR}/sce/include
        ${CMAKE_SOURCE_DIR}/sce/src
        ${CMAKE_SOURCE_DIR}/tests
        ${CMAKE_SOURCE_DIR}/tests/w3c
)

# Set project root path for resource discovery
target_compile_definitions(comprehensive_interactive_test
    PRIVATE
        SCE_PROJECT_ROOT="${CMAKE_SOURCE_DIR}"
)

# Native: Explicitly link pthread
if(NOT EMSCRIPTEN)
    target_link_libraries(comprehensive_interactive_test PRIVATE pthread)
endif()

# WASM: Add Emscripten embind support for JavaScript interop
if(EMSCRIPTEN)
    # The same Node host as w3c_test_cli: the BasicHTTP fixture server.
    set(POLYFILL_PRE_JS "${CMAKE_CURRENT_SOURCE_DIR}/w3c/wasm_polyfill.js")

    # What the interactive suite adds to the shape apply_sce_test_policies
    # gives every test program.
    target_link_options(comprehensive_interactive_test PRIVATE
        --bind                          # Enable Emscripten embind for val objects
        # One `SHELL:` option per pre-js, as w3c_test_cli's above says why.
        "SHELL:--pre-js ${W3C_REGISTRY_PRE_JS}"  # The registry's BasicHTTP fixtures, read by the polyfill
        "SHELL:--pre-js ${POLYFILL_PRE_JS}"      # The Node host: the BasicHTTP fixture server
        "-sEXPORTED_RUNTIME_METHODS=[addRunDependency,removeRunDependency]"
    )
    set_property(TARGET comprehensive_interactive_test APPEND PROPERTY
        LINK_DEPENDS "${W3C_REGISTRY_PRE_JS}" "${POLYFILL_PRE_JS}")
endif()

apply_sce_test_policies(comprehensive_interactive_test)

add_dependencies(comprehensive_interactive_test sce_runtime)
