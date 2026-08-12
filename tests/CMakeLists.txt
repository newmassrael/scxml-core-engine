cmake_minimum_required(VERSION 3.14)

# GoogleTest setup - FetchContent approach
find_package(GTest QUIET)

if(NOT GTest_FOUND)
    message(STATUS "GTest not found on system, using FetchContent to download")

    include(FetchContent)
    FetchContent_Declare(
        googletest
        GIT_REPOSITORY https://github.com/google/googletest.git
        GIT_TAG v1.14.0
        GIT_SHALLOW TRUE
        GIT_PROGRESS TRUE
    )

    # GoogleTest options (reduce build time)
    set(gtest_force_shared_crt ON CACHE BOOL "" FORCE)
    set(BUILD_GMOCK ON CACHE BOOL "" FORCE)
    set(BUILD_GTEST ON CACHE BOOL "" FORCE)
    set(INSTALL_GTEST OFF CACHE BOOL "" FORCE)

    FetchContent_MakeAvailable(googletest)

    # GoogleTest: Disable warnings-as-errors for external library builds
    # This prevents GoogleTest internal warnings (e.g., char8_t conversion) from failing the build
    target_compile_options(gtest PRIVATE -Wno-error -Wno-character-conversion)
    target_compile_options(gtest_main PRIVATE -Wno-error -Wno-character-conversion)
    target_compile_options(gmock PRIVATE -Wno-error -Wno-character-conversion)
    target_compile_options(gmock_main PRIVATE -Wno-error -Wno-character-conversion)

    # Treat GoogleTest as system library for downstream consumers
    get_target_property(GTEST_INCLUDE_DIRS gtest INTERFACE_INCLUDE_DIRECTORIES)
    set_target_properties(gtest PROPERTIES INTERFACE_SYSTEM_INCLUDE_DIRECTORIES "${GTEST_INCLUDE_DIRS}")

    get_target_property(GTEST_MAIN_INCLUDE_DIRS gtest_main INTERFACE_INCLUDE_DIRECTORIES)
    set_target_properties(gtest_main PROPERTIES INTERFACE_SYSTEM_INCLUDE_DIRECTORIES "${GTEST_MAIN_INCLUDE_DIRS}")

    get_target_property(GMOCK_INCLUDE_DIRS gmock INTERFACE_INCLUDE_DIRECTORIES)
    set_target_properties(gmock PROPERTIES INTERFACE_SYSTEM_INCLUDE_DIRECTORIES "${GMOCK_INCLUDE_DIRS}")

    get_target_property(GMOCK_MAIN_INCLUDE_DIRS gmock_main INTERFACE_INCLUDE_DIRECTORIES)
    set_target_properties(gmock_main PROPERTIES INTERFACE_SYSTEM_INCLUDE_DIRECTORIES "${GMOCK_MAIN_INCLUDE_DIRS}")

    # WASM builds: Apply pthread flags
    if(EMSCRIPTEN)
        target_compile_options(gtest PRIVATE -pthread -sUSE_PTHREADS=1)
        target_compile_options(gtest_main PRIVATE -pthread -sUSE_PTHREADS=1)
        target_compile_options(gmock PRIVATE -pthread -sUSE_PTHREADS=1)
        target_compile_options(gmock_main PRIVATE -pthread -sUSE_PTHREADS=1)

        target_link_options(gtest PRIVATE -pthread -sUSE_PTHREADS=1)
        target_link_options(gtest_main PRIVATE -pthread -sUSE_PTHREADS=1)
        target_link_options(gmock PRIVATE -pthread -sUSE_PTHREADS=1)
        target_link_options(gmock_main PRIVATE -pthread -sUSE_PTHREADS=1)
    endif()

    # Create alias (provides same interface as find_package)
    add_library(GTest::GTest ALIAS gtest)
    add_library(GTest::Main ALIAS gtest_main)
    add_library(GTest::gtest ALIAS gtest)
    add_library(GTest::gtest_main ALIAS gtest_main)
    add_library(GTest::gmock ALIAS gmock)
    add_library(GTest::gmock_main ALIAS gmock_main)
else()
    message(STATUS "Using system-installed GTest")
endif()

# ============================================================================
# C++17 ceiling for targets that advertise C++17
# ============================================================================
# `target_compile_features(<t> PRIVATE cxx_std_17)` states a MINIMUM. With
# the project's global `CMAKE_CXX_STANDARD 20` every such target actually
# compiles at `-std=c++20`, so the declaration is inert: a C++20-only
# construct in generated mesh output builds green in-tree and fails only
# in a downstream tree honouring the contract `sce_mesh_common` publishes
# (`cxx_std_17` PUBLIC, per SCE_MESH.md §10.4).
#
# This makes the declaration mean what it reads as. Use it on any target
# whose job is to prove generated code compiles at the advertised
# standard; `sce_verify_cxx17_ceilings` below fails configuration if a
# compile-only mesh target is added without it.
function(sce_pin_cxx17 target_name)
    set_target_properties(${target_name} PROPERTIES
        CXX_STANDARD 17
        CXX_STANDARD_REQUIRED ON
        CXX_EXTENSIONS OFF)
endfunction()

# ============================================================================
# SCE test configuration: Disable std::cout usage
# ============================================================================
function(apply_sce_test_policies target_name)
    # Force include DisableStdOut.h to all test targets
    target_compile_options(${target_name}
        PRIVATE
            -include ${CMAKE_SOURCE_DIR}/sce/include/common/DisableStdOut.h
    )

    # Expose the tests/ root so any target may include shared test-only
    # headers (e.g. common/TestUtils.h, common/TestScriptEngine.h).
    target_include_directories(${target_name}
        PRIVATE
            ${CMAKE_SOURCE_DIR}/tests
    )

    # SCE test policy definitions
    target_compile_definitions(${target_name}
        PRIVATE
            SCE_TEST_TARGET=1  # For test target identification
    )

    # WASM: Apply pthread configuration to test executables
    if(EMSCRIPTEN)
        target_compile_options(${target_name} PRIVATE
            -pthread
            -sUSE_PTHREADS=1
        )
        target_link_options(${target_name} PRIVATE
            -pthread
            -sUSE_PTHREADS=1
            -sPTHREAD_POOL_SIZE=4
            -sEXIT_RUNTIME=1  # Exit immediately after main()
            -sENVIRONMENT=node,web
            -sASYNCIFY
            -sASSERTIONS=2
            -sINITIAL_MEMORY=${WASM_INITIAL_MEMORY}  # Centrally managed in root CMakeLists.txt
            -sSTACK_SIZE=33554432
            -sDISABLE_EXCEPTION_CATCHING=0  # Enable C++ exception catching for tests
            -g1                             # Line number info only (allows full optimization)
            --profiling-funcs
        )
    endif()
endfunction()

# ============================================================================
# Performance Benchmarks (Optional)
# ============================================================================
# List of all test target names
set(SCE_TEST_TARGETS
    sce_scxml_tests
    js_engine_tests
    state_machine_tests
    states_tests
    action_tests
    history_tests
    http_bridge_tests
    core_parser_tests
    model_tests
    integration_tests
    concurrency_tests
    txml_converter_tests
    w3c_test_cli
    w3c_full_runner
    w3c_simple_tests
)

# ============================================================================
# 1. SCE SCXML Builtin Tests (currently working tests)
# ============================================================================
add_executable(sce_scxml_tests
    simple_scxml_test.cpp
    core/TypeRegistryTest.cpp
    core/InvokeHelperTest.cpp
    concurrency/TypeRegistryThreadSafetyTest.cpp
    common/EventDataHelperTest.cpp
    common/TestSummaryExitStatusTest.cpp
    common/UuidTest.cpp
)

target_include_directories(sce_scxml_tests
    PRIVATE
        ${CMAKE_SOURCE_DIR}
        ${CMAKE_SOURCE_DIR}/sce/include
        ${CMAKE_SOURCE_DIR}/tests
        ${CMAKE_SOURCE_DIR}/tests/w3c  # TestSummaryHelper.h reaches interfaces/
)

target_link_libraries(sce_scxml_tests
    PRIVATE
        sce_runtime
        GTest::gtest_main
        pthread
)

# Apply SCE test policy (disable cout, etc.)
apply_sce_test_policies(sce_scxml_tests)

# CTest registration is handled below in optimized order

# ============================================================================
# 2. JavaScript Engine Tests (fully updated to SCE)
# ============================================================================
add_executable(js_engine_tests
    engine/SessionManagementTest.cpp
    engine/DataModelTest.cpp
    engine/EventSystemTest.cpp
    engine/IOProcessorsTest.cpp
    engine/JSEngineBasicTest.cpp
    engine/JSResultTypeConversionTest.cpp
    engine/ECMAScriptComplianceTest.cpp
    mocks/MockEventRaiser.cpp
)

target_include_directories(js_engine_tests
    PRIVATE
        ${CMAKE_SOURCE_DIR}
        ${CMAKE_SOURCE_DIR}/sce/include
        ${CMAKE_SOURCE_DIR}/sce/src
        ${CMAKE_SOURCE_DIR}/tests
)

target_link_libraries(js_engine_tests
    PRIVATE
        sce_runtime
        GTest::gtest_main
        pthread
)

# GCC 13+ false positive: -Wmaybe-uninitialized in std::variant move constructor
# Occurs in JSEngineBasicTest.cpp with RelWithDebInfo (-O2) optimization
# Reference: tests/engine/JSEngineBasicTest.cpp:1094-1095 - ScriptValue variant moves
if(CMAKE_CXX_COMPILER_ID STREQUAL "GNU" AND CMAKE_CXX_COMPILER_VERSION VERSION_GREATER_EQUAL 13)
    target_compile_options(js_engine_tests PRIVATE -Wno-maybe-uninitialized)
endif()

# WASM: Add Emscripten-specific link options for HTTP support
if(EMSCRIPTEN)
    target_link_options(js_engine_tests PRIVATE
        -sFETCH                         # Enable Fetch API for HTTP requests (EmscriptenFetchClient)
        -sEXIT_RUNTIME=0                # Allow async operations after main() returns
        -sENVIRONMENT=node              # Node.js environment for tests
        -sASYNCIFY                      # Enable async/await support
        -sASSERTIONS=2
        -sINITIAL_MEMORY=${WASM_INITIAL_MEMORY}
        -sSTACK_SIZE=33554432
        -sDISABLE_EXCEPTION_CATCHING=0
        -g1
        --profiling-funcs
    )
endif()

# Apply SCE test policy
apply_sce_test_policies(js_engine_tests)

# ============================================================================
# 3. StateMachine Tests
# ============================================================================
add_executable(state_machine_tests
    engine/StateMachineTest.cpp
)

target_include_directories(state_machine_tests
    PRIVATE
        ${CMAKE_SOURCE_DIR}
        ${CMAKE_SOURCE_DIR}/sce/include
        ${CMAKE_SOURCE_DIR}/tests
)

target_link_libraries(state_machine_tests
    PRIVATE
        sce_runtime
        GTest::gtest_main
        pthread
)

# ============================================================================
# 3.5. States Tests (ConcurrentRegion Unit Tests)
# ============================================================================
add_executable(states_tests
    states/ConcurrentRegionTest.cpp
    states/ParallelPseudoStateChildTest.cpp
    mocks/MockActionExecutor.cpp
)

target_include_directories(states_tests
    PRIVATE
        ${CMAKE_SOURCE_DIR}
        ${CMAKE_SOURCE_DIR}/sce/include
        ${CMAKE_SOURCE_DIR}/sce/src
        ${CMAKE_SOURCE_DIR}/tests
)

target_link_libraries(states_tests
    PRIVATE
        sce_runtime
        GTest::gtest_main
        pthread
)

# ============================================================================
# 4. Action System Tests (SCXML Executable Content)
# ============================================================================

add_executable(action_tests
    actions/ScriptActionTest.cpp
    actions/AssignActionTest.cpp
    actions/ForeachActionTest.cpp
    actions/SendParamLocationTest.cpp
    runtime/ActionExecutorImplTest.cpp
    mocks/MockActionExecutor.cpp
    mocks/MockEventRaiser.cpp
)

target_include_directories(action_tests
    PRIVATE
        ${CMAKE_SOURCE_DIR}
        ${CMAKE_SOURCE_DIR}/sce/include
        ${CMAKE_SOURCE_DIR}/sce/src
        ${CMAKE_SOURCE_DIR}/tests
)

target_link_libraries(action_tests
    PRIVATE
        sce_runtime
        GTest::gtest_main
        pthread
)

# Note: ActionTests will be registered below with all other tests

# ============================================================================
# 4.4. BasicHTTP Event Bridge Tests (W3C SCXML C.2.1 inbound event naming)
# ============================================================================

add_executable(http_bridge_tests
    events/HttpEventBridgeTest.cpp
)

target_include_directories(http_bridge_tests
    PRIVATE
        ${CMAKE_SOURCE_DIR}
        ${CMAKE_SOURCE_DIR}/sce/include
        ${CMAKE_SOURCE_DIR}/tests
)

target_link_libraries(http_bridge_tests
    PRIVATE
        sce_runtime
        GTest::gtest_main
        pthread
)

# ============================================================================
# 4.4.1. Session Target Routing Tests (W3C SCXML C.1 `#_scxml_<sessionid>`)
# ============================================================================

add_executable(session_target_routing_tests
    events/SessionTargetRoutingTest.cpp
    mocks/MockEventRaiser.cpp
)

target_include_directories(session_target_routing_tests
    PRIVATE
        ${CMAKE_SOURCE_DIR}
        ${CMAKE_SOURCE_DIR}/sce/include
        ${CMAKE_SOURCE_DIR}/sce/src
        ${CMAKE_SOURCE_DIR}/tests
)

target_link_libraries(session_target_routing_tests
    PRIVATE
        sce_runtime
        GTest::gtest_main
        pthread
)

# ============================================================================
# 4.5. History Manager Tests (SOLID SCXML W3C Compliance)
# ============================================================================

add_executable(history_tests
    runtime/HistoryManagerTest.cpp
)

target_include_directories(history_tests
    PRIVATE
        ${CMAKE_SOURCE_DIR}
        ${CMAKE_SOURCE_DIR}/sce/include
        ${CMAKE_SOURCE_DIR}/tests
)

target_link_libraries(history_tests
    PRIVATE
        sce_runtime
        GTest::gtest_main
        pthread
)

# ============================================================================
# 4.6. ReadySCXMLEngine API Tests (High-level Facade)
# ============================================================================

add_executable(ready_scxml_engine_tests
    api/ReadySCXMLEngineTest.cpp
)

target_include_directories(ready_scxml_engine_tests
    PRIVATE
        ${CMAKE_SOURCE_DIR}
        ${CMAKE_SOURCE_DIR}/sce/include
        ${CMAKE_SOURCE_DIR}/tests
)

target_link_libraries(ready_scxml_engine_tests
    PRIVATE
        sce_runtime
        GTest::gtest_main
        pthread
)

# ============================================================================
# 4.7. TimerManager API Tests (W3C SCXML 6.2 Timer Wrapper)
# ============================================================================

add_executable(timer_manager_tests
    api/TimerManagerTest.cpp
)

# Generate state machine code from test SCXML
sce_add_state_machine(
    TARGET timer_manager_tests
    SCXML_FILE ${CMAKE_CURRENT_SOURCE_DIR}/api/timer_test.scxml
)

target_include_directories(timer_manager_tests
    PRIVATE
        ${CMAKE_SOURCE_DIR}
        ${CMAKE_SOURCE_DIR}/sce/include
        ${CMAKE_SOURCE_DIR}/tests
)

target_link_libraries(timer_manager_tests
    PRIVATE
        sce_runtime
        GTest::gtest_main
        pthread
)

apply_sce_test_policies(timer_manager_tests)

# ============================================================================
# 4b. Game Loop Timer Tests (Logical Time Scheduler + GameLoopTimer)
# ============================================================================

add_executable(game_loop_timer_tests
    api/GameLoopTimerTest.cpp
)

target_include_directories(game_loop_timer_tests
    PRIVATE
        ${CMAKE_SOURCE_DIR}
        ${CMAKE_SOURCE_DIR}/sce/include
        ${CMAKE_SOURCE_DIR}/tests
)

target_link_libraries(game_loop_timer_tests
    PRIVATE
        sce_runtime
        GTest::gtest_main
        pthread
)

apply_sce_test_policies(game_loop_timer_tests)

# ============================================================================
# 5. Core Parser Tests (SCE namespace updated)
# ============================================================================


add_executable(core_parser_tests
    core/SCXMLParserBasicTest.cpp
)

add_dependencies(core_parser_tests sce_runtime)


target_include_directories(core_parser_tests
    PRIVATE
        ${CMAKE_SOURCE_DIR}
        ${CMAKE_SOURCE_DIR}/sce/include
        ${CMAKE_SOURCE_DIR}/tests
)



target_link_libraries(core_parser_tests
    PRIVATE
        sce_runtime
        GTest::gtest_main
        pthread
)



# ============================================================================
# 6. Integration Tests (SCE architecture)
# ============================================================================
add_executable(integration_tests
    integration/StateMachineIntegrationTest.cpp
    integration/ActionIntegrationTest.cpp
    integration/DonedataLocalInvokeTest.cpp
    integration/DonedataLocalInvokeAotTest.cpp
    integration/DonedataLateCompletionTest.cpp
    integration/DonedataLateCompletionAotTest.cpp
    integration/AutoforwardEventFieldsTest.cpp
    integration/AutoforwardEventFieldsAotTest.cpp
    integration/EventOriginIsALocationTest.cpp
    integration/EventOriginIsALocationAotTest.cpp
    integration/SessionIdsAreDistinctTest.cpp
    integration/SessionIdsAreDistinctAotTest.cpp
    integration/AutoforwardDoneInvokeTest.cpp
    integration/AutoforwardDoneInvokeAotTest.cpp
    integration/AutoforwardInternalQueueTest.cpp
    integration/AutoforwardInternalQueueAotTest.cpp
    integration/AutoforwardDequeuePointTest.cpp
    integration/AutoforwardDequeuePointAotTest.cpp
    integration/InvokePrecedesExternalDequeueTest.cpp
    integration/InvokePrecedesExternalDequeueAotTest.cpp
    integration/InvokePrecedesDequeueMidrunTest.cpp
    integration/InvokePrecedesDequeueMidrunAotTest.cpp
    integration/InvokeUnsupportedTypeTest.cpp
    integration/InvokeUnsupportedTypeAotTest.cpp
    integration/NestedFinalNotTerminalTest.cpp
    integration/NestedFinalNotTerminalAotTest.cpp
    integration/SendParamPayloadTest.cpp
    integration/SendParamPayloadAotTest.cpp
    integration/SCXMLEventTest.cpp
    integration/ParallelStateIntegrationTest.cpp
    integration/SCXMLParallelParsingTest.cpp
    integration/ParallelStateEndToEndTest.cpp
    integration/SCXMLParallelComplianceTest.cpp
    integration/ConcurrentCompletionMonitoringTest.cpp
    integration/ExternalTransitionHandlerTest.cpp
    integration/ParallelStateEventBroadcastingTest.cpp
    integration/ParallelStateComponentTest.cpp
    integration/SCXMLForeachIntegrationTest.cpp
    integration/HistoryStateIntegrationTest.cpp
    integration/StaticCodegenIntegrationTest.cpp
    mocks/MockActionExecutor.cpp
    mocks/MockEventRaiser.cpp
)

# HTTP-related tests (Native builds only)
if(NOT EMSCRIPTEN)
    target_sources(integration_tests PRIVATE
        integration/EventSchedulingTest.cpp
        integration/HttpEventTargetTest.cpp
        integration/SimpleMockHttpServer.cpp
        w3c/W3CHttpTestServer.cpp
    )
endif()

add_dependencies(integration_tests sce_runtime)


target_include_directories(integration_tests
    PRIVATE
        ${CMAKE_SOURCE_DIR}
        ${CMAKE_SOURCE_DIR}/sce/include
        ${CMAKE_SOURCE_DIR}/sce/src
        ${CMAKE_SOURCE_DIR}/tests
)

# Interpreter-channel integration tests read the same canonical fixture the
# AOT channel compiles through `sce_generate_static_integration_test`, so
# both engines are held to one machine instead of two hand-kept copies.
target_compile_definitions(integration_tests
    PRIVATE
        SCE_PROJECT_ROOT="${CMAKE_SOURCE_DIR}"
)



target_link_libraries(integration_tests
    PRIVATE
        sce_runtime
        GTest::gtest_main
        GTest::gmock
        pthread
)

# WASM: Add Emscripten-specific link options for HTTP support
if(EMSCRIPTEN)
    target_link_options(integration_tests PRIVATE
        -sFETCH                         # Enable Fetch API for HTTP requests (EmscriptenFetchClient)
        -sEXIT_RUNTIME=0                # Allow async operations after main() returns
        -sENVIRONMENT=node              # Node.js environment for tests
        -sASYNCIFY                      # Enable async/await support
        -sASSERTIONS=2
        -sINITIAL_MEMORY=${WASM_INITIAL_MEMORY}
        -sSTACK_SIZE=33554432
        -sDISABLE_EXCEPTION_CATCHING=0
        -g1
        --profiling-funcs
    )
endif()



# ============================================================================
# 7. Model Layer Tests
# ============================================================================
add_executable(model_tests
    model/DataModelItemTest.cpp
)

add_dependencies(model_tests sce_runtime)


target_include_directories(model_tests
    PRIVATE
        ${CMAKE_SOURCE_DIR}
        ${CMAKE_SOURCE_DIR}/sce/include
        ${CMAKE_SOURCE_DIR}/tests
)



target_link_libraries(model_tests
    PRIVATE
        sce_runtime
        GTest::gtest_main
)



# ============================================================================
# 8. Concurrency and Deadlock Tests
# ============================================================================
add_executable(concurrency_tests
    concurrency/DeadlockReproductionTest.cpp
    concurrency/EventQueueManagerThreadSafetyTest.cpp
    concurrency/StdThreadDispatcherTest.cpp
    concurrency/AsyncStateMachineTest.cpp
    mocks/MockEventRaiser.cpp
)

target_include_directories(concurrency_tests
    PRIVATE
        ${CMAKE_SOURCE_DIR}
        ${CMAKE_SOURCE_DIR}/sce/include
        ${CMAKE_SOURCE_DIR}/sce/src
        ${CMAKE_SOURCE_DIR}/tests
)

target_link_libraries(concurrency_tests
    PRIVATE
        sce_runtime
        GTest::gtest_main
        GTest::gmock
        pthread
)

# WASM: Add Emscripten-specific link options for HTTP support
if(EMSCRIPTEN)
    target_link_options(concurrency_tests PRIVATE
        -sFETCH                         # Enable Fetch API for HTTP requests (EmscriptenFetchClient)
        -sEXIT_RUNTIME=0                # Allow async operations after main() returns
        -sENVIRONMENT=node              # Node.js environment for tests
        -sASYNCIFY                      # Enable async/await support
        -sASSERTIONS=2
        -sINITIAL_MEMORY=${WASM_INITIAL_MEMORY}
        -sSTACK_SIZE=33554432
        -sDISABLE_EXCEPTION_CATCHING=0
        -g1
        --profiling-funcs
    )
endif()

# ============================================================================
# 8a. Qt Dispatcher Tests (Optional - requires SCE_DISPATCHER_QT)
# ============================================================================
if(SCE_DISPATCHER_QT)
    # Find Qt for test file (uses Qt headers directly)
    find_package(Qt6 COMPONENTS Core QUIET)
    if(NOT Qt6_FOUND)
        find_package(Qt5 COMPONENTS Core REQUIRED)
        set(QT_TEST_LIBRARIES Qt5::Core)
    else()
        set(QT_TEST_LIBRARIES Qt6::Core)
    endif()

    add_executable(qt_dispatcher_tests
        concurrency/QtDispatcherTest.cpp
    )

    target_include_directories(qt_dispatcher_tests
        PRIVATE
            ${CMAKE_SOURCE_DIR}
            ${CMAKE_SOURCE_DIR}/sce/include
            ${CMAKE_SOURCE_DIR}/tests
    )

    target_link_libraries(qt_dispatcher_tests
        PRIVATE
            sce_qt_dispatcher
            ${QT_TEST_LIBRARIES}
            GTest::gtest_main
            pthread
    )

    add_test(NAME QtDispatcherTests COMMAND qt_dispatcher_tests)
    message(STATUS "SCE Tests: Qt dispatcher tests enabled")
endif()

# ============================================================================
# 8b. GLib Dispatcher Tests (Optional - requires SCE_DISPATCHER_GLIB)
# ============================================================================
if(SCE_DISPATCHER_GLIB)
    add_executable(glib_dispatcher_tests
        concurrency/GLibDispatcherTest.cpp
    )

    target_include_directories(glib_dispatcher_tests
        PRIVATE
            ${CMAKE_SOURCE_DIR}
            ${CMAKE_SOURCE_DIR}/sce/include
            ${CMAKE_SOURCE_DIR}/tests
            ${GLIB_INCLUDE_DIRS}
    )

    target_link_libraries(glib_dispatcher_tests
        PRIVATE
            sce_glib_dispatcher
            GTest::gtest_main
            pthread
    )

    add_test(NAME GLibDispatcherTests COMMAND glib_dispatcher_tests)
    message(STATUS "SCE Tests: GLib dispatcher tests enabled")
endif()

# ============================================================================
# 9. TXML Converter Tests (W3C SCXML Compliance Testing)
# ============================================================================
add_executable(txml_converter_tests
    w3c/TXMLConverterTest.cpp
    w3c/impl/TXMLConverter.cpp
)

target_include_directories(txml_converter_tests
    PRIVATE
        ${CMAKE_SOURCE_DIR}
        ${CMAKE_SOURCE_DIR}/sce/include
        ${CMAKE_SOURCE_DIR}/tests
)



target_link_libraries(txml_converter_tests
    PRIVATE
        sce_runtime
        GTest::gtest_main
        pthread
)



add_dependencies(txml_converter_tests sce_runtime)


# ============================================================================
# 9. W3C SCXML Compliance Tests (SOLID Architecture) - Native builds only
# ============================================================================
# WASM builds: W3C tests require txml-converter (Native-only tool) for
# TXML → SCXML conversion and sce-codegen for C++ code generation.

if(NOT EMSCRIPTEN)
# This section defines W3C SCXML conformance tests that run on both:
# - Interpreter Engine: Runtime SCXML parsing and execution
# - AOT Engine: Ahead-of-Time compiled static state machines
#
# Test Validation:
# - Use /validate-test-execution [test-number] to verify test correctness
# - Validates both engines for false positives by comparing:
#   1. Test intent (from resources/*/metadata.txt and *.txml)
#   2. Interpreter execution logs (trace level)
#   3. AOT execution logs (trace level)
#   4. 1:1 behavioral comparison (state transitions, events, data)
# - See .claude/commands/validate-test-execution.md for details
#
# Output directory for generated static W3C test code (used by both w3c_test_cli and w3c_static_test_cli)
set(STATIC_W3C_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/w3c_static_generated")

# Output directory for generated static integration-fixture code. Distinct
# from STATIC_W3C_OUTPUT_DIR so the build system treats W3C and integration
# trees as separate §6.2.6 drift contexts.
set(STATIC_INTEGRATION_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated")

# Generate C++ code for W3C tests BEFORE defining executables
include(${CMAKE_SOURCE_DIR}/cmake/SCEStaticW3CTest.cmake)
# Parallel CMake function for integration fixtures
# (`integration_resources/<stem>/<stem>.scxml` → `<output>/<stem>_sm.{h,inl}`).
include(${CMAKE_SOURCE_DIR}/cmake/SCEStaticIntegrationFixture.cmake)

# Initialize GENERATED_W3C_HEADERS and W3C_AOT_TESTS before any sce_generate_static_w3c_test calls
# W3C Test Registration Automation: W3C_AOT_TESTS is auto-accumulated by sce_generate_static_w3c_test()
# Adding a new test = adding ONE entry to tests/w3c/conformance/fixtures.json,
# which the loop below turns into a call (TestXXX.h + registry auto-generated)
set(GENERATED_W3C_HEADERS "")
set(W3C_AOT_TESTS "")
# Initialize integration-fixture header accumulator. sce_generate_static_integration_test
# appends to this in parent scope; the integration_tests target then DEPENDS on it.
set(GENERATED_INTEGRATION_HEADERS "")

# Integration fixtures.
# Build-time generation only (no committed tree); the build itself is the
# §6.2.6 freshness invariant.
sce_generate_static_integration_test(donedata_local_invoke ${STATIC_INTEGRATION_OUTPUT_DIR}
    SYNTH_INVOKE_CHILDREN inv_param inv_content
)

sce_generate_static_integration_test(autoforward_event_fields ${STATIC_INTEGRATION_OUTPUT_DIR}
    SYNTH_INVOKE_CHILDREN inv_echo
)

# W3C SCXML C.1: `_event.origin` must be the sender's published
# `_ioprocessors` location, and that location must be a usable <send>
# target. Test336 and test350 both send to the session they already are, so
# the corpus never puts a second session on the other end.
sce_generate_static_integration_test(event_origin_is_a_location ${STATIC_INTEGRATION_OUTPUT_DIR}
    SYNTH_INVOKE_CHILDREN inv_peer
)

# W3C SCXML 5.10: `_sessionid` names the session, so two live sessions carry
# two ids. C.1.1 derives the published address from that id, which is what
# makes one shared id a routing defect and not a cosmetic one. The corpus
# runs one session per test and cannot ask.
sce_generate_static_integration_test(session_ids_are_distinct ${STATIC_INTEGRATION_OUTPUT_DIR}
    SYNTH_INVOKE_CHILDREN inv_a inv_b
)

# W3C SCXML 6.4: Appendix D `mainEventLoop` forwards every event dequeued from
# the external queue to each autoforward child and excludes only the cancel
# event, and §6.4.2 puts `done.invoke.<id>` on that queue. Two concurrent
# invokes are what it takes to observe the forward, which is why no IRP test
# covers it.
sce_generate_static_integration_test(autoforward_done_invoke ${STATIC_INTEGRATION_OUTPUT_DIR}
    SYNTH_INVOKE_CHILDREN inv_watch inv_short
)

# W3C SCXML 6.4: the negative half of the same loop. The internal drain has
# no forwarding step, so `error.execution` (§6.2, unsupported `<send>` type)
# must never reach an autoforward child — excluded by the queue it was
# raised onto, not by a filter that recognises its name.
sce_generate_static_integration_test(autoforward_internal_queue ${STATIC_INTEGRATION_OUTPUT_DIR}
    SYNTH_INVOKE_CHILDREN inv_watch
)

# W3C SCXML 6.4: *where* in the loop the forward happens. Appendix D puts it
# one statement after `externalQueue.dequeue()`, and §6.4.2 in prose "at the
# point at which it removes it from the external event queue". The two
# fixtures above are blind to this by construction — queue order alone cannot
# tell the enqueue from the dequeue, so the parent marks the child directly
# with `<send target="#_inv_probe">` from inside a transition body.
sce_generate_static_integration_test(autoforward_dequeue_point ${STATIC_INTEGRATION_OUTPUT_DIR}
    SYNTH_INVOKE_CHILDREN inv_probe
)

# W3C SCXML Appendix D: *when* the invokes start relative to the external
# dequeue. `mainEventLoop` completes the macrostep on eventless and internal
# transitions alone, runs `invoke(inv)`, and only then reaches
# `externalQueue.dequeue()`. The fixture above cannot see this axis — its
# child opens the exchange, so it is alive before anything is queued; here the
# parent queues for itself in `<onentry>` and the child starts second.
sce_generate_static_integration_test(invoke_precedes_external_dequeue ${STATIC_INTEGRATION_OUTPUT_DIR}
    SYNTH_INVOKE_CHILDREN inv_watch
)

# W3C SCXML Appendix D: the same order, reached mid-run. `statesToInvoke` is
# filled by `enterStates` inside `microstep`, so a state entered by an external
# event's transition arms an invoke that must start before the next event is
# dequeued. The fixture above enters its invoking state at initialization and
# cannot see an engine that drains the external queue to exhaustion per step.
sce_generate_static_integration_test(invoke_precedes_dequeue_midrun ${STATIC_INTEGRATION_OUTPUT_DIR}
    SYNTH_INVOKE_CHILDREN inv_watch
)

# W3C SCXML 3.7: a `<final>` nested inside a compound state completes that
# state, not the session — Appendix D `enterStates` ends the session only when
# the final's parent is the `<scxml>` element. The fixture rests in the nested
# final so the completion predicate is sampled while the machine is still live.
sce_generate_static_integration_test(nested_final_not_terminal ${STATIC_INTEGRATION_OUTPUT_DIR})

# W3C SCXML 6.4.1: an `<invoke>` whose `type` names no processor this platform
# implements is valid SCXML that must raise `error.execution` — not a document
# to reject at build time. AOT used to drop such an `<invoke>` from the model
# entirely, so there was no observable at all where the Interpreter had one;
# the fixture rests in `probe` if any backend still does. No
# SYNTH_INVOKE_CHILDREN: §6.4.1 classification happens on `type` alone, before
# any child document would be resolved, so codegen emits no child here.
sce_generate_static_integration_test(invoke_unsupported_type ${STATIC_INTEGRATION_OUTPUT_DIR})

# W3C SCXML 5.5 + 6.3.1: donedata rides `done.invoke.<id>` wherever the child
# reaches its final state, not only where it finalises during start-up. The
# `donedata_local_invoke` fixture pins the payload shapes on a child that is
# already final at invoke time and is blind to this; here the child answers an
# event first and reaches `<final>` two macrosteps in.
sce_generate_static_integration_test(donedata_late_completion ${STATIC_INTEGRATION_OUTPUT_DIR}
    SYNTH_INVOKE_CHILDREN inv_late
)

# W3C SCXML 6.2: a `<send>` carries its `<param>` payload on both the paths
# that had no runtime witness — from a `datamodel="null"` child (which needs
# no script engine, so param emission must be judged per send rather than per
# machine) and to `#_internal` (whose raise used to take no event data). The
# two land in distinct final states so a failure names the path.
sce_generate_static_integration_test(send_param_payload ${STATIC_INTEGRATION_OUTPUT_DIR}
    SYNTH_INVOKE_CHILDREN inv_emitter
)

# Wire the integration headers + include path into the integration_tests
# target now that `sce_generate_static_integration_test` has populated
# GENERATED_INTEGRATION_HEADERS in this scope. The target itself was
# defined earlier (around the "Integration Tests" section), but its
# generated-source dependencies must be appended after the build-time
# headers are known so Ninja schedules codegen before compilation.
target_sources(integration_tests PRIVATE ${GENERATED_INTEGRATION_HEADERS})
target_include_directories(integration_tests PRIVATE ${STATIC_INTEGRATION_OUTPUT_DIR})

# ── W3C test registrations, derived from the conformance registry ──
#
# tests/w3c/conformance/fixtures.json is the source of truth for which
# upstream tests this repository runs and which harness each one needs.
# These calls used to be 202 hand-written lines here, and the generator
# read them back out with a regex — CMake was the registry, which meant
# a checkout without CMake could not enumerate the suite at all.
#
# `list-fixtures` hands the ids over as a CMake list, the same way every
# forge conformance harness already consumes its own catalog, so there is
# no JSON parsing in CMake and no second copy of the set to keep in step.
# `sce-build/tests/w3c_registry_cmake_parity.rs` pins that what is
# scheduled here is what the registry declares.
set(W3C_REGISTRY "${CMAKE_SOURCE_DIR}/tests/w3c/conformance/fixtures.json")

# Build an id -> harness lookup from the registry, then register in
# registry order. Grouping the calls by harness instead would give the
# same set in a different order, and the order reaches the generated
# W3CTestRegistry.cpp and the ctest schedule — registry order is the one
# a reader can predict from the file.
set(_W3C_HARNESSES simple scheduled http)
foreach(_W3C_HARNESS ${_W3C_HARNESSES})
    execute_process(
        COMMAND "${SCE_CODEGEN}" list-fixtures
                --manifest "${W3C_REGISTRY}"
                --catalog w3c
                --harness "${_W3C_HARNESS}"
                --format cmake
        OUTPUT_VARIABLE _W3C_IDS
        OUTPUT_STRIP_TRAILING_WHITESPACE
        RESULT_VARIABLE _W3C_LIST_RESULT
        ERROR_VARIABLE _W3C_LIST_ERROR
    )
    if(NOT _W3C_LIST_RESULT EQUAL 0)
        message(FATAL_ERROR
            "sce-codegen list-fixtures --harness ${_W3C_HARNESS} failed for "
            "${W3C_REGISTRY}: ${_W3C_LIST_ERROR}")
    endif()
    # An empty answer is a broken read, not an empty harness: the
    # registry rejects a harness no fixture names, so a silent empty
    # list here would drop a whole registration group.
    if(NOT _W3C_IDS)
        message(FATAL_ERROR
            "sce-codegen list-fixtures returned no test for harness "
            "${_W3C_HARNESS}; the registry read is broken")
    endif()
    foreach(_W3C_ID ${_W3C_IDS})
        set(_W3C_HARNESS_OF_${_W3C_ID} "${_W3C_HARNESS}")
    endforeach()
endforeach()

execute_process(
    COMMAND "${SCE_CODEGEN}" list-fixtures
            --manifest "${W3C_REGISTRY}"
            --catalog w3c
            --format cmake
    OUTPUT_VARIABLE _W3C_ALL_IDS
    OUTPUT_STRIP_TRAILING_WHITESPACE
    RESULT_VARIABLE _W3C_LIST_RESULT
    ERROR_VARIABLE _W3C_LIST_ERROR
)
if(NOT _W3C_LIST_RESULT EQUAL 0)
    message(FATAL_ERROR
        "sce-codegen list-fixtures failed for ${W3C_REGISTRY}: ${_W3C_LIST_ERROR}")
endif()
if(NOT _W3C_ALL_IDS)
    message(FATAL_ERROR
        "sce-codegen list-fixtures returned no test at all; the registry read is broken")
endif()

foreach(_W3C_ID ${_W3C_ALL_IDS})
    set(_W3C_HARNESS "${_W3C_HARNESS_OF_${_W3C_ID}}")
    if(NOT _W3C_HARNESS)
        message(FATAL_ERROR
            "test ${_W3C_ID} is listed by the registry but by no harness query; "
            "the two listings disagree")
    endif()
    if(_W3C_HARNESS STREQUAL "simple")
        sce_generate_static_w3c_test(${_W3C_ID} ${STATIC_W3C_OUTPUT_DIR})
    else()
        string(TOUPPER "${_W3C_HARNESS}" _W3C_TYPE)
        sce_generate_static_w3c_test(${_W3C_ID} ${STATIC_W3C_OUTPUT_DIR} TYPE ${_W3C_TYPE})
    endif()
endforeach()

# Re-run configure when the registry changes, so adding a test does not
# need a manual cmake invocation.
set_property(DIRECTORY APPEND PROPERTY CMAKE_CONFIGURE_DEPENDS "${W3C_REGISTRY}")

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
# VALIDATE: Run /validate-test-execution [test-number] to verify correctness.
#
# FILES GENERATED:
# - build/W3CAotTestList.cmake (auto-accumulated test list for WASM + tooling)
# - build/tests/w3c/W3CTestRegistry.cpp (from W3CTestRegistry.cpp.in template)
# - tests/w3c/runners/W3CTestRunner_Test*.cpp (CMake auto-generates)
# - tests/w3c/aot_tests/TestXXX.h (CMake auto-generates if not exists)
# ===========================================================================

# W3C Test Registration Automation: Save accumulated W3C_AOT_TESTS for WASM builds and tooling
# This file is auto-generated from sce_generate_static_w3c_test() calls above
# Use space-separated format (compatible with CMake set() and Python regex parsing)
string(REPLACE ";" " " _W3C_AOT_TESTS_SPACES "${W3C_AOT_TESTS}")
set(_W3C_AOT_TESTS_LIST_FILE "${CMAKE_SOURCE_DIR}/build/W3CAotTestList.cmake")
file(WRITE "${_W3C_AOT_TESTS_LIST_FILE}"
    "# Auto-generated from sce_generate_static_w3c_test() calls in tests/CMakeLists.txt\n"
    "# DO NOT EDIT - Run Native CMake configure to regenerate\n"
    "set(W3C_AOT_TESTS\n"
    "    ${_W3C_AOT_TESTS_SPACES}\n"
    ")\n"
)
message(STATUS "W3C AOT Tests auto-accumulated: ${W3C_AOT_TESTS}")
message(STATUS "Saved W3C_AOT_TESTS list to ${_W3C_AOT_TESTS_LIST_FILE}")

endif()  # NOT EMSCRIPTEN - W3C SCXML test generation (sce_generate_static_w3c_test calls)

# WASM: Use pre-generated AOT test headers and test list from Native build
# ARCHITECTURE.md: WASM builds cannot run txml-converter (requires filesystem operations)
if(EMSCRIPTEN)
    # Point to Native build's generated headers
    # NOTE: Requires Native build to be run first to generate AOT test headers
    set(STATIC_W3C_OUTPUT_DIR "${CMAKE_SOURCE_DIR}/build/tests/w3c_static_generated")

    if(NOT EXISTS "${STATIC_W3C_OUTPUT_DIR}")
        message(WARNING "WASM build requires pre-generated AOT test headers from Native build. Please run Native build first: cd ../build && cmake .. && cmake --build . --target w3c_test_cli")
    endif()

    # W3C Test Registration Automation: Read accumulated test list from Native build
    set(_W3C_AOT_TESTS_LIST_FILE "${CMAKE_SOURCE_DIR}/build/W3CAotTestList.cmake")
    if(EXISTS "${_W3C_AOT_TESTS_LIST_FILE}")
        include("${_W3C_AOT_TESTS_LIST_FILE}")
    else()
        message(WARNING "W3C_AOT_TESTS list file not found: ${_W3C_AOT_TESTS_LIST_FILE}")
        message(WARNING "Run Native CMake configure first to generate this file")
    endif()

    message(STATUS "WASM: Using pre-generated AOT test headers from ${STATIC_W3C_OUTPUT_DIR}")
endif()

# ============================================================================
# Auto-generate web/visualizer/test-list.js from W3C_AOT_TESTS
# ============================================================================
# Zero Duplication: the conformance registry is the single source, and
# `generate_test_list.py` reads it directly.
#
# It was handed `build/W3CAotTestList.cmake` instead — the CMake list this
# tree used to accumulate — which the script stopped understanding when the
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
                ${W3C_REGISTRY}
                ${CMAKE_SOURCE_DIR}/web/visualizer/test-list.js
        DEPENDS ${W3C_REGISTRY}
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
set(W3C_AOT_TEST_REGISTRATIONS "")
foreach(TEST_NUM ${W3C_AOT_TESTS})
    string(APPEND W3C_AOT_TEST_REGISTRATIONS "        {\"${TEST_NUM}\", TestEngine::AOT},\n")
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

# Auto-generate AOT test runner files
set(W3C_AOT_RUNNER_SOURCES "")
foreach(TEST_NUM ${W3C_AOT_TESTS})
    set(RUNNER_FILE "${CMAKE_CURRENT_SOURCE_DIR}/w3c/runners/W3CTestRunner_Test${TEST_NUM}.cpp")
    file(WRITE "${RUNNER_FILE}"
"// Auto-generated runner file for W3C SCXML Test ${TEST_NUM}
#include \"aot_tests/Test${TEST_NUM}.h\"
")
    list(APPEND W3C_AOT_RUNNER_SOURCES "w3c/runners/W3CTestRunner_Test${TEST_NUM}.cpp")
endforeach()

# ============================================================================
# W3C Test CLI Target (Both Native and WASM)
# ============================================================================
# Full W3C Test Runner for all W3C SCXML 1.0 compliance tests
# Build for both Native and WASM (HTTP tests excluded on WASM)
set(W3C_TEST_CLI_SOURCES
    w3c/W3CTestCLI.cpp
    w3c/W3CTestRunner.cpp
    ${CMAKE_CURRENT_BINARY_DIR}/w3c/W3CTestRegistry.cpp  # Auto-generated test registry
    ${W3C_AOT_RUNNER_SOURCES}  # Explicit AOT test runners (from W3C_AOT_TESTS list)
    w3c/impl/TXMLConverter.cpp
    w3c/impl/TestMetadataParser.cpp
    w3c/impl/W3CTestEventDispatcher.cpp
    mocks/MockEventRaiser.cpp
    ${GENERATED_W3C_HEADERS}  # Trigger generation of static test headers for AOT engine
)

# Native-only: HTTP server for BasicHTTP Event I/O Processor tests (W3C SCXML C.2)
if(NOT EMSCRIPTEN)
    list(APPEND W3C_TEST_CLI_SOURCES w3c/W3CHttpTestServer.cpp)
endif()

add_executable(w3c_test_cli ${W3C_TEST_CLI_SOURCES})

target_link_libraries(w3c_test_cli
    PRIVATE
        sce_runtime
)

# WASM: Disable warnings from pre-generated Native AOT headers
if(EMSCRIPTEN)
    target_compile_options(w3c_test_cli PRIVATE
        -Wno-unused-lambda-capture
        -Wno-unused-parameter
    )

    # WASM: Use pre-js for XMLHttpRequest polyfill, NODEFS mount, and HTTP server auto-management
    set(POLYFILL_PRE_JS "${CMAKE_CURRENT_SOURCE_DIR}/w3c/wasm_polyfill.js")

    # WASM: Add Emscripten-specific link options (NO pthread for browser compatibility)
    target_link_options(w3c_test_cli PRIVATE
        -sFETCH                         # Enable Fetch API for HTTP requests
        -sALLOW_TABLE_GROWTH=1          # Allow function table to grow dynamically (required for QuickJS JS_NewCFunction)
        -sEXIT_RUNTIME=0                # Keep runtime alive for async HTTP callbacks
        -sENVIRONMENT=node              # Node.js environment for wrapper script
        -sASYNCIFY                      # Enable async/await - allows future.get() to yield to event loop
        -O3                             # Optimization required for reasonable ASYNCIFY binary size
        -sASSERTIONS=2
        -sINITIAL_MEMORY=${WASM_INITIAL_MEMORY}      # Centrally managed in root CMakeLists.txt
        -sSTACK_SIZE=33554432
        -sDISABLE_EXCEPTION_CATCHING=0
        -g1                             # Line number info only
        --profiling-funcs
        --pre-js ${POLYFILL_PRE_JS}    # Inject XMLHttpRequest polyfill and NODEFS for Node.js
        -lnodefs.js                     # Enable NODEFS for Node.js filesystem access
        "-sEXPORTED_FUNCTIONS=['_main','_dom_getElementsByTagName_wrapper','_dom_getAttribute_wrapper']"  # Export main + DOM wrapper functions
    )

    # W3C SCXML C.2: Copy standalone HTTP server to build directory (spawned by polyfill_pre.js)
    add_custom_command(TARGET w3c_test_cli POST_BUILD
        COMMAND ${CMAKE_COMMAND} -E copy
            ${CMAKE_CURRENT_SOURCE_DIR}/w3c/standalone_http_server.js
            ${CMAKE_CURRENT_BINARY_DIR}/standalone_http_server.js
        COMMENT "Copying standalone HTTP server to build directory"
    )
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

# ============================================================================
# Comprehensive Interactive Debugging Verification Test
# ============================================================================
# Automated time-travel debugging verification for ALL W3C SCXML tests
# Verifies step forward, step back, and reset operations maintain perfect
# state consistency across all W3C SCXML 3.13 microstep semantics
add_executable(comprehensive_interactive_test
    w3c/ComprehensiveInteractiveTest.cpp
    w3c/InteractiveTestRunner.cpp
    w3c/impl/SnapshotComparator.cpp
    w3c/impl/TXMLConverter.cpp
    w3c/impl/TestMetadataParser.cpp
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
    # Reuse the same polyfill as w3c_test_cli for NODEFS mounting
    set(POLYFILL_PRE_JS "${CMAKE_CURRENT_SOURCE_DIR}/w3c/wasm_polyfill.js")

    target_link_options(comprehensive_interactive_test PRIVATE
        --bind                          # Enable Emscripten embind for val objects
        -sFETCH                         # Enable Fetch API for HTTP operations
        -sEXIT_RUNTIME=0                # Allow async operations after main() returns
        -sENVIRONMENT=node              # Node.js environment for tests
        -sASYNCIFY                      # Enable async/await support
        -sASSERTIONS=2
        -sINITIAL_MEMORY=${WASM_INITIAL_MEMORY}
        -sSTACK_SIZE=33554432
        -sDISABLE_EXCEPTION_CATCHING=0
        -g1
        --profiling-funcs
        --pre-js ${POLYFILL_PRE_JS}     # Inject XMLHttpRequest polyfill and NODEFS for Node.js
        -lnodefs.js                     # Enable NODEFS for Node.js filesystem access
    )
endif()

apply_sce_test_policies(comprehensive_interactive_test)

add_dependencies(comprehensive_interactive_test sce_runtime)


# ============================================================================
# SCE Mesh compile verification test
# ============================================================================
# Validates that generated mesh transport code compiles against generated SM
# headers. Compilation success = test pass (runtime assertions live in
# the *_runtime_verification tests below).
if(SCE_ENABLE_MESH AND NOT EMSCRIPTEN)
    set(MESH_TEST_DIR "${CMAKE_CURRENT_SOURCE_DIR}/mesh")
    set(MESH_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/mesh_generated")

    # Opt-in for §9.6 two-host fixtures that require Linux network
    # namespaces + a veth pair (mesh_someip_scxml_invoke_crossdev,
    # mesh_zenoh_scxml_invoke_crossdev). These exercise SOME/IP-SD
    # multicast and Zenoh peer-mesh discovery across a real network
    # stack — loopback alone cannot route multicast, and a single
    # routing table cannot reproduce the production "two ECUs on one
    # wire" topology the cross-device classifier targets.
    #
    # When ON, ctest registers the two-host fixtures via the helper in
    # tests/cmake/two_host_test.cmake. The orchestrator
    # (tests/mesh/run_two_host_fixture.sh) needs the netns to exist and
    # the test runner to have CAP_NET_ADMIN — otherwise it skips with
    # SKIP_RETURN_CODE 77 (Skipped, not Failed).
    #
    # One-time setup before turning this option on:
    #   sudo tests/mesh/setup_crossdev_netns.sh
    option(SCE_ENABLE_NETNS_TESTS
        "Register §9.6 two-host fixtures requiring sce-mesh-{parent,worker} netns; \
run tests/mesh/setup_crossdev_netns.sh as root before enabling"
        OFF)

    # Jinja2 template consumed by sce-codegen --deploy. Listed in the
    # DEPENDS of every *_transport.h custom_command so template edits
    # trigger re-codegen without manually deleting cached headers.
    set(SCE_MESH_CPP_TEMPLATE
        "${CMAKE_SOURCE_DIR}/tools/codegen/templates/mesh/cpp/mesh_transport.h.jinja2")

    # ── MeshEnvelope codec unit tests ─────────────────────────────
    # Pure unit tests on the CBOR envelope codec — no codegen, no
    # transport, no scheduler. Catches wire-format regressions early.
    add_executable(mesh_envelope_codec_test
        mesh/MeshEnvelopeCodecTest.cpp
    )

    target_include_directories(mesh_envelope_codec_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
    )

    target_link_libraries(mesh_envelope_codec_test
        PRIVATE
            sce_mesh_common
            GTest::gtest_main
            pthread
    )

    target_compile_features(mesh_envelope_codec_test PRIVATE cxx_std_17)

    apply_sce_test_policies(mesh_envelope_codec_test)

    add_test(NAME mesh_envelope_codec_verification COMMAND mesh_envelope_codec_test)

    # ── InvokeCorrelation unit tests (SCE_MESH.md §9.5 F2) ────────
    # Pure unit tests on the `<invoke type="sce:mesh-rpc">` correlation
    # registry. Verifies the state machine (register → reply / deadline
    # / cancel) and the two race conditions §9.5 defines as benign
    # drop. No transport, no scheduler — the scheduler-driven deadline
    # timing path is exercised end-to-end in F4 integration tests.
    add_executable(mesh_invoke_correlation_test
        mesh/InvokeCorrelationTest.cpp
    )

    target_include_directories(mesh_invoke_correlation_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
    )

    target_link_libraries(mesh_invoke_correlation_test
        PRIVATE
            sce_mesh_common
            GTest::gtest_main
            pthread
    )

    target_compile_features(mesh_invoke_correlation_test PRIVATE cxx_std_17)

    apply_sce_test_policies(mesh_invoke_correlation_test)

    add_test(NAME mesh_invoke_correlation_verification COMMAND mesh_invoke_correlation_test)

    # ── RetryingDispatcher unit tests (SCE_MESH.md §16.7 row 3) ──
    # Pure unit tests on the retry layer wrapping OutboundBuffer's
    # dispatcher boundary. Verifies the contract pinned in
    # `sce/include/mesh/RetryingDispatcher.h`: transient retry +
    # success, exhaustion with attempts==max_retries+1, terminal
    # fast-fail (retryable=false) with attempts==1, cancel-preempts-
    # retry, defensive max_retries=0 pass-through, exponential
    # backoff timing, jitter band bounds. The MeshDeadlineScheduler
    # is real (the timing-sensitive scheduler-thread path is what
    # we're verifying); no transport, no OutboundBuffer integration —
    # the wiring through OutboundBuffer is verified separately in
    # the codegen E2E flaky-transport ctest.
    add_executable(mesh_retrying_dispatcher_test
        mesh/RetryingDispatcherTest.cpp
    )

    target_include_directories(mesh_retrying_dispatcher_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
    )

    target_link_libraries(mesh_retrying_dispatcher_test
        PRIVATE
            sce_mesh_common
            GTest::gtest_main
            pthread
    )

    target_compile_features(mesh_retrying_dispatcher_test PRIVATE cxx_std_17)

    apply_sce_test_policies(mesh_retrying_dispatcher_test)

    add_test(NAME mesh_retrying_dispatcher_verification COMMAND mesh_retrying_dispatcher_test)

    # ── Deadline preempts retry composition (SCE_MESH.md §16.7 row 3
    #    follow-up) ────────────────────────────────────────────────
    # Pins the wiring contract emitted by the codegen deadline lambda:
    # `invoke_correlation_.handleDeadline(uuid)` followed by
    # `cancelEnvelopeRetryById(uuid)`. Drives RetryingDispatcher +
    # InvokeCorrelation + MeshDeadlineScheduler directly so the
    # composition contract is verified without a full someip / zenoh
    # transport in the loop.
    add_executable(mesh_deadline_preempts_retry_test
        mesh/test_mesh_deadline_preempts_retry.cpp
    )

    target_include_directories(mesh_deadline_preempts_retry_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
    )

    target_link_libraries(mesh_deadline_preempts_retry_test
        PRIVATE
            sce_mesh_common
            GTest::gtest_main
            pthread
    )

    target_compile_features(mesh_deadline_preempts_retry_test PRIVATE cxx_std_17)

    apply_sce_test_policies(mesh_deadline_preempts_retry_test)

    add_test(NAME mesh_deadline_preempts_retry_verification
             COMMAND mesh_deadline_preempts_retry_test)

    # ── AuthClassifier unit tests (SCE_MESH.md §16.7 row 10) ──────
    # Pins the Q3-lock keyword set the row-10 classifier uses to
    # decide row 1 TRANSPORT_UNAVAILABLE vs row 10 UNAUTHORIZED for
    # zenoh ZException::what() inputs. Sole consumer is the generated
    # router's ZException catch block, so a regression here directly
    # breaks the §16.7 row 10 emit contract.
    add_executable(mesh_auth_classifier_test
        mesh/AuthClassifierTest.cpp
    )

    target_include_directories(mesh_auth_classifier_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
    )

    target_link_libraries(mesh_auth_classifier_test
        PRIVATE
            sce_mesh_common
            GTest::gtest_main
            pthread
    )

    target_compile_features(mesh_auth_classifier_test PRIVATE cxx_std_17)

    apply_sce_test_policies(mesh_auth_classifier_test)

    add_test(NAME mesh_auth_classifier_verification
             COMMAND mesh_auth_classifier_test)

    # ── AuthClassifier axis-6 CI fixture (docs/SCE_AXIS_6_PATTERNS.md
    # A6-001) ─────────────────────────────────────────────────────
    # Dynamic verification gate: exercises isZenohAuthFailMessage
    # against the actually-linked zenoh-cpp binary by triggering a
    # real mTLS handshake failure between two in-process Sessions.
    # Catches zenoh-cpp upgrade-time drift in the auth-fail error
    # text before consumers see the regression.
    #
    # Gated on BOTH `openssl` (binary, for fresh per-test cert
    # generation) and zenoh-cpp (for the actual handshake). Either
    # missing, the fixture is not built. We need only the executable —
    # the test runtime shells out for cert generation rather than
    # linking against libssl. find_package is called here directly so
    # the gate doesn't depend on the zenoh-cpp compile block farther
    # down evaluating before this one.
    find_program(OPENSSL_EXECUTABLE openssl)
    find_package(zenohc QUIET)
    find_package(zenohcxx QUIET)
    if(OPENSSL_EXECUTABLE AND zenohc_FOUND AND zenohcxx_FOUND)
        add_executable(mesh_auth_classifier_ci_fixture
            mesh/AuthClassifierCIFixture.cpp
        )

        target_include_directories(mesh_auth_classifier_ci_fixture
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
        )

        target_link_libraries(mesh_auth_classifier_ci_fixture
            PRIVATE
                sce_mesh_common
                zenohcxx::zenohc
                GTest::gtest_main
                pthread
        )

        target_compile_features(mesh_auth_classifier_ci_fixture PRIVATE cxx_std_17)

        apply_sce_test_policies(mesh_auth_classifier_ci_fixture)

        add_test(NAME mesh_auth_classifier_ci_verification
                 COMMAND mesh_auth_classifier_ci_fixture)
        # This fixture opens zenoh sessions and drops them mid-run, which
        # surfaces libzenohc's own unreleased runtime allocations to
        # LeakSanitizer under a Debug (ASan) build. The suppression is scoped
        # to that shared object — see tests/sanitizers/lsan_zenoh.supp for the
        # measurements that rule out an SCE-side missing close.
        set_tests_properties(mesh_auth_classifier_ci_verification PROPERTIES
            TIMEOUT 30
            ENVIRONMENT "LSAN_OPTIONS=suppressions=${CMAKE_CURRENT_SOURCE_DIR}/sanitizers/lsan_zenoh.supp")
    else()
        message(STATUS "axis-6: openssl binary or zenoh-cpp not found — skipping mesh_auth_classifier_ci_fixture")
    endif()

    # ── SomeipAvailabilityProbe unit tests (docs/SCE_AXIS_6_PATTERNS.md
    # A6-002) ─────────────────────────────────────────────────────
    # Pins the defensive-idempotent absorption contract: the probe
    # invokes the caller's handler at least once even if the mock
    # vsomeip application skips the initial-edge callback, and the
    # synthesized invocation carries the application's current
    # is_available() state. The probe is templated on the application
    # type, so the test uses a minimal duck-typed mock — no vsomeip
    # runtime dependency in the test fixture itself, only the vsomeip
    # headers (availability_handler_t typedef + primitive_types).
    # vsomeip3 major-version pinned — vsomeip4 would require review
    # of the initial-callback contract against the probe's assumptions.
    find_package(vsomeip3 3 QUIET)
    if(vsomeip3_FOUND)
        add_executable(mesh_someip_availability_probe_test
            mesh/SomeipAvailabilityProbeTest.cpp
        )

        target_include_directories(mesh_someip_availability_probe_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
        )

        target_link_libraries(mesh_someip_availability_probe_test
            PRIVATE
                vsomeip3
                GTest::gtest_main
                pthread
        )

        target_compile_features(mesh_someip_availability_probe_test PRIVATE cxx_std_17)

        apply_sce_test_policies(mesh_someip_availability_probe_test)

        add_test(NAME mesh_someip_availability_probe_verification
                 COMMAND mesh_someip_availability_probe_test)
    else()
        message(STATUS "axis-6: vsomeip3 not found — skipping mesh_someip_availability_probe_test")
    endif()

    # ── DedupRouter unit tests (SCE_MESH.md §10.5) ────────────────
    # Pure unit tests on the per-sender envelope-id dedup filter.
    # Verifies the 256-entry ring (novel/repeat/eviction/wrap) plus
    # per-sender isolation and a concurrency smoke. Wired into the
    # generated TransportRouter via `admit(env.source, env.id)` on
    # inbound call sites for transports whose descriptor declares
    # `supplies_dedup: false`.
    add_executable(mesh_dedup_router_test
        mesh/DedupRouterTest.cpp
    )

    target_include_directories(mesh_dedup_router_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
    )

    target_link_libraries(mesh_dedup_router_test
        PRIVATE
            sce_mesh_common
            GTest::gtest_main
            pthread
    )

    target_compile_features(mesh_dedup_router_test PRIVATE cxx_std_17)

    apply_sce_test_policies(mesh_dedup_router_test)

    add_test(NAME mesh_dedup_router_verification COMMAND mesh_dedup_router_test)

    # ── OrderingBuffer unit tests (SCE_MESH.md §10.6) ─────────────
    # Pure unit tests on the per-sender sequence-ordered admit layer.
    # Verifies in-order/out-of-order/gap-fill/gap-timeout/per-sender
    # isolation/straggler-drop invariants. Wired into the generated
    # TransportRouter via `admit(env.source, env)` on inbound call
    # sites for bindings whose `ordering: required` combines with a
    # transport whose descriptor declares `supplies_ordering: false`.
    add_executable(mesh_ordering_buffer_test
        mesh/OrderingBufferTest.cpp
    )

    target_include_directories(mesh_ordering_buffer_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
    )

    target_link_libraries(mesh_ordering_buffer_test
        PRIVATE
            sce_mesh_common
            GTest::gtest_main
            pthread
    )

    target_compile_features(mesh_ordering_buffer_test PRIVATE cxx_std_17)

    apply_sce_test_policies(mesh_ordering_buffer_test)

    add_test(NAME mesh_ordering_buffer_verification COMMAND mesh_ordering_buffer_test)

    # ── OutboundBuffer unit tests (SCE_MESH.md §10.10 + §16.7 row 9) ─
    # Pure unit tests on the per-target readiness-gated admit layer.
    # Sibling DedupRouter / OrderingBuffer tests cover their inbound
    # invariants; this one closes the §16.7 row 9 BACKPRESSURE_DROP
    # contract — admit-time overflow drops the newest envelope and
    # raises `error.communication`. Existing E2E fixtures
    # (`mesh_someip_late_boot`, `mesh_zenoh_publisher_first`) cover
    # the orthogonal drain path; together they pin the §10.10
    # OutboundBuffer surface end-to-end.
    add_executable(mesh_outbound_buffer_test
        mesh/OutboundBufferTest.cpp
    )

    target_include_directories(mesh_outbound_buffer_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
    )

    target_link_libraries(mesh_outbound_buffer_test
        PRIVATE
            sce_mesh_common
            GTest::gtest_main
            pthread
    )

    target_compile_features(mesh_outbound_buffer_test PRIVATE cxx_std_17)

    apply_sce_test_policies(mesh_outbound_buffer_test)

    add_test(NAME mesh_outbound_buffer_verification COMMAND mesh_outbound_buffer_test)

    # ── custom_tcp SocketOptions unit tests (SCE_MESH.md §16.8.3) ──
    # The behavioural half of the socket-layer configuration axis: that
    # the runtime applies the options it is handed. The declaration half
    # (a deploy.yaml value reaching the emitted struct) is the cargo test
    # `mesh_custom_tcp_socket_options`; the two can fail independently,
    # so both exist.
    add_executable(mesh_custom_tcp_socket_options_test
        mesh/CustomTcpSocketOptionsTest.cpp
    )

    target_include_directories(mesh_custom_tcp_socket_options_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
    )

    target_link_libraries(mesh_custom_tcp_socket_options_test
        PRIVATE
            sce_mesh_common
            GTest::gtest_main
            pthread
    )

    target_compile_features(mesh_custom_tcp_socket_options_test PRIVATE cxx_std_20)

    apply_sce_test_policies(mesh_custom_tcp_socket_options_test)

    add_test(NAME mesh_custom_tcp_socket_options_verification
             COMMAND mesh_custom_tcp_socket_options_test)

    # ── CommunicationError unit tests (SCE_MESH.md §10.10.1 + §16.7) ─
    # Pins the JSON render shape of `_event.data` for `error.communication`
    # events raised by the mesh runtime (MISSING_SEQUENCE + ORDERING_GAP
    # catalogue rows landed in Session E2-gap-recovery). Byte-compare on
    # the happy path keeps the SCXML-author-visible contract stable.
    add_executable(mesh_communication_error_test
        mesh/CommunicationErrorTest.cpp
    )

    target_include_directories(mesh_communication_error_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
    )

    target_link_libraries(mesh_communication_error_test
        PRIVATE
            sce_mesh_common
            GTest::gtest_main
            pthread
    )

    target_compile_features(mesh_communication_error_test PRIVATE cxx_std_17)

    apply_sce_test_policies(mesh_communication_error_test)

    add_test(NAME mesh_communication_error_verification COMMAND mesh_communication_error_test)

    # ── ReasonCode catalog drift guard (SCE_MESH.md §16.7) ──────────
    # Reason-code catalog binding — binds the C++ `SCE::Mesh::ReasonCode`
    # enum + `kReasonCodeTable` to the canonical §16.7 row catalog
    # in SCE_MESH.md. Bidirectional coverage assertion catches:
    #   * variant added to enum without §16.7 row update;
    #   * §16.7 row added without enum extension;
    #   * wire-string typo in either source.
    # The markdown path is injected via target_compile_definitions so
    # the test resolves the file deterministically regardless of the
    # working directory (CI, packaging, IDE runs).
    add_executable(mesh_reason_code_catalog_test
        mesh/ReasonCodeCatalogTest.cpp
    )

    target_include_directories(mesh_reason_code_catalog_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
    )

    target_link_libraries(mesh_reason_code_catalog_test
        PRIVATE
            sce_mesh_common
            GTest::gtest_main
            pthread
    )

    target_compile_features(mesh_reason_code_catalog_test PRIVATE cxx_std_17)

    target_compile_definitions(mesh_reason_code_catalog_test
        PRIVATE
            SCE_MESH_MD_PATH="${CMAKE_SOURCE_DIR}/SCE_MESH.md"
    )

    apply_sce_test_policies(mesh_reason_code_catalog_test)

    add_test(NAME mesh_reason_code_catalog_verification COMMAND mesh_reason_code_catalog_test)

    # ── MeshDeadlineScheduler unit tests (SCE_MESH.md §9.5 F3c) ───
    # Pure unit tests on the callback-based deadline timer that backs
    # the mesh-rpc `_mesh_deadline_ms` behaviour. Covers the register /
    # fire / cancel / shutdown state machine plus the cancel-vs-fire
    # race and re-register-after-cancel edge cases. No codegen, no
    # transport — the end-to-end wiring is exercised in F4.
    add_executable(mesh_deadline_scheduler_test
        mesh/MeshDeadlineSchedulerTest.cpp
    )

    target_include_directories(mesh_deadline_scheduler_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
    )

    target_link_libraries(mesh_deadline_scheduler_test
        PRIVATE
            sce_mesh_common
            GTest::gtest_main
            pthread
    )

    target_compile_features(mesh_deadline_scheduler_test PRIVATE cxx_std_17)

    apply_sce_test_policies(mesh_deadline_scheduler_test)

    add_test(NAME mesh_deadline_scheduler_verification COMMAND mesh_deadline_scheduler_test)

    # ── MeshDispatch unit tests (SCE_MESH.md §9.5 G1) ────────────
    # Pure unit tests on the envelope-to-engine dispatch helper used by
    # ShmChannel::drain, TransportRouter::onIncoming, and local
    # TransportRouter::route_send. Locks in the inbound-vs-outbound pattern
    # classification so `RpcRequest` stays deliverable on the receiver side
    # (a prior revision rejected it, which hid the §9.5 request path end-to-end).
    add_executable(mesh_dispatch_test
        mesh/MeshDispatchTest.cpp
    )

    target_include_directories(mesh_dispatch_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
    )

    target_link_libraries(mesh_dispatch_test
        PRIVATE
            sce_mesh_common
            # ChildSessionAdapter.h's §9.3 envelope builders stamp a UUID v7
            # id, which lives in sce_base. sce_mesh_common deliberately does
            # not pull sce_base in (it links only header-only sce_core plus
            # tinycbor), so the dependency is declared here rather than
            # widening the library's tier for one test.
            sce_base
            GTest::gtest_main
            pthread
    )

    target_compile_features(mesh_dispatch_test PRIVATE cxx_std_17)

    apply_sce_test_policies(mesh_dispatch_test)

    add_test(NAME mesh_dispatch_verification COMMAND mesh_dispatch_test)

    # SCE_MESH.md §16.5: ParallelCompletionTracker — per-`<parallel>` root
    # partition barrier primitive used by the rule-12 codegen branch in
    # `mesh/cpp/parallel_final.jinja2`. Header-only; this suite exercises
    # threshold / single-shot / reset semantics without runtime wiring.
    add_executable(mesh_parallel_completion_tracker_test
        mesh/ParallelCompletionTrackerTest.cpp
    )

    target_include_directories(mesh_parallel_completion_tracker_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
    )

    target_link_libraries(mesh_parallel_completion_tracker_test
        PRIVATE
            sce_mesh_common
            GTest::gtest_main
            pthread
    )

    target_compile_features(mesh_parallel_completion_tracker_test PRIVATE cxx_std_17)

    apply_sce_test_policies(mesh_parallel_completion_tracker_test)

    add_test(NAME mesh_parallel_completion_tracker_verification
             COMMAND mesh_parallel_completion_tracker_test)

    # Generate brake_sm.h from brake.scxml
    add_custom_command(
        OUTPUT "${MESH_OUTPUT_DIR}/brake_sm.h"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_OUTPUT_DIR}"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake.scxml" -l cpp -o "${MESH_OUTPUT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy.yaml"
                --write-deps "${MESH_OUTPUT_DIR}/brake_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/brake.scxml" "${SCE_CODEGEN}"
        DEPFILE "${MESH_OUTPUT_DIR}/brake_sm.h.d"
        COMMENT "Mesh: Generating brake_sm.h"
        VERBATIM
    )

    # Generate motor_sm.h from motor.scxml
    add_custom_command(
        OUTPUT "${MESH_OUTPUT_DIR}/motor_sm.h"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor.scxml" -l cpp -o "${MESH_OUTPUT_DIR}"
                --write-deps "${MESH_OUTPUT_DIR}/motor_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/motor.scxml" "${SCE_CODEGEN}"
        DEPFILE "${MESH_OUTPUT_DIR}/motor_sm.h.d"
        COMMENT "Mesh: Generating motor_sm.h"
        VERBATIM
    )

    # Generate brake_transport.h from brake.scxml + deploy.yaml
    # DEPENDS on brake_sm.h to serialize after the first codegen run —
    # --deploy also regenerates brake_sm.h as a side effect, so parallel
    # builds must not race with the standalone brake_sm.h command above.
    add_custom_command(
        OUTPUT "${MESH_OUTPUT_DIR}/brake_transport.h"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake.scxml" -l cpp -o "${MESH_OUTPUT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy.yaml"
                --transport-only
                --write-deps "${MESH_OUTPUT_DIR}/brake_transport.h.d"
        DEPENDS "${MESH_TEST_DIR}/brake.scxml" "${MESH_TEST_DIR}/deploy.yaml"
                "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                "${MESH_OUTPUT_DIR}/brake_sm.h"
        DEPFILE "${MESH_OUTPUT_DIR}/brake_transport.h.d"
        COMMENT "Mesh: Generating brake_transport.h (local transport)"
        VERBATIM
    )

    add_executable(mesh_compile_test
        mesh/test_mesh_local.cpp
        "${MESH_OUTPUT_DIR}/brake_sm.h"
        "${MESH_OUTPUT_DIR}/motor_sm.h"
        "${MESH_OUTPUT_DIR}/brake_transport.h"
    )

    target_include_directories(mesh_compile_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
            ${MESH_OUTPUT_DIR}
    )

    target_link_libraries(mesh_compile_test
        PRIVATE
            sce_base
            sce_mesh_common
    )

    target_compile_features(mesh_compile_test PRIVATE cxx_std_17)
    sce_pin_cxx17(mesh_compile_test)

    apply_sce_test_policies(mesh_compile_test)

    add_test(NAME mesh_compile_verification COMMAND mesh_compile_test)

    # ── SCE_MESH §14: first-party `partitions:` end-to-end fixture ──
    # Drives a deploy.yaml whose `partitions:` block fully covers the
    # two `<parallel>` regions of motor_partition.scxml through the
    # full sce-build pipeline:
    #   parse_deploy → validate_partitions_schema (rules 6-10)
    #                → validate_partitions_against_scxml (rules 1/2/11)
    #                → compile_mesh_transport
    # Every §14 rule was covered only by Rust unit tests until this
    # fixture; the C++ build never exercised the CLI path. A regression
    # that bypassed `validate_partitions_against_scxml` from sce-codegen
    # would have shipped silently. Build success here proves the chain.
    #
    # The machine declares no bindings, so compile_mesh_transport's
    # `lib.rs:1092` early-return fires after validation succeeds and no
    # transport.h is emitted — this fixture's job is the §14 path, not
    # another transport-router compile.
    set(MESH_PARTITIONS_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/mesh_partitions_generated")

    add_custom_command(
        OUTPUT "${MESH_PARTITIONS_OUTPUT_DIR}/motor_partition_sm.h"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_PARTITIONS_OUTPUT_DIR}"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_partition.scxml"
                -l cpp -o "${MESH_PARTITIONS_OUTPUT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_partitions.yaml"
                --write-deps "${MESH_PARTITIONS_OUTPUT_DIR}/motor_partition_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/motor_partition.scxml"
                "${MESH_TEST_DIR}/deploy_partitions.yaml"
                "${SCE_CODEGEN}"
        DEPFILE "${MESH_PARTITIONS_OUTPUT_DIR}/motor_partition_sm.h.d"
        COMMENT "Mesh partitions: Generating motor_partition_sm.h via §14 deploy validation"
        VERBATIM
    )

    add_executable(mesh_partitions_test
        mesh/test_mesh_partitions.cpp
        "${MESH_PARTITIONS_OUTPUT_DIR}/motor_partition_sm.h"
    )

    target_include_directories(mesh_partitions_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
            ${MESH_PARTITIONS_OUTPUT_DIR}
    )

    target_link_libraries(mesh_partitions_test
        PRIVATE
            sce_base
    )

    target_compile_features(mesh_partitions_test PRIVATE cxx_std_17)
    # This target compiles the only generated statechart in the tree whose
    # source has parallel states, so it is the one place the done.state
    # branch of process_transition reaches a compiler. That branch emitted
    # `std::string_view::starts_with` — C++20 — and shipped green because
    # nothing here built at the standard the runtime advertises.
    sce_pin_cxx17(mesh_partitions_test)

    apply_sce_test_policies(mesh_partitions_test)

    add_test(NAME mesh_partitions_verification COMMAND mesh_partitions_test)

    # ── SCE_MESH §14 rule 12 / §16.5: inter-partition wire-21 E2E ──
    # End-to-end verification that a non-root partition's region-final
    # entry actually delivers a `ParallelRegionDone` envelope to the
    # root partition over the inter-partition shm channel — closing the
    # rule-12 atomic bundle's "Open runtime gap" carve-out (SCE_MESH.md
    # §14 L2844). Two partition-specific binaries are built from the
    # same `motor_partition.scxml` with `--partition motor_left`
    # (Root) and `--partition motor_right` (NonRoot); a driver fork+
    # execs both and asserts both return 0.
    #
    # The Root binary's TransportRouter ctor opens
    # `/sce_p21_motor_right_motor_left` in `Mode::Open`; the NonRoot
    # binary's ctor creates the same shm name in `Mode::Create` and
    # installs the wire-21 send callback on the SM via
    # `setParallelRegionDoneCallback`. Until this fixture landed, the
    # NonRoot's `sendParallelRegionDone` body was a fail-loud
    # `throw std::runtime_error(...)` — the runtime side of rule 12
    # had no end-to-end consumer (`feedback_built_but_unconsumed`).
    set(MESH_PARTITION_RULE12_LEFT_DIR  "${CMAKE_CURRENT_BINARY_DIR}/mesh_partition_rule12_left_generated")
    set(MESH_PARTITION_RULE12_RIGHT_DIR "${CMAKE_CURRENT_BINARY_DIR}/mesh_partition_rule12_right_generated")

    # Pattern C split (mesh_open_issues.md Issue 2 follow-up): each
    # (sm.h + transport.h) emit is two commands so a transport-only
    # edit (deploy.yaml change) doesn't churn the sm.h mtime and
    # cascade rebuilds. SM emit stays --deploy-aware so its model
    # mutations (server-response, partition, queue capacity) match
    # what the transport emit produces.
    add_custom_command(
        OUTPUT  "${MESH_PARTITION_RULE12_LEFT_DIR}/motor_partition_sm.h"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_PARTITION_RULE12_LEFT_DIR}"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_partition.scxml"
                -l cpp -o "${MESH_PARTITION_RULE12_LEFT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_partitions.yaml"
                --partition motor_left
                --write-deps "${MESH_PARTITION_RULE12_LEFT_DIR}/motor_partition_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/motor_partition.scxml"
                "${MESH_TEST_DIR}/deploy_partitions.yaml"
                "${SCE_CODEGEN}"
        DEPFILE "${MESH_PARTITION_RULE12_LEFT_DIR}/motor_partition_sm.h.d"
        COMMENT "Mesh rule 12: Generating motor_partition_sm.h (Root partition motor_left)"
        VERBATIM
    )

    add_custom_command(
        OUTPUT  "${MESH_PARTITION_RULE12_LEFT_DIR}/motor_partition_transport.h"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_partition.scxml"
                -l cpp -o "${MESH_PARTITION_RULE12_LEFT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_partitions.yaml"
                --transport-only
                --partition motor_left
                --write-deps "${MESH_PARTITION_RULE12_LEFT_DIR}/motor_partition_transport.h.d"
        DEPENDS "${MESH_TEST_DIR}/motor_partition.scxml"
                "${MESH_TEST_DIR}/deploy_partitions.yaml"
                "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                "${MESH_PARTITION_RULE12_LEFT_DIR}/motor_partition_sm.h"
        DEPFILE "${MESH_PARTITION_RULE12_LEFT_DIR}/motor_partition_transport.h.d"
        COMMENT "Mesh rule 12: Generating motor_partition_transport.h (Root partition motor_left)"
        VERBATIM
    )

    add_custom_command(
        OUTPUT  "${MESH_PARTITION_RULE12_RIGHT_DIR}/motor_partition_sm.h"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_PARTITION_RULE12_RIGHT_DIR}"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_partition.scxml"
                -l cpp -o "${MESH_PARTITION_RULE12_RIGHT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_partitions.yaml"
                --partition motor_right
                --write-deps "${MESH_PARTITION_RULE12_RIGHT_DIR}/motor_partition_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/motor_partition.scxml"
                "${MESH_TEST_DIR}/deploy_partitions.yaml"
                "${SCE_CODEGEN}"
        DEPFILE "${MESH_PARTITION_RULE12_RIGHT_DIR}/motor_partition_sm.h.d"
        COMMENT "Mesh rule 12: Generating motor_partition_sm.h (NonRoot partition motor_right)"
        VERBATIM
    )

    add_custom_command(
        OUTPUT  "${MESH_PARTITION_RULE12_RIGHT_DIR}/motor_partition_transport.h"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_partition.scxml"
                -l cpp -o "${MESH_PARTITION_RULE12_RIGHT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_partitions.yaml"
                --transport-only
                --partition motor_right
                --write-deps "${MESH_PARTITION_RULE12_RIGHT_DIR}/motor_partition_transport.h.d"
        DEPENDS "${MESH_TEST_DIR}/motor_partition.scxml"
                "${MESH_TEST_DIR}/deploy_partitions.yaml"
                "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                "${MESH_PARTITION_RULE12_RIGHT_DIR}/motor_partition_sm.h"
        DEPFILE "${MESH_PARTITION_RULE12_RIGHT_DIR}/motor_partition_transport.h.d"
        COMMENT "Mesh rule 12: Generating motor_partition_transport.h (NonRoot partition motor_right)"
        VERBATIM
    )

    add_executable(mesh_partition_rule12_left_proc
        mesh/test_mesh_partition_rule12_left_proc.cpp
        "${MESH_PARTITION_RULE12_LEFT_DIR}/motor_partition_sm.h"
        "${MESH_PARTITION_RULE12_LEFT_DIR}/motor_partition_transport.h"
    )
    target_include_directories(mesh_partition_rule12_left_proc
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
            ${MESH_PARTITION_RULE12_LEFT_DIR}
    )
    target_link_libraries(mesh_partition_rule12_left_proc
        PRIVATE
            sce_base
            sce_mesh_common
    )
    target_compile_features(mesh_partition_rule12_left_proc PRIVATE cxx_std_17)
    apply_sce_test_policies(mesh_partition_rule12_left_proc)

    add_executable(mesh_partition_rule12_right_proc
        mesh/test_mesh_partition_rule12_right_proc.cpp
        "${MESH_PARTITION_RULE12_RIGHT_DIR}/motor_partition_sm.h"
        "${MESH_PARTITION_RULE12_RIGHT_DIR}/motor_partition_transport.h"
    )
    target_include_directories(mesh_partition_rule12_right_proc
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
            ${MESH_PARTITION_RULE12_RIGHT_DIR}
    )
    target_link_libraries(mesh_partition_rule12_right_proc
        PRIVATE
            sce_base
            sce_mesh_common
    )
    target_compile_features(mesh_partition_rule12_right_proc PRIVATE cxx_std_17)
    apply_sce_test_policies(mesh_partition_rule12_right_proc)

    add_executable(mesh_partition_rule12_e2e
        mesh/test_mesh_partition_rule12_e2e.cpp
    )
    target_compile_definitions(mesh_partition_rule12_e2e
        PRIVATE
            MESH_PARTITION_RULE12_LEFT_BIN="$<TARGET_FILE:mesh_partition_rule12_left_proc>"
            MESH_PARTITION_RULE12_RIGHT_BIN="$<TARGET_FILE:mesh_partition_rule12_right_proc>"
    )
    add_dependencies(mesh_partition_rule12_e2e
        mesh_partition_rule12_left_proc
        mesh_partition_rule12_right_proc
    )
    target_compile_features(mesh_partition_rule12_e2e PRIVATE cxx_std_17)
    apply_sce_test_policies(mesh_partition_rule12_e2e)

    add_test(NAME mesh_partition_rule12_e2e COMMAND mesh_partition_rule12_e2e)

    # ── SCE_MESH §16.5 L3500 — barrier-timeout runtime E2E pair ──
    # The rule-12 wire-21 test above exercises the happy-path
    # convergence with a generous (5000 ms) timeout so the timer never
    # arms to completion. This pair fires the §16.7 row 6 raise path
    # and the cancel-wins-race invariant under a CI-fast 150 ms timeout.
    #
    # Shared codegen: `deploy_partitions_barrier_fast.yaml` drives both
    # Root and NonRoot partition headers, differing from the rule-12
    # generation only in the baked `barrier_timeout_ms` constant. The
    # `error.communication` transition lives on `motor_partition.scxml`
    # itself and is required by the §16.5 observability guard
    # (`reject_barrier_timeout_without_handler` in generator.rs) — a
    # machine with a finite `barrier_timeout_ms:` and no handler fails
    # codegen before this block would get a chance to build.
    set(MESH_BARRIER_TIMEOUT_LEFT_DIR
        "${CMAKE_CURRENT_BINARY_DIR}/mesh_barrier_timeout_left_generated")
    set(MESH_BARRIER_TIMEOUT_RIGHT_DIR
        "${CMAKE_CURRENT_BINARY_DIR}/mesh_barrier_timeout_right_generated")

    add_custom_command(
        OUTPUT  "${MESH_BARRIER_TIMEOUT_LEFT_DIR}/motor_partition_sm.h"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_BARRIER_TIMEOUT_LEFT_DIR}"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_partition.scxml"
                -l cpp -o "${MESH_BARRIER_TIMEOUT_LEFT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_partitions_barrier_fast.yaml"
                --partition motor_left
                --write-deps "${MESH_BARRIER_TIMEOUT_LEFT_DIR}/motor_partition_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/motor_partition.scxml"
                "${MESH_TEST_DIR}/deploy_partitions_barrier_fast.yaml"
                "${SCE_CODEGEN}"
        DEPFILE "${MESH_BARRIER_TIMEOUT_LEFT_DIR}/motor_partition_sm.h.d"
        COMMENT "Mesh §16.5 barrier-timeout: Generating motor_partition_sm.h (Root motor_left, fast)"
        VERBATIM
    )

    add_custom_command(
        OUTPUT  "${MESH_BARRIER_TIMEOUT_LEFT_DIR}/motor_partition_transport.h"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_partition.scxml"
                -l cpp -o "${MESH_BARRIER_TIMEOUT_LEFT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_partitions_barrier_fast.yaml"
                --transport-only
                --partition motor_left
                --write-deps "${MESH_BARRIER_TIMEOUT_LEFT_DIR}/motor_partition_transport.h.d"
        DEPENDS "${MESH_TEST_DIR}/motor_partition.scxml"
                "${MESH_TEST_DIR}/deploy_partitions_barrier_fast.yaml"
                "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                "${MESH_BARRIER_TIMEOUT_LEFT_DIR}/motor_partition_sm.h"
        DEPFILE "${MESH_BARRIER_TIMEOUT_LEFT_DIR}/motor_partition_transport.h.d"
        COMMENT "Mesh §16.5 barrier-timeout: Generating motor_partition_transport.h (Root motor_left, fast)"
        VERBATIM
    )

    add_custom_command(
        OUTPUT  "${MESH_BARRIER_TIMEOUT_RIGHT_DIR}/motor_partition_sm.h"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_BARRIER_TIMEOUT_RIGHT_DIR}"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_partition.scxml"
                -l cpp -o "${MESH_BARRIER_TIMEOUT_RIGHT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_partitions_barrier_fast.yaml"
                --partition motor_right
                --write-deps "${MESH_BARRIER_TIMEOUT_RIGHT_DIR}/motor_partition_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/motor_partition.scxml"
                "${MESH_TEST_DIR}/deploy_partitions_barrier_fast.yaml"
                "${SCE_CODEGEN}"
        DEPFILE "${MESH_BARRIER_TIMEOUT_RIGHT_DIR}/motor_partition_sm.h.d"
        COMMENT "Mesh §16.5 barrier-timeout: Generating motor_partition_sm.h (NonRoot motor_right, fast)"
        VERBATIM
    )

    add_custom_command(
        OUTPUT  "${MESH_BARRIER_TIMEOUT_RIGHT_DIR}/motor_partition_transport.h"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_partition.scxml"
                -l cpp -o "${MESH_BARRIER_TIMEOUT_RIGHT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_partitions_barrier_fast.yaml"
                --transport-only
                --partition motor_right
                --write-deps "${MESH_BARRIER_TIMEOUT_RIGHT_DIR}/motor_partition_transport.h.d"
        DEPENDS "${MESH_TEST_DIR}/motor_partition.scxml"
                "${MESH_TEST_DIR}/deploy_partitions_barrier_fast.yaml"
                "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                "${MESH_BARRIER_TIMEOUT_RIGHT_DIR}/motor_partition_sm.h"
        DEPFILE "${MESH_BARRIER_TIMEOUT_RIGHT_DIR}/motor_partition_transport.h.d"
        COMMENT "Mesh §16.5 barrier-timeout: Generating motor_partition_transport.h (NonRoot motor_right, fast)"
        VERBATIM
    )

    add_executable(mesh_barrier_timeout_root_proc
        mesh/test_mesh_barrier_timeout_root_proc.cpp
        "${MESH_BARRIER_TIMEOUT_LEFT_DIR}/motor_partition_sm.h"
        "${MESH_BARRIER_TIMEOUT_LEFT_DIR}/motor_partition_transport.h"
    )
    target_include_directories(mesh_barrier_timeout_root_proc
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
            ${MESH_BARRIER_TIMEOUT_LEFT_DIR}
    )
    target_link_libraries(mesh_barrier_timeout_root_proc
        PRIVATE
            sce_base
            sce_mesh_common
    )
    target_compile_features(mesh_barrier_timeout_root_proc PRIVATE cxx_std_17)
    apply_sce_test_policies(mesh_barrier_timeout_root_proc)

    add_executable(mesh_barrier_timeout_right_proc
        mesh/test_mesh_barrier_timeout_right_proc.cpp
        "${MESH_BARRIER_TIMEOUT_RIGHT_DIR}/motor_partition_sm.h"
        "${MESH_BARRIER_TIMEOUT_RIGHT_DIR}/motor_partition_transport.h"
    )
    target_include_directories(mesh_barrier_timeout_right_proc
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
            ${MESH_BARRIER_TIMEOUT_RIGHT_DIR}
    )
    target_link_libraries(mesh_barrier_timeout_right_proc
        PRIVATE
            sce_base
            sce_mesh_common
    )
    target_compile_features(mesh_barrier_timeout_right_proc PRIVATE cxx_std_17)
    apply_sce_test_policies(mesh_barrier_timeout_right_proc)

    add_executable(mesh_barrier_timeout_e2e
        mesh/test_mesh_barrier_timeout_e2e.cpp
    )
    target_compile_definitions(mesh_barrier_timeout_e2e
        PRIVATE
            MESH_BARRIER_TIMEOUT_ROOT_BIN="$<TARGET_FILE:mesh_barrier_timeout_root_proc>"
            MESH_BARRIER_TIMEOUT_RIGHT_BIN="$<TARGET_FILE:mesh_barrier_timeout_right_proc>"
    )
    add_dependencies(mesh_barrier_timeout_e2e
        mesh_barrier_timeout_root_proc
        mesh_barrier_timeout_right_proc
    )
    target_compile_features(mesh_barrier_timeout_e2e PRIVATE cxx_std_17)
    apply_sce_test_policies(mesh_barrier_timeout_e2e)

    add_test(NAME mesh_barrier_timeout_fires_e2e
             COMMAND mesh_barrier_timeout_e2e fires)
    add_test(NAME mesh_barrier_timeout_cancels_e2e
             COMMAND mesh_barrier_timeout_e2e cancels)
    # The rule-12 e2e, the barrier-timeout fires/cancels pair, and any
    # future motor-partition harness share the same shm channel name
    # (`/sce_p21_motor_right_motor_left`, derived from the partition
    # identifiers that both deploy yamls reuse). `ctest -j` against
    # that shared segment races on `shm_unlink` and `Mode::Create`.
    # The RESOURCE_LOCK serialises the family so each test owns the
    # channel end-to-end; test-wise cost is a few seconds on parallel
    # runs, correctness-wise the lock removes a latent CI flake.
    set_tests_properties(
        mesh_partition_rule12_e2e
        mesh_barrier_timeout_fires_e2e
        mesh_barrier_timeout_cancels_e2e
        PROPERTIES RESOURCE_LOCK motor_partition_shm
    )

    # ── SCE_MESH §9.5: <invoke type="sce:mesh-rpc"> plumbing verification ──
    # Byte-golden regression for `invoke_sites` threading from analyzer
    # through codegen::TargetContext into the transport template. A drop of
    # that plumbing (e.g. TargetContext field removed) yields a router
    # without invokeMeshRpc / cancelMeshRpc — this test fails to link.
    set(MESH_INVOKE_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/mesh_invoke_generated")

    add_custom_command(
        OUTPUT "${MESH_INVOKE_OUTPUT_DIR}/brake_invoke_sm.h"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_INVOKE_OUTPUT_DIR}"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_invoke.scxml" -l cpp -o "${MESH_INVOKE_OUTPUT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_invoke.yaml"
                --write-deps "${MESH_INVOKE_OUTPUT_DIR}/brake_invoke_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/brake_invoke.scxml" "${SCE_CODEGEN}"
        DEPFILE "${MESH_INVOKE_OUTPUT_DIR}/brake_invoke_sm.h.d"
        COMMENT "Mesh: Generating brake_invoke_sm.h (§9.5 invoke)"
        VERBATIM
    )

    add_custom_command(
        OUTPUT "${MESH_INVOKE_OUTPUT_DIR}/motor_invoke_sm.h"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_invoke.scxml" -l cpp -o "${MESH_INVOKE_OUTPUT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_invoke.yaml"
                --write-deps "${MESH_INVOKE_OUTPUT_DIR}/motor_invoke_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/motor_invoke.scxml" "${SCE_CODEGEN}"
        DEPFILE "${MESH_INVOKE_OUTPUT_DIR}/motor_invoke_sm.h.d"
        COMMENT "Mesh: Generating motor_invoke_sm.h (§9.5 invoke receiver)"
        VERBATIM
    )

    add_custom_command(
        OUTPUT "${MESH_INVOKE_OUTPUT_DIR}/brake_invoke_transport.h"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_invoke.scxml" -l cpp -o "${MESH_INVOKE_OUTPUT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_invoke.yaml"
                --transport-only
                --write-deps "${MESH_INVOKE_OUTPUT_DIR}/brake_invoke_transport.h.d"
        DEPENDS "${MESH_TEST_DIR}/brake_invoke.scxml" "${MESH_TEST_DIR}/deploy_invoke.yaml"
                "${MESH_TEST_DIR}/motor_invoke.scxml"
                "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                "${MESH_INVOKE_OUTPUT_DIR}/brake_invoke_sm.h"
        DEPFILE "${MESH_INVOKE_OUTPUT_DIR}/brake_invoke_transport.h.d"
        COMMENT "Mesh: Generating brake_invoke_transport.h (§9.5 invoke plumbing)"
        VERBATIM
    )

    # motor_invoke_transport.h: generated from the same deploy file so
    # the §9.5 round-trip test can host two linked routers in-process.
    # The deploy YAML declares a motor_invoke → #brake_invoke local
    # binding; without this rule, the receiver side of the round-trip
    # has no TransportRouter class to instantiate.
    add_custom_command(
        OUTPUT "${MESH_INVOKE_OUTPUT_DIR}/motor_invoke_transport.h"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_invoke.scxml" -l cpp -o "${MESH_INVOKE_OUTPUT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_invoke.yaml"
                --transport-only
                --write-deps "${MESH_INVOKE_OUTPUT_DIR}/motor_invoke_transport.h.d"
        DEPENDS "${MESH_TEST_DIR}/motor_invoke.scxml" "${MESH_TEST_DIR}/deploy_invoke.yaml"
                "${MESH_TEST_DIR}/brake_invoke.scxml"
                "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                "${MESH_INVOKE_OUTPUT_DIR}/motor_invoke_sm.h"
        DEPFILE "${MESH_INVOKE_OUTPUT_DIR}/motor_invoke_transport.h.d"
        COMMENT "Mesh: Generating motor_invoke_transport.h (§9.5 invoke reply path)"
        VERBATIM
    )

    add_executable(mesh_invoke_compile_test
        mesh/test_mesh_invoke_compile.cpp
        "${MESH_INVOKE_OUTPUT_DIR}/brake_invoke_sm.h"
        "${MESH_INVOKE_OUTPUT_DIR}/motor_invoke_sm.h"
        "${MESH_INVOKE_OUTPUT_DIR}/brake_invoke_transport.h"
    )

    target_include_directories(mesh_invoke_compile_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
            ${MESH_INVOKE_OUTPUT_DIR}
    )

    target_link_libraries(mesh_invoke_compile_test
        PRIVATE
            sce_base
            sce_mesh_common
    )

    target_compile_features(mesh_invoke_compile_test PRIVATE cxx_std_17)
    sce_pin_cxx17(mesh_invoke_compile_test)

    apply_sce_test_policies(mesh_invoke_compile_test)

    add_test(NAME mesh_invoke_compile_verification COMMAND mesh_invoke_compile_test)

    # ── SCE_MESH §9.6 Session F scaffold — SESSION_F_NOT_IMPLEMENTED ─
    # Closes the silent-broken window declared in SCE_MESH.md §9.6
    # line 1396: documents using `<invoke type="scxml" src="#peer">`
    # parse cleanly; until Session F lands the wire runtime (patterns
    # 14-20), the parent must raise `error.execution` with reason
    # `SESSION_F_NOT_IMPLEMENTED` per §10.7.1. Exercises the full
    # chain — classifier (`inject_partition_context_for` marks the
    # invoke via `remote_mesh_target`), template filter
    # (`scxml_remote` routes to the raise site), codegen emission
    # (entry-action `engine.raise(Error_execution, ...)`), and the
    # parent's runtime observation via a transition on
    # `error.execution`. No transport — the raise is local.
    set(MESH_SESSION_F_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/mesh_session_f_generated")

    add_custom_command(
        OUTPUT "${MESH_SESSION_F_OUTPUT_DIR}/parent_session_f_sm.h"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SESSION_F_OUTPUT_DIR}"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/parent_session_f.scxml"
                -l cpp -o "${MESH_SESSION_F_OUTPUT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_session_f.yaml"
                --write-deps "${MESH_SESSION_F_OUTPUT_DIR}/parent_session_f_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/parent_session_f.scxml"
                "${MESH_TEST_DIR}/worker_session_f.scxml"
                "${MESH_TEST_DIR}/deploy_session_f.yaml"
                "${SCE_CODEGEN}"
        DEPFILE "${MESH_SESSION_F_OUTPUT_DIR}/parent_session_f_sm.h.d"
        COMMENT "Mesh §9.6: Generating parent_session_f_sm.h (Session F scaffold)"
        VERBATIM
    )

    add_executable(mesh_session_f_not_implemented_test
        mesh/test_mesh_session_f_not_implemented.cpp
        "${MESH_SESSION_F_OUTPUT_DIR}/parent_session_f_sm.h"
    )

    target_include_directories(mesh_session_f_not_implemented_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
            ${MESH_SESSION_F_OUTPUT_DIR}
    )

    target_link_libraries(mesh_session_f_not_implemented_test
        PRIVATE
            sce_runtime
    )

    target_compile_features(mesh_session_f_not_implemented_test PRIVATE cxx_std_17)

    apply_sce_test_policies(mesh_session_f_not_implemented_test)

    add_test(NAME mesh_session_f_not_implemented_verification
             COMMAND mesh_session_f_not_implemented_test)

    # ── SCE_MESH §9.6 Session F wire round-trip ───────────────────
    # Exercises the transport-present half of the §9.6 line 1396
    # contract: parent emits wire-14 InvokeStart via shm, the worker's
    # `pumpScxmlInvokeRequests` routes it to `WorkerSessionHost`, which
    # instantiates an AOT child and answers wire-15 InvokeStarted then
    # wire-18 InvokeDone; the parent raises `done.invoke.*` and its
    # transition observes it. Single-process test — both routers share
    # the host process so the shm segments exchange data without fork.
    # Complements the transport-absent test above, which takes the
    # local SESSION_F_TRANSPORT_UNAVAILABLE raise instead.
    set(MESH_SESSION_F_WIRED_OUTPUT_DIR
        "${CMAKE_CURRENT_BINARY_DIR}/mesh_session_f_wired_generated")

    add_custom_command(
        OUTPUT "${MESH_SESSION_F_WIRED_OUTPUT_DIR}/parent_session_f_wired_sm.h"
               "${MESH_SESSION_F_WIRED_OUTPUT_DIR}/parent_session_f_wired_transport.h"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SESSION_F_WIRED_OUTPUT_DIR}"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/parent_session_f_wired.scxml"
                -l cpp -o "${MESH_SESSION_F_WIRED_OUTPUT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_session_f_wired.yaml"
                --write-deps "${MESH_SESSION_F_WIRED_OUTPUT_DIR}/parent_session_f_wired_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/parent_session_f_wired.scxml"
                "${MESH_TEST_DIR}/worker_session_f_wired.scxml"
                "${MESH_TEST_DIR}/deploy_session_f_wired.yaml"
                "${SCE_CODEGEN}"
        DEPFILE "${MESH_SESSION_F_WIRED_OUTPUT_DIR}/parent_session_f_wired_sm.h.d"
        COMMENT "Mesh §9.6: Generating parent_session_f_wired_{sm,transport}.h"
        VERBATIM
    )

    add_custom_command(
        OUTPUT "${MESH_SESSION_F_WIRED_OUTPUT_DIR}/worker_session_f_wired_sm.h"
               "${MESH_SESSION_F_WIRED_OUTPUT_DIR}/worker_session_f_wired_transport.h"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SESSION_F_WIRED_OUTPUT_DIR}"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/worker_session_f_wired.scxml"
                -l cpp -o "${MESH_SESSION_F_WIRED_OUTPUT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_session_f_wired.yaml"
                --write-deps "${MESH_SESSION_F_WIRED_OUTPUT_DIR}/worker_session_f_wired_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/parent_session_f_wired.scxml"
                "${MESH_TEST_DIR}/worker_session_f_wired.scxml"
                "${MESH_TEST_DIR}/deploy_session_f_wired.yaml"
                "${SCE_CODEGEN}"
        DEPFILE "${MESH_SESSION_F_WIRED_OUTPUT_DIR}/worker_session_f_wired_sm.h.d"
        COMMENT "Mesh §9.6: Generating worker_session_f_wired_{sm,transport}.h"
        VERBATIM
    )

    add_executable(mesh_session_f_wire_roundtrip_test
        mesh/test_mesh_session_f_wire_roundtrip.cpp
        "${MESH_SESSION_F_WIRED_OUTPUT_DIR}/parent_session_f_wired_sm.h"
        "${MESH_SESSION_F_WIRED_OUTPUT_DIR}/parent_session_f_wired_transport.h"
        "${MESH_SESSION_F_WIRED_OUTPUT_DIR}/worker_session_f_wired_sm.h"
        "${MESH_SESSION_F_WIRED_OUTPUT_DIR}/worker_session_f_wired_transport.h"
    )

    target_include_directories(mesh_session_f_wire_roundtrip_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
            ${MESH_SESSION_F_WIRED_OUTPUT_DIR}
    )

    target_link_libraries(mesh_session_f_wire_roundtrip_test
        PRIVATE
            sce_runtime
            sce_mesh_common
    )

    target_compile_features(mesh_session_f_wire_roundtrip_test PRIVATE cxx_std_17)

    apply_sce_test_policies(mesh_session_f_wire_roundtrip_test)

    add_test(NAME mesh_session_f_wire_roundtrip
             COMMAND mesh_session_f_wire_roundtrip_test)

    # ── SCE_MESH §9.6.6 rule 3 cross-partition synth override ─────
    # Closes §16.9 F exit criterion #4: "Inline <content> synthesized
    # child executes on a different partition." Unlike the §9.6.2
    # author-declared pair above (parent + worker both in source), here
    # the child is parser-emitted as a side-effect of parent codegen
    # (`sce-build/src/parser.rs:1607-1620`). The parser writes the synth
    # SCXML into the parent's `base_dir`; to keep tests/mesh/ source-tree
    # clean, we stage the parent + deploy.yaml into the build output
    # dir first so the synth materialises inside the build tree. Three
    # stages (staging → parent codegen → synth codegen) replace the
    # two author-declared `add_custom_command`s used by the wired
    # fixture — the synth `.scxml` is an intermediate output, not an
    # author input.
    set(MESH_SYNTH_OVERRIDE_OUTPUT_DIR
        "${CMAKE_CURRENT_BINARY_DIR}/mesh_synth_invoke_override_generated")

    # Stage 1: copy author inputs into the build dir.
    add_custom_command(
        OUTPUT "${MESH_SYNTH_OVERRIDE_OUTPUT_DIR}/parent_synth_inline.scxml"
               "${MESH_SYNTH_OVERRIDE_OUTPUT_DIR}/deploy_synth_inline.yaml"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SYNTH_OVERRIDE_OUTPUT_DIR}"
        COMMAND ${CMAKE_COMMAND} -E copy_if_different
                "${MESH_TEST_DIR}/parent_synth_inline.scxml"
                "${MESH_SYNTH_OVERRIDE_OUTPUT_DIR}/parent_synth_inline.scxml"
        COMMAND ${CMAKE_COMMAND} -E copy_if_different
                "${MESH_TEST_DIR}/deploy_synth_inline.yaml"
                "${MESH_SYNTH_OVERRIDE_OUTPUT_DIR}/deploy_synth_inline.yaml"
        DEPENDS "${MESH_TEST_DIR}/parent_synth_inline.scxml"
                "${MESH_TEST_DIR}/deploy_synth_inline.yaml"
        COMMENT "Mesh §9.6.6: Staging parent + deploy fixture for synth-invoke override"
        VERBATIM
    )

    # Stage 2: parent codegen. Parser side-effect materialises the synth
    # SCXML next to the staged parent. The synth path is declared as an
    # OUTPUT so the Stage 3 command can depend on it through CMake's
    # normal file-dependency graph.
    add_custom_command(
        OUTPUT "${MESH_SYNTH_OVERRIDE_OUTPUT_DIR}/parent_synth_inline_sm.h"
               "${MESH_SYNTH_OVERRIDE_OUTPUT_DIR}/parent_synth_inline_transport.h"
               "${MESH_SYNTH_OVERRIDE_OUTPUT_DIR}/parent_synth_inline__sce_synth_invoke__remote_inv.scxml"
        COMMAND "${SCE_CODEGEN}" generate
                "${MESH_SYNTH_OVERRIDE_OUTPUT_DIR}/parent_synth_inline.scxml"
                -l cpp -o "${MESH_SYNTH_OVERRIDE_OUTPUT_DIR}"
                --deploy "${MESH_SYNTH_OVERRIDE_OUTPUT_DIR}/deploy_synth_inline.yaml"
                --write-deps "${MESH_SYNTH_OVERRIDE_OUTPUT_DIR}/parent_synth_inline_sm.h.d"
        DEPENDS "${MESH_SYNTH_OVERRIDE_OUTPUT_DIR}/parent_synth_inline.scxml"
                "${MESH_SYNTH_OVERRIDE_OUTPUT_DIR}/deploy_synth_inline.yaml"
                "${SCE_CODEGEN}"
        DEPFILE "${MESH_SYNTH_OVERRIDE_OUTPUT_DIR}/parent_synth_inline_sm.h.d"
        COMMENT "Mesh §9.6.6: Generating parent_synth_inline_{sm,transport}.h (synth .scxml side-effect)"
        VERBATIM
    )

    # Stage 3: synth codegen with --deploy. Running against the
    # parser-emitted sibling engages `classify_remote_scxml_invokes` and
    # `collect_scxml_remote_peers` on the synth model; the §9.6.6
    # rule-3 infix inversion inside the latter adds the parent as an
    # inbound peer, flipping `is_remote_invoke_target=true`. Codegen
    # then picks the non-templated WorkerSessionHost shape. This is NOT
    # the `--as-child` path used by `w3c_static_generated`: that path
    # assumes a local-child shape, which would skip the mesh transport
    # emission this test exercises.
    add_custom_command(
        OUTPUT "${MESH_SYNTH_OVERRIDE_OUTPUT_DIR}/parent_synth_inline__sce_synth_invoke__remote_inv_sm.h"
               "${MESH_SYNTH_OVERRIDE_OUTPUT_DIR}/parent_synth_inline__sce_synth_invoke__remote_inv_transport.h"
        COMMAND "${SCE_CODEGEN}" generate
                "${MESH_SYNTH_OVERRIDE_OUTPUT_DIR}/parent_synth_inline__sce_synth_invoke__remote_inv.scxml"
                -l cpp -o "${MESH_SYNTH_OVERRIDE_OUTPUT_DIR}"
                --deploy "${MESH_SYNTH_OVERRIDE_OUTPUT_DIR}/deploy_synth_inline.yaml"
                --write-deps "${MESH_SYNTH_OVERRIDE_OUTPUT_DIR}/parent_synth_inline__sce_synth_invoke__remote_inv_sm.h.d"
        DEPENDS "${MESH_SYNTH_OVERRIDE_OUTPUT_DIR}/parent_synth_inline__sce_synth_invoke__remote_inv.scxml"
                "${MESH_SYNTH_OVERRIDE_OUTPUT_DIR}/deploy_synth_inline.yaml"
                "${SCE_CODEGEN}"
        DEPFILE "${MESH_SYNTH_OVERRIDE_OUTPUT_DIR}/parent_synth_inline__sce_synth_invoke__remote_inv_sm.h.d"
        COMMENT "Mesh §9.6.6: Generating synth sm+transport (rule-3 cross-partition override)"
        VERBATIM
    )

    add_executable(mesh_synth_invoke_override_e2e_test
        mesh/test_mesh_synth_invoke_override_e2e.cpp
        "${MESH_SYNTH_OVERRIDE_OUTPUT_DIR}/parent_synth_inline_sm.h"
        "${MESH_SYNTH_OVERRIDE_OUTPUT_DIR}/parent_synth_inline_transport.h"
        "${MESH_SYNTH_OVERRIDE_OUTPUT_DIR}/parent_synth_inline__sce_synth_invoke__remote_inv_sm.h"
        "${MESH_SYNTH_OVERRIDE_OUTPUT_DIR}/parent_synth_inline__sce_synth_invoke__remote_inv_transport.h"
    )

    target_include_directories(mesh_synth_invoke_override_e2e_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
            ${MESH_SYNTH_OVERRIDE_OUTPUT_DIR}
    )

    target_link_libraries(mesh_synth_invoke_override_e2e_test
        PRIVATE
            sce_runtime
            sce_mesh_common
    )

    target_compile_features(mesh_synth_invoke_override_e2e_test PRIVATE cxx_std_17)

    apply_sce_test_policies(mesh_synth_invoke_override_e2e_test)

    add_test(NAME mesh_synth_invoke_override_e2e
             COMMAND mesh_synth_invoke_override_e2e_test)

    # ── SCE_MESH §9.6.2 wire-16 + §9.6.4 finalize verification ─────
    # Parent has `datamodel="lua"` so its codegen emits script engine;
    # worker is a remote inbound peer so its codegen hits the new
    # `is_remote_invoke_target=true` shape — `<send target="#_parent">`
    # routes through `performMeshSend` → adapter's mesh callback →
    # wire-16 envelope. `<finalize>` fires before the parent's
    # transition selection (W3C §6.4.4) and the cond-guarded transition
    # observes the flipped datamodel variable.
    set(MESH_SESSION_F_EVENTFUL_OUTPUT_DIR
        "${CMAKE_CURRENT_BINARY_DIR}/mesh_session_f_eventful_generated")

    add_custom_command(
        OUTPUT "${MESH_SESSION_F_EVENTFUL_OUTPUT_DIR}/parent_session_f_eventful_sm.h"
               "${MESH_SESSION_F_EVENTFUL_OUTPUT_DIR}/parent_session_f_eventful_transport.h"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SESSION_F_EVENTFUL_OUTPUT_DIR}"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/parent_session_f_eventful.scxml"
                -l cpp -o "${MESH_SESSION_F_EVENTFUL_OUTPUT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_session_f_eventful.yaml"
                --write-deps "${MESH_SESSION_F_EVENTFUL_OUTPUT_DIR}/parent_session_f_eventful_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/parent_session_f_eventful.scxml"
                "${MESH_TEST_DIR}/worker_session_f_eventful.scxml"
                "${MESH_TEST_DIR}/deploy_session_f_eventful.yaml"
                "${SCE_CODEGEN}"
        DEPFILE "${MESH_SESSION_F_EVENTFUL_OUTPUT_DIR}/parent_session_f_eventful_sm.h.d"
        COMMENT "Mesh §9.6.2 eventful: Generating parent_session_f_eventful_{sm,transport}.h"
        VERBATIM
    )

    add_custom_command(
        OUTPUT "${MESH_SESSION_F_EVENTFUL_OUTPUT_DIR}/worker_session_f_eventful_sm.h"
               "${MESH_SESSION_F_EVENTFUL_OUTPUT_DIR}/worker_session_f_eventful_transport.h"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SESSION_F_EVENTFUL_OUTPUT_DIR}"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/worker_session_f_eventful.scxml"
                -l cpp -o "${MESH_SESSION_F_EVENTFUL_OUTPUT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_session_f_eventful.yaml"
                --write-deps "${MESH_SESSION_F_EVENTFUL_OUTPUT_DIR}/worker_session_f_eventful_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/parent_session_f_eventful.scxml"
                "${MESH_TEST_DIR}/worker_session_f_eventful.scxml"
                "${MESH_TEST_DIR}/deploy_session_f_eventful.yaml"
                "${SCE_CODEGEN}"
        DEPFILE "${MESH_SESSION_F_EVENTFUL_OUTPUT_DIR}/worker_session_f_eventful_sm.h.d"
        COMMENT "Mesh §9.6.2 eventful: Generating worker_session_f_eventful_{sm,transport}.h"
        VERBATIM
    )

    add_executable(mesh_session_f_eventful_test
        mesh/test_mesh_session_f_eventful.cpp
        "${MESH_SESSION_F_EVENTFUL_OUTPUT_DIR}/parent_session_f_eventful_sm.h"
        "${MESH_SESSION_F_EVENTFUL_OUTPUT_DIR}/parent_session_f_eventful_transport.h"
        "${MESH_SESSION_F_EVENTFUL_OUTPUT_DIR}/worker_session_f_eventful_sm.h"
        "${MESH_SESSION_F_EVENTFUL_OUTPUT_DIR}/worker_session_f_eventful_transport.h"
    )

    target_include_directories(mesh_session_f_eventful_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
            ${MESH_SESSION_F_EVENTFUL_OUTPUT_DIR}
    )

    target_link_libraries(mesh_session_f_eventful_test
        PRIVATE
            sce_runtime
            sce_mesh_common
    )

    target_compile_features(mesh_session_f_eventful_test PRIVATE cxx_std_17)

    apply_sce_test_policies(mesh_session_f_eventful_test)

    add_test(NAME mesh_session_f_eventful
             COMMAND mesh_session_f_eventful_test)

    # ── SCE_MESH §9.6.5 autoforward="true" verification (wire-17) ──
    # Parent autoforwards external `trigger` onto the child; child
    # transitions to `<final>` and wire-18 fires.
    set(MESH_SESSION_F_AUTOFWD_OUTPUT_DIR
        "${CMAKE_CURRENT_BINARY_DIR}/mesh_session_f_autoforward_generated")

    add_custom_command(
        OUTPUT "${MESH_SESSION_F_AUTOFWD_OUTPUT_DIR}/parent_session_f_autoforward_sm.h"
               "${MESH_SESSION_F_AUTOFWD_OUTPUT_DIR}/parent_session_f_autoforward_transport.h"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SESSION_F_AUTOFWD_OUTPUT_DIR}"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/parent_session_f_autoforward.scxml"
                -l cpp -o "${MESH_SESSION_F_AUTOFWD_OUTPUT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_session_f_autoforward.yaml"
                --write-deps "${MESH_SESSION_F_AUTOFWD_OUTPUT_DIR}/parent_session_f_autoforward_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/parent_session_f_autoforward.scxml"
                "${MESH_TEST_DIR}/worker_session_f_autoforward.scxml"
                "${MESH_TEST_DIR}/deploy_session_f_autoforward.yaml"
                "${SCE_CODEGEN}"
        DEPFILE "${MESH_SESSION_F_AUTOFWD_OUTPUT_DIR}/parent_session_f_autoforward_sm.h.d"
        COMMENT "Mesh §9.6.5 autoforward: Generating parent_session_f_autoforward_{sm,transport}.h"
        VERBATIM
    )

    add_custom_command(
        OUTPUT "${MESH_SESSION_F_AUTOFWD_OUTPUT_DIR}/worker_session_f_autoforward_sm.h"
               "${MESH_SESSION_F_AUTOFWD_OUTPUT_DIR}/worker_session_f_autoforward_transport.h"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SESSION_F_AUTOFWD_OUTPUT_DIR}"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/worker_session_f_autoforward.scxml"
                -l cpp -o "${MESH_SESSION_F_AUTOFWD_OUTPUT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_session_f_autoforward.yaml"
                --write-deps "${MESH_SESSION_F_AUTOFWD_OUTPUT_DIR}/worker_session_f_autoforward_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/parent_session_f_autoforward.scxml"
                "${MESH_TEST_DIR}/worker_session_f_autoforward.scxml"
                "${MESH_TEST_DIR}/deploy_session_f_autoforward.yaml"
                "${SCE_CODEGEN}"
        DEPFILE "${MESH_SESSION_F_AUTOFWD_OUTPUT_DIR}/worker_session_f_autoforward_sm.h.d"
        COMMENT "Mesh §9.6.5 autoforward: Generating worker_session_f_autoforward_{sm,transport}.h"
        VERBATIM
    )

    add_executable(mesh_session_f_autoforward_test
        mesh/test_mesh_session_f_autoforward.cpp
        "${MESH_SESSION_F_AUTOFWD_OUTPUT_DIR}/parent_session_f_autoforward_sm.h"
        "${MESH_SESSION_F_AUTOFWD_OUTPUT_DIR}/parent_session_f_autoforward_transport.h"
        "${MESH_SESSION_F_AUTOFWD_OUTPUT_DIR}/worker_session_f_autoforward_sm.h"
        "${MESH_SESSION_F_AUTOFWD_OUTPUT_DIR}/worker_session_f_autoforward_transport.h"
    )

    target_include_directories(mesh_session_f_autoforward_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
            ${MESH_SESSION_F_AUTOFWD_OUTPUT_DIR}
    )

    target_link_libraries(mesh_session_f_autoforward_test
        PRIVATE
            sce_runtime
            sce_mesh_common
    )

    target_compile_features(mesh_session_f_autoforward_test PRIVATE cxx_std_17)

    apply_sce_test_policies(mesh_session_f_autoforward_test)

    add_test(NAME mesh_session_f_autoforward
             COMMAND mesh_session_f_autoforward_test)

    # ── SCE_MESH §9.6 wire-19 InvokeCancel verification ────────────
    # Parent exits the invoking state via `<transition event="stop">`;
    # onexit emits wire-19. Worker adapter cancels and erases session.
    set(MESH_SESSION_F_CANCEL_OUTPUT_DIR
        "${CMAKE_CURRENT_BINARY_DIR}/mesh_session_f_cancel_generated")

    add_custom_command(
        OUTPUT "${MESH_SESSION_F_CANCEL_OUTPUT_DIR}/parent_session_f_cancel_sm.h"
               "${MESH_SESSION_F_CANCEL_OUTPUT_DIR}/parent_session_f_cancel_transport.h"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SESSION_F_CANCEL_OUTPUT_DIR}"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/parent_session_f_cancel.scxml"
                -l cpp -o "${MESH_SESSION_F_CANCEL_OUTPUT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_session_f_cancel.yaml"
                --write-deps "${MESH_SESSION_F_CANCEL_OUTPUT_DIR}/parent_session_f_cancel_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/parent_session_f_cancel.scxml"
                "${MESH_TEST_DIR}/worker_session_f_cancel.scxml"
                "${MESH_TEST_DIR}/deploy_session_f_cancel.yaml"
                "${SCE_CODEGEN}"
        DEPFILE "${MESH_SESSION_F_CANCEL_OUTPUT_DIR}/parent_session_f_cancel_sm.h.d"
        COMMENT "Mesh §9.6 cancel: Generating parent_session_f_cancel_{sm,transport}.h"
        VERBATIM
    )

    add_custom_command(
        OUTPUT "${MESH_SESSION_F_CANCEL_OUTPUT_DIR}/worker_session_f_cancel_sm.h"
               "${MESH_SESSION_F_CANCEL_OUTPUT_DIR}/worker_session_f_cancel_transport.h"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SESSION_F_CANCEL_OUTPUT_DIR}"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/worker_session_f_cancel.scxml"
                -l cpp -o "${MESH_SESSION_F_CANCEL_OUTPUT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_session_f_cancel.yaml"
                --write-deps "${MESH_SESSION_F_CANCEL_OUTPUT_DIR}/worker_session_f_cancel_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/parent_session_f_cancel.scxml"
                "${MESH_TEST_DIR}/worker_session_f_cancel.scxml"
                "${MESH_TEST_DIR}/deploy_session_f_cancel.yaml"
                "${SCE_CODEGEN}"
        DEPFILE "${MESH_SESSION_F_CANCEL_OUTPUT_DIR}/worker_session_f_cancel_sm.h.d"
        COMMENT "Mesh §9.6 cancel: Generating worker_session_f_cancel_{sm,transport}.h"
        VERBATIM
    )

    add_executable(mesh_session_f_cancel_test
        mesh/test_mesh_session_f_cancel.cpp
        "${MESH_SESSION_F_CANCEL_OUTPUT_DIR}/parent_session_f_cancel_sm.h"
        "${MESH_SESSION_F_CANCEL_OUTPUT_DIR}/parent_session_f_cancel_transport.h"
        "${MESH_SESSION_F_CANCEL_OUTPUT_DIR}/worker_session_f_cancel_sm.h"
        "${MESH_SESSION_F_CANCEL_OUTPUT_DIR}/worker_session_f_cancel_transport.h"
    )

    target_include_directories(mesh_session_f_cancel_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
            ${MESH_SESSION_F_CANCEL_OUTPUT_DIR}
    )

    target_link_libraries(mesh_session_f_cancel_test
        PRIVATE
            sce_runtime
            sce_mesh_common
    )

    target_compile_features(mesh_session_f_cancel_test PRIVATE cxx_std_17)

    apply_sce_test_policies(mesh_session_f_cancel_test)

    add_test(NAME mesh_session_f_cancel
             COMMAND mesh_session_f_cancel_test)

    # ── SCE_MESH §16.7 row 5 sub-atomic C peer-down emit ──────────
    # Parent has a §9.6 `<invoke type="scxml">` and an
    # `<transition event="error.communication" target="lost"/>`. The
    # test driver calls `failScxmlRemoteInvokesForPeer` directly on
    # the parent Policy — the same entry point the TransportRouter
    # peer-down handlers (Zenoh liveliness DELETE, SOMEIP machine
    # availability) reach in production. Verifies the policy method
    # contract: erase activeInvokes_ entry, raise
    # `error.communication{INVOKE_CHILD_LOST,...}` on the external
    # queue, idempotent on repeat call.
    set(MESH_SESSION_F_PEER_DROP_OUTPUT_DIR
        "${CMAKE_CURRENT_BINARY_DIR}/mesh_session_f_peer_drop_generated")

    add_custom_command(
        OUTPUT "${MESH_SESSION_F_PEER_DROP_OUTPUT_DIR}/parent_session_f_peer_drop_sm.h"
               "${MESH_SESSION_F_PEER_DROP_OUTPUT_DIR}/parent_session_f_peer_drop_transport.h"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SESSION_F_PEER_DROP_OUTPUT_DIR}"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/parent_session_f_peer_drop.scxml"
                -l cpp -o "${MESH_SESSION_F_PEER_DROP_OUTPUT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_session_f_peer_drop.yaml"
                --write-deps "${MESH_SESSION_F_PEER_DROP_OUTPUT_DIR}/parent_session_f_peer_drop_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/parent_session_f_peer_drop.scxml"
                "${MESH_TEST_DIR}/worker_session_f_peer_drop.scxml"
                "${MESH_TEST_DIR}/deploy_session_f_peer_drop.yaml"
                "${SCE_CODEGEN}"
        DEPFILE "${MESH_SESSION_F_PEER_DROP_OUTPUT_DIR}/parent_session_f_peer_drop_sm.h.d"
        COMMENT "Mesh §16.7 row 5 sub-atomic C: Generating parent_session_f_peer_drop_{sm,transport}.h"
        VERBATIM
    )

    add_custom_command(
        OUTPUT "${MESH_SESSION_F_PEER_DROP_OUTPUT_DIR}/worker_session_f_peer_drop_sm.h"
               "${MESH_SESSION_F_PEER_DROP_OUTPUT_DIR}/worker_session_f_peer_drop_transport.h"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SESSION_F_PEER_DROP_OUTPUT_DIR}"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/worker_session_f_peer_drop.scxml"
                -l cpp -o "${MESH_SESSION_F_PEER_DROP_OUTPUT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_session_f_peer_drop.yaml"
                --write-deps "${MESH_SESSION_F_PEER_DROP_OUTPUT_DIR}/worker_session_f_peer_drop_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/parent_session_f_peer_drop.scxml"
                "${MESH_TEST_DIR}/worker_session_f_peer_drop.scxml"
                "${MESH_TEST_DIR}/deploy_session_f_peer_drop.yaml"
                "${SCE_CODEGEN}"
        DEPFILE "${MESH_SESSION_F_PEER_DROP_OUTPUT_DIR}/worker_session_f_peer_drop_sm.h.d"
        COMMENT "Mesh §16.7 row 5 sub-atomic C: Generating worker_session_f_peer_drop_{sm,transport}.h"
        VERBATIM
    )

    add_executable(mesh_session_f_peer_drop_test
        mesh/test_mesh_session_f_peer_drop.cpp
        "${MESH_SESSION_F_PEER_DROP_OUTPUT_DIR}/parent_session_f_peer_drop_sm.h"
        "${MESH_SESSION_F_PEER_DROP_OUTPUT_DIR}/parent_session_f_peer_drop_transport.h"
        "${MESH_SESSION_F_PEER_DROP_OUTPUT_DIR}/worker_session_f_peer_drop_sm.h"
        "${MESH_SESSION_F_PEER_DROP_OUTPUT_DIR}/worker_session_f_peer_drop_transport.h"
    )

    target_include_directories(mesh_session_f_peer_drop_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
            ${MESH_SESSION_F_PEER_DROP_OUTPUT_DIR}
    )

    target_link_libraries(mesh_session_f_peer_drop_test
        PRIVATE
            sce_runtime
            sce_mesh_common
    )

    target_compile_features(mesh_session_f_peer_drop_test PRIVATE cxx_std_17)

    apply_sce_test_policies(mesh_session_f_peer_drop_test)

    add_test(NAME mesh_session_f_peer_drop
             COMMAND mesh_session_f_peer_drop_test)

    # ── SCE_MESH §9.6.2 wire-18 donedata (W3C SCXML 5.5 / 6.3.1) ──
    # Three machines: parent sequentially invokes one worker whose
    # `<donedata>` is a `<param>` and a second worker whose `<donedata>`
    # is a `<content expr>`. Closes the c06fae17 deferral
    # (`ChildSessionAdapter::getDonedata()` previously returned empty)
    # and the f3aa83ba `(void)donedata;` marker in the wire-18 receive
    # branch. Both branches round-trip through
    # `StaticExecutionEngine::stashDonedataAtFinal` on the worker engine
    # and `EventDataHelper::jsonStringToScriptValue` on the parent.
    set(MESH_SESSION_F_DONEDATA_OUTPUT_DIR
        "${CMAKE_CURRENT_BINARY_DIR}/mesh_session_f_donedata_generated")

    add_custom_command(
        OUTPUT "${MESH_SESSION_F_DONEDATA_OUTPUT_DIR}/parent_session_f_donedata_sm.h"
               "${MESH_SESSION_F_DONEDATA_OUTPUT_DIR}/parent_session_f_donedata_transport.h"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SESSION_F_DONEDATA_OUTPUT_DIR}"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/parent_session_f_donedata.scxml"
                -l cpp -o "${MESH_SESSION_F_DONEDATA_OUTPUT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_session_f_donedata.yaml"
                --write-deps "${MESH_SESSION_F_DONEDATA_OUTPUT_DIR}/parent_session_f_donedata_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/parent_session_f_donedata.scxml"
                "${MESH_TEST_DIR}/worker_session_f_donedata_param.scxml"
                "${MESH_TEST_DIR}/worker_session_f_donedata_content.scxml"
                "${MESH_TEST_DIR}/worker_session_f_donedata_nested.scxml"
                "${MESH_TEST_DIR}/deploy_session_f_donedata.yaml"
                "${SCE_CODEGEN}"
        DEPFILE "${MESH_SESSION_F_DONEDATA_OUTPUT_DIR}/parent_session_f_donedata_sm.h.d"
        COMMENT "Mesh §9.6.2 donedata: Generating parent_session_f_donedata_{sm,transport}.h"
        VERBATIM
    )

    add_custom_command(
        OUTPUT "${MESH_SESSION_F_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_param_sm.h"
               "${MESH_SESSION_F_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_param_transport.h"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SESSION_F_DONEDATA_OUTPUT_DIR}"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/worker_session_f_donedata_param.scxml"
                -l cpp -o "${MESH_SESSION_F_DONEDATA_OUTPUT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_session_f_donedata.yaml"
                --write-deps "${MESH_SESSION_F_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_param_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/parent_session_f_donedata.scxml"
                "${MESH_TEST_DIR}/worker_session_f_donedata_param.scxml"
                "${MESH_TEST_DIR}/worker_session_f_donedata_content.scxml"
                "${MESH_TEST_DIR}/worker_session_f_donedata_nested.scxml"
                "${MESH_TEST_DIR}/deploy_session_f_donedata.yaml"
                "${SCE_CODEGEN}"
        DEPFILE "${MESH_SESSION_F_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_param_sm.h.d"
        COMMENT "Mesh §9.6.2 donedata: Generating worker_session_f_donedata_param_{sm,transport}.h"
        VERBATIM
    )

    add_custom_command(
        OUTPUT "${MESH_SESSION_F_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_content_sm.h"
               "${MESH_SESSION_F_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_content_transport.h"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SESSION_F_DONEDATA_OUTPUT_DIR}"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/worker_session_f_donedata_content.scxml"
                -l cpp -o "${MESH_SESSION_F_DONEDATA_OUTPUT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_session_f_donedata.yaml"
                --write-deps "${MESH_SESSION_F_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_content_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/parent_session_f_donedata.scxml"
                "${MESH_TEST_DIR}/worker_session_f_donedata_param.scxml"
                "${MESH_TEST_DIR}/worker_session_f_donedata_content.scxml"
                "${MESH_TEST_DIR}/worker_session_f_donedata_nested.scxml"
                "${MESH_TEST_DIR}/deploy_session_f_donedata.yaml"
                "${SCE_CODEGEN}"
        DEPFILE "${MESH_SESSION_F_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_content_sm.h.d"
        COMMENT "Mesh §9.6.2 donedata: Generating worker_session_f_donedata_content_{sm,transport}.h"
        VERBATIM
    )

    add_custom_command(
        OUTPUT "${MESH_SESSION_F_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_nested_sm.h"
               "${MESH_SESSION_F_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_nested_transport.h"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SESSION_F_DONEDATA_OUTPUT_DIR}"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/worker_session_f_donedata_nested.scxml"
                -l cpp -o "${MESH_SESSION_F_DONEDATA_OUTPUT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_session_f_donedata.yaml"
                --write-deps "${MESH_SESSION_F_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_nested_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/parent_session_f_donedata.scxml"
                "${MESH_TEST_DIR}/worker_session_f_donedata_param.scxml"
                "${MESH_TEST_DIR}/worker_session_f_donedata_content.scxml"
                "${MESH_TEST_DIR}/worker_session_f_donedata_nested.scxml"
                "${MESH_TEST_DIR}/deploy_session_f_donedata.yaml"
                "${SCE_CODEGEN}"
        DEPFILE "${MESH_SESSION_F_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_nested_sm.h.d"
        COMMENT "Mesh §9.6.2 donedata: Generating worker_session_f_donedata_nested_{sm,transport}.h"
        VERBATIM
    )

    add_executable(mesh_session_f_donedata_test
        mesh/test_mesh_session_f_donedata.cpp
        "${MESH_SESSION_F_DONEDATA_OUTPUT_DIR}/parent_session_f_donedata_sm.h"
        "${MESH_SESSION_F_DONEDATA_OUTPUT_DIR}/parent_session_f_donedata_transport.h"
        "${MESH_SESSION_F_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_param_sm.h"
        "${MESH_SESSION_F_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_param_transport.h"
        "${MESH_SESSION_F_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_content_sm.h"
        "${MESH_SESSION_F_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_content_transport.h"
        "${MESH_SESSION_F_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_nested_sm.h"
        "${MESH_SESSION_F_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_nested_transport.h"
    )

    target_include_directories(mesh_session_f_donedata_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
            ${MESH_SESSION_F_DONEDATA_OUTPUT_DIR}
    )

    target_link_libraries(mesh_session_f_donedata_test
        PRIVATE
            sce_runtime
            sce_mesh_common
    )

    target_compile_features(mesh_session_f_donedata_test PRIVATE cxx_std_17)

    apply_sce_test_policies(mesh_session_f_donedata_test)

    add_test(NAME mesh_session_f_donedata
             COMMAND mesh_session_f_donedata_test)

    # ── SCE_MESH §9.5 srcexpr compile verification ──────────────
    # Forces the srcexpr `{% if invoke_info.srcexpr %}` branches in
    # entry_exit_actions.jinja2 + state_machine.jinja2 to render and
    # compile. Until this fixture landed, no SCXML in-tree used
    # `<invoke type="sce:mesh-rpc" srcexpr="...">`, so the
    # srcexpr code sat "built but unconsumed" — the member declaration
    # was even trapped inside the scxml/hybrid guard until this session
    # moved it out. Compile-only test with sce_scripting linked so the
    # generated `ensureScriptEngine()` + `evaluateExpression(...)` calls
    # resolve without pulling a concrete engine at runtime.
    set(MESH_SRCEXPR_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/mesh_srcexpr_generated")

    add_custom_command(
        OUTPUT "${MESH_SRCEXPR_OUTPUT_DIR}/srcexpr_client_sm.h"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SRCEXPR_OUTPUT_DIR}"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/srcexpr_client.scxml" -l cpp -o "${MESH_SRCEXPR_OUTPUT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_srcexpr.yaml"
                --write-deps "${MESH_SRCEXPR_OUTPUT_DIR}/srcexpr_client_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/srcexpr_client.scxml" "${SCE_CODEGEN}"
        DEPFILE "${MESH_SRCEXPR_OUTPUT_DIR}/srcexpr_client_sm.h.d"
        COMMENT "Mesh srcexpr: Generating srcexpr_client_sm.h (§9.5 srcexpr runtime target)"
        VERBATIM
    )

    add_custom_command(
        OUTPUT "${MESH_SRCEXPR_OUTPUT_DIR}/motor_srcexpr_sm.h"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_srcexpr.scxml" -l cpp -o "${MESH_SRCEXPR_OUTPUT_DIR}"
                --write-deps "${MESH_SRCEXPR_OUTPUT_DIR}/motor_srcexpr_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/motor_srcexpr.scxml" "${SCE_CODEGEN}"
        DEPFILE "${MESH_SRCEXPR_OUTPUT_DIR}/motor_srcexpr_sm.h.d"
        COMMENT "Mesh srcexpr: Generating motor_srcexpr_sm.h (§9.5 srcexpr receiver)"
        VERBATIM
    )

    add_custom_command(
        OUTPUT "${MESH_SRCEXPR_OUTPUT_DIR}/srcexpr_client_transport.h"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/srcexpr_client.scxml" -l cpp -o "${MESH_SRCEXPR_OUTPUT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_srcexpr.yaml"
                --transport-only
                --write-deps "${MESH_SRCEXPR_OUTPUT_DIR}/srcexpr_client_transport.h.d"
        DEPENDS "${MESH_TEST_DIR}/srcexpr_client.scxml" "${MESH_TEST_DIR}/deploy_srcexpr.yaml"
                "${MESH_TEST_DIR}/motor_srcexpr.scxml"
                "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                "${MESH_SRCEXPR_OUTPUT_DIR}/srcexpr_client_sm.h"
        DEPFILE "${MESH_SRCEXPR_OUTPUT_DIR}/srcexpr_client_transport.h.d"
        COMMENT "Mesh srcexpr: Generating srcexpr_client_transport.h (§9.5 srcexpr plumbing)"
        VERBATIM
    )

    add_executable(mesh_srcexpr_compile_test
        mesh/test_mesh_srcexpr_compile.cpp
        "${MESH_SRCEXPR_OUTPUT_DIR}/srcexpr_client_sm.h"
        "${MESH_SRCEXPR_OUTPUT_DIR}/motor_srcexpr_sm.h"
        "${MESH_SRCEXPR_OUTPUT_DIR}/srcexpr_client_transport.h"
    )

    target_include_directories(mesh_srcexpr_compile_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
            ${MESH_SRCEXPR_OUTPUT_DIR}
    )

    # The generated srcexpr entry block calls `ensureScriptEngine()`
    # which resolves through `ScriptEngineProvider::getScriptEngine()`
    # in sce_scripting. Ideally we would link just sce_scripting +
    # sce_base, but sce_scripting::LuaEngine references runtime-only
    # symbols (`EventRaiserService::getRegistry` etc.), so consumers
    # of sce_scripting currently must also link sce_runtime to resolve
    # the back-edge. Same pattern as mesh_shm_payload_runtime_test and
    # the W3C AOT harness. Refactoring LuaEngine to drop its
    # runtime-layer back-edge would let this collapse to just
    # sce_scripting, but that is an independent layering cleanup.
    target_link_libraries(mesh_srcexpr_compile_test
        PRIVATE
            sce_base
            sce_runtime
            sce_mesh_common
    )

    target_compile_features(mesh_srcexpr_compile_test PRIVATE cxx_std_17)
    sce_pin_cxx17(mesh_srcexpr_compile_test)

    apply_sce_test_policies(mesh_srcexpr_compile_test)

    add_test(NAME mesh_srcexpr_compile_verification COMMAND mesh_srcexpr_compile_test)

    # ── SCE_MESH §9.5 srcexpr target miss runtime verification ──
    # Runtime counterpart to mesh_srcexpr_compile_verification. The
    # compile test takes method pointers but never executes the raise
    # path; this fixture drives the §9.5 L1340 / §10.7.1 pre-envelope
    # setup-fault tier end-to-end — srcexpr resolves to a name absent
    # from deploy.yaml, performMeshInvoke returns false, and the
    # single error.execution raise fires. The scxml's `observe` state
    # also traps a 35340cc7 double-raise regression via a transition
    # to `double_raise_detected`.
    #
    # Same linkage posture as mesh_srcexpr_compile_test (sce_base +
    # sce_runtime + sce_mesh_common): sce_runtime resolves
    # LuaEngine's runtime-layer back-edge that ensureScriptEngine()
    # pulls in. Refactoring that back-edge is an independent cleanup.
    set(MESH_SRCEXPR_MISS_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/mesh_srcexpr_miss_generated")

    add_custom_command(
        OUTPUT "${MESH_SRCEXPR_MISS_OUTPUT_DIR}/srcexpr_miss_sm.h"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SRCEXPR_MISS_OUTPUT_DIR}"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/srcexpr_miss.scxml" -l cpp -o "${MESH_SRCEXPR_MISS_OUTPUT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_srcexpr_miss.yaml"
                --write-deps "${MESH_SRCEXPR_MISS_OUTPUT_DIR}/srcexpr_miss_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/srcexpr_miss.scxml" "${SCE_CODEGEN}"
        DEPFILE "${MESH_SRCEXPR_MISS_OUTPUT_DIR}/srcexpr_miss_sm.h.d"
        COMMENT "Mesh srcexpr miss: Generating srcexpr_miss_sm.h (§9.5 miss runtime SM)"
        VERBATIM
    )

    add_custom_command(
        OUTPUT "${MESH_SRCEXPR_MISS_OUTPUT_DIR}/motor_srcexpr_sm.h"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_srcexpr.scxml" -l cpp -o "${MESH_SRCEXPR_MISS_OUTPUT_DIR}"
                --write-deps "${MESH_SRCEXPR_MISS_OUTPUT_DIR}/motor_srcexpr_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/motor_srcexpr.scxml" "${SCE_CODEGEN}"
        DEPFILE "${MESH_SRCEXPR_MISS_OUTPUT_DIR}/motor_srcexpr_sm.h.d"
        COMMENT "Mesh srcexpr miss: Generating motor_srcexpr_sm.h (phantom binding stub)"
        VERBATIM
    )

    add_custom_command(
        OUTPUT "${MESH_SRCEXPR_MISS_OUTPUT_DIR}/srcexpr_miss_transport.h"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/srcexpr_miss.scxml" -l cpp -o "${MESH_SRCEXPR_MISS_OUTPUT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_srcexpr_miss.yaml"
                --transport-only
                --write-deps "${MESH_SRCEXPR_MISS_OUTPUT_DIR}/srcexpr_miss_transport.h.d"
        DEPENDS "${MESH_TEST_DIR}/srcexpr_miss.scxml" "${MESH_TEST_DIR}/deploy_srcexpr_miss.yaml"
                "${MESH_TEST_DIR}/motor_srcexpr.scxml"
                "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                "${MESH_SRCEXPR_MISS_OUTPUT_DIR}/srcexpr_miss_sm.h"
        DEPFILE "${MESH_SRCEXPR_MISS_OUTPUT_DIR}/srcexpr_miss_transport.h.d"
        COMMENT "Mesh srcexpr miss: Generating srcexpr_miss_transport.h (§9.5 miss plumbing)"
        VERBATIM
    )

    add_executable(mesh_srcexpr_miss_test
        mesh/test_mesh_srcexpr_miss.cpp
        "${MESH_SRCEXPR_MISS_OUTPUT_DIR}/srcexpr_miss_sm.h"
        "${MESH_SRCEXPR_MISS_OUTPUT_DIR}/motor_srcexpr_sm.h"
        "${MESH_SRCEXPR_MISS_OUTPUT_DIR}/srcexpr_miss_transport.h"
    )

    target_include_directories(mesh_srcexpr_miss_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
            ${MESH_SRCEXPR_MISS_OUTPUT_DIR}
    )

    target_link_libraries(mesh_srcexpr_miss_test
        PRIVATE
            sce_base
            sce_runtime
            sce_mesh_common
    )

    target_compile_features(mesh_srcexpr_miss_test PRIVATE cxx_std_17)

    apply_sce_test_policies(mesh_srcexpr_miss_test)

    add_test(NAME mesh_srcexpr_miss_verification COMMAND mesh_srcexpr_miss_test)

    # Round-trip runtime verification: exercises mesh-rpc end-to-end with
    # two linked routers in-process so an integration-level regression
    # (silent drop of auto-propagated _event.invokeid, dispatchToSender
    # not reached via linkTo, InvokeCorrelation not firing done.invoke)
    # surfaces here as brake stuck outside State::Ok. Separate TU from
    # the compile-only fixture so the two failure modes stay distinguishable.
    add_executable(mesh_invoke_rpc_runtime_test
        mesh/test_mesh_invoke_rpc_runtime.cpp
        "${MESH_INVOKE_OUTPUT_DIR}/brake_invoke_sm.h"
        "${MESH_INVOKE_OUTPUT_DIR}/motor_invoke_sm.h"
        "${MESH_INVOKE_OUTPUT_DIR}/brake_invoke_transport.h"
        "${MESH_INVOKE_OUTPUT_DIR}/motor_invoke_transport.h"
    )

    target_include_directories(mesh_invoke_rpc_runtime_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
            ${MESH_INVOKE_OUTPUT_DIR}
    )

    target_link_libraries(mesh_invoke_rpc_runtime_test
        PRIVATE
            sce_base
            sce_mesh_common
    )

    target_compile_features(mesh_invoke_rpc_runtime_test PRIVATE cxx_std_17)

    apply_sce_test_policies(mesh_invoke_rpc_runtime_test)

    add_test(NAME mesh_invoke_rpc_runtime_verification COMMAND mesh_invoke_rpc_runtime_test)

    # §9.5 correlation scope: adjudicates the §14.6 "Same-target
    # constraint" clause against the runtime. A reply carrying a live
    # invoke_id but a source that is not the invoke's target — and not
    # any machine in the topology — is injected into the client router.
    # Two control arms (correct source / novel invoke_id) isolate the
    # correlation axis so the verdict cannot be a fixture artifact.
    add_executable(mesh_cross_target_rpc_reply_test
        mesh/test_mesh_cross_target_rpc_reply.cpp
        "${MESH_INVOKE_OUTPUT_DIR}/brake_invoke_sm.h"
        "${MESH_INVOKE_OUTPUT_DIR}/motor_invoke_sm.h"
        "${MESH_INVOKE_OUTPUT_DIR}/brake_invoke_transport.h"
    )

    target_include_directories(mesh_cross_target_rpc_reply_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
            ${MESH_INVOKE_OUTPUT_DIR}
    )

    target_link_libraries(mesh_cross_target_rpc_reply_test
        PRIVATE
            sce_base
            sce_mesh_common
    )

    target_compile_features(mesh_cross_target_rpc_reply_test PRIVATE cxx_std_17)

    apply_sce_test_policies(mesh_cross_target_rpc_reply_test)

    add_test(NAME mesh_cross_target_rpc_reply_verification
             COMMAND mesh_cross_target_rpc_reply_test)

    # §9.5 deadline expiry: motor never replies, 50ms deadline fires
    # error.invoke on the scheduler thread. SCE_THREAD_SAFE enables the
    # EventQueueManager mutex so the scheduler thread's raiseExternal push
    # and the main thread's step() pop have a formal happens-before.
    add_executable(mesh_invoke_deadline_test
        mesh/test_mesh_invoke_deadline_expiry.cpp
        "${MESH_INVOKE_OUTPUT_DIR}/brake_invoke_sm.h"
        "${MESH_INVOKE_OUTPUT_DIR}/motor_invoke_sm.h"
        "${MESH_INVOKE_OUTPUT_DIR}/brake_invoke_transport.h"
        "${MESH_INVOKE_OUTPUT_DIR}/motor_invoke_transport.h"
    )

    target_include_directories(mesh_invoke_deadline_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
            ${MESH_INVOKE_OUTPUT_DIR}
    )

    target_link_libraries(mesh_invoke_deadline_test
        PRIVATE
            sce_base
            sce_mesh_common
    )

    target_compile_features(mesh_invoke_deadline_test PRIVATE cxx_std_17)
    target_compile_definitions(mesh_invoke_deadline_test PRIVATE SCE_THREAD_SAFE)

    apply_sce_test_policies(mesh_invoke_deadline_test)

    add_test(NAME mesh_invoke_deadline_expiry_verification COMMAND mesh_invoke_deadline_test)

    # §9.5 cancel via abort: brake exits Computing on `abort`, onexit
    # cancels the correlation. Late reply from motor is silently dropped.
    add_executable(mesh_invoke_cancel_test
        mesh/test_mesh_invoke_cancel_abort.cpp
        "${MESH_INVOKE_OUTPUT_DIR}/brake_invoke_sm.h"
        "${MESH_INVOKE_OUTPUT_DIR}/motor_invoke_sm.h"
        "${MESH_INVOKE_OUTPUT_DIR}/brake_invoke_transport.h"
        "${MESH_INVOKE_OUTPUT_DIR}/motor_invoke_transport.h"
    )

    target_include_directories(mesh_invoke_cancel_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
            ${MESH_INVOKE_OUTPUT_DIR}
    )

    target_link_libraries(mesh_invoke_cancel_test
        PRIVATE
            sce_base
            sce_mesh_common
    )

    target_compile_features(mesh_invoke_cancel_test PRIVATE cxx_std_17)

    apply_sce_test_policies(mesh_invoke_cancel_test)

    add_test(NAME mesh_invoke_cancel_abort_verification COMMAND mesh_invoke_cancel_test)

    # ── shm_transport compile verification ──────────────
    set(MESH_SHM_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/mesh_shm_generated")

    # Generate brake_sm.h for shm test (reuse motor_sm.h from above)
    add_custom_command(
        OUTPUT "${MESH_SHM_OUTPUT_DIR}/brake_sm.h"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SHM_OUTPUT_DIR}"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake.scxml" -l cpp -o "${MESH_SHM_OUTPUT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_shm.yaml"
                --write-deps "${MESH_SHM_OUTPUT_DIR}/brake_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/brake.scxml" "${SCE_CODEGEN}"
        DEPFILE "${MESH_SHM_OUTPUT_DIR}/brake_sm.h.d"
        COMMENT "Mesh shm: Generating brake_sm.h"
        VERBATIM
    )

    add_custom_command(
        OUTPUT "${MESH_SHM_OUTPUT_DIR}/motor_sm.h"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor.scxml" -l cpp -o "${MESH_SHM_OUTPUT_DIR}"
                --write-deps "${MESH_SHM_OUTPUT_DIR}/motor_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/motor.scxml" "${SCE_CODEGEN}"
        DEPFILE "${MESH_SHM_OUTPUT_DIR}/motor_sm.h.d"
        COMMENT "Mesh shm: Generating motor_sm.h"
        VERBATIM
    )

    # Generate brake_transport.h with shm transport
    add_custom_command(
        OUTPUT "${MESH_SHM_OUTPUT_DIR}/brake_transport.h"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake.scxml" -l cpp -o "${MESH_SHM_OUTPUT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_shm.yaml"
                --transport-only
                --write-deps "${MESH_SHM_OUTPUT_DIR}/brake_transport.h.d"
        DEPENDS "${MESH_TEST_DIR}/brake.scxml" "${MESH_TEST_DIR}/deploy_shm.yaml"
                "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                "${MESH_SHM_OUTPUT_DIR}/brake_sm.h"
        DEPFILE "${MESH_SHM_OUTPUT_DIR}/brake_transport.h.d"
        COMMENT "Mesh shm: Generating brake_transport.h (shm transport)"
        VERBATIM
    )

    add_executable(mesh_shm_compile_test
        mesh/test_mesh_shm.cpp
        "${MESH_SHM_OUTPUT_DIR}/brake_sm.h"
        "${MESH_SHM_OUTPUT_DIR}/motor_sm.h"
        "${MESH_SHM_OUTPUT_DIR}/brake_transport.h"
    )

    target_include_directories(mesh_shm_compile_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
            ${MESH_SHM_OUTPUT_DIR}
    )

    target_link_libraries(mesh_shm_compile_test
        PRIVATE
            sce_base
            sce_mesh_common
    )

    target_compile_features(mesh_shm_compile_test PRIVATE cxx_std_17)
    sce_pin_cxx17(mesh_shm_compile_test)

    apply_sce_test_policies(mesh_shm_compile_test)

    add_test(NAME mesh_shm_compile_verification COMMAND mesh_shm_compile_test)

    # ── local_transport runtime verification ─────────────────────
    # Exercises the full send chain: brake <send target="#motor"/>
    # → engine.raiseExternal → onMeshSend → router.wireTo lambda
    # → motor.processEvent. Compile-only tests cannot catch a missing
    # callback wire-up; this one fails if the chain regresses.
    add_executable(mesh_local_runtime_test
        mesh/test_mesh_local_runtime.cpp
        "${MESH_OUTPUT_DIR}/brake_sm.h"
        "${MESH_OUTPUT_DIR}/motor_sm.h"
        "${MESH_OUTPUT_DIR}/brake_transport.h"
    )

    target_include_directories(mesh_local_runtime_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
            ${MESH_OUTPUT_DIR}
    )

    target_link_libraries(mesh_local_runtime_test
        PRIVATE
            sce_base
            sce_mesh_common
    )

    target_compile_features(mesh_local_runtime_test PRIVATE cxx_std_17)

    apply_sce_test_policies(mesh_local_runtime_test)

    add_test(NAME mesh_local_runtime_verification COMMAND mesh_local_runtime_test)

    # ── shm_transport runtime verification (fork-based) ──────────
    # Parent sends via router; child opens shm and drains. Proves
    # READY_MAGIC handshake, end-to-end event delivery, and
    # shm_unlink ownership across process boundary.
    add_executable(mesh_shm_runtime_test
        mesh/test_mesh_shm_runtime.cpp
        "${MESH_SHM_OUTPUT_DIR}/brake_sm.h"
        "${MESH_SHM_OUTPUT_DIR}/motor_sm.h"
        "${MESH_SHM_OUTPUT_DIR}/brake_transport.h"
    )

    target_include_directories(mesh_shm_runtime_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
            ${MESH_SHM_OUTPUT_DIR}
    )

    target_link_libraries(mesh_shm_runtime_test
        PRIVATE
            sce_base
            sce_mesh_common
            rt       # POSIX shm_open / shm_unlink
            pthread
    )

    target_compile_features(mesh_shm_runtime_test PRIVATE cxx_std_17)

    apply_sce_test_policies(mesh_shm_runtime_test)

    add_test(NAME mesh_shm_runtime_verification COMMAND mesh_shm_runtime_test)
    set_tests_properties(mesh_shm_runtime_verification PROPERTIES
        TIMEOUT 30
        # The channel name "/sce_brake_motor" is fixed by the test's
        # deploy_shm.yaml, so parallel ctest invocations would collide
        # on a single POSIX shm object. Real deployments assign unique
        # names per binding; for testing we serialize with RESOURCE_LOCK.
        RESOURCE_LOCK sce_mesh_shm_brake_motor)

    # ── Session E2-1: custom_tcp reference transport (SCE_MESH.md §16.8.3)
    # Proves end-to-end FireForget across a real TCP loopback connection
    # plus per-sender FIFO conformance over 100 sequential sends. Uses
    # the same brake/motor SCXML as the local/shm tests but with two
    # routers and a CBOR-framed TCP wire.
    set(MESH_TCP_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/mesh_custom_tcp_generated")

    # Loopback port the harness binds. Cache variable so a developer
    # whose workstation already owns the default can override at
    # configure time (`-DSCE_TEST_CUSTOM_TCP_PORT=55822`). The same
    # value is substituted into deploy_custom_tcp.yaml.in via
    # configure_file AND propagated to the test binary as a compile
    # macro, so the codegen-baked port and the in-test recording
    # server's port stay in lockstep — diverging them would fail at
    # the test's parse_endpoint regression check.
    # Every fixed port in this file sits BELOW the kernel's ephemeral
    # floor, and that is the whole of the collision defence.
    #
    # These were in the IANA *dynamic* range (49152-65535), chosen for
    # being "not a registered service" — which is true and is the wrong
    # property. Linux hands out 32768-60999 (`ip_local_port_range`) to
    # every `bind(port 0)`, so a fixed port up there is one the kernel
    # may give to any sibling test at any moment. `RESOURCE_LOCK`
    # cannot help: it serialises tests that declare the same lock, and
    # the competitor here is an ephemeral bind that declares nothing.
    #
    # Measured 2026-08-12: `mesh_session_f_crossdev_donedata` failed a
    # full-suite run on `TransportRouter::init` while its own sibling
    # workers held 33837/36605/45821 — and all six fixed ports in this
    # file were in the range, so five of them were unlucky-not-yet
    # rather than safe. `sce-build/tests/fixed_test_ports.rs` keeps
    # them out of it.
    set(SCE_TEST_CUSTOM_TCP_PORT "20821" CACHE STRING
        "TCP loopback port for the custom_tcp harness test (below the kernel ephemeral floor)")
    # The configured deploy.yaml lives in the build dir, so its `source:`
    # paths must be relative-from-there to the SCXML files in the source
    # tree. (deploy.rs rejects absolute `source:` per the portability
    # contract — TopologyError::AbsoluteSourcePath.) Compute the relative
    # prefix at configure time so out-of-source builds work without
    # hard-coding the layout.
    file(RELATIVE_PATH SCE_MESH_TEST_DIR_REL
        "${MESH_TCP_OUTPUT_DIR}" "${MESH_TEST_DIR}")
    configure_file(
        "${MESH_TEST_DIR}/deploy_custom_tcp.yaml.in"
        "${MESH_TCP_OUTPUT_DIR}/deploy_custom_tcp.yaml"
        @ONLY)

    add_custom_command(
        OUTPUT "${MESH_TCP_OUTPUT_DIR}/brake_sm.h"
                "${MESH_TCP_OUTPUT_DIR}/brake_transport.h"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_TCP_OUTPUT_DIR}"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake.scxml"
                -l cpp -o "${MESH_TCP_OUTPUT_DIR}"
                --deploy "${MESH_TCP_OUTPUT_DIR}/deploy_custom_tcp.yaml"
                --write-deps "${MESH_TCP_OUTPUT_DIR}/brake_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/brake.scxml"
                "${MESH_TEST_DIR}/motor.scxml"
                "${MESH_TCP_OUTPUT_DIR}/deploy_custom_tcp.yaml"
                "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
        DEPFILE "${MESH_TCP_OUTPUT_DIR}/brake_sm.h.d"
        COMMENT "Mesh custom_tcp: Generating brake_sm.h + brake_transport.h"
        VERBATIM
    )

    add_custom_command(
        OUTPUT "${MESH_TCP_OUTPUT_DIR}/motor_sm.h"
                "${MESH_TCP_OUTPUT_DIR}/motor_transport.h"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_TCP_OUTPUT_DIR}"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor.scxml"
                -l cpp -o "${MESH_TCP_OUTPUT_DIR}"
                --deploy "${MESH_TCP_OUTPUT_DIR}/deploy_custom_tcp.yaml"
                --write-deps "${MESH_TCP_OUTPUT_DIR}/motor_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/brake.scxml"
                "${MESH_TEST_DIR}/motor.scxml"
                "${MESH_TCP_OUTPUT_DIR}/deploy_custom_tcp.yaml"
                "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
        DEPFILE "${MESH_TCP_OUTPUT_DIR}/motor_sm.h.d"
        COMMENT "Mesh custom_tcp: Generating motor_sm.h + motor_transport.h"
        VERBATIM
    )

    add_executable(mesh_custom_tcp_runtime_test
        mesh/test_mesh_custom_tcp_runtime.cpp
        "${MESH_TCP_OUTPUT_DIR}/brake_sm.h"
        "${MESH_TCP_OUTPUT_DIR}/brake_transport.h"
        "${MESH_TCP_OUTPUT_DIR}/motor_sm.h"
        "${MESH_TCP_OUTPUT_DIR}/motor_transport.h"
    )

    target_include_directories(mesh_custom_tcp_runtime_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
            ${MESH_TCP_OUTPUT_DIR}
    )

    target_link_libraries(mesh_custom_tcp_runtime_test
        PRIVATE
            sce_base
            sce_mesh_common
            pthread
    )

    target_compile_features(mesh_custom_tcp_runtime_test PRIVATE cxx_std_17)
    target_compile_definitions(mesh_custom_tcp_runtime_test PRIVATE
        SCE_TEST_CUSTOM_TCP_PORT=${SCE_TEST_CUSTOM_TCP_PORT})

    apply_sce_test_policies(mesh_custom_tcp_runtime_test)

    add_test(NAME mesh_custom_tcp_runtime_verification
             COMMAND mesh_custom_tcp_runtime_test)
    set_tests_properties(mesh_custom_tcp_runtime_verification PROPERTIES
        TIMEOUT 30
        # Loopback port from SCE_TEST_CUSTOM_TCP_PORT — serialise to
        # avoid bind() collisions when ctest runs in parallel. The lock
        # key includes the port so an override picks a different lock
        # bucket from the default 20821 (matters when a developer runs
        # parallel ctest invocations against different ports).
        RESOURCE_LOCK sce_mesh_custom_tcp_${SCE_TEST_CUSTOM_TCP_PORT})

    # ── custom_tcp multi-pattern realization (SCE_MESH.md §mesh-10.4.4)
    # Sibling of the FireForget test above, differing on exactly one axis:
    # the bindings carry RPC / FieldAccess / PubSub events and motor
    # declares a `server:` block. Proves the patterns the transport
    # advertises are the patterns it actually serves — a capability the
    # registry claims but no template arm realizes is a correctness bug,
    # not a documentation gap.
    set(MESH_TCP_MULTI_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/mesh_tcp_multi_generated")

    # Distinct from SCE_TEST_CUSTOM_TCP_PORT so the two custom_tcp tests
    # can run concurrently; each takes its own RESOURCE_LOCK bucket.
    set(SCE_TEST_TCP_MULTI_PORT "20831" CACHE STRING
        "TCP loopback port for the custom_tcp multi-pattern test (below the kernel ephemeral floor)")

    file(RELATIVE_PATH SCE_MESH_TEST_DIR_REL
        "${MESH_TCP_MULTI_OUTPUT_DIR}" "${MESH_TEST_DIR}")
    configure_file(
        "${MESH_TEST_DIR}/deploy_tcp_multi.yaml.in"
        "${MESH_TCP_MULTI_OUTPUT_DIR}/deploy_tcp_multi.yaml"
        @ONLY)

    foreach(tcp_multi_machine brake_tcp_multi motor_tcp_multi)
        add_custom_command(
            OUTPUT "${MESH_TCP_MULTI_OUTPUT_DIR}/${tcp_multi_machine}_sm.h"
                   "${MESH_TCP_MULTI_OUTPUT_DIR}/${tcp_multi_machine}_transport.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_TCP_MULTI_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/${tcp_multi_machine}.scxml"
                    -l cpp -o "${MESH_TCP_MULTI_OUTPUT_DIR}"
                    --deploy "${MESH_TCP_MULTI_OUTPUT_DIR}/deploy_tcp_multi.yaml"
                    --write-deps "${MESH_TCP_MULTI_OUTPUT_DIR}/${tcp_multi_machine}_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/brake_tcp_multi.scxml"
                    "${MESH_TEST_DIR}/motor_tcp_multi.scxml"
                    "${MESH_TCP_MULTI_OUTPUT_DIR}/deploy_tcp_multi.yaml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
            DEPFILE "${MESH_TCP_MULTI_OUTPUT_DIR}/${tcp_multi_machine}_sm.h.d"
            COMMENT "Mesh custom_tcp multi-pattern: Generating ${tcp_multi_machine}_sm.h + _transport.h"
            VERBATIM
        )
    endforeach()

    add_executable(mesh_tcp_multipattern_runtime_test
        mesh/test_mesh_tcp_multipattern_runtime.cpp
        "${MESH_TCP_MULTI_OUTPUT_DIR}/brake_tcp_multi_sm.h"
        "${MESH_TCP_MULTI_OUTPUT_DIR}/brake_tcp_multi_transport.h"
        "${MESH_TCP_MULTI_OUTPUT_DIR}/motor_tcp_multi_sm.h"
        "${MESH_TCP_MULTI_OUTPUT_DIR}/motor_tcp_multi_transport.h")

    target_include_directories(mesh_tcp_multipattern_runtime_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
            ${MESH_TCP_MULTI_OUTPUT_DIR}
    )

    target_link_libraries(mesh_tcp_multipattern_runtime_test
        PRIVATE
            sce_base
            sce_mesh_common
            pthread
    )

    target_compile_features(mesh_tcp_multipattern_runtime_test PRIVATE cxx_std_17)
    target_compile_definitions(mesh_tcp_multipattern_runtime_test PRIVATE
        SCE_TEST_TCP_MULTI_PORT=${SCE_TEST_TCP_MULTI_PORT})

    apply_sce_test_policies(mesh_tcp_multipattern_runtime_test)

    add_test(NAME mesh_tcp_multipattern_verification
             COMMAND mesh_tcp_multipattern_runtime_test)
    set_tests_properties(mesh_tcp_multipattern_verification PROPERTIES
        TIMEOUT 60
        RESOURCE_LOCK sce_mesh_tcp_multi_${SCE_TEST_TCP_MULTI_PORT})

    # ── custom_tcp server response deadline (SCE Mesh §9.5) ──
    # The custom_tcp arm of the deadline matrix. Where SOME/IP fills a
    # slot the protocol reserved (MT_ERROR / E_TIMEOUT), custom_tcp owns
    # its own framing — so what makes the notice correct is not a field
    # but its route: back on the stream the request arrived on, which is
    # exactly the handle the deadline releases.
    set(MESH_TCP_SERVER_DEADLINE_OUTPUT_DIR
        "${CMAKE_CURRENT_BINARY_DIR}/mesh_tcp_server_deadline_generated")

    # Distinct from the two custom_tcp ports above so all three fixtures
    # can run concurrently; each takes its own RESOURCE_LOCK bucket.
    set(SCE_TEST_TCP_SERVER_DEADLINE_PORT "20841" CACHE STRING
        "TCP loopback port for the custom_tcp server-deadline test (below the kernel ephemeral floor)")

    file(RELATIVE_PATH SCE_MESH_TEST_DIR_REL
        "${MESH_TCP_SERVER_DEADLINE_OUTPUT_DIR}" "${MESH_TEST_DIR}")
    configure_file(
        "${MESH_TEST_DIR}/deploy_tcp_server_deadline.yaml.in"
        "${MESH_TCP_SERVER_DEADLINE_OUTPUT_DIR}/deploy_tcp_server_deadline.yaml"
        @ONLY)

    foreach(tcp_sd_machine brake_tcp_server_deadline motor_tcp_server_deadline)
        add_custom_command(
            OUTPUT "${MESH_TCP_SERVER_DEADLINE_OUTPUT_DIR}/${tcp_sd_machine}_sm.h"
                   "${MESH_TCP_SERVER_DEADLINE_OUTPUT_DIR}/${tcp_sd_machine}_transport.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_TCP_SERVER_DEADLINE_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/${tcp_sd_machine}.scxml"
                    -l cpp -o "${MESH_TCP_SERVER_DEADLINE_OUTPUT_DIR}"
                    --deploy "${MESH_TCP_SERVER_DEADLINE_OUTPUT_DIR}/deploy_tcp_server_deadline.yaml"
                    --write-deps "${MESH_TCP_SERVER_DEADLINE_OUTPUT_DIR}/${tcp_sd_machine}_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/brake_tcp_server_deadline.scxml"
                    "${MESH_TEST_DIR}/motor_tcp_server_deadline.scxml"
                    "${MESH_TCP_SERVER_DEADLINE_OUTPUT_DIR}/deploy_tcp_server_deadline.yaml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
            DEPFILE "${MESH_TCP_SERVER_DEADLINE_OUTPUT_DIR}/${tcp_sd_machine}_sm.h.d"
            COMMENT "Mesh custom_tcp server deadline: Generating ${tcp_sd_machine}_sm.h + _transport.h"
            VERBATIM
        )
    endforeach()

    add_executable(mesh_tcp_server_deadline_test
        mesh/test_mesh_tcp_server_deadline.cpp
        "${MESH_TCP_SERVER_DEADLINE_OUTPUT_DIR}/brake_tcp_server_deadline_sm.h"
        "${MESH_TCP_SERVER_DEADLINE_OUTPUT_DIR}/brake_tcp_server_deadline_transport.h"
        "${MESH_TCP_SERVER_DEADLINE_OUTPUT_DIR}/motor_tcp_server_deadline_sm.h"
        "${MESH_TCP_SERVER_DEADLINE_OUTPUT_DIR}/motor_tcp_server_deadline_transport.h")

    target_include_directories(mesh_tcp_server_deadline_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
            ${MESH_TCP_SERVER_DEADLINE_OUTPUT_DIR}
            ${MESH_TEST_DIR}
    )

    target_link_libraries(mesh_tcp_server_deadline_test
        PRIVATE
            sce_base
            sce_mesh_common
            pthread
    )

    target_compile_features(mesh_tcp_server_deadline_test PRIVATE cxx_std_17)
    apply_sce_test_policies(mesh_tcp_server_deadline_test)

    add_test(NAME mesh_tcp_server_deadline_verification
             COMMAND mesh_tcp_server_deadline_test)
    set_tests_properties(mesh_tcp_server_deadline_verification PROPERTIES
        TIMEOUT 60
        RESOURCE_LOCK sce_mesh_tcp_server_deadline_${SCE_TEST_TCP_SERVER_DEADLINE_PORT})

    # ── custom_tcp machine-lifetime subscription (SCE Mesh §13) ──
    # The fixture the registry's `supports_machine_lifetime_subscribe` flag
    # waited on. Its subscriber document carries zero SCXML `<send>`s, so
    # every frame the router writes is attributable to deploy.yaml
    # `machines.<name>.subscriptions:` and to nothing else — which is what
    # lets the test assert on frame identity rather than on a delivery that
    # several paths could have produced.
    set(MESH_TCP_MACHINE_LIFETIME_OUTPUT_DIR
        "${CMAKE_CURRENT_BINARY_DIR}/mesh_tcp_machine_lifetime_generated")

    # Distinct from the custom_tcp ports above so all four fixtures can run
    # concurrently; each takes its own RESOURCE_LOCK bucket.
    set(SCE_TEST_TCP_MACHINE_LIFETIME_PORT "20851" CACHE STRING
        "TCP loopback port for the custom_tcp machine-lifetime subscribe test (below the kernel ephemeral floor)")

    file(RELATIVE_PATH SCE_MESH_TEST_DIR_REL
        "${MESH_TCP_MACHINE_LIFETIME_OUTPUT_DIR}" "${MESH_TEST_DIR}")
    configure_file(
        "${MESH_TEST_DIR}/deploy_tcp_machine_lifetime_subscribe.yaml.in"
        "${MESH_TCP_MACHINE_LIFETIME_OUTPUT_DIR}/deploy_tcp_machine_lifetime_subscribe.yaml"
        @ONLY)

    foreach(tcp_mls_machine brake_tcp_machine_lifetime_subscribe motor_tcp_machine_lifetime_subscribe)
        add_custom_command(
            OUTPUT "${MESH_TCP_MACHINE_LIFETIME_OUTPUT_DIR}/${tcp_mls_machine}_sm.h"
                   "${MESH_TCP_MACHINE_LIFETIME_OUTPUT_DIR}/${tcp_mls_machine}_transport.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_TCP_MACHINE_LIFETIME_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/${tcp_mls_machine}.scxml"
                    -l cpp -o "${MESH_TCP_MACHINE_LIFETIME_OUTPUT_DIR}"
                    --deploy "${MESH_TCP_MACHINE_LIFETIME_OUTPUT_DIR}/deploy_tcp_machine_lifetime_subscribe.yaml"
                    --write-deps "${MESH_TCP_MACHINE_LIFETIME_OUTPUT_DIR}/${tcp_mls_machine}_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/brake_tcp_machine_lifetime_subscribe.scxml"
                    "${MESH_TEST_DIR}/motor_tcp_machine_lifetime_subscribe.scxml"
                    "${MESH_TCP_MACHINE_LIFETIME_OUTPUT_DIR}/deploy_tcp_machine_lifetime_subscribe.yaml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
            DEPFILE "${MESH_TCP_MACHINE_LIFETIME_OUTPUT_DIR}/${tcp_mls_machine}_sm.h.d"
            COMMENT "Mesh custom_tcp machine-lifetime: Generating ${tcp_mls_machine}_sm.h + _transport.h"
            VERBATIM
        )
    endforeach()

    add_executable(mesh_tcp_machine_lifetime_subscribe_test
        mesh/test_mesh_tcp_machine_lifetime_subscribe.cpp
        "${MESH_TCP_MACHINE_LIFETIME_OUTPUT_DIR}/brake_tcp_machine_lifetime_subscribe_sm.h"
        "${MESH_TCP_MACHINE_LIFETIME_OUTPUT_DIR}/brake_tcp_machine_lifetime_subscribe_transport.h"
        "${MESH_TCP_MACHINE_LIFETIME_OUTPUT_DIR}/motor_tcp_machine_lifetime_subscribe_sm.h"
        "${MESH_TCP_MACHINE_LIFETIME_OUTPUT_DIR}/motor_tcp_machine_lifetime_subscribe_transport.h")

    target_include_directories(mesh_tcp_machine_lifetime_subscribe_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
            ${MESH_TCP_MACHINE_LIFETIME_OUTPUT_DIR}
            ${MESH_TEST_DIR}
    )

    target_link_libraries(mesh_tcp_machine_lifetime_subscribe_test
        PRIVATE
            sce_base
            sce_mesh_common
            pthread
    )

    target_compile_features(mesh_tcp_machine_lifetime_subscribe_test PRIVATE cxx_std_17)
    target_compile_definitions(mesh_tcp_machine_lifetime_subscribe_test PRIVATE
        SCE_TEST_TCP_MACHINE_LIFETIME_PORT=${SCE_TEST_TCP_MACHINE_LIFETIME_PORT})

    apply_sce_test_policies(mesh_tcp_machine_lifetime_subscribe_test)

    add_test(NAME mesh_tcp_machine_lifetime_subscribe_runtime
             COMMAND mesh_tcp_machine_lifetime_subscribe_test)
    set_tests_properties(mesh_tcp_machine_lifetime_subscribe_runtime PROPERTIES
        TIMEOUT 60
        RESOURCE_LOCK sce_mesh_tcp_machine_lifetime_${SCE_TEST_TCP_MACHINE_LIFETIME_PORT})

    # ── Generated mesh code compiles at C++17 (SCE_MESH.md §10.4) ──
    # `sce_mesh_common` declares `cxx_std_17` PUBLIC, so every consumer
    # linking it may build at 17 — but `target_compile_features` states a
    # MINIMUM, and this project's global `CMAKE_CXX_STANDARD 20` means the
    # mesh targets that declare `cxx_std_17` all actually compile at
    # `-std=c++20`. A C++20-only construct in the mesh template therefore
    # built green everywhere in-tree while breaking any downstream honouring
    # the declared contract.
    #
    # This target is the ceiling those declarations are not: setting the
    # CXX_STANDARD property (rather than a compile feature) pins the exact
    # dialect, so the violation becomes a build error where it is written.
    # Reuses the custom_tcp multi-pattern generated routers — the densest
    # emitted mesh output in the tree — rather than generating a fixture of
    # its own.
    add_executable(mesh_cxx17_conformance_test
        mesh/MeshCxx17ConformanceTest.cpp
        "${MESH_TCP_MULTI_OUTPUT_DIR}/brake_tcp_multi_transport.h"
        "${MESH_TCP_MULTI_OUTPUT_DIR}/motor_tcp_multi_transport.h")

    target_include_directories(mesh_cxx17_conformance_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
            ${MESH_TCP_MULTI_OUTPUT_DIR}
    )

    target_link_libraries(mesh_cxx17_conformance_test
        PRIVATE
            sce_base
            sce_mesh_common
            pthread
    )

    # The point of the target: an exact standard, not a floor.
    sce_pin_cxx17(mesh_cxx17_conformance_test)

    apply_sce_test_policies(mesh_cxx17_conformance_test)

    add_test(NAME mesh_cxx17_conformance COMMAND mesh_cxx17_conformance_test)

    # ── SCE_MESH.md §16.8.3 two-process orchestration smoke ────────
    # Proves the bash orchestrator in tests/mesh/run_two_process_fixture.sh
    # correctly:
    #   (1) spawns the worker, waits for its "LISTEN_ENDPOINT=host:port"
    #       stderr handshake line, resolved by Server::local_endpoint()
    #       after a bind on "127.0.0.1:0" (A1 ephemeral + A2 readback);
    #   (2) exports the endpoint as MESH_PEER_ENDPOINT and launches the
    #       parent, which connects its CustomTcp::Client there and sends
    #       one envelope (the runtime-override pattern consumed by
    #       Stage B cross-device fixtures);
    #   (3) SIGTERMs the worker after the parent exits and propagates the
    #       parent's exit code as the ctest result.
    # Unlike the other mesh tests this one uses a kernel-ephemeral port,
    # so it needs no RESOURCE_LOCK against SCE_TEST_CUSTOM_TCP_PORT.
    add_executable(mesh_two_process_smoke_worker
        mesh/test_two_process_smoke_worker.cpp)
    target_include_directories(mesh_two_process_smoke_worker
        PRIVATE ${CMAKE_SOURCE_DIR}/sce/include)
    target_link_libraries(mesh_two_process_smoke_worker
        PRIVATE sce_base sce_mesh_common pthread)
    target_compile_features(mesh_two_process_smoke_worker PRIVATE cxx_std_17)
    apply_sce_test_policies(mesh_two_process_smoke_worker)

    add_executable(mesh_two_process_smoke_parent
        mesh/test_two_process_smoke_parent.cpp)
    target_include_directories(mesh_two_process_smoke_parent
        PRIVATE ${CMAKE_SOURCE_DIR}/sce/include)
    target_link_libraries(mesh_two_process_smoke_parent
        PRIVATE sce_base sce_mesh_common pthread)
    target_compile_features(mesh_two_process_smoke_parent PRIVATE cxx_std_17)
    apply_sce_test_policies(mesh_two_process_smoke_parent)

    include(${CMAKE_CURRENT_SOURCE_DIR}/cmake/two_process_test.cmake)
    sce_register_two_process_mesh_test(
        NAME    mesh_two_process_smoke
        WORKER  mesh_two_process_smoke_worker
        PARENT  mesh_two_process_smoke_parent
        TIMEOUT 30)

    # ── Session 3 Stage B: cross-device wire-14/15/18 lifecycle ────
    # Real TransportRouter on both sides of a TCP loopback boundary.
    # The worker hosts `worker_session_f_wired` with a kernel-ephemeral
    # listen (`"127.0.0.1:0"`), announces the assigned port via
    # `custom_tcp_local_endpoint()` on stderr, and the parent consumes
    # it through `PortOverride::peer_connect_endpoints`. Parent's own
    # listen uses a static CMake-configurable port so the worker's
    # reply channel resolves at codegen time without a reverse
    # handshake. The SCXMLs are reused from deploy_session_f_wired.yaml
    # verbatim — the crossdev variant differs only in deploy.yaml
    # topology (two ecus) + per-peer `transport: custom_tcp` binding.
    set(MESH_CROSSDEV_LIFECYCLE_OUTPUT_DIR
        "${CMAKE_CURRENT_BINARY_DIR}/mesh_session_f_crossdev_lifecycle_generated")

    set(SCE_TEST_CROSSDEV_LIFECYCLE_PORT "20823" CACHE STRING
        "Static TCP loopback port the crossdev lifecycle parent Server binds \
(below the kernel ephemeral floor). Override with \
-DSCE_TEST_CROSSDEV_LIFECYCLE_PORT=N when the default collides; \
RESOURCE_LOCK keys off the value so parallel ctest stays safe.")

    file(RELATIVE_PATH SCE_MESH_TEST_DIR_REL
        "${MESH_CROSSDEV_LIFECYCLE_OUTPUT_DIR}" "${MESH_TEST_DIR}")
    configure_file(
        "${MESH_TEST_DIR}/deploy_session_f_crossdev_lifecycle.yaml.in"
        "${MESH_CROSSDEV_LIFECYCLE_OUTPUT_DIR}/deploy_session_f_crossdev_lifecycle.yaml"
        @ONLY)

    add_custom_command(
        OUTPUT "${MESH_CROSSDEV_LIFECYCLE_OUTPUT_DIR}/parent_session_f_wired_sm.h"
                "${MESH_CROSSDEV_LIFECYCLE_OUTPUT_DIR}/parent_session_f_wired_transport.h"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_CROSSDEV_LIFECYCLE_OUTPUT_DIR}"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/parent_session_f_wired.scxml"
                -l cpp -o "${MESH_CROSSDEV_LIFECYCLE_OUTPUT_DIR}"
                --deploy "${MESH_CROSSDEV_LIFECYCLE_OUTPUT_DIR}/deploy_session_f_crossdev_lifecycle.yaml"
                --write-deps "${MESH_CROSSDEV_LIFECYCLE_OUTPUT_DIR}/parent_session_f_wired_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/parent_session_f_wired.scxml"
                "${MESH_TEST_DIR}/worker_session_f_wired.scxml"
                "${MESH_CROSSDEV_LIFECYCLE_OUTPUT_DIR}/deploy_session_f_crossdev_lifecycle.yaml"
                "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
        DEPFILE "${MESH_CROSSDEV_LIFECYCLE_OUTPUT_DIR}/parent_session_f_wired_sm.h.d"
        COMMENT "Mesh §9.6 crossdev lifecycle: Generating parent_session_f_wired_{sm,transport}.h"
        VERBATIM
    )

    add_custom_command(
        OUTPUT "${MESH_CROSSDEV_LIFECYCLE_OUTPUT_DIR}/worker_session_f_wired_sm.h"
                "${MESH_CROSSDEV_LIFECYCLE_OUTPUT_DIR}/worker_session_f_wired_transport.h"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_CROSSDEV_LIFECYCLE_OUTPUT_DIR}"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/worker_session_f_wired.scxml"
                -l cpp -o "${MESH_CROSSDEV_LIFECYCLE_OUTPUT_DIR}"
                --deploy "${MESH_CROSSDEV_LIFECYCLE_OUTPUT_DIR}/deploy_session_f_crossdev_lifecycle.yaml"
                --write-deps "${MESH_CROSSDEV_LIFECYCLE_OUTPUT_DIR}/worker_session_f_wired_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/parent_session_f_wired.scxml"
                "${MESH_TEST_DIR}/worker_session_f_wired.scxml"
                "${MESH_CROSSDEV_LIFECYCLE_OUTPUT_DIR}/deploy_session_f_crossdev_lifecycle.yaml"
                "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
        DEPFILE "${MESH_CROSSDEV_LIFECYCLE_OUTPUT_DIR}/worker_session_f_wired_sm.h.d"
        COMMENT "Mesh §9.6 crossdev lifecycle: Generating worker_session_f_wired_{sm,transport}.h"
        VERBATIM
    )

    add_executable(mesh_session_f_crossdev_lifecycle_worker
        mesh/test_mesh_session_f_crossdev_lifecycle_worker.cpp
        "${MESH_CROSSDEV_LIFECYCLE_OUTPUT_DIR}/worker_session_f_wired_sm.h"
        "${MESH_CROSSDEV_LIFECYCLE_OUTPUT_DIR}/worker_session_f_wired_transport.h")
    target_include_directories(mesh_session_f_crossdev_lifecycle_worker
        PRIVATE ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_CROSSDEV_LIFECYCLE_OUTPUT_DIR})
    target_link_libraries(mesh_session_f_crossdev_lifecycle_worker
        PRIVATE sce_runtime sce_mesh_common)
    target_compile_features(mesh_session_f_crossdev_lifecycle_worker PRIVATE cxx_std_17)
    apply_sce_test_policies(mesh_session_f_crossdev_lifecycle_worker)

    add_executable(mesh_session_f_crossdev_lifecycle_parent
        mesh/test_mesh_session_f_crossdev_lifecycle_parent.cpp
        "${MESH_CROSSDEV_LIFECYCLE_OUTPUT_DIR}/parent_session_f_wired_sm.h"
        "${MESH_CROSSDEV_LIFECYCLE_OUTPUT_DIR}/parent_session_f_wired_transport.h")
    target_include_directories(mesh_session_f_crossdev_lifecycle_parent
        PRIVATE ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_CROSSDEV_LIFECYCLE_OUTPUT_DIR})
    target_link_libraries(mesh_session_f_crossdev_lifecycle_parent
        PRIVATE sce_runtime sce_mesh_common GTest::gtest GTest::gtest_main)
    target_compile_features(mesh_session_f_crossdev_lifecycle_parent PRIVATE cxx_std_17)
    apply_sce_test_policies(mesh_session_f_crossdev_lifecycle_parent)

    sce_register_two_process_mesh_test(
        NAME    mesh_session_f_crossdev_lifecycle
        WORKER  mesh_session_f_crossdev_lifecycle_worker
        PARENT  mesh_session_f_crossdev_lifecycle_parent
        TIMEOUT 30)
    set_tests_properties(mesh_session_f_crossdev_lifecycle PROPERTIES
        # Lock on the parent's static Server port so parallel ctest
        # cannot collide across this fixture and any future test that
        # reuses the same port cache variable.
        RESOURCE_LOCK sce_mesh_crossdev_lifecycle_${SCE_TEST_CROSSDEV_LIFECYCLE_PORT})

    # ── Session 3 Stage B: cross-device wire-18 donedata coverage ──
    # Three worker shapes (param / content / nested) on one ecu, each
    # binding its own kernel-ephemeral Server. Worker announces all
    # three ports via `LISTEN_ENDPOINT_<peer>=` lines + `LISTEN_READY`
    # barrier; the orchestrator exports each as
    # `MESH_PEER_ENDPOINT_<peer>` and the parent seeds
    # `PortOverride::peer_connect_endpoints` with three entries in one
    # shot. SCXMLs reused verbatim from deploy_session_f_donedata.yaml
    # — the crossdev variant differs only in the two-device topology
    # plus three `transport: custom_tcp` bindings.
    set(MESH_CROSSDEV_DONEDATA_OUTPUT_DIR
        "${CMAKE_CURRENT_BINARY_DIR}/mesh_session_f_crossdev_donedata_generated")

    set(SCE_TEST_CROSSDEV_DONEDATA_PORT "20824" CACHE STRING
        "Static TCP loopback port the crossdev donedata parent Server binds \
(below the kernel ephemeral floor). Separate from SCE_TEST_CROSSDEV_LIFECYCLE_PORT so \
parallel ctest runs of the two crossdev fixtures don't serialise; override \
with -DSCE_TEST_CROSSDEV_DONEDATA_PORT=N on collision.")

    file(RELATIVE_PATH SCE_MESH_TEST_DIR_REL
        "${MESH_CROSSDEV_DONEDATA_OUTPUT_DIR}" "${MESH_TEST_DIR}")
    configure_file(
        "${MESH_TEST_DIR}/deploy_session_f_crossdev_donedata.yaml.in"
        "${MESH_CROSSDEV_DONEDATA_OUTPUT_DIR}/deploy_session_f_crossdev_donedata.yaml"
        @ONLY)

    set(MESH_CROSSDEV_DONEDATA_SCXMLS
        parent_session_f_donedata
        worker_session_f_donedata_param
        worker_session_f_donedata_content
        worker_session_f_donedata_nested)

    set(MESH_CROSSDEV_DONEDATA_SCXML_DEPS
        "${MESH_TEST_DIR}/parent_session_f_donedata.scxml"
        "${MESH_TEST_DIR}/worker_session_f_donedata_param.scxml"
        "${MESH_TEST_DIR}/worker_session_f_donedata_content.scxml"
        "${MESH_TEST_DIR}/worker_session_f_donedata_nested.scxml")

    foreach(machine ${MESH_CROSSDEV_DONEDATA_SCXMLS})
        add_custom_command(
            OUTPUT "${MESH_CROSSDEV_DONEDATA_OUTPUT_DIR}/${machine}_sm.h"
                    "${MESH_CROSSDEV_DONEDATA_OUTPUT_DIR}/${machine}_transport.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_CROSSDEV_DONEDATA_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/${machine}.scxml"
                    -l cpp -o "${MESH_CROSSDEV_DONEDATA_OUTPUT_DIR}"
                    --deploy "${MESH_CROSSDEV_DONEDATA_OUTPUT_DIR}/deploy_session_f_crossdev_donedata.yaml"
                    --write-deps "${MESH_CROSSDEV_DONEDATA_OUTPUT_DIR}/${machine}_sm.h.d"
            DEPENDS ${MESH_CROSSDEV_DONEDATA_SCXML_DEPS}
                    "${MESH_CROSSDEV_DONEDATA_OUTPUT_DIR}/deploy_session_f_crossdev_donedata.yaml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
            DEPFILE "${MESH_CROSSDEV_DONEDATA_OUTPUT_DIR}/${machine}_sm.h.d"
            COMMENT "Mesh §9.6 crossdev donedata: Generating ${machine}_{sm,transport}.h"
            VERBATIM
        )
    endforeach()

    add_executable(mesh_session_f_crossdev_donedata_worker
        mesh/test_mesh_session_f_crossdev_donedata_worker.cpp
        "${MESH_CROSSDEV_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_param_sm.h"
        "${MESH_CROSSDEV_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_param_transport.h"
        "${MESH_CROSSDEV_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_content_sm.h"
        "${MESH_CROSSDEV_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_content_transport.h"
        "${MESH_CROSSDEV_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_nested_sm.h"
        "${MESH_CROSSDEV_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_nested_transport.h")
    target_include_directories(mesh_session_f_crossdev_donedata_worker
        PRIVATE ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_CROSSDEV_DONEDATA_OUTPUT_DIR})
    target_link_libraries(mesh_session_f_crossdev_donedata_worker
        PRIVATE sce_runtime sce_mesh_common)
    target_compile_features(mesh_session_f_crossdev_donedata_worker PRIVATE cxx_std_17)
    apply_sce_test_policies(mesh_session_f_crossdev_donedata_worker)

    add_executable(mesh_session_f_crossdev_donedata_parent
        mesh/test_mesh_session_f_crossdev_donedata_parent.cpp
        "${MESH_CROSSDEV_DONEDATA_OUTPUT_DIR}/parent_session_f_donedata_sm.h"
        "${MESH_CROSSDEV_DONEDATA_OUTPUT_DIR}/parent_session_f_donedata_transport.h")
    target_include_directories(mesh_session_f_crossdev_donedata_parent
        PRIVATE ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_CROSSDEV_DONEDATA_OUTPUT_DIR})
    target_link_libraries(mesh_session_f_crossdev_donedata_parent
        PRIVATE sce_runtime sce_mesh_common GTest::gtest GTest::gtest_main)
    target_compile_features(mesh_session_f_crossdev_donedata_parent PRIVATE cxx_std_17)
    apply_sce_test_policies(mesh_session_f_crossdev_donedata_parent)

    sce_register_two_process_mesh_test(
        NAME    mesh_session_f_crossdev_donedata
        WORKER  mesh_session_f_crossdev_donedata_worker
        PARENT  mesh_session_f_crossdev_donedata_parent
        TIMEOUT 45)
    set_tests_properties(mesh_session_f_crossdev_donedata PROPERTIES
        # Lock on parent's static Server port so parallel runs against
        # the same port cache variable serialise cleanly.
        RESOURCE_LOCK sce_mesh_crossdev_donedata_${SCE_TEST_CROSSDEV_DONEDATA_PORT})

    # ── End-to-end payload verification ────────────────────────────
    # Proves <param> data survives: sender → JSON → wire → drain
    # → raiseExternal → _event.data → receiver cond matches.
    set(MESH_SHM_PAYLOAD_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/mesh_shm_payload_generated")

    add_custom_command(
        OUTPUT "${MESH_SHM_PAYLOAD_OUTPUT_DIR}/brake_payload_sm.h"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SHM_PAYLOAD_OUTPUT_DIR}"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_payload.scxml"
                -l cpp -o "${MESH_SHM_PAYLOAD_OUTPUT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_shm_payload.yaml"
                --write-deps "${MESH_SHM_PAYLOAD_OUTPUT_DIR}/brake_payload_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/brake_payload.scxml" "${SCE_CODEGEN}"
        DEPFILE "${MESH_SHM_PAYLOAD_OUTPUT_DIR}/brake_payload_sm.h.d"
        COMMENT "Mesh shm payload: Generating brake_payload_sm.h"
        VERBATIM
    )

    add_custom_command(
        OUTPUT "${MESH_SHM_PAYLOAD_OUTPUT_DIR}/motor_payload_sm.h"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SHM_PAYLOAD_OUTPUT_DIR}"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_payload.scxml"
                -l cpp -o "${MESH_SHM_PAYLOAD_OUTPUT_DIR}"
                --write-deps "${MESH_SHM_PAYLOAD_OUTPUT_DIR}/motor_payload_sm.h.d"
        DEPENDS "${MESH_TEST_DIR}/motor_payload.scxml" "${SCE_CODEGEN}"
        DEPFILE "${MESH_SHM_PAYLOAD_OUTPUT_DIR}/motor_payload_sm.h.d"
        COMMENT "Mesh shm payload: Generating motor_payload_sm.h"
        VERBATIM
    )

    add_custom_command(
        OUTPUT "${MESH_SHM_PAYLOAD_OUTPUT_DIR}/brake_payload_transport.h"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_payload.scxml"
                -l cpp -o "${MESH_SHM_PAYLOAD_OUTPUT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_shm_payload.yaml"
                --transport-only
                --write-deps "${MESH_SHM_PAYLOAD_OUTPUT_DIR}/brake_payload_transport.h.d"
        DEPENDS "${MESH_TEST_DIR}/brake_payload.scxml"
                "${MESH_TEST_DIR}/motor_payload.scxml"
                "${MESH_TEST_DIR}/deploy_shm_payload.yaml"
                "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                "${MESH_SHM_PAYLOAD_OUTPUT_DIR}/brake_payload_sm.h"
        DEPFILE "${MESH_SHM_PAYLOAD_OUTPUT_DIR}/brake_payload_transport.h.d"
        COMMENT "Mesh shm payload: Generating brake_payload_transport.h"
        VERBATIM
    )

    add_executable(mesh_shm_payload_runtime_test
        mesh/test_mesh_shm_payload_runtime.cpp
        "${MESH_SHM_PAYLOAD_OUTPUT_DIR}/brake_payload_sm.h"
        "${MESH_SHM_PAYLOAD_OUTPUT_DIR}/motor_payload_sm.h"
        "${MESH_SHM_PAYLOAD_OUTPUT_DIR}/brake_payload_transport.h"
    )

    target_include_directories(mesh_shm_payload_runtime_test
        PRIVATE
            ${CMAKE_SOURCE_DIR}/sce/include
            ${MESH_SHM_PAYLOAD_OUTPUT_DIR}
    )

    target_link_libraries(mesh_shm_payload_runtime_test
        PRIVATE
            sce_base
            sce_runtime
            sce_mesh_common
            rt
            pthread
    )

    target_compile_features(mesh_shm_payload_runtime_test PRIVATE cxx_std_17)

    apply_sce_test_policies(mesh_shm_payload_runtime_test)

    add_test(NAME mesh_shm_payload_runtime_verification
             COMMAND mesh_shm_payload_runtime_test)
    set_tests_properties(mesh_shm_payload_runtime_verification PROPERTIES
        TIMEOUT 30
        RESOURCE_LOCK sce_mesh_shm_brake_payload_motor_payload)

    # ── someip_transport compile verification ──────────────────
    # Validates that generated SOME/IP transport code compiles against
    # real vsomeip 3.x headers. Gated on find_package(vsomeip3).
    find_package(vsomeip3 QUIET)
    if(vsomeip3_FOUND)
        set(MESH_SOMEIP_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/mesh_someip_generated")

        add_custom_command(
            OUTPUT "${MESH_SOMEIP_OUTPUT_DIR}/brake_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SOMEIP_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake.scxml" -l cpp -o "${MESH_SOMEIP_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_someip.yaml"
                    --write-deps "${MESH_SOMEIP_OUTPUT_DIR}/brake_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/brake.scxml" "${SCE_CODEGEN}"
            DEPFILE "${MESH_SOMEIP_OUTPUT_DIR}/brake_sm.h.d"
            COMMENT "Mesh someip: Generating brake_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_SOMEIP_OUTPUT_DIR}/motor_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SOMEIP_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor.scxml" -l cpp -o "${MESH_SOMEIP_OUTPUT_DIR}"
                    --write-deps "${MESH_SOMEIP_OUTPUT_DIR}/motor_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/motor.scxml" "${SCE_CODEGEN}"
            DEPFILE "${MESH_SOMEIP_OUTPUT_DIR}/motor_sm.h.d"
            COMMENT "Mesh someip: Generating motor_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_SOMEIP_OUTPUT_DIR}/brake_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake.scxml" -l cpp -o "${MESH_SOMEIP_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_someip.yaml"
                    --transport-only
                    --write-deps "${MESH_SOMEIP_OUTPUT_DIR}/brake_transport.h.d"
            DEPENDS "${MESH_TEST_DIR}/brake.scxml" "${MESH_TEST_DIR}/deploy_someip.yaml"
                    "${MESH_TEST_DIR}/motor.scxml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_SOMEIP_OUTPUT_DIR}/brake_sm.h"
            DEPFILE "${MESH_SOMEIP_OUTPUT_DIR}/brake_transport.h.d"
            COMMENT "Mesh someip: Generating brake_transport.h (someip transport)"
            VERBATIM
        )

        add_executable(mesh_someip_compile_test
            mesh/test_mesh_someip.cpp
            "${MESH_SOMEIP_OUTPUT_DIR}/brake_sm.h"
            "${MESH_SOMEIP_OUTPUT_DIR}/motor_sm.h"
            "${MESH_SOMEIP_OUTPUT_DIR}/brake_transport.h"
        )

        target_include_directories(mesh_someip_compile_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_SOMEIP_OUTPUT_DIR}
        )

        target_link_libraries(mesh_someip_compile_test
            PRIVATE
                sce_base
                sce_mesh_common
                vsomeip3
        )

        target_compile_features(mesh_someip_compile_test PRIVATE cxx_std_17)
        sce_pin_cxx17(mesh_someip_compile_test)

        apply_sce_test_policies(mesh_someip_compile_test)

        add_test(NAME mesh_someip_compile_verification COMMAND mesh_someip_compile_test)

        # ── Session C: SOME/IP multi-pattern compile verification ─────
        # Exercises all 4 SOME/IP capability categories (FireForget,
        # RequestReply, PubSub, FieldAccess) on a single target. Validates
        # that per-field_kind template branches compile against vsomeip 3.7.x.
        set(MESH_SOMEIP_MULTI_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/mesh_someip_multi_generated")

        add_custom_command(
            OUTPUT "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/brake_someip_multi_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SOMEIP_MULTI_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_someip_multi.scxml" -l cpp -o "${MESH_SOMEIP_MULTI_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_someip_multi.yaml"
                    --write-deps "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/brake_someip_multi_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/brake_someip_multi.scxml" "${SCE_CODEGEN}"
            DEPFILE "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/brake_someip_multi_sm.h.d"
            COMMENT "Mesh someip multi: Generating brake_someip_multi_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/motor_someip_multi_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SOMEIP_MULTI_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_someip_multi.scxml" -l cpp -o "${MESH_SOMEIP_MULTI_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_someip_multi.yaml"
                    --write-deps "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/motor_someip_multi_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/motor_someip_multi.scxml" "${SCE_CODEGEN}"
            DEPFILE "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/motor_someip_multi_sm.h.d"
            COMMENT "Mesh someip multi: Generating motor_someip_multi_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/brake_someip_multi_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_someip_multi.scxml" -l cpp -o "${MESH_SOMEIP_MULTI_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_someip_multi.yaml"
                    --transport-only
                    --write-deps "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/brake_someip_multi_transport.h.d"
            DEPENDS "${MESH_TEST_DIR}/brake_someip_multi.scxml" "${MESH_TEST_DIR}/deploy_someip_multi.yaml"
                    "${MESH_TEST_DIR}/motor_someip_multi.scxml" "${MESH_TEST_DIR}/vsomeip_motor_multi.json"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/brake_someip_multi_sm.h"
            DEPFILE "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/brake_someip_multi_transport.h.d"
            COMMENT "Mesh someip multi: Generating brake_someip_multi_transport.h (multi-pattern)"
            VERBATIM
        )

        # Session E: generate motor (server) transport header
        add_custom_command(
            OUTPUT "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/motor_someip_multi_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_someip_multi.scxml" -l cpp -o "${MESH_SOMEIP_MULTI_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_someip_multi.yaml"
                    --transport-only
                    --write-deps "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/motor_someip_multi_transport.h.d"
            DEPENDS "${MESH_TEST_DIR}/motor_someip_multi.scxml" "${MESH_TEST_DIR}/deploy_someip_multi.yaml"
                    "${MESH_TEST_DIR}/vsomeip_motor_multi.json"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/motor_someip_multi_sm.h"
            DEPFILE "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/motor_someip_multi_transport.h.d"
            COMMENT "Mesh someip multi: Generating motor_someip_multi_transport.h (server-side Session E)"
            VERBATIM
        )

        add_executable(mesh_someip_multipattern_compile_test
            mesh/test_mesh_someip_multipattern.cpp
            "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/brake_someip_multi_sm.h"
            "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/motor_someip_multi_sm.h"
            "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/brake_someip_multi_transport.h"
            "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/motor_someip_multi_transport.h"
        )

        target_include_directories(mesh_someip_multipattern_compile_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_SOMEIP_MULTI_OUTPUT_DIR}
        )

        target_link_libraries(mesh_someip_multipattern_compile_test
            PRIVATE
                sce_base
                sce_mesh_common
                vsomeip3
        )

        target_compile_features(mesh_someip_multipattern_compile_test PRIVATE cxx_std_17)
        sce_pin_cxx17(mesh_someip_multipattern_compile_test)
        apply_sce_test_policies(mesh_someip_multipattern_compile_test)
        add_test(NAME mesh_someip_multipattern_compile_verification
                 COMMAND mesh_someip_multipattern_compile_test)

        # ── §13 machine-lifetime subscription over SOME/IP ────────────
        # The subscriber document declares no <send>, so every SOME/IP call
        # in its generated router traces back to deploy.yaml
        # `subscriptions:` resolving against vsomeip.json. Compiling it is
        # what proves the emitted request_event / subscribe / unsubscribe
        # calls type-check against vsomeip 3.x; the Rust-side generation
        # test only proves the text is there.
        set(MESH_SOMEIP_MLSUB_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/mesh_someip_machine_lifetime_generated")

        add_custom_command(
            OUTPUT "${MESH_SOMEIP_MLSUB_OUTPUT_DIR}/brake_someip_machine_lifetime_subscribe_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SOMEIP_MLSUB_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_someip_machine_lifetime_subscribe.scxml"
                    -l cpp -o "${MESH_SOMEIP_MLSUB_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_someip_machine_lifetime_subscribe.yaml"
                    --write-deps "${MESH_SOMEIP_MLSUB_OUTPUT_DIR}/brake_someip_machine_lifetime_subscribe_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/brake_someip_machine_lifetime_subscribe.scxml" "${SCE_CODEGEN}"
            DEPFILE "${MESH_SOMEIP_MLSUB_OUTPUT_DIR}/brake_someip_machine_lifetime_subscribe_sm.h.d"
            COMMENT "Mesh someip machine-lifetime: Generating brake_someip_machine_lifetime_subscribe_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_SOMEIP_MLSUB_OUTPUT_DIR}/motor_someip_machine_lifetime_subscribe_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SOMEIP_MLSUB_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_someip_machine_lifetime_subscribe.scxml"
                    -l cpp -o "${MESH_SOMEIP_MLSUB_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_someip_machine_lifetime_subscribe.yaml"
                    --write-deps "${MESH_SOMEIP_MLSUB_OUTPUT_DIR}/motor_someip_machine_lifetime_subscribe_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/motor_someip_machine_lifetime_subscribe.scxml" "${SCE_CODEGEN}"
            DEPFILE "${MESH_SOMEIP_MLSUB_OUTPUT_DIR}/motor_someip_machine_lifetime_subscribe_sm.h.d"
            COMMENT "Mesh someip machine-lifetime: Generating motor_someip_machine_lifetime_subscribe_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_SOMEIP_MLSUB_OUTPUT_DIR}/brake_someip_machine_lifetime_subscribe_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_someip_machine_lifetime_subscribe.scxml"
                    -l cpp -o "${MESH_SOMEIP_MLSUB_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_someip_machine_lifetime_subscribe.yaml"
                    --transport-only
                    --write-deps "${MESH_SOMEIP_MLSUB_OUTPUT_DIR}/brake_someip_machine_lifetime_subscribe_transport.h.d"
            DEPENDS "${MESH_TEST_DIR}/brake_someip_machine_lifetime_subscribe.scxml"
                    "${MESH_TEST_DIR}/motor_someip_machine_lifetime_subscribe.scxml"
                    "${MESH_TEST_DIR}/deploy_someip_machine_lifetime_subscribe.yaml"
                    "${MESH_TEST_DIR}/vsomeip_someip_machine_lifetime.json"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_SOMEIP_MLSUB_OUTPUT_DIR}/brake_someip_machine_lifetime_subscribe_sm.h"
            DEPFILE "${MESH_SOMEIP_MLSUB_OUTPUT_DIR}/brake_someip_machine_lifetime_subscribe_transport.h.d"
            COMMENT "Mesh someip machine-lifetime: Generating brake_someip_machine_lifetime_subscribe_transport.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_SOMEIP_MLSUB_OUTPUT_DIR}/motor_someip_machine_lifetime_subscribe_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_someip_machine_lifetime_subscribe.scxml"
                    -l cpp -o "${MESH_SOMEIP_MLSUB_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_someip_machine_lifetime_subscribe.yaml"
                    --transport-only
                    --write-deps "${MESH_SOMEIP_MLSUB_OUTPUT_DIR}/motor_someip_machine_lifetime_subscribe_transport.h.d"
            DEPENDS "${MESH_TEST_DIR}/motor_someip_machine_lifetime_subscribe.scxml"
                    "${MESH_TEST_DIR}/deploy_someip_machine_lifetime_subscribe.yaml"
                    "${MESH_TEST_DIR}/vsomeip_someip_machine_lifetime.json"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_SOMEIP_MLSUB_OUTPUT_DIR}/motor_someip_machine_lifetime_subscribe_sm.h"
            DEPFILE "${MESH_SOMEIP_MLSUB_OUTPUT_DIR}/motor_someip_machine_lifetime_subscribe_transport.h.d"
            COMMENT "Mesh someip machine-lifetime: Generating motor_someip_machine_lifetime_subscribe_transport.h (publisher)"
            VERBATIM
        )

        add_executable(mesh_someip_machine_lifetime_subscribe_test
            mesh/test_mesh_someip_machine_lifetime_subscribe.cpp
            "${MESH_SOMEIP_MLSUB_OUTPUT_DIR}/brake_someip_machine_lifetime_subscribe_sm.h"
            "${MESH_SOMEIP_MLSUB_OUTPUT_DIR}/motor_someip_machine_lifetime_subscribe_sm.h"
            "${MESH_SOMEIP_MLSUB_OUTPUT_DIR}/brake_someip_machine_lifetime_subscribe_transport.h"
            "${MESH_SOMEIP_MLSUB_OUTPUT_DIR}/motor_someip_machine_lifetime_subscribe_transport.h"
        )

        target_include_directories(mesh_someip_machine_lifetime_subscribe_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_SOMEIP_MLSUB_OUTPUT_DIR}
        )

        target_link_libraries(mesh_someip_machine_lifetime_subscribe_test
            PRIVATE
                sce_base
                sce_mesh_common
                vsomeip3
        )

        target_compile_features(mesh_someip_machine_lifetime_subscribe_test PRIVATE cxx_std_17)
        sce_pin_cxx17(mesh_someip_machine_lifetime_subscribe_test)
        apply_sce_test_policies(mesh_someip_machine_lifetime_subscribe_test)
        add_test(NAME mesh_someip_machine_lifetime_subscribe_verification
                 COMMAND mesh_someip_machine_lifetime_subscribe_test)

        # ── §mesh-13 machine-lifetime unsubscribe: the frame itself ──
        # Runtime sibling of the compile test above. The DDS and
        # custom_tcp fixtures assert the unsubscribe by watching delivery
        # stop, which cannot see the `machine_subscriptions_armed_`
        # guard's actual claim — that there is exactly ONE retraction.
        # A duplicate is invisible from the receiving end because
        # delivery has already stopped.
        #
        # SOME/IP is where it is observable: a subscription is an SD
        # SubscribeEventgroup addressed to the offering application, and
        # vsomeip reports every transition to it through
        # register_subscription_handler. This test counts them, so §3's
        # "a second shutdown() emits nothing" is a direct read of the
        # wire rather than an inference.
        add_executable(mesh_someip_machine_lifetime_unsubscribe_test
            mesh/test_mesh_someip_machine_lifetime_unsubscribe.cpp
            "${MESH_SOMEIP_MLSUB_OUTPUT_DIR}/brake_someip_machine_lifetime_subscribe_sm.h"
            "${MESH_SOMEIP_MLSUB_OUTPUT_DIR}/brake_someip_machine_lifetime_subscribe_transport.h"
        )

        target_include_directories(mesh_someip_machine_lifetime_unsubscribe_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${CMAKE_CURRENT_SOURCE_DIR}/mesh
                ${MESH_SOMEIP_MLSUB_OUTPUT_DIR}
        )

        target_link_libraries(mesh_someip_machine_lifetime_unsubscribe_test
            PRIVATE
                sce_base
                sce_mesh_common
                vsomeip3
        )

        target_compile_features(mesh_someip_machine_lifetime_unsubscribe_test PRIVATE cxx_std_17)
        apply_sce_test_policies(mesh_someip_machine_lifetime_unsubscribe_test)
        add_test(NAME mesh_someip_machine_lifetime_unsubscribe
                 COMMAND mesh_someip_machine_lifetime_unsubscribe_test)
        set_tests_properties(mesh_someip_machine_lifetime_unsubscribe PROPERTIES
            TIMEOUT 30
            ENVIRONMENT "VSOMEIP_CONFIGURATION=${CMAKE_CURRENT_SOURCE_DIR}/mesh/vsomeip_e2e_machine_lifetime.json;LD_LIBRARY_PATH=${vsomeip3_DIR}:$ENV{LD_LIBRARY_PATH}")

        # ── Session F: SOME/IP runtime E2E ────────────────────────────
        # Exercises both client and server TransportRouters end-to-end
        # via vsomeip internal routing (single process, no daemon).
        # Proves: (a) offer_service + register_message_handler receives
        # requests; (b) handleServerResponse creates vsomeip response;
        # (c) RPC correlation round-trips through CBOR + vsomeip wire.
        add_executable(mesh_someip_runtime_test
            mesh/test_mesh_someip_runtime.cpp
            "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/brake_someip_multi_sm.h"
            "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/motor_someip_multi_sm.h"
            "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/brake_someip_multi_transport.h"
            "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/motor_someip_multi_transport.h"
        )

        target_include_directories(mesh_someip_runtime_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_SOMEIP_MULTI_OUTPUT_DIR}
                ${CMAKE_CURRENT_SOURCE_DIR}/mesh
        )

        target_link_libraries(mesh_someip_runtime_test
            PRIVATE
                sce_base
                sce_mesh_common
                vsomeip3
        )

        target_compile_features(mesh_someip_runtime_test PRIVATE cxx_std_17)
        apply_sce_test_policies(mesh_someip_runtime_test)
        add_test(NAME mesh_someip_runtime
                 COMMAND mesh_someip_runtime_test)
        # vsomeip builds socket paths as "<VSOMEIP_BASE_PATH>/<network>-..."
        # (utility::get_base_path, configuration_impl::load_network). Each
        # mesh_someip_* test declares a unique `network` field in its JSON
        # so /tmp/<network>-* namespaces never collide, which lets ctest -j
        # run them in parallel without a shared routing-socket lock.
        set_tests_properties(mesh_someip_runtime PROPERTIES
            TIMEOUT 30
            ENVIRONMENT "VSOMEIP_CONFIGURATION=${CMAKE_CURRENT_SOURCE_DIR}/mesh/vsomeip_e2e_runtime.json;LD_LIBRARY_PATH=${vsomeip3_DIR}:$ENV{LD_LIBRARY_PATH}")

        # ── §9.6 Session 4b: SOME/IP scxml-invoke roundtrip ──────────
        # Single-process proof that the §9.6 wire-14/20 lifecycle
        # travels over SOME/IP. Parent + worker each instantiate a
        # dedicated `<machine>_scxml_invoke_app_` vsomeip application
        # (§13 OEM boundary: SCE-reserved services 0x8100..0x81FF
        # never share an application with OEM <send> targets). The
        # fixture trivial-final child reaches `<final>` during
        # initialize(), so the worker emits wire-15 + wire-18 in the
        # same pump tick and the parent reaches State::Pass via
        # done.invoke.remote_inv.
        set(MESH_SOMEIP_SCXML_INVOKE_OUTPUT_DIR
            "${CMAKE_CURRENT_BINARY_DIR}/mesh_someip_scxml_invoke_generated")

        add_custom_command(
            OUTPUT "${MESH_SOMEIP_SCXML_INVOKE_OUTPUT_DIR}/scxml_invoke_someip_parent_sm.h"
                   "${MESH_SOMEIP_SCXML_INVOKE_OUTPUT_DIR}/scxml_invoke_someip_parent_transport.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SOMEIP_SCXML_INVOKE_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/scxml_invoke_someip_parent.scxml"
                    -l cpp -o "${MESH_SOMEIP_SCXML_INVOKE_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_scxml_invoke_someip.yaml"
                    --write-deps "${MESH_SOMEIP_SCXML_INVOKE_OUTPUT_DIR}/scxml_invoke_someip_parent_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/scxml_invoke_someip_parent.scxml"
                    "${MESH_TEST_DIR}/scxml_invoke_someip_worker.scxml"
                    "${MESH_TEST_DIR}/deploy_scxml_invoke_someip.yaml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
            DEPFILE "${MESH_SOMEIP_SCXML_INVOKE_OUTPUT_DIR}/scxml_invoke_someip_parent_sm.h.d"
            COMMENT "Mesh §9.6 Session 4b: Generating scxml_invoke_someip_parent_{sm,transport}.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_SOMEIP_SCXML_INVOKE_OUTPUT_DIR}/scxml_invoke_someip_worker_sm.h"
                   "${MESH_SOMEIP_SCXML_INVOKE_OUTPUT_DIR}/scxml_invoke_someip_worker_transport.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SOMEIP_SCXML_INVOKE_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/scxml_invoke_someip_worker.scxml"
                    -l cpp -o "${MESH_SOMEIP_SCXML_INVOKE_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_scxml_invoke_someip.yaml"
                    --write-deps "${MESH_SOMEIP_SCXML_INVOKE_OUTPUT_DIR}/scxml_invoke_someip_worker_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/scxml_invoke_someip_parent.scxml"
                    "${MESH_TEST_DIR}/scxml_invoke_someip_worker.scxml"
                    "${MESH_TEST_DIR}/deploy_scxml_invoke_someip.yaml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
            DEPFILE "${MESH_SOMEIP_SCXML_INVOKE_OUTPUT_DIR}/scxml_invoke_someip_worker_sm.h.d"
            COMMENT "Mesh §9.6 Session 4b: Generating scxml_invoke_someip_worker_{sm,transport}.h"
            VERBATIM
        )

        add_executable(mesh_someip_scxml_invoke_roundtrip_test
            mesh/test_mesh_someip_scxml_invoke_roundtrip.cpp
            "${MESH_SOMEIP_SCXML_INVOKE_OUTPUT_DIR}/scxml_invoke_someip_parent_sm.h"
            "${MESH_SOMEIP_SCXML_INVOKE_OUTPUT_DIR}/scxml_invoke_someip_parent_transport.h"
            "${MESH_SOMEIP_SCXML_INVOKE_OUTPUT_DIR}/scxml_invoke_someip_worker_sm.h"
            "${MESH_SOMEIP_SCXML_INVOKE_OUTPUT_DIR}/scxml_invoke_someip_worker_transport.h"
        )

        target_include_directories(mesh_someip_scxml_invoke_roundtrip_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_SOMEIP_SCXML_INVOKE_OUTPUT_DIR}
                ${CMAKE_CURRENT_SOURCE_DIR}/mesh
        )

        target_link_libraries(mesh_someip_scxml_invoke_roundtrip_test
            PRIVATE
                sce_runtime
                sce_mesh_common
                vsomeip3
        )

        target_compile_features(mesh_someip_scxml_invoke_roundtrip_test PRIVATE cxx_std_17)
        apply_sce_test_policies(mesh_someip_scxml_invoke_roundtrip_test)
        add_test(NAME mesh_someip_scxml_invoke_roundtrip
                 COMMAND mesh_someip_scxml_invoke_roundtrip_test)
        set_tests_properties(mesh_someip_scxml_invoke_roundtrip PROPERTIES
            TIMEOUT 30
            ENVIRONMENT "VSOMEIP_CONFIGURATION=${CMAKE_CURRENT_SOURCE_DIR}/mesh/vsomeip_scxml_invoke.json;LD_LIBRARY_PATH=${vsomeip3_DIR}:$ENV{LD_LIBRARY_PATH}")

        # ── RFC F.X-2 D8: invokeReceiveSafely boundary ───────────────
        # Defense-in-depth try-catch at the vsomeip → SCE callback
        # boundary inside the consolidated SCE-namespaced app. The test
        # invokes `SCE::Mesh::Someip::invokeReceiveSafely` directly with
        # throwing/non-throwing callbacks — no vsomeip runtime needed,
        # so it gates only on the vsomeip3 headers (already established
        # by the surrounding `if(vsomeip3_FOUND)` block).
        add_executable(mesh_someip_invoke_receive_safely_test
            mesh/test_mesh_someip_invoke_receive_safely.cpp
        )
        target_include_directories(mesh_someip_invoke_receive_safely_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
        )
        target_link_libraries(mesh_someip_invoke_receive_safely_test
            PRIVATE
                sce_runtime
                sce_mesh_common
                vsomeip3
        )
        target_compile_features(mesh_someip_invoke_receive_safely_test PRIVATE cxx_std_17)
        apply_sce_test_policies(mesh_someip_invoke_receive_safely_test)
        add_test(NAME mesh_someip_invoke_receive_safely
                 COMMAND mesh_someip_invoke_receive_safely_test)
        set_tests_properties(mesh_someip_invoke_receive_safely PROPERTIES TIMEOUT 10)

        # ── Session 5b: two-host SOME/IP scxml-invoke crossdev ───────
        # Parent and worker run in distinct Linux netns wired by veth,
        # each with its own vsomeip routing manager + unicast IP. They
        # discover each other through SOME/IP-SD multicast, exercising
        # the wire-14/15/18 lifecycle across a real network stack —
        # what the 1-process Session 4b roundtrip cannot cover because
        # loopback alone does not route multicast or expose distinct
        # routing-manager identities.
        #
        # Gated on SCE_ENABLE_NETNS_TESTS because the orchestrator
        # (run_two_host_fixture.sh) needs root + the netns from
        # setup_crossdev_netns.sh; without those it would fail in a
        # fresh checkout. Default OFF keeps `cmake .. && ctest` green
        # for non-root developers.
        if(SCE_ENABLE_NETNS_TESTS)
            include(${CMAKE_CURRENT_SOURCE_DIR}/cmake/two_host_test.cmake)

            set(MESH_SOMEIP_CROSSDEV_OUTPUT_DIR
                "${CMAKE_CURRENT_BINARY_DIR}/mesh_someip_scxml_invoke_crossdev_generated")

            add_custom_command(
                OUTPUT "${MESH_SOMEIP_CROSSDEV_OUTPUT_DIR}/scxml_invoke_someip_crossdev_parent_sm.h"
                       "${MESH_SOMEIP_CROSSDEV_OUTPUT_DIR}/scxml_invoke_someip_crossdev_parent_transport.h"
                COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SOMEIP_CROSSDEV_OUTPUT_DIR}"
                COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/scxml_invoke_someip_crossdev_parent.scxml"
                        -l cpp -o "${MESH_SOMEIP_CROSSDEV_OUTPUT_DIR}"
                        --deploy "${MESH_TEST_DIR}/deploy_scxml_invoke_someip_crossdev.yaml"
                        --write-deps "${MESH_SOMEIP_CROSSDEV_OUTPUT_DIR}/scxml_invoke_someip_crossdev_parent_sm.h.d"
                DEPENDS "${MESH_TEST_DIR}/scxml_invoke_someip_crossdev_parent.scxml"
                        "${MESH_TEST_DIR}/scxml_invoke_someip_crossdev_worker.scxml"
                        "${MESH_TEST_DIR}/deploy_scxml_invoke_someip_crossdev.yaml"
                        "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                DEPFILE "${MESH_SOMEIP_CROSSDEV_OUTPUT_DIR}/scxml_invoke_someip_crossdev_parent_sm.h.d"
                COMMENT "Mesh §9.6 Session 5b: Generating scxml_invoke_someip_crossdev_parent_{sm,transport}.h"
                VERBATIM
            )

            add_custom_command(
                OUTPUT "${MESH_SOMEIP_CROSSDEV_OUTPUT_DIR}/scxml_invoke_someip_crossdev_worker_sm.h"
                       "${MESH_SOMEIP_CROSSDEV_OUTPUT_DIR}/scxml_invoke_someip_crossdev_worker_transport.h"
                COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SOMEIP_CROSSDEV_OUTPUT_DIR}"
                COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/scxml_invoke_someip_crossdev_worker.scxml"
                        -l cpp -o "${MESH_SOMEIP_CROSSDEV_OUTPUT_DIR}"
                        --deploy "${MESH_TEST_DIR}/deploy_scxml_invoke_someip_crossdev.yaml"
                        --write-deps "${MESH_SOMEIP_CROSSDEV_OUTPUT_DIR}/scxml_invoke_someip_crossdev_worker_sm.h.d"
                DEPENDS "${MESH_TEST_DIR}/scxml_invoke_someip_crossdev_parent.scxml"
                        "${MESH_TEST_DIR}/scxml_invoke_someip_crossdev_worker.scxml"
                        "${MESH_TEST_DIR}/deploy_scxml_invoke_someip_crossdev.yaml"
                        "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                DEPFILE "${MESH_SOMEIP_CROSSDEV_OUTPUT_DIR}/scxml_invoke_someip_crossdev_worker_sm.h.d"
                COMMENT "Mesh §9.6 Session 5b: Generating scxml_invoke_someip_crossdev_worker_{sm,transport}.h"
                VERBATIM
            )

            add_executable(mesh_someip_scxml_invoke_crossdev_parent
                mesh/test_mesh_someip_scxml_invoke_crossdev_parent.cpp
                "${MESH_SOMEIP_CROSSDEV_OUTPUT_DIR}/scxml_invoke_someip_crossdev_parent_sm.h"
                "${MESH_SOMEIP_CROSSDEV_OUTPUT_DIR}/scxml_invoke_someip_crossdev_parent_transport.h"
            )
            target_include_directories(mesh_someip_scxml_invoke_crossdev_parent
                PRIVATE
                    ${CMAKE_SOURCE_DIR}/sce/include
                    ${MESH_SOMEIP_CROSSDEV_OUTPUT_DIR}
                    ${CMAKE_CURRENT_SOURCE_DIR}/mesh
            )
            target_link_libraries(mesh_someip_scxml_invoke_crossdev_parent
                PRIVATE sce_runtime sce_mesh_common vsomeip3)
            # VSOMEIP_CONFIG_PATH baked into the binary because ctest's
            # ENVIRONMENT property cannot vary by side and the two halves
            # need distinct vsomeip.json (parent on 172.16.10.1, worker
            # on 172.16.10.2).
            target_compile_definitions(mesh_someip_scxml_invoke_crossdev_parent
                PRIVATE VSOMEIP_CONFIG_PATH="${CMAKE_CURRENT_SOURCE_DIR}/mesh/vsomeip_scxml_invoke_crossdev_parent.json")
            target_compile_features(mesh_someip_scxml_invoke_crossdev_parent PRIVATE cxx_std_17)
            apply_sce_test_policies(mesh_someip_scxml_invoke_crossdev_parent)

            add_executable(mesh_someip_scxml_invoke_crossdev_worker
                mesh/test_mesh_someip_scxml_invoke_crossdev_worker.cpp
                "${MESH_SOMEIP_CROSSDEV_OUTPUT_DIR}/scxml_invoke_someip_crossdev_worker_sm.h"
                "${MESH_SOMEIP_CROSSDEV_OUTPUT_DIR}/scxml_invoke_someip_crossdev_worker_transport.h"
            )
            target_include_directories(mesh_someip_scxml_invoke_crossdev_worker
                PRIVATE
                    ${CMAKE_SOURCE_DIR}/sce/include
                    ${MESH_SOMEIP_CROSSDEV_OUTPUT_DIR}
                    ${CMAKE_CURRENT_SOURCE_DIR}/mesh
            )
            target_link_libraries(mesh_someip_scxml_invoke_crossdev_worker
                PRIVATE sce_runtime sce_mesh_common vsomeip3)
            target_compile_definitions(mesh_someip_scxml_invoke_crossdev_worker
                PRIVATE VSOMEIP_CONFIG_PATH="${CMAKE_CURRENT_SOURCE_DIR}/mesh/vsomeip_scxml_invoke_crossdev_worker.json")
            target_compile_features(mesh_someip_scxml_invoke_crossdev_worker PRIVATE cxx_std_17)
            apply_sce_test_policies(mesh_someip_scxml_invoke_crossdev_worker)

            sce_register_two_host_mesh_test(
                NAME    mesh_someip_scxml_invoke_crossdev
                WORKER  mesh_someip_scxml_invoke_crossdev_worker
                PARENT  mesh_someip_scxml_invoke_crossdev_parent
                TIMEOUT 60)
            # vsomeip routing-manager UNIX sockets live in /tmp, which is
            # shared across netns. Distinct `network` fields per side keep
            # paths apart, but two parallel runs of *this* fixture would
            # still race on the same paths — serialise via RESOURCE_LOCK.
            set_tests_properties(mesh_someip_scxml_invoke_crossdev PROPERTIES
                ENVIRONMENT "LD_LIBRARY_PATH=${vsomeip3_DIR}:$ENV{LD_LIBRARY_PATH}"
                RESOURCE_LOCK sce_mesh_someip_crossdev_netns)

            # ── Session 5c: two-host SOME/IP donedata crossdev ───────
            # Sister to mesh_zenoh_session_f_crossdev_donedata (Session
            # 5c Zenoh) and mesh_someip_scxml_invoke_crossdev (5b
            # lifecycle). Reuses parent_session_f_donedata.scxml + the
            # three worker SCXMLs verbatim — same §9.6 transport-axis
            # abstraction proof as the Zenoh donedata fixture, but over
            # SOMEIP-SD multicast + vsomeip routing-manager instead of
            # Zenoh's TCP gossip.
            #
            # Worker process hosts THREE vsomeip applications (one per
            # worker machine, codegen-derived `<machine>_scxml_invoke`
            # names). Worker-side `routing` field selects
            # `worker_session_f_donedata_content_scxml_invoke` as the
            # routing-manager owner; the other two run as RM-clients.
            # First fixture in SCE that exercises this multi-app shape
            # live across netns — if vsomeip mishandles RM-client
            # multiplexing the regression surfaces here.
            #
            # Service IDs are FNV-1a hashes per machine | 0x8100:
            #   parent: 0x81f4 / port 30599
            #   worker param: 0x8128 / port 30586
            #   worker content: 0x81ec / port 30587
            #   worker nested: 0x8120 / port 30588
            # Distinct from 5b lifecycle (worker port 30556, parent
            # 30568) so /tmp socket churn between the two fixtures
            # under parallel ctest doesn't cross-contaminate.
            set(MESH_SOMEIP_CROSSDEV_DONEDATA_OUTPUT_DIR
                "${CMAKE_CURRENT_BINARY_DIR}/mesh_someip_session_f_crossdev_donedata_generated")

            set(MESH_SOMEIP_CROSSDEV_DONEDATA_SCXMLS
                parent_session_f_donedata
                worker_session_f_donedata_param
                worker_session_f_donedata_content
                worker_session_f_donedata_nested)

            set(MESH_SOMEIP_CROSSDEV_DONEDATA_SCXML_DEPS
                "${MESH_TEST_DIR}/parent_session_f_donedata.scxml"
                "${MESH_TEST_DIR}/worker_session_f_donedata_param.scxml"
                "${MESH_TEST_DIR}/worker_session_f_donedata_content.scxml"
                "${MESH_TEST_DIR}/worker_session_f_donedata_nested.scxml")

            foreach(machine ${MESH_SOMEIP_CROSSDEV_DONEDATA_SCXMLS})
                add_custom_command(
                    OUTPUT "${MESH_SOMEIP_CROSSDEV_DONEDATA_OUTPUT_DIR}/${machine}_sm.h"
                            "${MESH_SOMEIP_CROSSDEV_DONEDATA_OUTPUT_DIR}/${machine}_transport.h"
                    COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SOMEIP_CROSSDEV_DONEDATA_OUTPUT_DIR}"
                    COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/${machine}.scxml"
                            -l cpp -o "${MESH_SOMEIP_CROSSDEV_DONEDATA_OUTPUT_DIR}"
                            --deploy "${MESH_TEST_DIR}/deploy_session_f_donedata_someip_crossdev.yaml"
                            --write-deps "${MESH_SOMEIP_CROSSDEV_DONEDATA_OUTPUT_DIR}/${machine}_sm.h.d"
                    DEPENDS ${MESH_SOMEIP_CROSSDEV_DONEDATA_SCXML_DEPS}
                            "${MESH_TEST_DIR}/deploy_session_f_donedata_someip_crossdev.yaml"
                            "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    DEPFILE "${MESH_SOMEIP_CROSSDEV_DONEDATA_OUTPUT_DIR}/${machine}_sm.h.d"
                    COMMENT "Mesh §9.6 Session 5c SOME/IP donedata: Generating ${machine}_{sm,transport}.h"
                    VERBATIM
                )
            endforeach()

            add_executable(mesh_someip_session_f_crossdev_donedata_worker
                mesh/test_mesh_someip_session_f_crossdev_donedata_worker.cpp
                "${MESH_SOMEIP_CROSSDEV_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_param_sm.h"
                "${MESH_SOMEIP_CROSSDEV_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_param_transport.h"
                "${MESH_SOMEIP_CROSSDEV_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_content_sm.h"
                "${MESH_SOMEIP_CROSSDEV_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_content_transport.h"
                "${MESH_SOMEIP_CROSSDEV_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_nested_sm.h"
                "${MESH_SOMEIP_CROSSDEV_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_nested_transport.h")
            target_include_directories(mesh_someip_session_f_crossdev_donedata_worker
                PRIVATE
                    ${CMAKE_SOURCE_DIR}/sce/include
                    ${MESH_SOMEIP_CROSSDEV_DONEDATA_OUTPUT_DIR}
                    ${CMAKE_CURRENT_SOURCE_DIR}/mesh)
            target_link_libraries(mesh_someip_session_f_crossdev_donedata_worker
                PRIVATE sce_runtime sce_mesh_common vsomeip3)
            target_compile_definitions(mesh_someip_session_f_crossdev_donedata_worker
                PRIVATE VSOMEIP_CONFIG_PATH="${CMAKE_CURRENT_SOURCE_DIR}/mesh/vsomeip_session_f_donedata_crossdev_worker.json")
            target_compile_features(mesh_someip_session_f_crossdev_donedata_worker PRIVATE cxx_std_17)
            apply_sce_test_policies(mesh_someip_session_f_crossdev_donedata_worker)

            add_executable(mesh_someip_session_f_crossdev_donedata_parent
                mesh/test_mesh_someip_session_f_crossdev_donedata_parent.cpp
                "${MESH_SOMEIP_CROSSDEV_DONEDATA_OUTPUT_DIR}/parent_session_f_donedata_sm.h"
                "${MESH_SOMEIP_CROSSDEV_DONEDATA_OUTPUT_DIR}/parent_session_f_donedata_transport.h")
            target_include_directories(mesh_someip_session_f_crossdev_donedata_parent
                PRIVATE
                    ${CMAKE_SOURCE_DIR}/sce/include
                    ${MESH_SOMEIP_CROSSDEV_DONEDATA_OUTPUT_DIR}
                    ${CMAKE_CURRENT_SOURCE_DIR}/mesh)
            target_link_libraries(mesh_someip_session_f_crossdev_donedata_parent
                PRIVATE sce_runtime sce_mesh_common vsomeip3)
            target_compile_definitions(mesh_someip_session_f_crossdev_donedata_parent
                PRIVATE VSOMEIP_CONFIG_PATH="${CMAKE_CURRENT_SOURCE_DIR}/mesh/vsomeip_session_f_donedata_crossdev_parent.json")
            target_compile_features(mesh_someip_session_f_crossdev_donedata_parent PRIVATE cxx_std_17)
            apply_sce_test_policies(mesh_someip_session_f_crossdev_donedata_parent)

            sce_register_two_host_mesh_test(
                NAME    mesh_someip_session_f_crossdev_donedata
                WORKER  mesh_someip_session_f_crossdev_donedata_worker
                PARENT  mesh_someip_session_f_crossdev_donedata_parent
                TIMEOUT 60)
            # Share RESOURCE_LOCK with mesh_someip_scxml_invoke_crossdev
            # (5b lifecycle): both fixtures bind their parent RM to the
            # SAME netns + SAME SD multicast group (224.244.224.245:30490).
            # Distinct `network` fields keep /tmp UNIX socket paths apart,
            # but the multicast group is shared by netns design — running
            # 5b and 5c in parallel under ctest -j cross-pollutes their SD
            # offer/find traffic and one parent never converges within its
            # 20 s deadline. Distinct RESOURCE_LOCK names had this exact
            # symptom in initial validation; sharing the lock serialises
            # them at ctest level which is the textbook resolution for
            # multicast-group sharing across fixtures.
            set_tests_properties(mesh_someip_session_f_crossdev_donedata PROPERTIES
                ENVIRONMENT "LD_LIBRARY_PATH=${vsomeip3_DIR}:$ENV{LD_LIBRARY_PATH}"
                RESOURCE_LOCK sce_mesh_someip_crossdev_netns)
        endif()

        # ── §16.4 region-partition liveness (RFC F.X-3 SOMEIP) ───────
        # Sibling to mesh_zenoh_region_liveness_verification (Zenoh row
        # 13). Stage A-C of RFC F.X-3 wired the codegen path
        # (assigner → DeployError variants → DiagnosticCode variants →
        # template SOMEIP-side liveness emit on the F.X-2 consolidated
        # SCE app); Stage D lands the live netns fixture proving the
        # row-13 raise survives the cross-host SOMEIP-SD multicast +
        # vsomeip routing-manager STOP_OFFER round-trip. Reuses
        # brake_region_liveness.scxml + peer_region_liveness.scxml
        # verbatim (per RFC §4) — the SCXML layer is transport-agnostic;
        # only deploy.yaml swaps the wire and the partition split is
        # identical to the Zenoh fixture.
        #
        # The codegen + compile-verification pair is always-on (inside
        # the vsomeip3 gate) so a regression in the §16.4 SOMEIP
        # template branch surfaces in fresh checkouts too. The runtime
        # two-host fixture below is netns-gated.
        set(MESH_SOMEIP_REGION_LIVENESS_OUTPUT_DIR
            "${CMAKE_CURRENT_BINARY_DIR}/mesh_someip_region_liveness_generated")

        foreach(part brake_left_part brake_right_part)
            add_custom_command(
                OUTPUT "${MESH_SOMEIP_REGION_LIVENESS_OUTPUT_DIR}/${part}/brake_region_liveness_sm.h"
                       "${MESH_SOMEIP_REGION_LIVENESS_OUTPUT_DIR}/${part}/brake_region_liveness_transport.h"
                COMMAND ${CMAKE_COMMAND} -E make_directory
                        "${MESH_SOMEIP_REGION_LIVENESS_OUTPUT_DIR}/${part}"
                COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_region_liveness.scxml"
                        -l cpp -o "${MESH_SOMEIP_REGION_LIVENESS_OUTPUT_DIR}/${part}"
                        --deploy "${MESH_TEST_DIR}/deploy_someip_region_liveness.yaml"
                        --partition ${part}
                        --write-deps
                        "${MESH_SOMEIP_REGION_LIVENESS_OUTPUT_DIR}/${part}/brake_region_liveness_sm.h.d"
                DEPENDS "${MESH_TEST_DIR}/brake_region_liveness.scxml"
                        "${MESH_TEST_DIR}/peer_region_liveness.scxml"
                        "${MESH_TEST_DIR}/deploy_someip_region_liveness.yaml"
                        "${MESH_TEST_DIR}/vsomeip_someip_region_liveness_codegen.json"
                        "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                DEPFILE "${MESH_SOMEIP_REGION_LIVENESS_OUTPUT_DIR}/${part}/brake_region_liveness_sm.h.d"
                COMMENT "Mesh §16.4 SOMEIP region liveness: Generating ${part}/brake_region_liveness_{sm,transport}.h"
                VERBATIM
            )
        endforeach()

        # Per-partition compile-verification binaries. Two binaries are
        # required because both --partition outputs share the
        # `SCE::Generated::brake_region_liveness::` namespace — including
        # both `_transport.h` files in one TU would ODR-violate every
        # inline definition. SCE_LIVENESS_PARTITION_{LEFT,RIGHT} drives
        # the per-partition `static_assert` block in the test source so
        # the lex-sorted counter assignments (left → 0x8180, right →
        # 0x8181) are byte-pinned at compile time.
        add_executable(mesh_someip_region_liveness_left_compile_test
            mesh/test_mesh_someip_region_liveness_compile.cpp
            "${MESH_SOMEIP_REGION_LIVENESS_OUTPUT_DIR}/brake_left_part/brake_region_liveness_sm.h"
            "${MESH_SOMEIP_REGION_LIVENESS_OUTPUT_DIR}/brake_left_part/brake_region_liveness_transport.h")
        target_include_directories(mesh_someip_region_liveness_left_compile_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                "${MESH_SOMEIP_REGION_LIVENESS_OUTPUT_DIR}/brake_left_part"
                ${CMAKE_CURRENT_SOURCE_DIR}/mesh)
        target_link_libraries(mesh_someip_region_liveness_left_compile_test
            PRIVATE sce_runtime sce_mesh_common vsomeip3)
        target_compile_definitions(mesh_someip_region_liveness_left_compile_test
            PRIVATE SCE_LIVENESS_PARTITION_LEFT)
        target_compile_features(mesh_someip_region_liveness_left_compile_test PRIVATE cxx_std_17)
        sce_pin_cxx17(mesh_someip_region_liveness_left_compile_test)
        apply_sce_test_policies(mesh_someip_region_liveness_left_compile_test)
        add_test(NAME mesh_someip_region_liveness_left_compile
                 COMMAND mesh_someip_region_liveness_left_compile_test)
        set_tests_properties(mesh_someip_region_liveness_left_compile PROPERTIES TIMEOUT 5)

        add_executable(mesh_someip_region_liveness_right_compile_test
            mesh/test_mesh_someip_region_liveness_compile.cpp
            "${MESH_SOMEIP_REGION_LIVENESS_OUTPUT_DIR}/brake_right_part/brake_region_liveness_sm.h"
            "${MESH_SOMEIP_REGION_LIVENESS_OUTPUT_DIR}/brake_right_part/brake_region_liveness_transport.h")
        target_include_directories(mesh_someip_region_liveness_right_compile_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                "${MESH_SOMEIP_REGION_LIVENESS_OUTPUT_DIR}/brake_right_part"
                ${CMAKE_CURRENT_SOURCE_DIR}/mesh)
        target_link_libraries(mesh_someip_region_liveness_right_compile_test
            PRIVATE sce_runtime sce_mesh_common vsomeip3)
        target_compile_definitions(mesh_someip_region_liveness_right_compile_test
            PRIVATE SCE_LIVENESS_PARTITION_RIGHT)
        target_compile_features(mesh_someip_region_liveness_right_compile_test PRIVATE cxx_std_17)
        sce_pin_cxx17(mesh_someip_region_liveness_right_compile_test)
        apply_sce_test_policies(mesh_someip_region_liveness_right_compile_test)
        add_test(NAME mesh_someip_region_liveness_right_compile
                 COMMAND mesh_someip_region_liveness_right_compile_test)
        set_tests_properties(mesh_someip_region_liveness_right_compile PROPERTIES TIMEOUT 5)

        # ── §16.7 row 8 machine-level liveness (RFC F.X-4 SOMEIP) ────
        # Sister to mesh_someip_region_liveness above (RFC F.X-3 row 13).
        # F.X-4 D1 carves a third disjoint SCE-internal sub-range
        # [0x8280, 0x82FF] for machine-level liveness; the
        # `register_availability_handler` shape mirrors F.X-3 but on
        # per-machine service IDs (vs F.X-3's per-partition keys), so
        # single-partition SOMEIP machines that opt into `liveliness:`
        # are now valid under D4-shape-1 implicit machine-level emit.
        # Stage C lands the compile-verification gate; Stage D adds the
        # netns-runtime fixture (kill peer SCE app → SCXML observes
        # `error.communication / PEER_PARTITIONED`).
        set(MESH_SOMEIP_MACHINE_LIVENESS_OUTPUT_DIR
            "${CMAKE_CURRENT_BINARY_DIR}/mesh_someip_machine_liveness_generated")

        foreach(machine_id alpha beta)
            set(machine_name ${machine_id}_machine_liveness)
            add_custom_command(
                OUTPUT "${MESH_SOMEIP_MACHINE_LIVENESS_OUTPUT_DIR}/${machine_id}/${machine_name}_sm.h"
                       "${MESH_SOMEIP_MACHINE_LIVENESS_OUTPUT_DIR}/${machine_id}/${machine_name}_transport.h"
                COMMAND ${CMAKE_COMMAND} -E make_directory
                        "${MESH_SOMEIP_MACHINE_LIVENESS_OUTPUT_DIR}/${machine_id}"
                COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/${machine_name}.scxml"
                        -l cpp -o "${MESH_SOMEIP_MACHINE_LIVENESS_OUTPUT_DIR}/${machine_id}"
                        --deploy "${MESH_TEST_DIR}/deploy_someip_machine_liveness.yaml"
                        --write-deps
                        "${MESH_SOMEIP_MACHINE_LIVENESS_OUTPUT_DIR}/${machine_id}/${machine_name}_sm.h.d"
                DEPENDS "${MESH_TEST_DIR}/alpha_machine_liveness.scxml"
                        "${MESH_TEST_DIR}/beta_machine_liveness.scxml"
                        "${MESH_TEST_DIR}/deploy_someip_machine_liveness.yaml"
                        "${MESH_TEST_DIR}/vsomeip_someip_machine_liveness_codegen.json"
                        "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                DEPFILE "${MESH_SOMEIP_MACHINE_LIVENESS_OUTPUT_DIR}/${machine_id}/${machine_name}_sm.h.d"
                COMMENT "Mesh §16.7 SOMEIP machine liveness: Generating ${machine_id}/${machine_name}_{sm,transport}.h"
                VERBATIM
            )
        endforeach()

        # Per-machine compile-verification binaries. Two binaries are
        # required because each machine's generated files live under its
        # own `SCE::Generated::<machine>::` namespace; SCE_F4_MACHINE_*
        # macros drive the per-machine `static_assert` block in the test
        # source so the lex-sorted counter assignments (alpha → 0x8280,
        # beta → 0x8281) are byte-pinned at compile time.
        add_executable(mesh_someip_machine_liveness_alpha_compile_test
            mesh/test_mesh_someip_machine_liveness_compile.cpp
            "${MESH_SOMEIP_MACHINE_LIVENESS_OUTPUT_DIR}/alpha/alpha_machine_liveness_sm.h"
            "${MESH_SOMEIP_MACHINE_LIVENESS_OUTPUT_DIR}/alpha/alpha_machine_liveness_transport.h")
        target_include_directories(mesh_someip_machine_liveness_alpha_compile_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                "${MESH_SOMEIP_MACHINE_LIVENESS_OUTPUT_DIR}/alpha"
                ${CMAKE_CURRENT_SOURCE_DIR}/mesh)
        target_link_libraries(mesh_someip_machine_liveness_alpha_compile_test
            PRIVATE sce_runtime sce_mesh_common vsomeip3)
        target_compile_definitions(mesh_someip_machine_liveness_alpha_compile_test
            PRIVATE SCE_F4_MACHINE_ALPHA)
        target_compile_features(mesh_someip_machine_liveness_alpha_compile_test PRIVATE cxx_std_17)
        sce_pin_cxx17(mesh_someip_machine_liveness_alpha_compile_test)
        apply_sce_test_policies(mesh_someip_machine_liveness_alpha_compile_test)
        add_test(NAME mesh_someip_machine_liveness_alpha_compile
                 COMMAND mesh_someip_machine_liveness_alpha_compile_test)
        set_tests_properties(mesh_someip_machine_liveness_alpha_compile PROPERTIES TIMEOUT 5)

        add_executable(mesh_someip_machine_liveness_beta_compile_test
            mesh/test_mesh_someip_machine_liveness_compile.cpp
            "${MESH_SOMEIP_MACHINE_LIVENESS_OUTPUT_DIR}/beta/beta_machine_liveness_sm.h"
            "${MESH_SOMEIP_MACHINE_LIVENESS_OUTPUT_DIR}/beta/beta_machine_liveness_transport.h")
        target_include_directories(mesh_someip_machine_liveness_beta_compile_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                "${MESH_SOMEIP_MACHINE_LIVENESS_OUTPUT_DIR}/beta"
                ${CMAKE_CURRENT_SOURCE_DIR}/mesh)
        target_link_libraries(mesh_someip_machine_liveness_beta_compile_test
            PRIVATE sce_runtime sce_mesh_common vsomeip3)
        target_compile_definitions(mesh_someip_machine_liveness_beta_compile_test
            PRIVATE SCE_F4_MACHINE_BETA)
        target_compile_features(mesh_someip_machine_liveness_beta_compile_test PRIVATE cxx_std_17)
        sce_pin_cxx17(mesh_someip_machine_liveness_beta_compile_test)
        apply_sce_test_policies(mesh_someip_machine_liveness_beta_compile_test)
        add_test(NAME mesh_someip_machine_liveness_beta_compile
                 COMMAND mesh_someip_machine_liveness_beta_compile_test)
        set_tests_properties(mesh_someip_machine_liveness_beta_compile PROPERTIES TIMEOUT 5)

        if(SCE_ENABLE_NETNS_TESTS)
            include(${CMAKE_CURRENT_SOURCE_DIR}/cmake/two_host_test.cmake)

            add_executable(mesh_someip_region_liveness_parent
                mesh/test_mesh_someip_region_liveness_parent.cpp
                "${MESH_SOMEIP_REGION_LIVENESS_OUTPUT_DIR}/brake_left_part/brake_region_liveness_sm.h"
                "${MESH_SOMEIP_REGION_LIVENESS_OUTPUT_DIR}/brake_left_part/brake_region_liveness_transport.h")
            target_include_directories(mesh_someip_region_liveness_parent
                PRIVATE
                    ${CMAKE_SOURCE_DIR}/sce/include
                    "${MESH_SOMEIP_REGION_LIVENESS_OUTPUT_DIR}/brake_left_part"
                    ${CMAKE_CURRENT_SOURCE_DIR}/mesh)
            target_link_libraries(mesh_someip_region_liveness_parent
                PRIVATE sce_runtime sce_mesh_common vsomeip3)
            target_compile_definitions(mesh_someip_region_liveness_parent
                PRIVATE VSOMEIP_CONFIG_PATH="${CMAKE_CURRENT_SOURCE_DIR}/mesh/vsomeip_someip_region_liveness_left.json")
            target_compile_features(mesh_someip_region_liveness_parent PRIVATE cxx_std_17)
            apply_sce_test_policies(mesh_someip_region_liveness_parent)

            add_executable(mesh_someip_region_liveness_worker
                mesh/test_mesh_someip_region_liveness_worker.cpp
                "${MESH_SOMEIP_REGION_LIVENESS_OUTPUT_DIR}/brake_right_part/brake_region_liveness_sm.h"
                "${MESH_SOMEIP_REGION_LIVENESS_OUTPUT_DIR}/brake_right_part/brake_region_liveness_transport.h")
            target_include_directories(mesh_someip_region_liveness_worker
                PRIVATE
                    ${CMAKE_SOURCE_DIR}/sce/include
                    "${MESH_SOMEIP_REGION_LIVENESS_OUTPUT_DIR}/brake_right_part"
                    ${CMAKE_CURRENT_SOURCE_DIR}/mesh)
            target_link_libraries(mesh_someip_region_liveness_worker
                PRIVATE sce_runtime sce_mesh_common vsomeip3)
            target_compile_definitions(mesh_someip_region_liveness_worker
                PRIVATE VSOMEIP_CONFIG_PATH="${CMAKE_CURRENT_SOURCE_DIR}/mesh/vsomeip_someip_region_liveness_right.json")
            target_compile_features(mesh_someip_region_liveness_worker PRIVATE cxx_std_17)
            apply_sce_test_policies(mesh_someip_region_liveness_worker)

            sce_register_two_host_mesh_test(
                NAME    mesh_someip_region_liveness_verification
                WORKER  mesh_someip_region_liveness_worker
                PARENT  mesh_someip_region_liveness_parent
                TIMEOUT 30)
            # RESOURCE_LOCK shared with mesh_someip_scxml_invoke_crossdev
            # (5b) and mesh_someip_session_f_crossdev_donedata (5c) per
            # `feedback_someip_netns_lock_sharing.md`: every SOMEIP netns
            # fixture binds its RM into the same sce-mesh-{parent,worker}
            # netns + multicast group (224.244.224.245:30490). Distinct
            # `network` fields keep /tmp UNIX socket paths apart, but the
            # multicast group is shared by netns design — running two such
            # fixtures in parallel under `ctest -j` cross-pollutes their
            # SD offer/find traffic and one parent never converges within
            # its deadline. Sharing the lock is the textbook resolution.
            set_tests_properties(mesh_someip_region_liveness_verification PROPERTIES
                ENVIRONMENT "LD_LIBRARY_PATH=${vsomeip3_DIR}:$ENV{LD_LIBRARY_PATH}"
                RESOURCE_LOCK sce_mesh_someip_crossdev_netns)

            # ── §16.7 row 8 machine-level liveness (RFC F.X-4) ───────
            # Sister to mesh_someip_region_liveness_verification above.
            # Single-partition shape — alpha + beta have NO `partitions:`
            # block, so the F.X-3 region-liveness axis is silent (zero
            # row-13 participants under the assigner's narrowed projection)
            # and ONLY the F.X-4 machine-level axis emits handlers. The
            # parent driver pins this orthogonality at runtime: a stray
            # row-13 raise on this trace would be a codegen axis-confusion
            # regression. RESOURCE_LOCK shared with all SOMEIP netns
            # fixtures (multicast group + sce-mesh-{parent,worker} netns
            # are shared by netns design).
            add_executable(mesh_someip_machine_liveness_parent
                mesh/test_mesh_someip_machine_liveness_parent.cpp
                "${MESH_SOMEIP_MACHINE_LIVENESS_OUTPUT_DIR}/alpha/alpha_machine_liveness_sm.h"
                "${MESH_SOMEIP_MACHINE_LIVENESS_OUTPUT_DIR}/alpha/alpha_machine_liveness_transport.h")
            target_include_directories(mesh_someip_machine_liveness_parent
                PRIVATE
                    ${CMAKE_SOURCE_DIR}/sce/include
                    "${MESH_SOMEIP_MACHINE_LIVENESS_OUTPUT_DIR}/alpha"
                    ${CMAKE_CURRENT_SOURCE_DIR}/mesh)
            target_link_libraries(mesh_someip_machine_liveness_parent
                PRIVATE sce_runtime sce_mesh_common vsomeip3)
            target_compile_definitions(mesh_someip_machine_liveness_parent
                PRIVATE VSOMEIP_CONFIG_PATH="${CMAKE_CURRENT_SOURCE_DIR}/mesh/vsomeip_someip_machine_liveness_alpha.json")
            target_compile_features(mesh_someip_machine_liveness_parent PRIVATE cxx_std_17)
            apply_sce_test_policies(mesh_someip_machine_liveness_parent)

            add_executable(mesh_someip_machine_liveness_worker
                mesh/test_mesh_someip_machine_liveness_worker.cpp
                "${MESH_SOMEIP_MACHINE_LIVENESS_OUTPUT_DIR}/beta/beta_machine_liveness_sm.h"
                "${MESH_SOMEIP_MACHINE_LIVENESS_OUTPUT_DIR}/beta/beta_machine_liveness_transport.h")
            target_include_directories(mesh_someip_machine_liveness_worker
                PRIVATE
                    ${CMAKE_SOURCE_DIR}/sce/include
                    "${MESH_SOMEIP_MACHINE_LIVENESS_OUTPUT_DIR}/beta"
                    ${CMAKE_CURRENT_SOURCE_DIR}/mesh)
            target_link_libraries(mesh_someip_machine_liveness_worker
                PRIVATE sce_runtime sce_mesh_common vsomeip3)
            target_compile_definitions(mesh_someip_machine_liveness_worker
                PRIVATE VSOMEIP_CONFIG_PATH="${CMAKE_CURRENT_SOURCE_DIR}/mesh/vsomeip_someip_machine_liveness_beta.json")
            target_compile_features(mesh_someip_machine_liveness_worker PRIVATE cxx_std_17)
            apply_sce_test_policies(mesh_someip_machine_liveness_worker)

            sce_register_two_host_mesh_test(
                NAME    mesh_someip_machine_liveness_verification
                WORKER  mesh_someip_machine_liveness_worker
                PARENT  mesh_someip_machine_liveness_parent
                TIMEOUT 30)
            set_tests_properties(mesh_someip_machine_liveness_verification PROPERTIES
                ENVIRONMENT "LD_LIBRARY_PATH=${vsomeip3_DIR}:$ENV{LD_LIBRARY_PATH}"
                RESOURCE_LOCK sce_mesh_someip_crossdev_netns)
        endif()

        # ── Session I: engine-driven RPC/FieldRead over SOME/IP ──────
        # Sibling to mesh_someip_runtime — binds the real motor_someip_multi
        # SM to TransportRouter and drives it with a step() pump so the
        # reply is produced by the engine's onentry <raise> and codegen-
        # injected <send>. Brake stays on TestSenderEngine (test harness
        # observing the reply, not system under test).
        add_executable(mesh_someip_rpc_engine_driven_test
            mesh/test_mesh_someip_rpc_engine_driven.cpp
            "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/brake_someip_multi_sm.h"
            "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/motor_someip_multi_sm.h"
            "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/brake_someip_multi_transport.h"
            "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/motor_someip_multi_transport.h"
        )

        target_include_directories(mesh_someip_rpc_engine_driven_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_SOMEIP_MULTI_OUTPUT_DIR}
                ${CMAKE_CURRENT_SOURCE_DIR}/mesh
        )

        target_link_libraries(mesh_someip_rpc_engine_driven_test
            PRIVATE
                sce_base
                sce_mesh_common
                vsomeip3
        )

        target_compile_features(mesh_someip_rpc_engine_driven_test PRIVATE cxx_std_17)
        apply_sce_test_policies(mesh_someip_rpc_engine_driven_test)
        add_test(NAME mesh_someip_rpc_engine_driven
                 COMMAND mesh_someip_rpc_engine_driven_test)
        set_tests_properties(mesh_someip_rpc_engine_driven PROPERTIES
            TIMEOUT 30
            ENVIRONMENT "VSOMEIP_CONFIGURATION=${CMAKE_CURRENT_SOURCE_DIR}/mesh/vsomeip_e2e_rpc_engine.json;LD_LIBRARY_PATH=${vsomeip3_DIR}:$ENV{LD_LIBRARY_PATH}")

        # ── §9.5 server response deadline over live SOME/IP ──────────
        # Motor declares `server.response_deadline_ms: 500` and binds a
        # recorder engine that never answers, so the deadline is the only
        # thing that can retire the stashed request. Unlike the zenoh arm
        # — where expiry drops the stored query and the client can only
        # infer Unavailable — SOME/IP answers MT_ERROR / E_TIMEOUT, so
        # brake is present to observe that the notice actually arrives.
        set(MESH_SOMEIP_SERVER_DEADLINE_OUTPUT_DIR
            "${CMAKE_CURRENT_BINARY_DIR}/mesh_someip_server_deadline_generated")

        foreach(SD_MACHINE brake_someip_server_deadline motor_someip_server_deadline)
            add_custom_command(
                OUTPUT "${MESH_SOMEIP_SERVER_DEADLINE_OUTPUT_DIR}/${SD_MACHINE}_sm.h"
                COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SOMEIP_SERVER_DEADLINE_OUTPUT_DIR}"
                COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/${SD_MACHINE}.scxml"
                        -l cpp -o "${MESH_SOMEIP_SERVER_DEADLINE_OUTPUT_DIR}"
                        --deploy "${MESH_TEST_DIR}/deploy_someip_server_deadline.yaml"
                        --write-deps "${MESH_SOMEIP_SERVER_DEADLINE_OUTPUT_DIR}/${SD_MACHINE}_sm.h.d"
                DEPENDS "${MESH_TEST_DIR}/${SD_MACHINE}.scxml" "${SCE_CODEGEN}"
                DEPFILE "${MESH_SOMEIP_SERVER_DEADLINE_OUTPUT_DIR}/${SD_MACHINE}_sm.h.d"
                COMMENT "Mesh someip server deadline: Generating ${SD_MACHINE}_sm.h"
                VERBATIM
            )

            add_custom_command(
                OUTPUT "${MESH_SOMEIP_SERVER_DEADLINE_OUTPUT_DIR}/${SD_MACHINE}_transport.h"
                COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/${SD_MACHINE}.scxml"
                        -l cpp -o "${MESH_SOMEIP_SERVER_DEADLINE_OUTPUT_DIR}"
                        --deploy "${MESH_TEST_DIR}/deploy_someip_server_deadline.yaml"
                        --transport-only
                        --write-deps "${MESH_SOMEIP_SERVER_DEADLINE_OUTPUT_DIR}/${SD_MACHINE}_transport.h.d"
                DEPENDS "${MESH_TEST_DIR}/${SD_MACHINE}.scxml"
                        "${MESH_TEST_DIR}/deploy_someip_server_deadline.yaml"
                        "${MESH_TEST_DIR}/vsomeip_someip_server_deadline.json"
                        "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                        "${MESH_SOMEIP_SERVER_DEADLINE_OUTPUT_DIR}/${SD_MACHINE}_sm.h"
                DEPFILE "${MESH_SOMEIP_SERVER_DEADLINE_OUTPUT_DIR}/${SD_MACHINE}_transport.h.d"
                COMMENT "Mesh someip server deadline: Generating ${SD_MACHINE}_transport.h"
                VERBATIM
            )
        endforeach()

        add_executable(mesh_someip_server_deadline_test
            mesh/test_mesh_someip_server_deadline.cpp
            "${MESH_SOMEIP_SERVER_DEADLINE_OUTPUT_DIR}/brake_someip_server_deadline_sm.h"
            "${MESH_SOMEIP_SERVER_DEADLINE_OUTPUT_DIR}/motor_someip_server_deadline_sm.h"
            "${MESH_SOMEIP_SERVER_DEADLINE_OUTPUT_DIR}/brake_someip_server_deadline_transport.h"
            "${MESH_SOMEIP_SERVER_DEADLINE_OUTPUT_DIR}/motor_someip_server_deadline_transport.h"
        )

        target_include_directories(mesh_someip_server_deadline_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_SOMEIP_SERVER_DEADLINE_OUTPUT_DIR}
                ${CMAKE_CURRENT_SOURCE_DIR}/mesh
        )

        target_link_libraries(mesh_someip_server_deadline_test
            PRIVATE
                sce_base
                sce_mesh_common
                vsomeip3
        )

        # SCE_THREAD_SAFE: the scheduler thread fires
        # onServerRequestTimedOut concurrently with the main thread
        # reading pending_server_size().
        target_compile_definitions(mesh_someip_server_deadline_test PRIVATE SCE_THREAD_SAFE)
        target_compile_features(mesh_someip_server_deadline_test PRIVATE cxx_std_17)
        apply_sce_test_policies(mesh_someip_server_deadline_test)
        add_test(NAME mesh_someip_server_deadline_verification
                 COMMAND mesh_someip_server_deadline_test)
        set_tests_properties(mesh_someip_server_deadline_verification PROPERTIES
            TIMEOUT 60
            ENVIRONMENT "VSOMEIP_CONFIGURATION=${CMAKE_CURRENT_SOURCE_DIR}/mesh/vsomeip_e2e_server_deadline.json;LD_LIBRARY_PATH=${vsomeip3_DIR}:$ENV{LD_LIBRARY_PATH}")

        # ── §14.6 same-target constraint adjudication over live SOME/IP ──
        # A client with two SOME/IP RPC targets, so codegen installs a
        # response handler on each service and both consult the same
        # router-scoped pending_rpcs_ table. Both servers are raw vsomeip
        # applications owned by the test: the adjudication needs a server
        # that replies with a correlation_id it did not receive, which a
        # generated server never does.
        set(MESH_CROSS_TARGET_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/mesh_cross_target_generated")

        add_custom_command(
            OUTPUT "${MESH_CROSS_TARGET_OUTPUT_DIR}/cross_target_client_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_CROSS_TARGET_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/cross_target_client.scxml" -l cpp -o "${MESH_CROSS_TARGET_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_cross_target.yaml"
                    --write-deps "${MESH_CROSS_TARGET_OUTPUT_DIR}/cross_target_client_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/cross_target_client.scxml" "${SCE_CODEGEN}"
            DEPFILE "${MESH_CROSS_TARGET_OUTPUT_DIR}/cross_target_client_sm.h.d"
            COMMENT "Mesh cross-target (§14.6): Generating cross_target_client_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_CROSS_TARGET_OUTPUT_DIR}/cross_target_client_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/cross_target_client.scxml" -l cpp -o "${MESH_CROSS_TARGET_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_cross_target.yaml"
                    --transport-only
                    --write-deps "${MESH_CROSS_TARGET_OUTPUT_DIR}/cross_target_client_transport.h.d"
            DEPENDS "${MESH_TEST_DIR}/cross_target_client.scxml" "${MESH_TEST_DIR}/deploy_cross_target.yaml"
                    "${MESH_TEST_DIR}/cross_target_alpha.scxml" "${MESH_TEST_DIR}/cross_target_beta.scxml"
                    "${MESH_TEST_DIR}/vsomeip_cross_target.json"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_CROSS_TARGET_OUTPUT_DIR}/cross_target_client_sm.h"
            DEPFILE "${MESH_CROSS_TARGET_OUTPUT_DIR}/cross_target_client_transport.h.d"
            COMMENT "Mesh cross-target (§14.6): Generating cross_target_client_transport.h"
            VERBATIM
        )

        add_executable(mesh_someip_cross_target_reply_test
            mesh/test_mesh_someip_cross_target_reply.cpp
            "${MESH_CROSS_TARGET_OUTPUT_DIR}/cross_target_client_sm.h"
            "${MESH_CROSS_TARGET_OUTPUT_DIR}/cross_target_client_transport.h"
        )

        target_include_directories(mesh_someip_cross_target_reply_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_CROSS_TARGET_OUTPUT_DIR}
                ${CMAKE_CURRENT_SOURCE_DIR}/mesh
        )

        target_link_libraries(mesh_someip_cross_target_reply_test
            PRIVATE
                sce_base
                sce_mesh_common
                vsomeip3
        )

        target_compile_features(mesh_someip_cross_target_reply_test PRIVATE cxx_std_17)
        apply_sce_test_policies(mesh_someip_cross_target_reply_test)
        add_test(NAME mesh_someip_cross_target_reply
                 COMMAND mesh_someip_cross_target_reply_test)
        set_tests_properties(mesh_someip_cross_target_reply PROPERTIES
            TIMEOUT 60
            RUN_SERIAL TRUE
            ENVIRONMENT "VSOMEIP_CONFIGURATION=${CMAKE_CURRENT_SOURCE_DIR}/mesh/vsomeip_cross_target.json;LD_LIBRARY_PATH=${vsomeip3_DIR}:$ENV{LD_LIBRARY_PATH}")

        # ── §14.6 declared responder set: the positive half ──────────
        # Same SCXML as the cross-target fixture above, different
        # deploy.yaml (`reply_from:` declared), so the pair proves the
        # gate keys on the declared set and nothing else.
        set(MESH_XT_DECLARED_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/mesh_cross_target_declared_generated")

        add_custom_command(
            OUTPUT "${MESH_XT_DECLARED_OUTPUT_DIR}/cross_target_client_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_XT_DECLARED_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/cross_target_client.scxml" -l cpp -o "${MESH_XT_DECLARED_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_cross_target_declared.yaml"
                    --write-deps "${MESH_XT_DECLARED_OUTPUT_DIR}/cross_target_client_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/cross_target_client.scxml" "${SCE_CODEGEN}"
            DEPFILE "${MESH_XT_DECLARED_OUTPUT_DIR}/cross_target_client_sm.h.d"
            COMMENT "Mesh cross-target declared (§14.6): Generating cross_target_client_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_XT_DECLARED_OUTPUT_DIR}/cross_target_client_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/cross_target_client.scxml" -l cpp -o "${MESH_XT_DECLARED_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_cross_target_declared.yaml"
                    --transport-only
                    --write-deps "${MESH_XT_DECLARED_OUTPUT_DIR}/cross_target_client_transport.h.d"
            DEPENDS "${MESH_TEST_DIR}/cross_target_client.scxml" "${MESH_TEST_DIR}/deploy_cross_target_declared.yaml"
                    "${MESH_TEST_DIR}/cross_target_alpha.scxml" "${MESH_TEST_DIR}/cross_target_beta.scxml"
                    "${MESH_TEST_DIR}/vsomeip_cross_target.json"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_XT_DECLARED_OUTPUT_DIR}/cross_target_client_sm.h"
            DEPFILE "${MESH_XT_DECLARED_OUTPUT_DIR}/cross_target_client_transport.h.d"
            COMMENT "Mesh cross-target declared (§14.6): Generating cross_target_client_transport.h"
            VERBATIM
        )

        add_executable(mesh_someip_declared_responder_test
            mesh/test_mesh_someip_declared_responder.cpp
            "${MESH_XT_DECLARED_OUTPUT_DIR}/cross_target_client_sm.h"
            "${MESH_XT_DECLARED_OUTPUT_DIR}/cross_target_client_transport.h"
        )

        target_include_directories(mesh_someip_declared_responder_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_XT_DECLARED_OUTPUT_DIR}
                ${CMAKE_CURRENT_SOURCE_DIR}/mesh
        )

        target_link_libraries(mesh_someip_declared_responder_test
            PRIVATE
                sce_base
                sce_mesh_common
                vsomeip3
        )

        target_compile_features(mesh_someip_declared_responder_test PRIVATE cxx_std_17)
        apply_sce_test_policies(mesh_someip_declared_responder_test)
        add_test(NAME mesh_someip_declared_responder
                 COMMAND mesh_someip_declared_responder_test)
        set_tests_properties(mesh_someip_declared_responder PROPERTIES
            TIMEOUT 60
            RUN_SERIAL TRUE
            ENVIRONMENT "VSOMEIP_CONFIGURATION=${CMAKE_CURRENT_SOURCE_DIR}/mesh/vsomeip_cross_target.json;LD_LIBRARY_PATH=${vsomeip3_DIR}:$ENV{LD_LIBRARY_PATH}")

        # ── SCE Mesh §10.10 Gap 5 (i): SOME/IP late-boot OutboundBuffer ──
        # Re-runs sce-codegen on the same brake_someip_multi /
        # motor_someip_multi SCXML files but with a different deploy.yaml
        # (`deploy_someip_late_boot.yaml`) so the generated TransportRouter
        # picks up `outbound_buffer: max_pending_per_target: 8` on brake.
        # Outputs land in a separate directory so the same _transport.h
        # name does not collide with the `MESH_SOMEIP_MULTI_OUTPUT_DIR`
        # variant that the happy-path fixtures consume.
        set(MESH_SOMEIP_LATE_BOOT_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/mesh_someip_late_boot_generated")

        add_custom_command(
            OUTPUT "${MESH_SOMEIP_LATE_BOOT_OUTPUT_DIR}/brake_someip_multi_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SOMEIP_LATE_BOOT_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_someip_multi.scxml" -l cpp -o "${MESH_SOMEIP_LATE_BOOT_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_someip_late_boot.yaml"
                    --write-deps "${MESH_SOMEIP_LATE_BOOT_OUTPUT_DIR}/brake_someip_multi_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/brake_someip_multi.scxml" "${SCE_CODEGEN}"
            DEPFILE "${MESH_SOMEIP_LATE_BOOT_OUTPUT_DIR}/brake_someip_multi_sm.h.d"
            COMMENT "Mesh someip late-boot (§10.10): Generating brake_someip_multi_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_SOMEIP_LATE_BOOT_OUTPUT_DIR}/motor_someip_multi_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SOMEIP_LATE_BOOT_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_someip_multi.scxml" -l cpp -o "${MESH_SOMEIP_LATE_BOOT_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_someip_late_boot.yaml"
                    --write-deps "${MESH_SOMEIP_LATE_BOOT_OUTPUT_DIR}/motor_someip_multi_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/motor_someip_multi.scxml" "${SCE_CODEGEN}"
            DEPFILE "${MESH_SOMEIP_LATE_BOOT_OUTPUT_DIR}/motor_someip_multi_sm.h.d"
            COMMENT "Mesh someip late-boot (§10.10): Generating motor_someip_multi_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_SOMEIP_LATE_BOOT_OUTPUT_DIR}/brake_someip_multi_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_someip_multi.scxml" -l cpp -o "${MESH_SOMEIP_LATE_BOOT_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_someip_late_boot.yaml"
                    --transport-only
                    --write-deps "${MESH_SOMEIP_LATE_BOOT_OUTPUT_DIR}/brake_someip_multi_transport.h.d"
            DEPENDS "${MESH_TEST_DIR}/brake_someip_multi.scxml" "${MESH_TEST_DIR}/deploy_someip_late_boot.yaml"
                    "${MESH_TEST_DIR}/motor_someip_multi.scxml" "${MESH_TEST_DIR}/vsomeip_motor_multi.json"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_SOMEIP_LATE_BOOT_OUTPUT_DIR}/brake_someip_multi_sm.h"
            DEPFILE "${MESH_SOMEIP_LATE_BOOT_OUTPUT_DIR}/brake_someip_multi_transport.h.d"
            COMMENT "Mesh someip late-boot (§10.10): Generating brake_someip_multi_transport.h with outbound_buffer"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_SOMEIP_LATE_BOOT_OUTPUT_DIR}/motor_someip_multi_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_someip_multi.scxml" -l cpp -o "${MESH_SOMEIP_LATE_BOOT_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_someip_late_boot.yaml"
                    --transport-only
                    --write-deps "${MESH_SOMEIP_LATE_BOOT_OUTPUT_DIR}/motor_someip_multi_transport.h.d"
            DEPENDS "${MESH_TEST_DIR}/motor_someip_multi.scxml" "${MESH_TEST_DIR}/deploy_someip_late_boot.yaml"
                    "${MESH_TEST_DIR}/vsomeip_motor_multi.json"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_SOMEIP_LATE_BOOT_OUTPUT_DIR}/motor_someip_multi_sm.h"
            DEPFILE "${MESH_SOMEIP_LATE_BOOT_OUTPUT_DIR}/motor_someip_multi_transport.h.d"
            COMMENT "Mesh someip late-boot (§10.10): Generating motor_someip_multi_transport.h"
            VERBATIM
        )

        add_executable(mesh_someip_late_boot_test
            mesh/test_mesh_someip_late_boot.cpp
            "${MESH_SOMEIP_LATE_BOOT_OUTPUT_DIR}/brake_someip_multi_sm.h"
            "${MESH_SOMEIP_LATE_BOOT_OUTPUT_DIR}/motor_someip_multi_sm.h"
            "${MESH_SOMEIP_LATE_BOOT_OUTPUT_DIR}/brake_someip_multi_transport.h"
            "${MESH_SOMEIP_LATE_BOOT_OUTPUT_DIR}/motor_someip_multi_transport.h"
        )

        target_include_directories(mesh_someip_late_boot_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_SOMEIP_LATE_BOOT_OUTPUT_DIR}
                ${CMAKE_CURRENT_SOURCE_DIR}/mesh
        )

        target_link_libraries(mesh_someip_late_boot_test
            PRIVATE
                sce_base
                sce_mesh_common
                vsomeip3
        )

        target_compile_features(mesh_someip_late_boot_test PRIVATE cxx_std_17)
        apply_sce_test_policies(mesh_someip_late_boot_test)
        add_test(NAME mesh_someip_late_boot
                 COMMAND mesh_someip_late_boot_test)
        set_tests_properties(mesh_someip_late_boot PROPERTIES
            TIMEOUT 30
            ENVIRONMENT "VSOMEIP_CONFIGURATION=${CMAKE_CURRENT_SOURCE_DIR}/mesh/vsomeip_e2e_someip_late_boot.json;LD_LIBRARY_PATH=${vsomeip3_DIR}:$ENV{LD_LIBRARY_PATH}")

        # Session J: SOME/IP eventgroup notification (server-initiated push).
        # Motor raises field.notify.vehicle_speed spontaneously (no preceding
        # getter/setter) → offer_event + notify path.
        add_executable(mesh_someip_eventgroup_engine_driven_test
            mesh/test_mesh_someip_eventgroup_engine_driven.cpp
            "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/brake_someip_multi_sm.h"
            "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/motor_someip_multi_sm.h"
            "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/brake_someip_multi_transport.h"
            "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/motor_someip_multi_transport.h"
        )

        target_include_directories(mesh_someip_eventgroup_engine_driven_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_SOMEIP_MULTI_OUTPUT_DIR}
                ${CMAKE_CURRENT_SOURCE_DIR}/mesh
        )

        target_link_libraries(mesh_someip_eventgroup_engine_driven_test
            PRIVATE
                sce_base
                sce_mesh_common
                vsomeip3
        )

        target_compile_features(mesh_someip_eventgroup_engine_driven_test PRIVATE cxx_std_17)
        apply_sce_test_policies(mesh_someip_eventgroup_engine_driven_test)
        add_test(NAME mesh_someip_eventgroup_engine_driven
                 COMMAND mesh_someip_eventgroup_engine_driven_test)
        set_tests_properties(mesh_someip_eventgroup_engine_driven PROPERTIES
            TIMEOUT 30
            ENVIRONMENT "VSOMEIP_CONFIGURATION=${CMAKE_CURRENT_SOURCE_DIR}/mesh/vsomeip_e2e_eventgroup.json;LD_LIBRARY_PATH=${vsomeip3_DIR}:$ENV{LD_LIBRARY_PATH}")

        # ── Gap S6: SOME/IP EventUnsubscribe E2E ──────────────────────
        # Closes next_session_someip_s6_event_unsubscribe.md: verifies
        # that app.unsubscribe() on PatternKind::EventUnsubscribe
        # retracts the subscription end-to-end. Dedicated fixture pair
        # so Session C's multi-pattern assertions stay undisturbed.
        set(MESH_SOMEIP_UNSUB_OUTPUT_DIR
            "${CMAKE_CURRENT_BINARY_DIR}/mesh_someip_unsub_generated")

        add_custom_command(
            OUTPUT "${MESH_SOMEIP_UNSUB_OUTPUT_DIR}/brake_someip_unsub_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SOMEIP_UNSUB_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_someip_unsub.scxml"
                    -l cpp -o "${MESH_SOMEIP_UNSUB_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_someip_unsub.yaml"
                    --write-deps "${MESH_SOMEIP_UNSUB_OUTPUT_DIR}/brake_someip_unsub_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/brake_someip_unsub.scxml" "${SCE_CODEGEN}"
            DEPFILE "${MESH_SOMEIP_UNSUB_OUTPUT_DIR}/brake_someip_unsub_sm.h.d"
            COMMENT "Mesh someip unsub: Generating brake_someip_unsub_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_SOMEIP_UNSUB_OUTPUT_DIR}/motor_someip_unsub_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SOMEIP_UNSUB_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_someip_unsub.scxml"
                    -l cpp -o "${MESH_SOMEIP_UNSUB_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_someip_unsub.yaml"
                    --write-deps "${MESH_SOMEIP_UNSUB_OUTPUT_DIR}/motor_someip_unsub_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/motor_someip_unsub.scxml" "${SCE_CODEGEN}"
            DEPFILE "${MESH_SOMEIP_UNSUB_OUTPUT_DIR}/motor_someip_unsub_sm.h.d"
            COMMENT "Mesh someip unsub: Generating motor_someip_unsub_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_SOMEIP_UNSUB_OUTPUT_DIR}/brake_someip_unsub_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_someip_unsub.scxml"
                    -l cpp -o "${MESH_SOMEIP_UNSUB_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_someip_unsub.yaml"
                    --transport-only
                    --write-deps "${MESH_SOMEIP_UNSUB_OUTPUT_DIR}/brake_someip_unsub_transport.h.d"
            DEPENDS "${MESH_TEST_DIR}/brake_someip_unsub.scxml"
                    "${MESH_TEST_DIR}/motor_someip_unsub.scxml"
                    "${MESH_TEST_DIR}/deploy_someip_unsub.yaml"
                    "${MESH_TEST_DIR}/vsomeip_motor_unsub.json"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_SOMEIP_UNSUB_OUTPUT_DIR}/brake_someip_unsub_sm.h"
            DEPFILE "${MESH_SOMEIP_UNSUB_OUTPUT_DIR}/brake_someip_unsub_transport.h.d"
            COMMENT "Mesh someip unsub: Generating brake_someip_unsub_transport.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_SOMEIP_UNSUB_OUTPUT_DIR}/motor_someip_unsub_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_someip_unsub.scxml"
                    -l cpp -o "${MESH_SOMEIP_UNSUB_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_someip_unsub.yaml"
                    --transport-only
                    --write-deps "${MESH_SOMEIP_UNSUB_OUTPUT_DIR}/motor_someip_unsub_transport.h.d"
            DEPENDS "${MESH_TEST_DIR}/motor_someip_unsub.scxml"
                    "${MESH_TEST_DIR}/deploy_someip_unsub.yaml"
                    "${MESH_TEST_DIR}/vsomeip_motor_unsub.json"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_SOMEIP_UNSUB_OUTPUT_DIR}/motor_someip_unsub_sm.h"
            DEPFILE "${MESH_SOMEIP_UNSUB_OUTPUT_DIR}/motor_someip_unsub_transport.h.d"
            COMMENT "Mesh someip unsub: Generating motor_someip_unsub_transport.h (server)"
            VERBATIM
        )

        add_executable(mesh_someip_unsubscribe_test
            mesh/test_mesh_someip_unsubscribe.cpp
            "${MESH_SOMEIP_UNSUB_OUTPUT_DIR}/brake_someip_unsub_sm.h"
            "${MESH_SOMEIP_UNSUB_OUTPUT_DIR}/motor_someip_unsub_sm.h"
            "${MESH_SOMEIP_UNSUB_OUTPUT_DIR}/brake_someip_unsub_transport.h"
            "${MESH_SOMEIP_UNSUB_OUTPUT_DIR}/motor_someip_unsub_transport.h"
        )

        target_include_directories(mesh_someip_unsubscribe_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_SOMEIP_UNSUB_OUTPUT_DIR}
                ${CMAKE_CURRENT_SOURCE_DIR}/mesh
        )

        target_link_libraries(mesh_someip_unsubscribe_test
            PRIVATE
                sce_base
                sce_mesh_common
                vsomeip3
        )

        target_compile_features(mesh_someip_unsubscribe_test PRIVATE cxx_std_17)
        apply_sce_test_policies(mesh_someip_unsubscribe_test)
        add_test(NAME mesh_someip_unsubscribe
                 COMMAND mesh_someip_unsubscribe_test)
        set_tests_properties(mesh_someip_unsubscribe PROPERTIES
            TIMEOUT 30
            ENVIRONMENT "VSOMEIP_CONFIGURATION=${CMAKE_CURRENT_SOURCE_DIR}/mesh/vsomeip_e2e_unsub.json;LD_LIBRARY_PATH=${vsomeip3_DIR}:$ENV{LD_LIBRARY_PATH}")
    else()
        message(STATUS "vsomeip3 not found — skipping mesh_someip_compile_test")
    endif()

    # ── zenoh_transport compile verification ──────────────────
    # Validates that generated Zenoh transport code compiles against
    # real zenoh-cpp headers. Gated on find_package(zenohcxx).
    #
    # zenoh-cpp's CMake config imports `zenohc::lib` to define the
    # `zenohcxx::zenohc` target — so zenohc must be discovered first.
    find_package(zenohc QUIET)
    find_package(zenohcxx QUIET)
    if(zenohc_FOUND AND zenohcxx_FOUND)
        set(MESH_ZENOH_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/mesh_zenoh_generated")

        add_custom_command(
            OUTPUT "${MESH_ZENOH_OUTPUT_DIR}/brake_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_ZENOH_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake.scxml" -l cpp -o "${MESH_ZENOH_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_zenoh.yaml"
                    --write-deps "${MESH_ZENOH_OUTPUT_DIR}/brake_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/brake.scxml" "${SCE_CODEGEN}"
            DEPFILE "${MESH_ZENOH_OUTPUT_DIR}/brake_sm.h.d"
            COMMENT "Mesh zenoh: Generating brake_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_ZENOH_OUTPUT_DIR}/motor_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_ZENOH_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor.scxml" -l cpp -o "${MESH_ZENOH_OUTPUT_DIR}"
                    --write-deps "${MESH_ZENOH_OUTPUT_DIR}/motor_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/motor.scxml" "${SCE_CODEGEN}"
            DEPFILE "${MESH_ZENOH_OUTPUT_DIR}/motor_sm.h.d"
            COMMENT "Mesh zenoh: Generating motor_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_ZENOH_OUTPUT_DIR}/brake_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake.scxml" -l cpp -o "${MESH_ZENOH_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_zenoh.yaml"
                    --transport-only
                    --write-deps "${MESH_ZENOH_OUTPUT_DIR}/brake_transport.h.d"
            DEPENDS "${MESH_TEST_DIR}/brake.scxml" "${MESH_TEST_DIR}/deploy_zenoh.yaml"
                    "${MESH_TEST_DIR}/motor.scxml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_ZENOH_OUTPUT_DIR}/brake_sm.h"
            DEPFILE "${MESH_ZENOH_OUTPUT_DIR}/brake_transport.h.d"
            COMMENT "Mesh zenoh: Generating brake_transport.h (zenoh transport)"
            VERBATIM
        )

        add_executable(mesh_zenoh_compile_test
            mesh/test_mesh_zenoh.cpp
            "${MESH_ZENOH_OUTPUT_DIR}/brake_sm.h"
            "${MESH_ZENOH_OUTPUT_DIR}/motor_sm.h"
            "${MESH_ZENOH_OUTPUT_DIR}/brake_transport.h"
        )

        target_include_directories(mesh_zenoh_compile_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_ZENOH_OUTPUT_DIR}
        )

        target_link_libraries(mesh_zenoh_compile_test
            PRIVATE
                sce_base
                sce_mesh_common
                zenohcxx::zenohc
        )

        target_compile_features(mesh_zenoh_compile_test PRIVATE cxx_std_17)
        sce_pin_cxx17(mesh_zenoh_compile_test)

        apply_sce_test_policies(mesh_zenoh_compile_test)

        add_test(NAME mesh_zenoh_compile_verification COMMAND mesh_zenoh_compile_test)

        # ── Session G: base zenoh single-pattern runtime E2E ──────────
        # Counterpart to the multi-pattern tests — exercises the simple
        # brake.scxml + deploy_zenoh.yaml fixture end-to-end (FireForget
        # only). Shares RESOURCE_LOCK with the multi-pattern runtime tests
        # because both fixtures listen on tcp/127.0.0.1:17447.
        add_executable(mesh_zenoh_runtime_test
            mesh/test_mesh_zenoh_runtime.cpp
            "${MESH_ZENOH_OUTPUT_DIR}/brake_sm.h"
            "${MESH_ZENOH_OUTPUT_DIR}/brake_transport.h"
        )

        target_include_directories(mesh_zenoh_runtime_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_ZENOH_OUTPUT_DIR}
                ${CMAKE_CURRENT_SOURCE_DIR}/mesh
        )

        target_link_libraries(mesh_zenoh_runtime_test
            PRIVATE
                sce_base
                sce_mesh_common
                zenohcxx::zenohc
        )

        target_compile_features(mesh_zenoh_runtime_test PRIVATE cxx_std_17)
        apply_sce_test_policies(mesh_zenoh_runtime_test)
        add_test(NAME mesh_zenoh_runtime COMMAND mesh_zenoh_runtime_test)
        set_tests_properties(mesh_zenoh_runtime PROPERTIES
            TIMEOUT 30
            RESOURCE_LOCK zenoh_multi_port_17447
        )

        # ── §9.6 Session 5: Zenoh scxml-invoke roundtrip ─────────────
        # Single-process proof that the §9.6 wire-14/20 lifecycle
        # travels over Zenoh. Parent + worker each open their own
        # zenoh::Session sharing it with no `<send>` zenoh target
        # (the codegen inserts "zenoh" into transport_types when a
        # §9.6 zenoh peer exists, which fires the existing zenoh.hxx
        # include block + zenoh::Session::open init() block). The
        # fixture trivial-final child reaches `<final>` during
        # initialize(), so the worker emits wire-15 + wire-18 in
        # the same pump tick and the parent reaches State::Pass via
        # done.invoke.remote_inv. The shared port 17448 is
        # serialised through ctest RESOURCE_LOCK
        # zenoh_invoke_port_17448 (distinct from
        # zenoh_multi_port_17447 used by the `<send>` zenoh tests).
        set(MESH_ZENOH_SCXML_INVOKE_OUTPUT_DIR
            "${CMAKE_CURRENT_BINARY_DIR}/mesh_zenoh_scxml_invoke_generated")

        add_custom_command(
            OUTPUT "${MESH_ZENOH_SCXML_INVOKE_OUTPUT_DIR}/scxml_invoke_zenoh_parent_sm.h"
                   "${MESH_ZENOH_SCXML_INVOKE_OUTPUT_DIR}/scxml_invoke_zenoh_parent_transport.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_ZENOH_SCXML_INVOKE_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/scxml_invoke_zenoh_parent.scxml"
                    -l cpp -o "${MESH_ZENOH_SCXML_INVOKE_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_scxml_invoke_zenoh.yaml"
                    --write-deps "${MESH_ZENOH_SCXML_INVOKE_OUTPUT_DIR}/scxml_invoke_zenoh_parent_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/scxml_invoke_zenoh_parent.scxml"
                    "${MESH_TEST_DIR}/scxml_invoke_zenoh_worker.scxml"
                    "${MESH_TEST_DIR}/deploy_scxml_invoke_zenoh.yaml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
            DEPFILE "${MESH_ZENOH_SCXML_INVOKE_OUTPUT_DIR}/scxml_invoke_zenoh_parent_sm.h.d"
            COMMENT "Mesh §9.6 Session 5: Generating scxml_invoke_zenoh_parent_{sm,transport}.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_ZENOH_SCXML_INVOKE_OUTPUT_DIR}/scxml_invoke_zenoh_worker_sm.h"
                   "${MESH_ZENOH_SCXML_INVOKE_OUTPUT_DIR}/scxml_invoke_zenoh_worker_transport.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_ZENOH_SCXML_INVOKE_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/scxml_invoke_zenoh_worker.scxml"
                    -l cpp -o "${MESH_ZENOH_SCXML_INVOKE_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_scxml_invoke_zenoh.yaml"
                    --write-deps "${MESH_ZENOH_SCXML_INVOKE_OUTPUT_DIR}/scxml_invoke_zenoh_worker_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/scxml_invoke_zenoh_parent.scxml"
                    "${MESH_TEST_DIR}/scxml_invoke_zenoh_worker.scxml"
                    "${MESH_TEST_DIR}/deploy_scxml_invoke_zenoh.yaml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
            DEPFILE "${MESH_ZENOH_SCXML_INVOKE_OUTPUT_DIR}/scxml_invoke_zenoh_worker_sm.h.d"
            COMMENT "Mesh §9.6 Session 5: Generating scxml_invoke_zenoh_worker_{sm,transport}.h"
            VERBATIM
        )

        add_executable(mesh_zenoh_scxml_invoke_roundtrip_test
            mesh/test_mesh_zenoh_scxml_invoke_roundtrip.cpp
            "${MESH_ZENOH_SCXML_INVOKE_OUTPUT_DIR}/scxml_invoke_zenoh_parent_sm.h"
            "${MESH_ZENOH_SCXML_INVOKE_OUTPUT_DIR}/scxml_invoke_zenoh_parent_transport.h"
            "${MESH_ZENOH_SCXML_INVOKE_OUTPUT_DIR}/scxml_invoke_zenoh_worker_sm.h"
            "${MESH_ZENOH_SCXML_INVOKE_OUTPUT_DIR}/scxml_invoke_zenoh_worker_transport.h"
        )

        target_include_directories(mesh_zenoh_scxml_invoke_roundtrip_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_ZENOH_SCXML_INVOKE_OUTPUT_DIR}
                ${CMAKE_CURRENT_SOURCE_DIR}/mesh
        )

        target_link_libraries(mesh_zenoh_scxml_invoke_roundtrip_test
            PRIVATE
                sce_runtime
                sce_mesh_common
                zenohcxx::zenohc
        )

        target_compile_features(mesh_zenoh_scxml_invoke_roundtrip_test PRIVATE cxx_std_17)
        apply_sce_test_policies(mesh_zenoh_scxml_invoke_roundtrip_test)
        add_test(NAME mesh_zenoh_scxml_invoke_roundtrip
                 COMMAND mesh_zenoh_scxml_invoke_roundtrip_test)
        set_tests_properties(mesh_zenoh_scxml_invoke_roundtrip PROPERTIES
            TIMEOUT 30
            # Port 17448 is shared with mesh_zenoh_liveliness_verification;
            # ctest only serialises tests that name the same RESOURCE_LOCK,
            # so both fixtures use the port-keyed identity to avoid the
            # under -j flake observed pre-2026-05-18 (different LOCK names
            # let ctest run them in parallel and bind() collided). Mirrors
            # the SOMEIP socket-namespace fix recorded in
            # `[[feedback-someip-netns-lock-sharing]]`.
            RESOURCE_LOCK zenoh_port_17448
        )

        # ── Session 5b: two-host Zenoh scxml-invoke crossdev ─────────
        # Parent and worker run in distinct Linux netns wired by veth,
        # each opening its own Zenoh peer session. Parent ECU listens
        # at tcp/172.16.10.1:17449; worker ECU connects to that
        # address. The §9.6 wire-14/15/18 lifecycle is exercised
        # across a real network stack — what the 1-process Session 5
        # roundtrip cannot cover because loopback collapses the
        # peer-mesh routing fabric Zenoh's TCP transport relies on.
        #
        # Distinct port (17449) from the 1-process fixture (17448) +
        # dedicated RESOURCE_LOCK so parallel ctest runs of the two
        # zenoh fixtures don't serialise on the same listen socket.
        # Gated on SCE_ENABLE_NETNS_TESTS (same opt-in as the someip
        # crossdev) — ctest skips with code 77 in non-root checkouts.
        if(SCE_ENABLE_NETNS_TESTS)
            include(${CMAKE_CURRENT_SOURCE_DIR}/cmake/two_host_test.cmake)

            set(MESH_ZENOH_CROSSDEV_OUTPUT_DIR
                "${CMAKE_CURRENT_BINARY_DIR}/mesh_zenoh_scxml_invoke_crossdev_generated")

            add_custom_command(
                OUTPUT "${MESH_ZENOH_CROSSDEV_OUTPUT_DIR}/scxml_invoke_zenoh_crossdev_parent_sm.h"
                       "${MESH_ZENOH_CROSSDEV_OUTPUT_DIR}/scxml_invoke_zenoh_crossdev_parent_transport.h"
                COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_ZENOH_CROSSDEV_OUTPUT_DIR}"
                COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/scxml_invoke_zenoh_crossdev_parent.scxml"
                        -l cpp -o "${MESH_ZENOH_CROSSDEV_OUTPUT_DIR}"
                        --deploy "${MESH_TEST_DIR}/deploy_scxml_invoke_zenoh_crossdev.yaml"
                        --write-deps "${MESH_ZENOH_CROSSDEV_OUTPUT_DIR}/scxml_invoke_zenoh_crossdev_parent_sm.h.d"
                DEPENDS "${MESH_TEST_DIR}/scxml_invoke_zenoh_crossdev_parent.scxml"
                        "${MESH_TEST_DIR}/scxml_invoke_zenoh_crossdev_worker.scxml"
                        "${MESH_TEST_DIR}/deploy_scxml_invoke_zenoh_crossdev.yaml"
                        "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                DEPFILE "${MESH_ZENOH_CROSSDEV_OUTPUT_DIR}/scxml_invoke_zenoh_crossdev_parent_sm.h.d"
                COMMENT "Mesh §9.6 Session 5b: Generating scxml_invoke_zenoh_crossdev_parent_{sm,transport}.h"
                VERBATIM
            )

            add_custom_command(
                OUTPUT "${MESH_ZENOH_CROSSDEV_OUTPUT_DIR}/scxml_invoke_zenoh_crossdev_worker_sm.h"
                       "${MESH_ZENOH_CROSSDEV_OUTPUT_DIR}/scxml_invoke_zenoh_crossdev_worker_transport.h"
                COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_ZENOH_CROSSDEV_OUTPUT_DIR}"
                COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/scxml_invoke_zenoh_crossdev_worker.scxml"
                        -l cpp -o "${MESH_ZENOH_CROSSDEV_OUTPUT_DIR}"
                        --deploy "${MESH_TEST_DIR}/deploy_scxml_invoke_zenoh_crossdev.yaml"
                        --write-deps "${MESH_ZENOH_CROSSDEV_OUTPUT_DIR}/scxml_invoke_zenoh_crossdev_worker_sm.h.d"
                DEPENDS "${MESH_TEST_DIR}/scxml_invoke_zenoh_crossdev_parent.scxml"
                        "${MESH_TEST_DIR}/scxml_invoke_zenoh_crossdev_worker.scxml"
                        "${MESH_TEST_DIR}/deploy_scxml_invoke_zenoh_crossdev.yaml"
                        "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                DEPFILE "${MESH_ZENOH_CROSSDEV_OUTPUT_DIR}/scxml_invoke_zenoh_crossdev_worker_sm.h.d"
                COMMENT "Mesh §9.6 Session 5b: Generating scxml_invoke_zenoh_crossdev_worker_{sm,transport}.h"
                VERBATIM
            )

            add_executable(mesh_zenoh_scxml_invoke_crossdev_parent
                mesh/test_mesh_zenoh_scxml_invoke_crossdev_parent.cpp
                "${MESH_ZENOH_CROSSDEV_OUTPUT_DIR}/scxml_invoke_zenoh_crossdev_parent_sm.h"
                "${MESH_ZENOH_CROSSDEV_OUTPUT_DIR}/scxml_invoke_zenoh_crossdev_parent_transport.h"
            )
            target_include_directories(mesh_zenoh_scxml_invoke_crossdev_parent
                PRIVATE
                    ${CMAKE_SOURCE_DIR}/sce/include
                    ${MESH_ZENOH_CROSSDEV_OUTPUT_DIR}
                    ${CMAKE_CURRENT_SOURCE_DIR}/mesh
            )
            target_link_libraries(mesh_zenoh_scxml_invoke_crossdev_parent
                PRIVATE sce_runtime sce_mesh_common zenohcxx::zenohc)
            target_compile_features(mesh_zenoh_scxml_invoke_crossdev_parent PRIVATE cxx_std_17)
            apply_sce_test_policies(mesh_zenoh_scxml_invoke_crossdev_parent)

            add_executable(mesh_zenoh_scxml_invoke_crossdev_worker
                mesh/test_mesh_zenoh_scxml_invoke_crossdev_worker.cpp
                "${MESH_ZENOH_CROSSDEV_OUTPUT_DIR}/scxml_invoke_zenoh_crossdev_worker_sm.h"
                "${MESH_ZENOH_CROSSDEV_OUTPUT_DIR}/scxml_invoke_zenoh_crossdev_worker_transport.h"
            )
            target_include_directories(mesh_zenoh_scxml_invoke_crossdev_worker
                PRIVATE
                    ${CMAKE_SOURCE_DIR}/sce/include
                    ${MESH_ZENOH_CROSSDEV_OUTPUT_DIR}
                    ${CMAKE_CURRENT_SOURCE_DIR}/mesh
            )
            target_link_libraries(mesh_zenoh_scxml_invoke_crossdev_worker
                PRIVATE sce_runtime sce_mesh_common zenohcxx::zenohc)
            target_compile_features(mesh_zenoh_scxml_invoke_crossdev_worker PRIVATE cxx_std_17)
            apply_sce_test_policies(mesh_zenoh_scxml_invoke_crossdev_worker)

            sce_register_two_host_mesh_test(
                NAME    mesh_zenoh_scxml_invoke_crossdev
                WORKER  mesh_zenoh_scxml_invoke_crossdev_worker
                PARENT  mesh_zenoh_scxml_invoke_crossdev_parent
                TIMEOUT 60)
            set_tests_properties(mesh_zenoh_scxml_invoke_crossdev PROPERTIES
                RESOURCE_LOCK zenoh_port_17449)  # shared with mesh_zenoh_on_drop

            # ── Session 5c: two-host Zenoh donedata crossdev ─────────
            # Sister to mesh_session_f_crossdev_donedata (custom_tcp
            # 2-process) and mesh_zenoh_scxml_invoke_crossdev (5b
            # lifecycle). Reuses parent_session_f_donedata.scxml +
            # worker_session_f_donedata_{param,content,nested}.scxml
            # verbatim — the ONLY axis that changes is the deploy.yaml
            # rebinding peers onto Zenoh, which is the textbook §9.6
            # transport-axis abstraction proof. Reaching State::Pass
            # implies all three donedata shapes (param object, primitive
            # content, nested object/array/integer) survived a real
            # netns + veth + Zenoh TCP round-trip with semantically
            # equivalent decoded `_event.data` to the shm baseline.
            #
            # Distinct port (17450) + dedicated RESOURCE_LOCK from the
            # 5b lifecycle (17449) so parallel ctest runs don't race.
            set(MESH_ZENOH_CROSSDEV_DONEDATA_OUTPUT_DIR
                "${CMAKE_CURRENT_BINARY_DIR}/mesh_zenoh_session_f_crossdev_donedata_generated")

            set(MESH_ZENOH_CROSSDEV_DONEDATA_SCXMLS
                parent_session_f_donedata
                worker_session_f_donedata_param
                worker_session_f_donedata_content
                worker_session_f_donedata_nested)

            set(MESH_ZENOH_CROSSDEV_DONEDATA_SCXML_DEPS
                "${MESH_TEST_DIR}/parent_session_f_donedata.scxml"
                "${MESH_TEST_DIR}/worker_session_f_donedata_param.scxml"
                "${MESH_TEST_DIR}/worker_session_f_donedata_content.scxml"
                "${MESH_TEST_DIR}/worker_session_f_donedata_nested.scxml")

            foreach(machine ${MESH_ZENOH_CROSSDEV_DONEDATA_SCXMLS})
                add_custom_command(
                    OUTPUT "${MESH_ZENOH_CROSSDEV_DONEDATA_OUTPUT_DIR}/${machine}_sm.h"
                            "${MESH_ZENOH_CROSSDEV_DONEDATA_OUTPUT_DIR}/${machine}_transport.h"
                    COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_ZENOH_CROSSDEV_DONEDATA_OUTPUT_DIR}"
                    COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/${machine}.scxml"
                            -l cpp -o "${MESH_ZENOH_CROSSDEV_DONEDATA_OUTPUT_DIR}"
                            --deploy "${MESH_TEST_DIR}/deploy_session_f_donedata_zenoh_crossdev.yaml"
                            --write-deps "${MESH_ZENOH_CROSSDEV_DONEDATA_OUTPUT_DIR}/${machine}_sm.h.d"
                    DEPENDS ${MESH_ZENOH_CROSSDEV_DONEDATA_SCXML_DEPS}
                            "${MESH_TEST_DIR}/deploy_session_f_donedata_zenoh_crossdev.yaml"
                            "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    DEPFILE "${MESH_ZENOH_CROSSDEV_DONEDATA_OUTPUT_DIR}/${machine}_sm.h.d"
                    COMMENT "Mesh §9.6 Session 5c Zenoh donedata: Generating ${machine}_{sm,transport}.h"
                    VERBATIM
                )
            endforeach()

            add_executable(mesh_zenoh_session_f_crossdev_donedata_worker
                mesh/test_mesh_zenoh_session_f_crossdev_donedata_worker.cpp
                "${MESH_ZENOH_CROSSDEV_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_param_sm.h"
                "${MESH_ZENOH_CROSSDEV_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_param_transport.h"
                "${MESH_ZENOH_CROSSDEV_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_content_sm.h"
                "${MESH_ZENOH_CROSSDEV_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_content_transport.h"
                "${MESH_ZENOH_CROSSDEV_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_nested_sm.h"
                "${MESH_ZENOH_CROSSDEV_DONEDATA_OUTPUT_DIR}/worker_session_f_donedata_nested_transport.h")
            target_include_directories(mesh_zenoh_session_f_crossdev_donedata_worker
                PRIVATE ${CMAKE_SOURCE_DIR}/sce/include
                        ${MESH_ZENOH_CROSSDEV_DONEDATA_OUTPUT_DIR})
            target_link_libraries(mesh_zenoh_session_f_crossdev_donedata_worker
                PRIVATE sce_runtime sce_mesh_common zenohcxx::zenohc)
            target_compile_features(mesh_zenoh_session_f_crossdev_donedata_worker PRIVATE cxx_std_17)
            apply_sce_test_policies(mesh_zenoh_session_f_crossdev_donedata_worker)

            add_executable(mesh_zenoh_session_f_crossdev_donedata_parent
                mesh/test_mesh_zenoh_session_f_crossdev_donedata_parent.cpp
                "${MESH_ZENOH_CROSSDEV_DONEDATA_OUTPUT_DIR}/parent_session_f_donedata_sm.h"
                "${MESH_ZENOH_CROSSDEV_DONEDATA_OUTPUT_DIR}/parent_session_f_donedata_transport.h")
            target_include_directories(mesh_zenoh_session_f_crossdev_donedata_parent
                PRIVATE ${CMAKE_SOURCE_DIR}/sce/include
                        ${MESH_ZENOH_CROSSDEV_DONEDATA_OUTPUT_DIR})
            target_link_libraries(mesh_zenoh_session_f_crossdev_donedata_parent
                PRIVATE sce_runtime sce_mesh_common zenohcxx::zenohc)
            target_compile_features(mesh_zenoh_session_f_crossdev_donedata_parent PRIVATE cxx_std_17)
            apply_sce_test_policies(mesh_zenoh_session_f_crossdev_donedata_parent)

            sce_register_two_host_mesh_test(
                NAME    mesh_zenoh_session_f_crossdev_donedata
                WORKER  mesh_zenoh_session_f_crossdev_donedata_worker
                PARENT  mesh_zenoh_session_f_crossdev_donedata_parent
                TIMEOUT 60)
            set_tests_properties(mesh_zenoh_session_f_crossdev_donedata PROPERTIES
                RESOURCE_LOCK zenoh_invoke_crossdev_donedata_port_17450)
        endif()

        # ── Base zenoh EventSubscribe runtime E2E ─────────────────────
        # Closes the gap flagged by next_session_cleanup_small.md Item 2:
        # the base zenoh codegen path had no EventSubscribe runtime
        # coverage (mesh_zenoh_runtime only exercised FireForget). New
        # fixture pair (brake_zenoh_base_subscribe.scxml +
        # motor_zenoh_base_subscribe.scxml) keeps the shared brake.scxml
        # untouched. Shares RESOURCE_LOCK zenoh_multi_port_17447 with
        # the other zenoh runtime tests.
        set(MESH_ZENOH_BASE_SUB_OUTPUT_DIR
            "${CMAKE_CURRENT_BINARY_DIR}/mesh_zenoh_base_subscribe_generated")

        add_custom_command(
            OUTPUT "${MESH_ZENOH_BASE_SUB_OUTPUT_DIR}/brake_zenoh_base_subscribe_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_ZENOH_BASE_SUB_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_zenoh_base_subscribe.scxml"
                    -l cpp -o "${MESH_ZENOH_BASE_SUB_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_zenoh_base_subscribe.yaml"
                    --write-deps "${MESH_ZENOH_BASE_SUB_OUTPUT_DIR}/brake_zenoh_base_subscribe_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/brake_zenoh_base_subscribe.scxml" "${SCE_CODEGEN}"
            DEPFILE "${MESH_ZENOH_BASE_SUB_OUTPUT_DIR}/brake_zenoh_base_subscribe_sm.h.d"
            COMMENT "Mesh zenoh base subscribe: Generating brake_zenoh_base_subscribe_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_ZENOH_BASE_SUB_OUTPUT_DIR}/brake_zenoh_base_subscribe_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_zenoh_base_subscribe.scxml"
                    -l cpp -o "${MESH_ZENOH_BASE_SUB_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_zenoh_base_subscribe.yaml"
                    --transport-only
                    --write-deps "${MESH_ZENOH_BASE_SUB_OUTPUT_DIR}/brake_zenoh_base_subscribe_transport.h.d"
            DEPENDS "${MESH_TEST_DIR}/brake_zenoh_base_subscribe.scxml"
                    "${MESH_TEST_DIR}/motor_zenoh_base_subscribe.scxml"
                    "${MESH_TEST_DIR}/deploy_zenoh_base_subscribe.yaml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_ZENOH_BASE_SUB_OUTPUT_DIR}/brake_zenoh_base_subscribe_sm.h"
            DEPFILE "${MESH_ZENOH_BASE_SUB_OUTPUT_DIR}/brake_zenoh_base_subscribe_transport.h.d"
            COMMENT "Mesh zenoh base subscribe: Generating brake_zenoh_base_subscribe_transport.h"
            VERBATIM
        )

        add_executable(mesh_zenoh_base_subscribe_test
            mesh/test_mesh_zenoh_base_subscribe.cpp
            "${MESH_ZENOH_BASE_SUB_OUTPUT_DIR}/brake_zenoh_base_subscribe_sm.h"
            "${MESH_ZENOH_BASE_SUB_OUTPUT_DIR}/brake_zenoh_base_subscribe_transport.h"
        )

        target_include_directories(mesh_zenoh_base_subscribe_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_ZENOH_BASE_SUB_OUTPUT_DIR}
                ${CMAKE_CURRENT_SOURCE_DIR}/mesh
        )

        target_link_libraries(mesh_zenoh_base_subscribe_test
            PRIVATE
                sce_base
                sce_mesh_common
                zenohcxx::zenohc
        )

        target_compile_features(mesh_zenoh_base_subscribe_test PRIVATE cxx_std_17)
        apply_sce_test_policies(mesh_zenoh_base_subscribe_test)
        add_test(NAME mesh_zenoh_base_subscribe_verification
                 COMMAND mesh_zenoh_base_subscribe_test)
        set_tests_properties(mesh_zenoh_base_subscribe_verification PROPERTIES
            TIMEOUT 30
            RESOURCE_LOCK zenoh_multi_port_17447
        )

        # ── Z5a: Zenoh machine-lifetime subscription runtime E2E (§13) ─
        # Pins the Z5a fix from mesh_zenoh_gaps_roadmap.md: deploy.yaml
        # `machines.<name>.subscriptions:` on a subscriptions-only
        # machine now reaches the wire through topology-synthesised
        # implicit targets. Before the synthesis landed, the generated
        # `route_send` had no arm for the subscription source (targets
        # came from SCXML <send> only), so the init()-time subscribe
        # envelope dropped silently. This fixture fails closed if
        # Z5a regresses — the subscriber is never declared and the
        # five published envelopes are all lost.
        #
        # Port 17453 is distinct from the 17447–17452 ports used by
        # the other zenoh runtime tests; isolating the RESOURCE_LOCK
        # keeps subscription state from bleeding into / from them.
        set(MESH_ZENOH_MACHINE_LIFETIME_OUTPUT_DIR
            "${CMAKE_CURRENT_BINARY_DIR}/mesh_zenoh_machine_lifetime_subscribe_generated")

        add_custom_command(
            OUTPUT "${MESH_ZENOH_MACHINE_LIFETIME_OUTPUT_DIR}/brake_zenoh_machine_lifetime_subscribe_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_ZENOH_MACHINE_LIFETIME_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_zenoh_machine_lifetime_subscribe.scxml"
                    -l cpp -o "${MESH_ZENOH_MACHINE_LIFETIME_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_zenoh_machine_lifetime_subscribe.yaml"
                    --write-deps "${MESH_ZENOH_MACHINE_LIFETIME_OUTPUT_DIR}/brake_zenoh_machine_lifetime_subscribe_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/brake_zenoh_machine_lifetime_subscribe.scxml" "${SCE_CODEGEN}"
            DEPFILE "${MESH_ZENOH_MACHINE_LIFETIME_OUTPUT_DIR}/brake_zenoh_machine_lifetime_subscribe_sm.h.d"
            COMMENT "Mesh zenoh machine-lifetime subscribe: Generating brake_zenoh_machine_lifetime_subscribe_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_ZENOH_MACHINE_LIFETIME_OUTPUT_DIR}/brake_zenoh_machine_lifetime_subscribe_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_zenoh_machine_lifetime_subscribe.scxml"
                    -l cpp -o "${MESH_ZENOH_MACHINE_LIFETIME_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_zenoh_machine_lifetime_subscribe.yaml"
                    --transport-only
                    --write-deps "${MESH_ZENOH_MACHINE_LIFETIME_OUTPUT_DIR}/brake_zenoh_machine_lifetime_subscribe_transport.h.d"
            DEPENDS "${MESH_TEST_DIR}/brake_zenoh_machine_lifetime_subscribe.scxml"
                    "${MESH_TEST_DIR}/motor_zenoh_base_subscribe.scxml"
                    "${MESH_TEST_DIR}/deploy_zenoh_machine_lifetime_subscribe.yaml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_ZENOH_MACHINE_LIFETIME_OUTPUT_DIR}/brake_zenoh_machine_lifetime_subscribe_sm.h"
            DEPFILE "${MESH_ZENOH_MACHINE_LIFETIME_OUTPUT_DIR}/brake_zenoh_machine_lifetime_subscribe_transport.h.d"
            COMMENT "Mesh zenoh machine-lifetime subscribe: Generating brake_zenoh_machine_lifetime_subscribe_transport.h"
            VERBATIM
        )

        add_executable(mesh_zenoh_machine_lifetime_subscribe_test
            mesh/test_mesh_zenoh_machine_lifetime_subscribe.cpp
            "${MESH_ZENOH_MACHINE_LIFETIME_OUTPUT_DIR}/brake_zenoh_machine_lifetime_subscribe_sm.h"
            "${MESH_ZENOH_MACHINE_LIFETIME_OUTPUT_DIR}/brake_zenoh_machine_lifetime_subscribe_transport.h"
        )

        target_include_directories(mesh_zenoh_machine_lifetime_subscribe_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_ZENOH_MACHINE_LIFETIME_OUTPUT_DIR}
                ${CMAKE_CURRENT_SOURCE_DIR}/mesh
        )

        target_link_libraries(mesh_zenoh_machine_lifetime_subscribe_test
            PRIVATE
                sce_base
                sce_mesh_common
                zenohcxx::zenohc
        )

        target_compile_features(mesh_zenoh_machine_lifetime_subscribe_test PRIVATE cxx_std_17)
        apply_sce_test_policies(mesh_zenoh_machine_lifetime_subscribe_test)
        add_test(NAME mesh_zenoh_machine_lifetime_subscribe_verification
                 COMMAND mesh_zenoh_machine_lifetime_subscribe_test)
        set_tests_properties(mesh_zenoh_machine_lifetime_subscribe_verification PROPERTIES
            TIMEOUT 30
            RESOURCE_LOCK zenoh_machine_lifetime_port_17453
        )

        # ── Gap 2a: zenoh_subscribers_ clobber reproduction (§10.9) ───
        # Pins the Gap 2a fix from docs/SCE_MESH_KEYING_AUDIT.md: under
        # the pre-fix codegen, subscription_refcount_ keyed on
        # (target, event) is paired with zenoh_subscribers_ keyed on
        # target alone, so an unsubscribe for any one (target, event)
        # that drives the refcount to zero erases the target-keyed
        # transport subscriber that siblings still need. This fixture
        # drives subscribe.alpha + subscribe.beta on #motor_clobber,
        # confirms both notifications reach the sender while live, then
        # unsubscribes alpha and asserts that notification.beta still
        # arrives. Under the pre-fix codegen the final wait times out.
        #
        # Port 17454 is distinct from the 17447–17453 ports used by the
        # other zenoh runtime tests; isolating the RESOURCE_LOCK keeps
        # subscription state from bleeding into / from them.
        set(MESH_ZENOH_SUB_CLOBBER_OUTPUT_DIR
            "${CMAKE_CURRENT_BINARY_DIR}/mesh_zenoh_subscription_clobber_generated")

        add_custom_command(
            OUTPUT "${MESH_ZENOH_SUB_CLOBBER_OUTPUT_DIR}/brake_zenoh_subscription_clobber_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_ZENOH_SUB_CLOBBER_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_zenoh_subscription_clobber.scxml"
                    -l cpp -o "${MESH_ZENOH_SUB_CLOBBER_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_zenoh_subscription_clobber.yaml"
                    --write-deps "${MESH_ZENOH_SUB_CLOBBER_OUTPUT_DIR}/brake_zenoh_subscription_clobber_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/brake_zenoh_subscription_clobber.scxml" "${SCE_CODEGEN}"
            DEPFILE "${MESH_ZENOH_SUB_CLOBBER_OUTPUT_DIR}/brake_zenoh_subscription_clobber_sm.h.d"
            COMMENT "Mesh zenoh subscription clobber: Generating brake_zenoh_subscription_clobber_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_ZENOH_SUB_CLOBBER_OUTPUT_DIR}/brake_zenoh_subscription_clobber_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_zenoh_subscription_clobber.scxml"
                    -l cpp -o "${MESH_ZENOH_SUB_CLOBBER_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_zenoh_subscription_clobber.yaml"
                    --transport-only
                    --write-deps "${MESH_ZENOH_SUB_CLOBBER_OUTPUT_DIR}/brake_zenoh_subscription_clobber_transport.h.d"
            DEPENDS "${MESH_TEST_DIR}/brake_zenoh_subscription_clobber.scxml"
                    "${MESH_TEST_DIR}/motor_zenoh_subscription_clobber.scxml"
                    "${MESH_TEST_DIR}/deploy_zenoh_subscription_clobber.yaml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_ZENOH_SUB_CLOBBER_OUTPUT_DIR}/brake_zenoh_subscription_clobber_sm.h"
            DEPFILE "${MESH_ZENOH_SUB_CLOBBER_OUTPUT_DIR}/brake_zenoh_subscription_clobber_transport.h.d"
            COMMENT "Mesh zenoh subscription clobber: Generating brake_zenoh_subscription_clobber_transport.h"
            VERBATIM
        )

        add_executable(mesh_zenoh_subscription_clobber_test
            mesh/test_mesh_zenoh_subscription_clobber.cpp
            "${MESH_ZENOH_SUB_CLOBBER_OUTPUT_DIR}/brake_zenoh_subscription_clobber_sm.h"
            "${MESH_ZENOH_SUB_CLOBBER_OUTPUT_DIR}/brake_zenoh_subscription_clobber_transport.h"
        )

        target_include_directories(mesh_zenoh_subscription_clobber_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_ZENOH_SUB_CLOBBER_OUTPUT_DIR}
                ${CMAKE_CURRENT_SOURCE_DIR}/mesh
        )

        target_link_libraries(mesh_zenoh_subscription_clobber_test
            PRIVATE
                sce_base
                sce_mesh_common
                zenohcxx::zenohc
        )

        target_compile_features(mesh_zenoh_subscription_clobber_test PRIVATE cxx_std_17)
        apply_sce_test_policies(mesh_zenoh_subscription_clobber_test)
        add_test(NAME mesh_zenoh_subscription_clobber_verification
                 COMMAND mesh_zenoh_subscription_clobber_test)
        set_tests_properties(mesh_zenoh_subscription_clobber_verification PROPERTIES
            TIMEOUT 30
            RESOURCE_LOCK zenoh_subscription_clobber_port_17454
        )

        # ── SCE Mesh §10.10 Gap 5 (iii): Zenoh publisher-first OutboundBuffer ─
        # Closes the silent-drop on PUT-style patterns (FireForget,
        # FieldWrite) when the publisher declares before any subscriber.
        # Pre-§10.10 the `session.put` reached zenoh but was dropped (no
        # retention on a publisher-first sample). Post-§10.10 the
        # generated router declares a `zenoh::Publisher` + a
        # `MatchingListener` at init(), seeds readiness from
        # `get_matching_status`, and routes FireForget / FieldWrite
        # sends through `OutboundBuffer::admit`. When a subscriber
        # finally appears, `declare_matching_listener` fires →
        # `markReady` drains the buffer via publisher.put → subscriber
        # observes the previously-buffered envelope.
        #
        # Port 17455 is distinct from 17447–17454; RESOURCE_LOCK
        # `zenoh_publisher_first_port_17455` serialises this fixture.
        set(MESH_ZENOH_PUBLISHER_FIRST_OUTPUT_DIR
            "${CMAKE_CURRENT_BINARY_DIR}/mesh_zenoh_publisher_first_generated")

        add_custom_command(
            OUTPUT "${MESH_ZENOH_PUBLISHER_FIRST_OUTPUT_DIR}/brake_zenoh_publisher_first_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_ZENOH_PUBLISHER_FIRST_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_zenoh_publisher_first.scxml"
                    -l cpp -o "${MESH_ZENOH_PUBLISHER_FIRST_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_zenoh_publisher_first.yaml"
                    --write-deps "${MESH_ZENOH_PUBLISHER_FIRST_OUTPUT_DIR}/brake_zenoh_publisher_first_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/brake_zenoh_publisher_first.scxml" "${SCE_CODEGEN}"
            DEPFILE "${MESH_ZENOH_PUBLISHER_FIRST_OUTPUT_DIR}/brake_zenoh_publisher_first_sm.h.d"
            COMMENT "Mesh zenoh publisher-first (§10.10): Generating brake_zenoh_publisher_first_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_ZENOH_PUBLISHER_FIRST_OUTPUT_DIR}/brake_zenoh_publisher_first_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_zenoh_publisher_first.scxml"
                    -l cpp -o "${MESH_ZENOH_PUBLISHER_FIRST_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_zenoh_publisher_first.yaml"
                    --transport-only
                    --write-deps "${MESH_ZENOH_PUBLISHER_FIRST_OUTPUT_DIR}/brake_zenoh_publisher_first_transport.h.d"
            DEPENDS "${MESH_TEST_DIR}/brake_zenoh_publisher_first.scxml"
                    "${MESH_TEST_DIR}/motor_zenoh_publisher_first.scxml"
                    "${MESH_TEST_DIR}/deploy_zenoh_publisher_first.yaml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_ZENOH_PUBLISHER_FIRST_OUTPUT_DIR}/brake_zenoh_publisher_first_sm.h"
            DEPFILE "${MESH_ZENOH_PUBLISHER_FIRST_OUTPUT_DIR}/brake_zenoh_publisher_first_transport.h.d"
            COMMENT "Mesh zenoh publisher-first (§10.10): Generating brake_zenoh_publisher_first_transport.h with outbound_buffer"
            VERBATIM
        )

        add_executable(mesh_zenoh_publisher_first_test
            mesh/test_mesh_zenoh_publisher_first.cpp
            "${MESH_ZENOH_PUBLISHER_FIRST_OUTPUT_DIR}/brake_zenoh_publisher_first_sm.h"
            "${MESH_ZENOH_PUBLISHER_FIRST_OUTPUT_DIR}/brake_zenoh_publisher_first_transport.h"
        )

        target_include_directories(mesh_zenoh_publisher_first_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_ZENOH_PUBLISHER_FIRST_OUTPUT_DIR}
                ${CMAKE_CURRENT_SOURCE_DIR}/mesh
        )

        target_link_libraries(mesh_zenoh_publisher_first_test
            PRIVATE
                sce_base
                sce_mesh_common
                zenohcxx::zenohc
        )

        target_compile_features(mesh_zenoh_publisher_first_test PRIVATE cxx_std_17)
        apply_sce_test_policies(mesh_zenoh_publisher_first_test)
        add_test(NAME mesh_zenoh_publisher_first_verification
                 COMMAND mesh_zenoh_publisher_first_test)
        set_tests_properties(mesh_zenoh_publisher_first_verification PROPERTIES
            TIMEOUT 30
            RESOURCE_LOCK zenoh_publisher_first_port_17455
        )

        # ── Session Z1: Zenoh liveliness PEER_PARTITIONED E2E (§16.7) ─
        # Verifies the generated router's liveliness token + subscriber
        # pair surfaces `error.communication` with reason
        # PEER_PARTITIONED when a peer's liveliness token is
        # undeclared. Port 17448 is distinct from the 17447 pool the
        # other mesh_zenoh_* tests share; isolating the lock keeps
        # liveliness state from bleeding between tests.
        set(MESH_ZENOH_LIVELINESS_OUTPUT_DIR
            "${CMAKE_CURRENT_BINARY_DIR}/mesh_zenoh_liveliness_generated")

        add_custom_command(
            OUTPUT "${MESH_ZENOH_LIVELINESS_OUTPUT_DIR}/brake_zenoh_liveliness_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_ZENOH_LIVELINESS_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_zenoh_liveliness.scxml"
                    -l cpp -o "${MESH_ZENOH_LIVELINESS_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_zenoh_liveliness.yaml"
                    --write-deps "${MESH_ZENOH_LIVELINESS_OUTPUT_DIR}/brake_zenoh_liveliness_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/brake_zenoh_liveliness.scxml" "${SCE_CODEGEN}"
            DEPFILE "${MESH_ZENOH_LIVELINESS_OUTPUT_DIR}/brake_zenoh_liveliness_sm.h.d"
            COMMENT "Mesh zenoh liveliness: Generating brake_zenoh_liveliness_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_ZENOH_LIVELINESS_OUTPUT_DIR}/brake_zenoh_liveliness_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_zenoh_liveliness.scxml"
                    -l cpp -o "${MESH_ZENOH_LIVELINESS_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_zenoh_liveliness.yaml"
                    --transport-only
                    --write-deps "${MESH_ZENOH_LIVELINESS_OUTPUT_DIR}/brake_zenoh_liveliness_transport.h.d"
            DEPENDS "${MESH_TEST_DIR}/brake_zenoh_liveliness.scxml"
                    "${MESH_TEST_DIR}/motor_zenoh_liveliness.scxml"
                    "${MESH_TEST_DIR}/deploy_zenoh_liveliness.yaml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_ZENOH_LIVELINESS_OUTPUT_DIR}/brake_zenoh_liveliness_sm.h"
            DEPFILE "${MESH_ZENOH_LIVELINESS_OUTPUT_DIR}/brake_zenoh_liveliness_transport.h.d"
            COMMENT "Mesh zenoh liveliness: Generating brake_zenoh_liveliness_transport.h"
            VERBATIM
        )

        add_executable(mesh_zenoh_liveliness_test
            mesh/test_mesh_zenoh_liveliness.cpp
            "${MESH_ZENOH_LIVELINESS_OUTPUT_DIR}/brake_zenoh_liveliness_sm.h"
            "${MESH_ZENOH_LIVELINESS_OUTPUT_DIR}/brake_zenoh_liveliness_transport.h"
        )

        target_include_directories(mesh_zenoh_liveliness_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_ZENOH_LIVELINESS_OUTPUT_DIR}
                ${CMAKE_CURRENT_SOURCE_DIR}/mesh
        )

        target_link_libraries(mesh_zenoh_liveliness_test
            PRIVATE
                sce_base
                sce_mesh_common
                zenohcxx::zenohc
        )

        target_compile_features(mesh_zenoh_liveliness_test PRIVATE cxx_std_17)
        apply_sce_test_policies(mesh_zenoh_liveliness_test)
        add_test(NAME mesh_zenoh_liveliness_verification
                 COMMAND mesh_zenoh_liveliness_test)
        set_tests_properties(mesh_zenoh_liveliness_verification PROPERTIES
            TIMEOUT 30
            RESOURCE_LOCK zenoh_port_17448  # shared with mesh_zenoh_scxml_invoke_roundtrip
        )

        # ── §16.4 / §16.7 row 13 region-partition liveness E2E ───────
        # Session json5 that disables zenoh's UDP-multicast scouting.
        # Without it, the generated brake session discovers every other
        # zenoh peer running on the host regardless of TCP port choice
        # (zenoh's scouting is out-of-band and port-agnostic) — a
        # DELETE sample from e.g. mesh_zenoh_liveliness_verification
        # (port 17448) would leak into this observer and surface as a
        # stray PEER_PARTITIONED raise before the test's own drop.
        # Mirrors the isolation strategy used by the Z3 on_drop fixture.
        configure_file(
            "${CMAKE_CURRENT_SOURCE_DIR}/mesh/zenoh_region_liveness_session.json5"
            "${CMAKE_CURRENT_BINARY_DIR}/zenoh_region_liveness_session.json5"
            COPYONLY
        )

        # Orthogonal counterpart to mesh_zenoh_liveliness_verification.
        # brake_region_liveness is a two-region <parallel> machine
        # split across `brake_left_part` / `brake_right_part`; this
        # target compiles brake with `--partition brake_left_part`
        # so the generated router emits both the machine-level token
        # `sce/live/brake_region_liveness` and the partition-level
        # token `sce/live/brake_region_liveness/brake_left_part`.
        # A raw zenoh peer in the test declares the sibling partition
        # token (`.../brake_right_part`) and drops it, exercising the
        # row-13 REGION_PARTITIONED raise path. Port 17456 is
        # distinct from 17447-17455 used by the other zenoh runtime
        # fixtures; RESOURCE_LOCK isolates.
        set(MESH_ZENOH_REGION_LIVENESS_OUTPUT_DIR
            "${CMAKE_CURRENT_BINARY_DIR}/mesh_zenoh_region_liveness_generated")

        add_custom_command(
            OUTPUT "${MESH_ZENOH_REGION_LIVENESS_OUTPUT_DIR}/brake_region_liveness_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_ZENOH_REGION_LIVENESS_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_region_liveness.scxml"
                    -l cpp -o "${MESH_ZENOH_REGION_LIVENESS_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_zenoh_region_liveness.yaml"
                    --partition brake_left_part
                    --write-deps "${MESH_ZENOH_REGION_LIVENESS_OUTPUT_DIR}/brake_region_liveness_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/brake_region_liveness.scxml"
                    "${MESH_TEST_DIR}/deploy_zenoh_region_liveness.yaml"
                    "${SCE_CODEGEN}"
            DEPFILE "${MESH_ZENOH_REGION_LIVENESS_OUTPUT_DIR}/brake_region_liveness_sm.h.d"
            COMMENT "Mesh zenoh region liveness: Generating brake_region_liveness_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_ZENOH_REGION_LIVENESS_OUTPUT_DIR}/brake_region_liveness_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_region_liveness.scxml"
                    -l cpp -o "${MESH_ZENOH_REGION_LIVENESS_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_zenoh_region_liveness.yaml"
                    --transport-only
                    --partition brake_left_part
                    --write-deps "${MESH_ZENOH_REGION_LIVENESS_OUTPUT_DIR}/brake_region_liveness_transport.h.d"
            DEPENDS "${MESH_TEST_DIR}/brake_region_liveness.scxml"
                    "${MESH_TEST_DIR}/peer_region_liveness.scxml"
                    "${MESH_TEST_DIR}/deploy_zenoh_region_liveness.yaml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_ZENOH_REGION_LIVENESS_OUTPUT_DIR}/brake_region_liveness_sm.h"
            DEPFILE "${MESH_ZENOH_REGION_LIVENESS_OUTPUT_DIR}/brake_region_liveness_transport.h.d"
            COMMENT "Mesh zenoh region liveness: Generating brake_region_liveness_transport.h"
            VERBATIM
        )

        add_executable(mesh_zenoh_region_liveness_test
            mesh/test_mesh_region_liveness_zenoh.cpp
            "${MESH_ZENOH_REGION_LIVENESS_OUTPUT_DIR}/brake_region_liveness_sm.h"
            "${MESH_ZENOH_REGION_LIVENESS_OUTPUT_DIR}/brake_region_liveness_transport.h"
        )

        target_include_directories(mesh_zenoh_region_liveness_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_ZENOH_REGION_LIVENESS_OUTPUT_DIR}
                ${CMAKE_CURRENT_SOURCE_DIR}/mesh
        )

        target_link_libraries(mesh_zenoh_region_liveness_test
            PRIVATE
                sce_base
                sce_mesh_common
                zenohcxx::zenohc
        )

        target_compile_features(mesh_zenoh_region_liveness_test PRIVATE cxx_std_17)
        apply_sce_test_policies(mesh_zenoh_region_liveness_test)
        add_test(NAME mesh_zenoh_region_liveness_verification
                 COMMAND mesh_zenoh_region_liveness_test)
        set_tests_properties(mesh_zenoh_region_liveness_verification PROPERTIES
            TIMEOUT 30
            RESOURCE_LOCK zenoh_region_liveness_port_17456
        )

        # ── Gap Z3: Zenoh session.get on_drop early-cancel E2E ────────
        # Verifies the generated router translates zenoh's query-drop
        # signal into `error.invoke.<id>` on a mesh-rpc invoke whose
        # queryable never answers. `zenoh_on_drop_session.json5` shortens
        # queries_default_timeout from 10 s to 300 ms so the test
        # completes in seconds; without the Z3 wiring brake would sit
        # out the 10 s `_mesh_deadline_ms` budget before error.invoke
        # surfaced. Port 17449 is distinct from the 17447 pool (multi-
        # pattern) and 17448 (liveliness); RESOURCE_LOCK isolates.
        configure_file(
            "${CMAKE_CURRENT_SOURCE_DIR}/mesh/zenoh_on_drop_session.json5"
            "${CMAKE_CURRENT_BINARY_DIR}/zenoh_on_drop_session.json5"
            COPYONLY
        )

        set(MESH_ZENOH_ON_DROP_OUTPUT_DIR
            "${CMAKE_CURRENT_BINARY_DIR}/mesh_zenoh_on_drop_generated")

        add_custom_command(
            OUTPUT "${MESH_ZENOH_ON_DROP_OUTPUT_DIR}/brake_zenoh_on_drop_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_ZENOH_ON_DROP_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_zenoh_on_drop.scxml"
                    -l cpp -o "${MESH_ZENOH_ON_DROP_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_zenoh_on_drop.yaml"
                    --write-deps "${MESH_ZENOH_ON_DROP_OUTPUT_DIR}/brake_zenoh_on_drop_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/brake_zenoh_on_drop.scxml" "${SCE_CODEGEN}"
            DEPFILE "${MESH_ZENOH_ON_DROP_OUTPUT_DIR}/brake_zenoh_on_drop_sm.h.d"
            COMMENT "Mesh zenoh on_drop: Generating brake_zenoh_on_drop_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_ZENOH_ON_DROP_OUTPUT_DIR}/brake_zenoh_on_drop_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_zenoh_on_drop.scxml"
                    -l cpp -o "${MESH_ZENOH_ON_DROP_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_zenoh_on_drop.yaml"
                    --transport-only
                    --write-deps "${MESH_ZENOH_ON_DROP_OUTPUT_DIR}/brake_zenoh_on_drop_transport.h.d"
            DEPENDS "${MESH_TEST_DIR}/brake_zenoh_on_drop.scxml"
                    "${MESH_TEST_DIR}/motor_zenoh_on_drop.scxml"
                    "${MESH_TEST_DIR}/deploy_zenoh_on_drop.yaml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_ZENOH_ON_DROP_OUTPUT_DIR}/brake_zenoh_on_drop_sm.h"
            DEPFILE "${MESH_ZENOH_ON_DROP_OUTPUT_DIR}/brake_zenoh_on_drop_transport.h.d"
            COMMENT "Mesh zenoh on_drop: Generating brake_zenoh_on_drop_transport.h"
            VERBATIM
        )

        add_executable(mesh_zenoh_on_drop_test
            mesh/test_mesh_zenoh_on_drop.cpp
            "${MESH_ZENOH_ON_DROP_OUTPUT_DIR}/brake_zenoh_on_drop_sm.h"
            "${MESH_ZENOH_ON_DROP_OUTPUT_DIR}/brake_zenoh_on_drop_transport.h"
        )

        target_include_directories(mesh_zenoh_on_drop_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_ZENOH_ON_DROP_OUTPUT_DIR}
                ${CMAKE_CURRENT_SOURCE_DIR}/mesh
        )

        target_link_libraries(mesh_zenoh_on_drop_test
            PRIVATE
                sce_base
                sce_mesh_common
                zenohcxx::zenohc
        )

        # SCE_THREAD_SAFE: zenoh runtime thread raises error.invoke on
        # brake's external queue while the main thread drives step().
        # Without the define, std::sleep_for alone does not establish
        # happens-before per the C++ memory model — same rationale as
        # mesh_invoke_deadline_expiry.
        target_compile_definitions(mesh_zenoh_on_drop_test PRIVATE SCE_THREAD_SAFE)
        target_compile_features(mesh_zenoh_on_drop_test PRIVATE cxx_std_17)
        apply_sce_test_policies(mesh_zenoh_on_drop_test)
        add_test(NAME mesh_zenoh_on_drop_verification
                 COMMAND mesh_zenoh_on_drop_test
                 WORKING_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}")
        set_tests_properties(mesh_zenoh_on_drop_verification PROPERTIES
            TIMEOUT 30
            RESOURCE_LOCK zenoh_port_17449  # shared with mesh_zenoh_scxml_invoke_crossdev (port 17449)
        )

        # ── Session Z2: server-side queryable deadline verification ───
        # Motor declares a zenoh server binding with query_timeout_ms=500
        # in deploy_zenoh_server_timeout.yaml. The test dispatches inbound
        # requests via a raw zenoh peer and verifies pending_server_queries_
        # entries are erased within timeout + slack when the engine never
        # responds. Port 17451 is distinct from the 17447/17448/17449/17450
        # pool; RESOURCE_LOCK isolates the listener.
        set(MESH_ZENOH_SERVER_TIMEOUT_OUTPUT_DIR
            "${CMAKE_CURRENT_BINARY_DIR}/mesh_zenoh_server_timeout_generated")

        add_custom_command(
            OUTPUT "${MESH_ZENOH_SERVER_TIMEOUT_OUTPUT_DIR}/motor_zenoh_server_timeout_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_ZENOH_SERVER_TIMEOUT_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_zenoh_server_timeout.scxml"
                    -l cpp -o "${MESH_ZENOH_SERVER_TIMEOUT_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_zenoh_server_timeout.yaml"
                    --write-deps "${MESH_ZENOH_SERVER_TIMEOUT_OUTPUT_DIR}/motor_zenoh_server_timeout_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/motor_zenoh_server_timeout.scxml" "${SCE_CODEGEN}"
            DEPFILE "${MESH_ZENOH_SERVER_TIMEOUT_OUTPUT_DIR}/motor_zenoh_server_timeout_sm.h.d"
            COMMENT "Mesh zenoh server_timeout: Generating motor_zenoh_server_timeout_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_ZENOH_SERVER_TIMEOUT_OUTPUT_DIR}/motor_zenoh_server_timeout_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_zenoh_server_timeout.scxml"
                    -l cpp -o "${MESH_ZENOH_SERVER_TIMEOUT_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_zenoh_server_timeout.yaml"
                    --transport-only
                    --write-deps "${MESH_ZENOH_SERVER_TIMEOUT_OUTPUT_DIR}/motor_zenoh_server_timeout_transport.h.d"
            DEPENDS "${MESH_TEST_DIR}/motor_zenoh_server_timeout.scxml"
                    "${MESH_TEST_DIR}/deploy_zenoh_server_timeout.yaml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_ZENOH_SERVER_TIMEOUT_OUTPUT_DIR}/motor_zenoh_server_timeout_sm.h"
            DEPFILE "${MESH_ZENOH_SERVER_TIMEOUT_OUTPUT_DIR}/motor_zenoh_server_timeout_transport.h.d"
            COMMENT "Mesh zenoh server_timeout: Generating motor_zenoh_server_timeout_transport.h"
            VERBATIM
        )

        add_executable(mesh_zenoh_server_timeout_test
            mesh/test_mesh_zenoh_server_timeout.cpp
            "${MESH_ZENOH_SERVER_TIMEOUT_OUTPUT_DIR}/motor_zenoh_server_timeout_sm.h"
            "${MESH_ZENOH_SERVER_TIMEOUT_OUTPUT_DIR}/motor_zenoh_server_timeout_transport.h"
        )

        target_include_directories(mesh_zenoh_server_timeout_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_ZENOH_SERVER_TIMEOUT_OUTPUT_DIR}
                ${CMAKE_CURRENT_SOURCE_DIR}/mesh
        )

        target_link_libraries(mesh_zenoh_server_timeout_test
            PRIVATE
                sce_base
                sce_mesh_common
                zenohcxx::zenohc
        )

        # SCE_THREAD_SAFE: scheduler thread fires onServerQueryTimedOut
        # concurrently with the main thread reading pending_server_size().
        target_compile_definitions(mesh_zenoh_server_timeout_test PRIVATE SCE_THREAD_SAFE)
        target_compile_features(mesh_zenoh_server_timeout_test PRIVATE cxx_std_17)
        apply_sce_test_policies(mesh_zenoh_server_timeout_test)
        add_test(NAME mesh_zenoh_server_timeout_verification
                 COMMAND mesh_zenoh_server_timeout_test
                 WORKING_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}")
        set_tests_properties(mesh_zenoh_server_timeout_verification PROPERTIES
            TIMEOUT 30
            RESOURCE_LOCK zenoh_server_timeout_port_17451
        )

        # ── Session E2-2: dedup injection regression (§10.5) ──────────
        # Hermetic (no live zenoh session, no network) integration test
        # that injects the same envelope twice into a generated Zenoh
        # router and asserts admitInbound drops the duplicate. Links
        # zenohcxx because the brake transport header includes <zenoh.hxx>
        # at the top-level, but no session is ever opened so the test is
        # both fast and lock-free.
        add_executable(mesh_dedup_injection_test
            mesh/test_mesh_dedup_injection.cpp
            "${MESH_ZENOH_OUTPUT_DIR}/brake_sm.h"
            "${MESH_ZENOH_OUTPUT_DIR}/brake_transport.h"
        )

        target_include_directories(mesh_dedup_injection_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_ZENOH_OUTPUT_DIR}
                ${CMAKE_CURRENT_SOURCE_DIR}/mesh
        )

        target_link_libraries(mesh_dedup_injection_test
            PRIVATE
                sce_base
                sce_mesh_common
                zenohcxx::zenohc
        )

        target_compile_features(mesh_dedup_injection_test PRIVATE cxx_std_17)
        apply_sce_test_policies(mesh_dedup_injection_test)
        add_test(NAME mesh_dedup_injection_verification
                 COMMAND mesh_dedup_injection_test)
        set_tests_properties(mesh_dedup_injection_verification PROPERTIES
            TIMEOUT 10
        )

        # ── Session E2-ordering: ordering injection regression (§10.6) ─
        # Hermetic integration test for the generated admitOrdered on a
        # Zenoh binding with `ordering: required`. The fixture is a
        # dedicated deploy.yaml (deploy_ordered.yaml) so the existing
        # zenoh tests keep their baseline behavior. No session is
        # opened — envelopes are stamped by hand and injected directly
        # into admitOrdered. Verifies out-of-order → sorted release,
        # gap-timeout → error.communication.gap callback via
        # tickOrdering, and RpcReply bypass.
        set(MESH_ORDERED_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/mesh_ordered_generated")

        add_custom_command(
            OUTPUT "${MESH_ORDERED_OUTPUT_DIR}/brake_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_ORDERED_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake.scxml" -l cpp -o "${MESH_ORDERED_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_ordered.yaml"
                    --write-deps "${MESH_ORDERED_OUTPUT_DIR}/brake_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/brake.scxml" "${SCE_CODEGEN}"
            DEPFILE "${MESH_ORDERED_OUTPUT_DIR}/brake_sm.h.d"
            COMMENT "Mesh ordered: Generating brake_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_ORDERED_OUTPUT_DIR}/brake_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake.scxml" -l cpp -o "${MESH_ORDERED_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_ordered.yaml"
                    --transport-only
                    --write-deps "${MESH_ORDERED_OUTPUT_DIR}/brake_transport.h.d"
            DEPENDS "${MESH_TEST_DIR}/brake.scxml" "${MESH_TEST_DIR}/deploy_ordered.yaml"
                    "${MESH_TEST_DIR}/motor.scxml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_ORDERED_OUTPUT_DIR}/brake_sm.h"
            DEPFILE "${MESH_ORDERED_OUTPUT_DIR}/brake_transport.h.d"
            COMMENT "Mesh ordered: Generating brake_transport.h (zenoh + ordering:required)"
            VERBATIM
        )

        add_executable(mesh_order_injection_test
            mesh/test_mesh_order_injection.cpp
            "${MESH_ORDERED_OUTPUT_DIR}/brake_sm.h"
            "${MESH_ORDERED_OUTPUT_DIR}/brake_transport.h"
        )

        target_include_directories(mesh_order_injection_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_ORDERED_OUTPUT_DIR}
                ${CMAKE_CURRENT_SOURCE_DIR}/mesh
        )

        target_link_libraries(mesh_order_injection_test
            PRIVATE
                sce_base
                sce_mesh_common
                zenohcxx::zenohc
        )

        target_compile_features(mesh_order_injection_test PRIVATE cxx_std_17)
        apply_sce_test_policies(mesh_order_injection_test)
        add_test(NAME mesh_order_injection_verification
                 COMMAND mesh_order_injection_test)
        set_tests_properties(mesh_order_injection_verification PROPERTIES
            TIMEOUT 10
        )

        # ── Session D: zenoh multi-pattern compile verification ──────
        # brake_zenoh_multi.scxml exercises all four zenoh capability
        # categories (FireForget/RpcRequest/EventSubscribe/FieldRead/
        # FieldWrite). Validates that the generated send_zenoh dispatcher
        # compiles against zenoh-cpp with every switch branch realized.
        set(MESH_ZENOH_MULTI_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/mesh_zenoh_multi_generated")

        add_custom_command(
            OUTPUT "${MESH_ZENOH_MULTI_OUTPUT_DIR}/brake_zenoh_multi_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_ZENOH_MULTI_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_zenoh_multi.scxml" -l cpp -o "${MESH_ZENOH_MULTI_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_zenoh_multi.yaml"
                    --write-deps "${MESH_ZENOH_MULTI_OUTPUT_DIR}/brake_zenoh_multi_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/brake_zenoh_multi.scxml" "${SCE_CODEGEN}"
            DEPFILE "${MESH_ZENOH_MULTI_OUTPUT_DIR}/brake_zenoh_multi_sm.h.d"
            COMMENT "Mesh zenoh multi: Generating brake_zenoh_multi_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_ZENOH_MULTI_OUTPUT_DIR}/motor_zenoh_multi_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_ZENOH_MULTI_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_zenoh_multi.scxml" -l cpp -o "${MESH_ZENOH_MULTI_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_zenoh_multi.yaml"
                    --write-deps "${MESH_ZENOH_MULTI_OUTPUT_DIR}/motor_zenoh_multi_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/motor_zenoh_multi.scxml" "${SCE_CODEGEN}"
            DEPFILE "${MESH_ZENOH_MULTI_OUTPUT_DIR}/motor_zenoh_multi_sm.h.d"
            COMMENT "Mesh zenoh multi: Generating motor_zenoh_multi_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_ZENOH_MULTI_OUTPUT_DIR}/brake_zenoh_multi_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_zenoh_multi.scxml" -l cpp -o "${MESH_ZENOH_MULTI_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_zenoh_multi.yaml"
                    --transport-only
                    --write-deps "${MESH_ZENOH_MULTI_OUTPUT_DIR}/brake_zenoh_multi_transport.h.d"
            DEPENDS "${MESH_TEST_DIR}/brake_zenoh_multi.scxml" "${MESH_TEST_DIR}/deploy_zenoh_multi.yaml"
                    "${MESH_TEST_DIR}/motor_zenoh_multi.scxml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_ZENOH_MULTI_OUTPUT_DIR}/brake_zenoh_multi_sm.h"
            DEPFILE "${MESH_ZENOH_MULTI_OUTPUT_DIR}/brake_zenoh_multi_transport.h.d"
            COMMENT "Mesh zenoh multi: Generating brake_zenoh_multi_transport.h (multi-pattern)"
            VERBATIM
        )

        # Session E: generate motor (server) transport header
        add_custom_command(
            OUTPUT "${MESH_ZENOH_MULTI_OUTPUT_DIR}/motor_zenoh_multi_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_zenoh_multi.scxml" -l cpp -o "${MESH_ZENOH_MULTI_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_zenoh_multi.yaml"
                    --transport-only
                    --write-deps "${MESH_ZENOH_MULTI_OUTPUT_DIR}/motor_zenoh_multi_transport.h.d"
            DEPENDS "${MESH_TEST_DIR}/motor_zenoh_multi.scxml" "${MESH_TEST_DIR}/deploy_zenoh_multi.yaml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_ZENOH_MULTI_OUTPUT_DIR}/motor_zenoh_multi_sm.h"
            DEPFILE "${MESH_ZENOH_MULTI_OUTPUT_DIR}/motor_zenoh_multi_transport.h.d"
            COMMENT "Mesh zenoh multi: Generating motor_zenoh_multi_transport.h (server-side Session E)"
            VERBATIM
        )

        add_executable(mesh_zenoh_multipattern_compile_test
            mesh/test_mesh_zenoh_multipattern.cpp
            "${MESH_ZENOH_MULTI_OUTPUT_DIR}/brake_zenoh_multi_sm.h"
            "${MESH_ZENOH_MULTI_OUTPUT_DIR}/motor_zenoh_multi_sm.h"
            "${MESH_ZENOH_MULTI_OUTPUT_DIR}/brake_zenoh_multi_transport.h"
            "${MESH_ZENOH_MULTI_OUTPUT_DIR}/motor_zenoh_multi_transport.h"
        )

        target_include_directories(mesh_zenoh_multipattern_compile_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_ZENOH_MULTI_OUTPUT_DIR}
        )

        target_link_libraries(mesh_zenoh_multipattern_compile_test
            PRIVATE
                sce_base
                sce_mesh_common
                zenohcxx::zenohc
        )

        target_compile_features(mesh_zenoh_multipattern_compile_test PRIVATE cxx_std_17)
        sce_pin_cxx17(mesh_zenoh_multipattern_compile_test)
        apply_sce_test_policies(mesh_zenoh_multipattern_compile_test)
        add_test(NAME mesh_zenoh_multipattern_compile_verification
                 COMMAND mesh_zenoh_multipattern_compile_test)

        # ── Session D: zenoh multi-pattern peer-mode runtime E2E ─────
        # Exercises the generated send_zenoh dispatcher end-to-end in peer
        # mode (no daemon). Two zenoh::Session instances in the same
        # process with explicit locators for deterministic discovery.
        # Proves: (a) zenoh peer-mode discovery works without routerd;
        # (b) envelope round-trip across FireForget/RpcRequest/EventSubscribe
        # preserves PatternKind + payload via CBOR encode/decode;
        # (c) RPC reply correlation is native (session.get on_reply closure).
        add_executable(mesh_zenoh_multipattern_runtime_test
            mesh/test_mesh_zenoh_multipattern_runtime.cpp
            "${MESH_ZENOH_MULTI_OUTPUT_DIR}/brake_zenoh_multi_sm.h"
            "${MESH_ZENOH_MULTI_OUTPUT_DIR}/motor_zenoh_multi_sm.h"
            "${MESH_ZENOH_MULTI_OUTPUT_DIR}/brake_zenoh_multi_transport.h"
        )

        target_include_directories(mesh_zenoh_multipattern_runtime_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_ZENOH_MULTI_OUTPUT_DIR}
                ${CMAKE_CURRENT_SOURCE_DIR}/mesh
        )

        target_link_libraries(mesh_zenoh_multipattern_runtime_test
            PRIVATE
                sce_base
                sce_mesh_common
                zenohcxx::zenohc
        )

        target_compile_features(mesh_zenoh_multipattern_runtime_test PRIVATE cxx_std_17)
        apply_sce_test_policies(mesh_zenoh_multipattern_runtime_test)
        add_test(NAME mesh_zenoh_multipattern_runtime
                 COMMAND mesh_zenoh_multipattern_runtime_test)
        # Give peer discovery room even on busy CI nodes. RESOURCE_LOCK
        # serializes tests that bind the shared loopback port declared in
        # deploy_zenoh_multi.yaml so parallel runs don't fight for it.
        set_tests_properties(mesh_zenoh_multipattern_runtime PROPERTIES
            TIMEOUT 30
            RESOURCE_LOCK zenoh_multi_port_17447
        )

        # ── Session F: zenoh server-side runtime E2E ──────────────────
        # Exercises the generated server-side TransportRouter (declare_queryable,
        # pending_server_queries_, handleServerResponse) end-to-end in peer
        # mode. Proves: (a) queryable callback receives and stores Query;
        # (b) handleServerResponse correlates by UUID and calls Query::reply;
        # (c) CBOR payload round-trips through server encode/decode.
        add_executable(mesh_zenoh_server_runtime_test
            mesh/test_mesh_zenoh_server_runtime.cpp
            "${MESH_ZENOH_MULTI_OUTPUT_DIR}/motor_zenoh_multi_sm.h"
            "${MESH_ZENOH_MULTI_OUTPUT_DIR}/motor_zenoh_multi_transport.h"
        )

        target_include_directories(mesh_zenoh_server_runtime_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_ZENOH_MULTI_OUTPUT_DIR}
                ${CMAKE_CURRENT_SOURCE_DIR}/mesh
        )

        target_link_libraries(mesh_zenoh_server_runtime_test
            PRIVATE
                sce_base
                sce_mesh_common
                zenohcxx::zenohc
        )

        target_compile_features(mesh_zenoh_server_runtime_test PRIVATE cxx_std_17)
        apply_sce_test_policies(mesh_zenoh_server_runtime_test)
        add_test(NAME mesh_zenoh_server_runtime
                 COMMAND mesh_zenoh_server_runtime_test)
        set_tests_properties(mesh_zenoh_server_runtime PROPERTIES
            TIMEOUT 30
            RESOURCE_LOCK zenoh_multi_port_17447
        )

        # ── Session F: mesh-rpc over Zenoh end-to-end ─────────────────
        # Full client↔server round-trip: brake send_zenoh RpcRequest →
        # motor queryable → handleServerResponse → Query::reply → brake
        # on_reply → type rewrite → brake dispatchToSender. Both routers
        # use generated transport headers with peer-mode sessions.
        add_executable(mesh_zenoh_rpc_e2e_test
            mesh/test_mesh_zenoh_rpc_e2e.cpp
            "${MESH_ZENOH_MULTI_OUTPUT_DIR}/brake_zenoh_multi_sm.h"
            "${MESH_ZENOH_MULTI_OUTPUT_DIR}/motor_zenoh_multi_sm.h"
            "${MESH_ZENOH_MULTI_OUTPUT_DIR}/brake_zenoh_multi_transport.h"
            "${MESH_ZENOH_MULTI_OUTPUT_DIR}/motor_zenoh_multi_transport.h"
        )

        target_include_directories(mesh_zenoh_rpc_e2e_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_ZENOH_MULTI_OUTPUT_DIR}
                ${CMAKE_CURRENT_SOURCE_DIR}/mesh
        )

        target_link_libraries(mesh_zenoh_rpc_e2e_test
            PRIVATE
                sce_base
                sce_mesh_common
                zenohcxx::zenohc
        )

        target_compile_features(mesh_zenoh_rpc_e2e_test PRIVATE cxx_std_17)
        apply_sce_test_policies(mesh_zenoh_rpc_e2e_test)
        add_test(NAME mesh_zenoh_rpc_e2e
                 COMMAND mesh_zenoh_rpc_e2e_test)
        set_tests_properties(mesh_zenoh_rpc_e2e PROPERTIES
            TIMEOUT 30
            RESOURCE_LOCK zenoh_multi_port_17447
        )

        # ── Session I: engine-driven RPC/FieldRead over Zenoh ─────────
        # Unlike mesh_zenoh_server_runtime (TestSenderEngine recorder +
        # manual handleServerResponse), this test binds the real
        # motor_zenoh_multi SM to TransportRouter and drives it with a
        # step() pump thread. The engine's onentry <raise> and codegen-
        # injected <send> are what produce the reply — any regression in
        # that chain (raise suppression, invoke_id propagation,
        # resolvePattern → RpcReply mis-routing, handleServerResponse
        # correlation key mismatch) surfaces here.
        add_executable(mesh_zenoh_rpc_engine_driven_test
            mesh/test_mesh_zenoh_rpc_engine_driven.cpp
            "${MESH_ZENOH_MULTI_OUTPUT_DIR}/motor_zenoh_multi_sm.h"
            "${MESH_ZENOH_MULTI_OUTPUT_DIR}/motor_zenoh_multi_transport.h"
        )

        target_include_directories(mesh_zenoh_rpc_engine_driven_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_ZENOH_MULTI_OUTPUT_DIR}
                ${CMAKE_CURRENT_SOURCE_DIR}/mesh
        )

        target_link_libraries(mesh_zenoh_rpc_engine_driven_test
            PRIVATE
                sce_base
                sce_mesh_common
                zenohcxx::zenohc
        )

        target_compile_features(mesh_zenoh_rpc_engine_driven_test PRIVATE cxx_std_17)
        apply_sce_test_policies(mesh_zenoh_rpc_engine_driven_test)
        add_test(NAME mesh_zenoh_rpc_engine_driven
                 COMMAND mesh_zenoh_rpc_engine_driven_test)
        set_tests_properties(mesh_zenoh_rpc_engine_driven PROPERTIES
            TIMEOUT 30
            RESOURCE_LOCK zenoh_multi_port_17447
        )

        # Session J: Zenoh eventgroup notification (server-initiated push).
        # Motor raises field.notify.position spontaneously (no preceding
        # getter/setter) → session.put on the server key.
        add_executable(mesh_zenoh_eventgroup_engine_driven_test
            mesh/test_mesh_zenoh_eventgroup_engine_driven.cpp
            "${MESH_ZENOH_MULTI_OUTPUT_DIR}/motor_zenoh_multi_sm.h"
            "${MESH_ZENOH_MULTI_OUTPUT_DIR}/motor_zenoh_multi_transport.h"
        )

        target_include_directories(mesh_zenoh_eventgroup_engine_driven_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_ZENOH_MULTI_OUTPUT_DIR}
                ${CMAKE_CURRENT_SOURCE_DIR}/mesh
        )

        target_link_libraries(mesh_zenoh_eventgroup_engine_driven_test
            PRIVATE
                sce_base
                sce_mesh_common
                zenohcxx::zenohc
        )

        target_compile_features(mesh_zenoh_eventgroup_engine_driven_test PRIVATE cxx_std_17)
        apply_sce_test_policies(mesh_zenoh_eventgroup_engine_driven_test)
        add_test(NAME mesh_zenoh_eventgroup_engine_driven
                 COMMAND mesh_zenoh_eventgroup_engine_driven_test)
        set_tests_properties(mesh_zenoh_eventgroup_engine_driven PROPERTIES
            TIMEOUT 30
            RESOURCE_LOCK zenoh_multi_port_17447
        )
    else()
        message(STATUS "zenohcxx not found — skipping mesh_zenoh_compile_test")
    endif()

    # DDS discovery is hoisted here from the §8.2 multi-pattern block
    # below: the §14.4 pool fixture binds a DDS target too, and a gate
    # that reads `CycloneDDS_FOUND` before `find_package` has run would
    # silently take the else-branch on a machine that HAS Cyclone
    # installed — skipping the very fixture that proves the DDS pool
    # arm. `find_package(... QUIET)` is idempotent, so the later block
    # is unaffected.
    find_package(CycloneDDS QUIET)
    find_package(CycloneDDS-CXX QUIET)

    # ── SCE_MESH.md §14.4 pool compile verification ──────────────
    # pool_client exercises every cell of the (pool shape x member
    # carrier) registry matrix in a single generated header:
    #   * Zenoh, Open + StringSegment — runtime KeyExpr assembled from
    #     the `id` <param> and the binding's `sce/player/{id}` key.
    #   * SOME/IP, Bounded + TypedInstanceId — runtime instance id
    #     selected from `inst`, validated against `instances: [1, 2, 3]`.
    #   * DDS, Bounded + StringSegment — one `Dds::Client` per `members:`
    #     entry built at init() on a build-time-substituted topic, with
    #     the runtime `corner` <param> selecting among them.
    # The DDS binding is what makes this a matrix rather than a list: it
    # shares a shape with SOME/IP and a carrier with Zenoh, so a template
    # that keyed emission off the transport name instead of the two axes
    # would fail here.
    # Gated on all three stacks being discovered because the fixture's
    # deploy.yaml binds each transport for at least one target.
    if(vsomeip3_FOUND AND zenohc_FOUND AND zenohcxx_FOUND
       AND CycloneDDS_FOUND AND CycloneDDS-CXX_FOUND)
        set(MESH_POOL_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/mesh_pool_generated")

        add_custom_command(
            OUTPUT "${MESH_POOL_OUTPUT_DIR}/pool_client_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_POOL_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/pool_client.scxml" -l cpp -o "${MESH_POOL_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_pool.yaml"
                    --write-deps "${MESH_POOL_OUTPUT_DIR}/pool_client_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/pool_client.scxml" "${SCE_CODEGEN}"
            DEPFILE "${MESH_POOL_OUTPUT_DIR}/pool_client_sm.h.d"
            COMMENT "Mesh pool: Generating pool_client_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_POOL_OUTPUT_DIR}/pool_client_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/pool_client.scxml" -l cpp -o "${MESH_POOL_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_pool.yaml"
                    --transport-only
                    --write-deps "${MESH_POOL_OUTPUT_DIR}/pool_client_transport.h.d"
            DEPENDS "${MESH_TEST_DIR}/pool_client.scxml" "${MESH_TEST_DIR}/deploy_pool.yaml"
                    "${MESH_TEST_DIR}/vsomeip_pool_controller.json"
                    "${MESH_TEST_DIR}/sensor.scxml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_POOL_OUTPUT_DIR}/pool_client_sm.h"
            DEPFILE "${MESH_POOL_OUTPUT_DIR}/pool_client_transport.h.d"
            COMMENT "Mesh pool: Generating pool_client_transport.h (§14.4 pool dispatch)"
            VERBATIM
        )

        add_executable(mesh_pool_compile_test
            mesh/test_mesh_pool_compile.cpp
            "${MESH_POOL_OUTPUT_DIR}/pool_client_sm.h"
            "${MESH_POOL_OUTPUT_DIR}/pool_client_transport.h"
        )

        target_include_directories(mesh_pool_compile_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_POOL_OUTPUT_DIR}
        )

        target_link_libraries(mesh_pool_compile_test
            PRIVATE
                sce_base
                sce_mesh_common
                sce_mesh_dds_types
                vsomeip3
                zenohcxx::zenohc
                CycloneDDS-CXX::ddscxx
        )

        target_compile_features(mesh_pool_compile_test PRIVATE cxx_std_17)
        sce_pin_cxx17(mesh_pool_compile_test)

        apply_sce_test_policies(mesh_pool_compile_test)

        add_test(NAME mesh_pool_compile_verification COMMAND mesh_pool_compile_test)
    else()
        message(STATUS "vsomeip3 / zenohcxx / CycloneDDS not all found — skipping mesh_pool_compile_test")
    endif()

    # ── SCE_MESH.md §14.4 server-pool compile verification ──
    # motor_pool exercises the SERVER-side multi-instance branch of
    # mesh_transport.h.jinja2 (Gap 7):
    #   * deploy.yaml `server.instances: [1, 2]` accepted at parse
    #     (transport registry flag supports_multi_instance_server=true)
    #   * ServerBinding.instance_pool carries the list through topology
    #   * template emits SOMEIP_SERVER_INSTANCES std::array and init()
    #     iterates it (one offer_service + one register_message_handler
    #     per (instance, method))
    # Compilation + static_assert are the test; runtime per-instance
    # dispatch (msg->get_instance() → per-session SCXMLSession) is
    # tracked separately under the Gap 7 roadmap. SOME/IP-only —
    # Zenoh's registry flag stays false (KeyExpr is not a peer
    # identity), so no Zenoh counterpart exists.
    if(vsomeip3_FOUND)
        set(MESH_SERVER_POOL_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/mesh_someip_server_pool_generated")

        add_custom_command(
            OUTPUT "${MESH_SERVER_POOL_OUTPUT_DIR}/motor_pool_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SERVER_POOL_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_pool.scxml" -l cpp -o "${MESH_SERVER_POOL_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_someip_pool_server.yaml"
                    --write-deps "${MESH_SERVER_POOL_OUTPUT_DIR}/motor_pool_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/motor_pool.scxml" "${SCE_CODEGEN}"
            DEPFILE "${MESH_SERVER_POOL_OUTPUT_DIR}/motor_pool_sm.h.d"
            COMMENT "Mesh server pool: Generating motor_pool_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_SERVER_POOL_OUTPUT_DIR}/motor_pool_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_pool.scxml" -l cpp -o "${MESH_SERVER_POOL_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_someip_pool_server.yaml"
                    --transport-only
                    --write-deps "${MESH_SERVER_POOL_OUTPUT_DIR}/motor_pool_transport.h.d"
            DEPENDS "${MESH_TEST_DIR}/motor_pool.scxml" "${MESH_TEST_DIR}/deploy_someip_pool_server.yaml"
                    "${MESH_TEST_DIR}/vsomeip_motor_pool.json"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_SERVER_POOL_OUTPUT_DIR}/motor_pool_sm.h"
            DEPFILE "${MESH_SERVER_POOL_OUTPUT_DIR}/motor_pool_transport.h.d"
            COMMENT "Mesh server pool: Generating motor_pool_transport.h (§14.4)"
            VERBATIM
        )

        add_executable(mesh_someip_server_pool_compile_test
            mesh/test_mesh_someip_server_pool_compile.cpp
            "${MESH_SERVER_POOL_OUTPUT_DIR}/motor_pool_sm.h"
            "${MESH_SERVER_POOL_OUTPUT_DIR}/motor_pool_transport.h"
        )

        target_include_directories(mesh_someip_server_pool_compile_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_SERVER_POOL_OUTPUT_DIR}
        )

        target_link_libraries(mesh_someip_server_pool_compile_test
            PRIVATE
                sce_base
                sce_mesh_common
                vsomeip3
        )

        target_compile_features(mesh_someip_server_pool_compile_test PRIVATE cxx_std_17)
        sce_pin_cxx17(mesh_someip_server_pool_compile_test)

        apply_sce_test_policies(mesh_someip_server_pool_compile_test)

        add_test(NAME mesh_someip_server_pool_compile
                 COMMAND mesh_someip_server_pool_compile_test)

        # ── SCE_MESH.md §14.4 server-pool runtime dispatch proof ──
        # Companion to mesh_someip_server_pool_compile. The compile test
        # only proves SOMEIP_SERVER_INSTANCES + N_SESSIONS +
        # session_index_for_instance render correctly; this test proves
        # the per-instance handler lambdas actually dispatch inbound
        # vsomeip messages to the matching sessions_[idx] slot.
        # Without it the whole §14.4 runtime dispatch contract
        # (msg->get_instance() → session_idx → sessions_[idx]) is
        # unobservable and a regression that mis-captured server_instance
        # or short-circuited dispatch to sessions_[0] would compile clean.
        add_executable(mesh_someip_server_pool_runtime_test
            mesh/test_mesh_someip_server_pool_runtime.cpp
            "${MESH_SERVER_POOL_OUTPUT_DIR}/motor_pool_sm.h"
            "${MESH_SERVER_POOL_OUTPUT_DIR}/motor_pool_transport.h"
        )

        target_include_directories(mesh_someip_server_pool_runtime_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${MESH_SERVER_POOL_OUTPUT_DIR}
                ${CMAKE_CURRENT_SOURCE_DIR}/mesh
        )

        target_link_libraries(mesh_someip_server_pool_runtime_test
            PRIVATE
                sce_base
                sce_mesh_common
                vsomeip3
        )

        target_compile_features(mesh_someip_server_pool_runtime_test PRIVATE cxx_std_17)
        apply_sce_test_policies(mesh_someip_server_pool_runtime_test)
        add_test(NAME mesh_someip_server_pool_runtime
                 COMMAND mesh_someip_server_pool_runtime_test)
        set_tests_properties(mesh_someip_server_pool_runtime PROPERTIES
            TIMEOUT 30
            ENVIRONMENT "VSOMEIP_CONFIGURATION=${CMAKE_CURRENT_SOURCE_DIR}/mesh/vsomeip_e2e_motor_pool.json;LD_LIBRARY_PATH=${vsomeip3_DIR}:$ENV{LD_LIBRARY_PATH}")
    else()
        message(STATUS "vsomeip3 not found — skipping mesh_someip_server_pool_compile_test")
    endif()

    # ── SCE_MESH.md §14.4 SOME/IP pool `<send>` runtime verification ──
    # The server-pool tests above cover the receiving end (one router
    # hosting N instances). This covers the CLIENT end on the `<send>`
    # path: which declared member a send addressed to a pool binding
    # actually lands on.
    #
    #   engine onentry → <param inst> evaluated against the Lua datamodel
    #     → buildEventDataJson → mesh send callback → route_send
    #     → SOME/IP pool arm → set_instance(runtime id) → app.send
    #     → raw harness offering BOTH instances counts per member
    #
    # It is the pool leg with the worst failure mode: an unresolved
    # instance id is not absent, it is the binding default, so the
    # request is delivered to the wrong member with no error on either
    # end. A generated router cannot be the observer — it binds one
    # instance, and the claim is about telling two apart.
    #
    # The harness owns routing (`routing: pool_send_harness` in
    # vsomeip_e2e_pool_send.json) and the fixture declares its own
    # `network:` so /tmp/sce_someip_pool_send-* never collides with a
    # sibling SOME/IP fixture under ctest -j.
    if(vsomeip3_FOUND)
        set(MESH_SOMEIP_POOL_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/mesh_someip_pool_generated")

        add_custom_command(
            OUTPUT "${MESH_SOMEIP_POOL_OUTPUT_DIR}/pool_sender_someip_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SOMEIP_POOL_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/pool_sender_someip.scxml" -l cpp
                    -o "${MESH_SOMEIP_POOL_OUTPUT_DIR}"
                    --write-deps "${MESH_SOMEIP_POOL_OUTPUT_DIR}/pool_sender_someip_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/pool_sender_someip.scxml" "${SCE_CODEGEN}"
            DEPFILE "${MESH_SOMEIP_POOL_OUTPUT_DIR}/pool_sender_someip_sm.h.d"
            COMMENT "Mesh someip pool send: Generating pool_sender_someip_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_SOMEIP_POOL_OUTPUT_DIR}/pool_sender_someip_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/pool_sender_someip.scxml" -l cpp
                    -o "${MESH_SOMEIP_POOL_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_someip_pool.yaml"
                    --transport-only
                    --write-deps "${MESH_SOMEIP_POOL_OUTPUT_DIR}/pool_sender_someip_transport.h.d"
            DEPENDS "${MESH_TEST_DIR}/pool_sender_someip.scxml" "${MESH_TEST_DIR}/deploy_someip_pool.yaml"
                    "${MESH_TEST_DIR}/controller_someip_pool.scxml"
                    "${MESH_TEST_DIR}/vsomeip_pool_send.json"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_SOMEIP_POOL_OUTPUT_DIR}/pool_sender_someip_sm.h"
            DEPFILE "${MESH_SOMEIP_POOL_OUTPUT_DIR}/pool_sender_someip_transport.h.d"
            COMMENT "Mesh someip pool send: Generating pool_sender_someip_transport.h (§14.4)"
            VERBATIM
        )

        add_executable(mesh_someip_pool_test
            mesh/test_mesh_someip_pool.cpp
            "${MESH_SOMEIP_POOL_OUTPUT_DIR}/pool_sender_someip_sm.h"
            "${MESH_SOMEIP_POOL_OUTPUT_DIR}/pool_sender_someip_transport.h"
        )

        target_include_directories(mesh_someip_pool_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${CMAKE_CURRENT_SOURCE_DIR}/mesh
                ${MESH_SOMEIP_POOL_OUTPUT_DIR}
        )

        target_link_libraries(mesh_someip_pool_test
            PRIVATE
                sce_base
                sce_runtime
                sce_mesh_common
                vsomeip3
        )

        target_compile_features(mesh_someip_pool_test PRIVATE cxx_std_17)
        apply_sce_test_policies(mesh_someip_pool_test)

        add_test(NAME mesh_someip_pool_send_runtime COMMAND mesh_someip_pool_test)
        set_tests_properties(mesh_someip_pool_send_runtime PROPERTIES
            TIMEOUT 30
            ENVIRONMENT "VSOMEIP_CONFIGURATION=${CMAKE_CURRENT_SOURCE_DIR}/mesh/vsomeip_e2e_pool_send.json;LD_LIBRARY_PATH=${vsomeip3_DIR}:$ENV{LD_LIBRARY_PATH}")
    else()
        message(STATUS "vsomeip3 not found — skipping mesh_someip_pool_test")
    endif()

    # ── SCE_MESH.md §14.4 Zenoh pool runtime verification ──────────
    # Compile-only pool_client verifies the generated pool branches
    # instantiate; this test drives the Zenoh pool path end-to-end:
    #
    #   engine onentry → performMeshInvoke → Zenoh pool branch
    #     → substitute `{id}` token ("42") into `sce/player/{id}`
    #     → zenoh_session_.get("sce/player/42", ...)
    #     → harness queryable replies with invoke_id preserved
    #     → on_reply → admitInbound → dispatchToSender → handleReply
    #     → done.invoke raises → calling_gamer → ok
    #
    # Without this test a regression in the JSON decode / placeholder
    # scan / pool_key.replace flow would emit a well-formed binary
    # whose queries never reach any peer — the deadline path would
    # silently mask it as error.invoke.
    #
    # Zenoh-only fixture (pool_zenoh_client.scxml) so the test does
    # not need the vsomeip routing-manager daemon. Port 17452 is
    # reserved via RESOURCE_LOCK to stay clear of the 17447/17448/
    # 17449/17450 ports used by other Zenoh runtime tests.
    if(zenohc_FOUND AND zenohcxx_FOUND)
        set(MESH_POOL_ZENOH_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/mesh_pool_zenoh_generated")

        add_custom_command(
            OUTPUT "${MESH_POOL_ZENOH_OUTPUT_DIR}/pool_zenoh_client_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_POOL_ZENOH_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/pool_zenoh_client.scxml" -l cpp -o "${MESH_POOL_ZENOH_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_pool_zenoh.yaml"
                    --write-deps "${MESH_POOL_ZENOH_OUTPUT_DIR}/pool_zenoh_client_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/pool_zenoh_client.scxml" "${SCE_CODEGEN}"
            DEPFILE "${MESH_POOL_ZENOH_OUTPUT_DIR}/pool_zenoh_client_sm.h.d"
            COMMENT "Mesh pool Zenoh: Generating pool_zenoh_client_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_POOL_ZENOH_OUTPUT_DIR}/pool_zenoh_client_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/pool_zenoh_client.scxml" -l cpp -o "${MESH_POOL_ZENOH_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_pool_zenoh.yaml"
                    --transport-only
                    --write-deps "${MESH_POOL_ZENOH_OUTPUT_DIR}/pool_zenoh_client_transport.h.d"
            DEPENDS "${MESH_TEST_DIR}/pool_zenoh_client.scxml" "${MESH_TEST_DIR}/deploy_pool_zenoh.yaml"
                    "${MESH_TEST_DIR}/gamer_pool.scxml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_POOL_ZENOH_OUTPUT_DIR}/pool_zenoh_client_sm.h"
            DEPFILE "${MESH_POOL_ZENOH_OUTPUT_DIR}/pool_zenoh_client_transport.h.d"
            COMMENT "Mesh pool Zenoh: Generating pool_zenoh_client_transport.h (§14.4 runtime)"
            VERBATIM
        )

        add_executable(mesh_pool_zenoh_runtime_test
            mesh/test_mesh_pool_zenoh_runtime.cpp
            "${MESH_POOL_ZENOH_OUTPUT_DIR}/pool_zenoh_client_sm.h"
            "${MESH_POOL_ZENOH_OUTPUT_DIR}/pool_zenoh_client_transport.h"
        )

        target_include_directories(mesh_pool_zenoh_runtime_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${CMAKE_SOURCE_DIR}/tests/mesh
                ${MESH_POOL_ZENOH_OUTPUT_DIR}
        )

        target_link_libraries(mesh_pool_zenoh_runtime_test
            PRIVATE
                sce_base
                sce_mesh_common
                zenohcxx::zenohc
                pthread
        )

        target_compile_features(mesh_pool_zenoh_runtime_test PRIVATE cxx_std_17)
        target_compile_definitions(mesh_pool_zenoh_runtime_test PRIVATE SCE_THREAD_SAFE)

        apply_sce_test_policies(mesh_pool_zenoh_runtime_test)

        add_test(NAME mesh_pool_zenoh_runtime_verification
                 COMMAND mesh_pool_zenoh_runtime_test)
        set_tests_properties(mesh_pool_zenoh_runtime_verification PROPERTIES
            TIMEOUT 30
            RESOURCE_LOCK zenoh_pool_port_17452
        )
    else()
        message(STATUS "zenohcxx not found — skipping mesh_pool_zenoh_runtime_test")
    endif()

    # ── SCE_MESH.md §14.4 Zenoh pool `<send>` runtime verification ──
    # The sibling test above drives the open pool through `<invoke>`;
    # this one drives it through `<send>`, the other dispatch path
    # §14.4 names. The two fail differently: a misaddressed `<invoke>`
    # eventually surfaces as error.invoke, while a misaddressed `<send>`
    # is a Zenoh put nobody subscribes to — lost with no error anywhere.
    #
    #   engine onentry → <param id> evaluated against the Lua datamodel
    #     → buildEventDataJson → mesh send callback → route_send
    #     → Zenoh pool arm → put on the substituted KeyExpr
    #     → raw WILDCARD subscriber records which address it took
    #
    # The subscriber is a wildcard rather than one bound to the expected
    # key so a regression names the address it actually used instead of
    # only reporting an absence. Port 17457 is this fixture's own
    # (17447-17456 are taken by the sibling Zenoh runtime fixtures).
    if(zenohc_FOUND AND zenohcxx_FOUND)
        set(MESH_ZENOH_POOL_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/mesh_zenoh_pool_generated")

        add_custom_command(
            OUTPUT "${MESH_ZENOH_POOL_OUTPUT_DIR}/pool_sender_zenoh_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_ZENOH_POOL_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/pool_sender_zenoh.scxml" -l cpp
                    -o "${MESH_ZENOH_POOL_OUTPUT_DIR}"
                    --write-deps "${MESH_ZENOH_POOL_OUTPUT_DIR}/pool_sender_zenoh_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/pool_sender_zenoh.scxml" "${SCE_CODEGEN}"
            DEPFILE "${MESH_ZENOH_POOL_OUTPUT_DIR}/pool_sender_zenoh_sm.h.d"
            COMMENT "Mesh zenoh pool send: Generating pool_sender_zenoh_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_ZENOH_POOL_OUTPUT_DIR}/pool_sender_zenoh_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/pool_sender_zenoh.scxml" -l cpp
                    -o "${MESH_ZENOH_POOL_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_zenoh_pool.yaml"
                    --transport-only
                    --write-deps "${MESH_ZENOH_POOL_OUTPUT_DIR}/pool_sender_zenoh_transport.h.d"
            DEPENDS "${MESH_TEST_DIR}/pool_sender_zenoh.scxml" "${MESH_TEST_DIR}/deploy_zenoh_pool.yaml"
                    "${MESH_TEST_DIR}/gamer_zenoh_pool.scxml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_ZENOH_POOL_OUTPUT_DIR}/pool_sender_zenoh_sm.h"
            DEPFILE "${MESH_ZENOH_POOL_OUTPUT_DIR}/pool_sender_zenoh_transport.h.d"
            COMMENT "Mesh zenoh pool send: Generating pool_sender_zenoh_transport.h (§14.4)"
            VERBATIM
        )

        add_executable(mesh_zenoh_pool_test
            mesh/test_mesh_zenoh_pool.cpp
            "${MESH_ZENOH_POOL_OUTPUT_DIR}/pool_sender_zenoh_sm.h"
            "${MESH_ZENOH_POOL_OUTPUT_DIR}/pool_sender_zenoh_transport.h"
        )

        target_include_directories(mesh_zenoh_pool_test
            PRIVATE
                ${CMAKE_SOURCE_DIR}/sce/include
                ${CMAKE_CURRENT_SOURCE_DIR}/mesh
                ${MESH_ZENOH_POOL_OUTPUT_DIR}
        )

        target_link_libraries(mesh_zenoh_pool_test
            PRIVATE
                sce_base
                sce_runtime
                sce_mesh_common
                zenohcxx::zenohc
                pthread
        )

        target_compile_features(mesh_zenoh_pool_test PRIVATE cxx_std_17)
        apply_sce_test_policies(mesh_zenoh_pool_test)

        add_test(NAME mesh_zenoh_pool_send_runtime COMMAND mesh_zenoh_pool_test)
        set_tests_properties(mesh_zenoh_pool_send_runtime PROPERTIES
            TIMEOUT 30
            RESOURCE_LOCK zenoh_pool_send_port_17457
        )
    else()
        message(STATUS "zenohcxx not found — skipping mesh_zenoh_pool_test")
    endif()

    # ── SCE_TEMPLATE_DIR injection for mesh rejection ctests ───────────
    # sce-codegen's `find_template_base` refuses the compile-time
    # CARGO_MANIFEST_DIR fallback in release builds (the only profile
    # cmake/SCECodegen.cmake accepts), so each ctest that fork+execs
    # sce-codegen must supply SCE_TEMPLATE_DIR via set_tests_properties
    # ENVIRONMENT — inheriting the developer's shell export is not a
    # contract ctest can rely on. Tests append their name to
    # SCE_REJECTION_TESTS; a single foreach at the end of the rejection
    # family applies the env so an 18th entry needs only the list append.
    set(SCE_REJECTION_TEST_ENV
        "SCE_TEMPLATE_DIR=${CMAKE_SOURCE_DIR}/tools/codegen/templates")
    set(SCE_REJECTION_TESTS)

    # ── Negative: event coverage violation must fail codegen ────
    # brake_bad.scxml sends "brake.activte" (typo) to #motor. The event
    # has no matching <transition> in motor.scxml, so TopologyError::
    # UncoveredEvents should fire before any code is written.
    #
    # We use a wrapper script rather than WILL_FAIL so the test asserts
    # the *specific* error path fires — a future regression where codegen
    # exits non-zero for an unrelated reason would otherwise masquerade
    # as a passing negative test.
    set(MESH_NEG_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/mesh_neg_generated")
    add_test(
        NAME mesh_event_coverage_rejection
        COMMAND "${MESH_TEST_DIR}/expect_coverage_failure.sh"
                "${SCE_CODEGEN}"
                "${MESH_TEST_DIR}/brake_bad.scxml"
                "${MESH_TEST_DIR}/deploy_bad.yaml"
                "${MESH_NEG_OUTPUT_DIR}"
    )
    list(APPEND SCE_REJECTION_TESTS mesh_event_coverage_rejection)

    # ── Negative: pattern capability violation must fail codegen ──
    # SHM advertises {FireForget, FieldAccess} only — it is a one-way
    # transport with no return channel. brake_shm_request.scxml sends
    # service.request.motor_status to #motor over SHM, which requires
    # RequestReply. Codegen MUST reject with
    # TopologyError::PatternCapabilityViolation BEFORE emitting any code.
    #
    # This test replaced the earlier zenoh rejection case (Session D made
    # zenoh a full RequestReply provider via queryable/query). Keeping the
    # positive coverage live ensures the capability-validation code path
    # doesn't silently die on future refactors.
    set(MESH_PATTERN_NEG_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/mesh_pattern_neg_generated")
    add_test(
        NAME mesh_pattern_capability_rejection
        COMMAND "${MESH_TEST_DIR}/expect_pattern_failure.sh"
                "${SCE_CODEGEN}"
                "${MESH_TEST_DIR}/brake_shm_request.scxml"
                "${MESH_TEST_DIR}/deploy_shm_request.yaml"
                "${MESH_PATTERN_NEG_OUTPUT_DIR}"
    )
    list(APPEND SCE_REJECTION_TESTS mesh_pattern_capability_rejection)

    # ── SCE_MESH §14 partition rule mutation matrix ─────────────────────
    # The deploy.rs (rules 6-10) and partitions.rs (rules 1, 2, 11) unit
    # suites in sce-build cover each rule's Rust-level semantics. These
    # ctests lift that coverage to the CLI + filesystem + JSON-diagnostic
    # layer: each fixture triggers one specific partition rule, and the
    # wrapper asserts a non-zero exit AND an NDJSON record whose `code`
    # field matches the expected slash path. A future regression that
    # silently disables a validator, or that re-routes the diagnostic
    # through a different code, falls out at ctest time.
    #
    # Entries: rule_id | diagnostic slash path | SCXML fixture.
    # The diagnostic codes mirror sce-build/src/forge/diagnostic.rs
    # `as_str` (authoritative) — drift is caught by the wrapper's grep
    # on the exact code string. Rule 11 shares a code with rule 2 by
    # design (same author-facing mistake via a different enumeration
    # path); its regression value comes from the nested-parallel SCXML
    # fixture, which fails back to an exit-zero run if enumerate_units
    # ever stopped recursing past the first `<parallel>`.
    set(MESH_PARTITION_NEG_MATRIX
        "1|mesh/deploy-partition-uncovered-unit|motor_partition_nested.scxml"
        "2|mesh/deploy-partition-partial-coverage-requires-default|motor_partition.scxml"
        # SCE_MESH.md §14 rule 5 — author-declared id carries the reserved
        # `__sce_synth_invoke__` infix with no matching parent stem in
        # `topology.*.machines`. The §9.6.6 rule 3 override carve-out
        # is not triggered because the parent would have to be declared
        # as its own machine for the infix-bearing entry to be admitted.
        "5|mesh/deploy-partition-synth-infix-collision|motor_partition.scxml"
        "6|mesh/deploy-partition-duplicate-name|motor_partition.scxml"
        "7|mesh/deploy-partition-multi-device|motor_partition.scxml"
        "8|mesh/deploy-partition-unit-duplicate|motor_partition.scxml"
        "9|mesh/deploy-partition-machine-not-listed|motor_partition.scxml"
        "10|mesh/deploy-partition-empty|motor_partition.scxml"
        "11|mesh/deploy-partition-partial-coverage-requires-default|motor_partition_nested.scxml"
        # SCE_MESH.md §14 rule 12 — five rejection shapes. 12a/b/c/e use
        # motor_partition.scxml (two regions, distributable). 12d uses
        # the same fixture plus a second `other_machine` entry so the
        # non-host claimant partition can carry a non-empty `contains:`
        # without hosting any region of the claimed parallel.
        "12a|mesh/partition-parallel-root-undesignated|motor_partition.scxml"
        "12b|mesh/partition-parallel-root-ambiguous|motor_partition.scxml"
        "12c|mesh/partition-parallel-root-not-in-machines|motor_partition.scxml"
        "12d|mesh/partition-parallel-root-non-host|motor_partition.scxml"
        "12e|mesh/partition-barrier-timeout-without-root|motor_partition.scxml"
        # §16.5 wire-21 transport implementation gap — custom_tcp
        # transport_binding on a partition that participates in a
        # distributed `<parallel>` route is rejected at deploy time
        # because the wire-21 channel emitter is shm-only.
        "12f|mesh/partition-wire21-custom-tcp-unimplemented|motor_partition.scxml"
    )
    foreach(_entry IN LISTS MESH_PARTITION_NEG_MATRIX)
        string(REPLACE "|" ";" _parts "${_entry}")
        list(GET _parts 0 _rule_id)
        list(GET _parts 1 _expected_code)
        list(GET _parts 2 _scxml)
        file(GLOB _yaml_glob
            "${MESH_TEST_DIR}/partition_rule${_rule_id}_*.yaml"
        )
        list(LENGTH _yaml_glob _yaml_count)
        if(NOT _yaml_count EQUAL 1)
            message(FATAL_ERROR
                "Mesh §14 partition matrix: expected exactly one "
                "partition_rule${_rule_id}_*.yaml fixture, found ${_yaml_count} "
                "(${_yaml_glob}). The wrapper invocation below pairs a rule ID "
                "with a single fixture; rename stale files so the glob resolves."
            )
        endif()
        list(GET _yaml_glob 0 _yaml)
        add_test(
            NAME mesh_partition_rule${_rule_id}_rejection
            COMMAND "${MESH_TEST_DIR}/expect_partition_failure.sh"
                    "${SCE_CODEGEN}"
                    "${MESH_TEST_DIR}/${_scxml}"
                    "${_yaml}"
                    "${CMAKE_CURRENT_BINARY_DIR}/mesh_partition_rule${_rule_id}_generated"
                    "${_expected_code}"
        )
        list(APPEND SCE_REJECTION_TESTS mesh_partition_rule${_rule_id}_rejection)
    endforeach()

    # ── SCE_MESH.md §9.6 L1393 cross-device scxml-remote transport ─────
    # Parent issues `<invoke type="scxml" src="#worker">` where worker
    # is declared on a different device. Without a
    # `machines.parent.bindings["#worker"].transport` entry the Session F
    # wire-14/20 dispatch has no target device, so
    # `validate_scxml_invoke_transport` rejects at deploy time. The
    # Rust-level unit suite in sce-build/src/lib.rs covers all three
    # failure variants (MissingBinding / TransportIncapable /
    # TransportUnwired); this ctest lifts the simplest variant to the
    # CLI + NDJSON layer so a future regression that silently disables
    # the validator or re-routes its diagnostic through a different code
    # fails at ctest time.
    add_test(
        NAME mesh_scxml_invoke_cross_device_missing_rejection
        COMMAND "${MESH_TEST_DIR}/expect_partition_failure.sh"
                "${SCE_CODEGEN}"
                "${MESH_TEST_DIR}/parent_crossdev_missing.scxml"
                "${MESH_TEST_DIR}/scxml_invoke_cross_device_missing_binding.yaml"
                "${CMAKE_CURRENT_BINARY_DIR}/mesh_scxml_invoke_cross_device_missing_generated"
                "mesh/deploy-scxml-invoke-cross-device-transport"
    )
    list(APPEND SCE_REJECTION_TESTS mesh_scxml_invoke_cross_device_missing_rejection)

    # ── deploy.yaml-only middleware switch demo ────────────────────────
    # Proves that the SAME brake.scxml compiles against four different
    # transports with zero source changes — only the deploy.yaml differs.
    # Each variant is verified to produce a transport header containing
    # a transport-specific marker (ensures dispatch actually happened).
    set(MESH_SWITCH_OUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/mesh_middleware_switch")
    add_test(
        NAME mesh_middleware_switch_demo
        COMMAND "${MESH_TEST_DIR}/mesh_middleware_switch_demo.sh"
                "${SCE_CODEGEN}"
                "${MESH_TEST_DIR}/brake.scxml"
                "${MESH_SWITCH_OUT_DIR}"
                "local:${MESH_TEST_DIR}/deploy.yaml:motor_.getPolicy()"
                "shm:${MESH_TEST_DIR}/deploy_shm.yaml:SCE::Mesh::ShmChannel"
                "someip:${MESH_TEST_DIR}/deploy_someip.yaml:vsomeip::application"
                "zenoh:${MESH_TEST_DIR}/deploy_zenoh.yaml:zenoh::Session"
    )
    list(APPEND SCE_REJECTION_TESTS mesh_middleware_switch_demo)

    # Apply SCE_TEMPLATE_DIR to every rejection-family ctest in one pass.
    foreach(_rejection_test IN LISTS SCE_REJECTION_TESTS)
        set_tests_properties(${_rejection_test} PROPERTIES
            ENVIRONMENT "${SCE_REJECTION_TEST_ENV}")
    endforeach()

    # ── SCE Mesh §16.8 distributed conformance harness — seed ──────────
    # §16.8.1 target shape: each W3C IRP test declared `yes` in
    # `tests/w3c_distributed_manifest.yaml` spawns its partitions as OS
    # processes and cross-compares the distributed verdict against the
    # §16.2 weak-equivalence properties. The seed below covers exactly
    # one test (test417 — `done.state.<parallel>` convergence on two
    # orthogonal regions); S5+ sessions extend the macro call list.
    #
    # The macro is scoped to this mesh guard so it never fires in
    # non-mesh builds and so the included CRTP headers can pull in the
    # wire-21 transport + tracker types without adding a conditional
    # compile path to sce_base.
    set(W3C_DIST_SRC_DIR "${CMAKE_CURRENT_SOURCE_DIR}/w3c/dist")
    set(W3C_DIST_OUT_ROOT "${CMAKE_CURRENT_BINARY_DIR}/w3c_dist_generated")

    # sce_generate_distributed_w3c_test(TEST_NUM)
    #
    # Wires one §16.8 seed into the build + ctest graph:
    #   1. Stage `resources/${TEST_NUM}/test${TEST_NUM}.txml` into the
    #      build tree as `.../test${TEST_NUM}/test${TEST_NUM}.scxml`
    #      and copy the seed's deploy.yaml alongside, so sce-codegen's
    #      `source: test${TEST_NUM}.scxml` reference resolves without
    #      an absolute path in the yaml.
    #   2. Run sce-codegen twice with `--partition test${TEST_NUM}_main`
    #      and `--partition test${TEST_NUM}_worker`, emitting per-
    #      partition `test${TEST_NUM}_sm.h`/`_transport.h` into disjoint
    #      subdirs (main/, worker/). Per-partition namespaces are
    #      now baked as inner segments (`SCE::Generated::testXXX::
    #      P_<partition>`), so both headers can be ODR-co-included in
    #      a future single-runner binary. The shm fork+exec shape
    #      below is retained for symmetry with the rule-12 precedent.
    #   3. Build one partition binary per partition from the shared
    #      `w3c_dist_partition_runner.cpp` source with a per-target
    #      `SCE_W3C_DIST_PARTITION_HEADER=` define that selects which
    #      `Test${TEST_NUM}_{Main,Worker}.h` registers into the CRTP
    #      `PartitionRegistry`. Each binary ends up with exactly one
    #      entry in its registry.
    #   4. Build one driver binary from `run_distributed.cpp` with
    #      CMake-baked absolute paths to each partition binary +
    #      deploy_dir + partition names. The driver fork+execs both,
    #      waits, and turns per-partition verdicts into a ctest PASS/FAIL.
    #   5. Register the driver as ctest `w3c_distributed_conformance_testN`
    #      under the `w3c_distributed_conformance` label for S5+'s
    #      `ctest -L w3c_distributed_conformance -j` acid-test sweep.
    function(sce_generate_distributed_w3c_test TEST_NUM)
        set(test_id "${TEST_NUM}")
        set(test_src_dir "${W3C_DIST_SRC_DIR}/test${TEST_NUM}")
        set(test_out_dir "${W3C_DIST_OUT_ROOT}/test${TEST_NUM}")
        set(main_out "${test_out_dir}/main")
        set(worker_out "${test_out_dir}/worker")
        set(staged_scxml "${test_out_dir}/test${TEST_NUM}.scxml")
        set(staged_deploy "${test_out_dir}/deploy.yaml")

        # Stage SCXML into the build tree. The TXML source is the
        # existing W3C IRP resource; no seed-specific SCXML lives in
        # source — keeps the seed honest against the same TXML every
        # other W3C pipeline consumes.
        add_custom_command(
            OUTPUT "${staged_scxml}"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${test_out_dir}"
            COMMAND txml-converter
                    "${CMAKE_SOURCE_DIR}/resources/${TEST_NUM}/test${TEST_NUM}.txml"
                    "${staged_scxml}"
            DEPENDS txml-converter
                    "${CMAKE_SOURCE_DIR}/resources/${TEST_NUM}/test${TEST_NUM}.txml"
            COMMENT "§16.8 seed: staging test${TEST_NUM}.scxml into build tree"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${staged_deploy}"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${test_out_dir}"
            COMMAND ${CMAKE_COMMAND} -E copy
                    "${test_src_dir}/deploy.yaml" "${staged_deploy}"
            DEPENDS "${test_src_dir}/deploy.yaml"
            COMMENT "§16.8 seed: staging test${TEST_NUM} deploy.yaml"
            VERBATIM
        )

        # Per-partition codegen. Two invocations of sce-codegen produce
        # the Root and NonRoot shapes from the same SCXML + deploy.yaml;
        # the `--partition` flag switches `parallel_final.jinja2`
        # between tracker-owning (Root) and remote-send-callback (NonRoot).
        add_custom_command(
            OUTPUT "${main_out}/test${TEST_NUM}_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${main_out}"
            COMMAND "${SCE_CODEGEN}" generate "${staged_scxml}"
                    -l cpp -o "${main_out}"
                    --deploy "${staged_deploy}"
                    --partition "test${TEST_NUM}_main"
                    --write-deps "${main_out}/test${TEST_NUM}_sm.h.d"
            DEPENDS "${staged_scxml}" "${staged_deploy}" "${SCE_CODEGEN}"
            DEPFILE "${main_out}/test${TEST_NUM}_sm.h.d"
            COMMENT "§16.8 seed: codegen test${TEST_NUM}_sm.h main (Root) partition"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${main_out}/test${TEST_NUM}_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${staged_scxml}"
                    -l cpp -o "${main_out}"
                    --deploy "${staged_deploy}"
                    --transport-only
                    --partition "test${TEST_NUM}_main"
                    --write-deps "${main_out}/test${TEST_NUM}_transport.h.d"
            DEPENDS "${staged_scxml}" "${staged_deploy}" "${SCE_CODEGEN}"
                    "${main_out}/test${TEST_NUM}_sm.h"
            DEPFILE "${main_out}/test${TEST_NUM}_transport.h.d"
            COMMENT "§16.8 seed: codegen test${TEST_NUM}_transport.h main (Root) partition"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${worker_out}/test${TEST_NUM}_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${worker_out}"
            COMMAND "${SCE_CODEGEN}" generate "${staged_scxml}"
                    -l cpp -o "${worker_out}"
                    --deploy "${staged_deploy}"
                    --partition "test${TEST_NUM}_worker"
                    --write-deps "${worker_out}/test${TEST_NUM}_sm.h.d"
            DEPENDS "${staged_scxml}" "${staged_deploy}" "${SCE_CODEGEN}"
            DEPFILE "${worker_out}/test${TEST_NUM}_sm.h.d"
            COMMENT "§16.8 seed: codegen test${TEST_NUM}_sm.h worker (NonRoot) partition"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${worker_out}/test${TEST_NUM}_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${staged_scxml}"
                    -l cpp -o "${worker_out}"
                    --deploy "${staged_deploy}"
                    --transport-only
                    --partition "test${TEST_NUM}_worker"
                    --write-deps "${worker_out}/test${TEST_NUM}_transport.h.d"
            DEPENDS "${staged_scxml}" "${staged_deploy}" "${SCE_CODEGEN}"
                    "${worker_out}/test${TEST_NUM}_sm.h"
            DEPFILE "${worker_out}/test${TEST_NUM}_transport.h.d"
            COMMENT "§16.8 seed: codegen test${TEST_NUM}_transport.h worker (NonRoot) partition"
            VERBATIM
        )

        # Partition binaries. Each shares `w3c_dist_partition_runner.cpp`
        # but selects its CRTP header via preprocessor define so exactly
        # one partition is registered per binary.
        set(main_bin "w3c_dist_test${TEST_NUM}_main")
        add_executable(${main_bin}
            "${W3C_DIST_SRC_DIR}/w3c_dist_partition_runner.cpp"
            "${main_out}/test${TEST_NUM}_sm.h"
            "${main_out}/test${TEST_NUM}_transport.h"
        )
        target_include_directories(${main_bin}
            PRIVATE
                "${CMAKE_SOURCE_DIR}/sce/include"
                "${W3C_DIST_SRC_DIR}"
                "${main_out}"
        )
        target_compile_definitions(${main_bin}
            PRIVATE
                "SCE_W3C_DIST_PARTITION_HEADER=\"test${TEST_NUM}/Test${TEST_NUM}_Main.h\""
        )
        target_link_libraries(${main_bin} PRIVATE sce_base sce_mesh_common)
        target_compile_features(${main_bin} PRIVATE cxx_std_17)
        apply_sce_test_policies(${main_bin})

        set(worker_bin "w3c_dist_test${TEST_NUM}_worker")
        add_executable(${worker_bin}
            "${W3C_DIST_SRC_DIR}/w3c_dist_partition_runner.cpp"
            "${worker_out}/test${TEST_NUM}_sm.h"
            "${worker_out}/test${TEST_NUM}_transport.h"
        )
        target_include_directories(${worker_bin}
            PRIVATE
                "${CMAKE_SOURCE_DIR}/sce/include"
                "${W3C_DIST_SRC_DIR}"
                "${worker_out}"
        )
        target_compile_definitions(${worker_bin}
            PRIVATE
                "SCE_W3C_DIST_PARTITION_HEADER=\"test${TEST_NUM}/Test${TEST_NUM}_Worker.h\""
        )
        target_link_libraries(${worker_bin} PRIVATE sce_base sce_mesh_common)
        target_compile_features(${worker_bin} PRIVATE cxx_std_17)
        apply_sce_test_policies(${worker_bin})

        # Driver: per-test binary that fork+execs the partitions. Paths
        # come from `$<TARGET_FILE:>` generator expressions so the
        # driver is location-agnostic and survives `ctest
        # --output-on-failure` from any cwd (matches rule-12 pattern).
        set(driver_bin "w3c_dist_test${TEST_NUM}_driver")
        add_executable(${driver_bin} "${W3C_DIST_SRC_DIR}/run_distributed.cpp")
        target_compile_definitions(${driver_bin}
            PRIVATE
                "SCE_W3C_DIST_TEST_ID=\"${TEST_NUM}\""
                "SCE_W3C_DIST_MAIN_BIN=\"$<TARGET_FILE:${main_bin}>\""
                "SCE_W3C_DIST_MAIN_PARTITION=\"test${TEST_NUM}_main\""
                "SCE_W3C_DIST_WORKER_BIN=\"$<TARGET_FILE:${worker_bin}>\""
                "SCE_W3C_DIST_WORKER_PARTITION=\"test${TEST_NUM}_worker\""
                "SCE_W3C_DIST_DEPLOY_DIR=\"${test_out_dir}\""
        )
        add_dependencies(${driver_bin} ${main_bin} ${worker_bin})
        target_compile_features(${driver_bin} PRIVATE cxx_std_17)
        apply_sce_test_policies(${driver_bin})

        add_test(NAME w3c_distributed_conformance_test${TEST_NUM}
                 COMMAND ${driver_bin})
        set_tests_properties(w3c_distributed_conformance_test${TEST_NUM}
            PROPERTIES LABELS "w3c_distributed_conformance")
    endfunction()

    # §16.8 S4 seed. Per-test-unique partition names (`test${TEST_NUM}_*`)
    # baked into the codegen wire-21 shm channel keep the channel names
    # disjoint across tests, so S5+ additions can run truly in parallel
    # under `ctest -j` without a RESOURCE_LOCK family.
    sce_generate_distributed_w3c_test(417)

    # ── DDS multi-pattern verification ───────────────────────────────
    # Gated on Cyclone DDS being installed, the same way the zenoh and SOME/IP
    # fixtures gate on theirs: a developer without the transport gets a tree
    # that configures rather than a hard failure.
    #
    # Inside the mesh guard, not after it. These fixtures read MESH_TEST_DIR
    # and SCE_MESH_CPP_TEMPLATE and link sce_mesh_common, none of which exist
    # when SCE_ENABLE_MESH is OFF — which is the default. Registered outside
    # the guard they still configured cleanly on a machine with Cyclone
    # installed, and emitted rules whose input path was the empty variable
    # expanded to `/`, so the failure surfaced only at build time as
    # `ninja: error: '/brake_dds_multi.scxml' ... missing`.
    find_package(CycloneDDS QUIET)
    find_package(CycloneDDS-CXX QUIET)
    if(CycloneDDS_FOUND AND CycloneDDS-CXX_FOUND)
        # One IDL type for the whole mesh (sce/idl/sce_mesh.idl): an opaque
        # octet sequence carrying the CBOR MeshEnvelope. Adding an event or a
        # machine changes no IDL, so this target never goes stale against a
        # topology — the divergence from OMG DDS-RPC's interface-per-service.
        idlcxx_generate(TARGET sce_mesh_dds_types
            FILES "${CMAKE_SOURCE_DIR}/sce/idl/sce_mesh.idl"
            WARNINGS no-implicit-extensibility)

        set(MESH_DDS_MULTI_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/mesh_dds_multi_generated")

        foreach(_dds_machine brake_dds_multi motor_dds_multi)
            add_custom_command(
                OUTPUT "${MESH_DDS_MULTI_OUTPUT_DIR}/${_dds_machine}_sm.h"
                COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_DDS_MULTI_OUTPUT_DIR}"
                COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/${_dds_machine}.scxml" -l cpp
                        -o "${MESH_DDS_MULTI_OUTPUT_DIR}"
                        --deploy "${MESH_TEST_DIR}/deploy_dds_multi.yaml"
                        --write-deps "${MESH_DDS_MULTI_OUTPUT_DIR}/${_dds_machine}_sm.h.d"
                DEPENDS "${MESH_TEST_DIR}/${_dds_machine}.scxml" "${MESH_TEST_DIR}/deploy_dds_multi.yaml"
                        "${MESH_TEST_DIR}/brake_dds_multi.scxml" "${MESH_TEST_DIR}/motor_dds_multi.scxml"
                        "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                DEPFILE "${MESH_DDS_MULTI_OUTPUT_DIR}/${_dds_machine}_sm.h.d"
                COMMENT "Mesh dds multi: Generating ${_dds_machine}_sm.h + _transport.h"
                VERBATIM
            )
        endforeach()

        add_executable(mesh_dds_multipattern_test
            mesh/test_mesh_dds_multipattern.cpp
            "${MESH_DDS_MULTI_OUTPUT_DIR}/brake_dds_multi_sm.h"
            "${MESH_DDS_MULTI_OUTPUT_DIR}/motor_dds_multi_sm.h"
        )
        target_include_directories(mesh_dds_multipattern_test
            PRIVATE "${MESH_DDS_MULTI_OUTPUT_DIR}" "${MESH_TEST_DIR}" "${CMAKE_CURRENT_SOURCE_DIR}"
        )
        target_link_libraries(mesh_dds_multipattern_test
            PRIVATE sce_mesh_common sce_base sce_mesh_dds_types CycloneDDS-CXX::ddscxx
        )
        target_compile_features(mesh_dds_multipattern_test PRIVATE cxx_std_17)
        # DDS is the one transport with no compile-only target of its own, so
        # its emitted shape reaches C++17 through this runtime target.
        sce_pin_cxx17(mesh_dds_multipattern_test)
        apply_sce_test_policies(mesh_dds_multipattern_test)
        add_test(NAME mesh_dds_multipattern COMMAND mesh_dds_multipattern_test)
        set_tests_properties(mesh_dds_multipattern PROPERTIES
            TIMEOUT 90
        )

        # ── DDS server response deadline (SCE Mesh §9.5) ──
        # The DDS arm of the deadline matrix. Sibling of the SOME/IP and
        # custom_tcp arms; what it adds is the transport that parks the least
        # state — only the admitted correlation, because the reply rides the
        # paired topic rather than the request — so erase-before-publish IS
        # the one-shot guarantee here and has to be proven, not inferred.
        set(MESH_DDS_SERVER_DEADLINE_OUTPUT_DIR
            "${CMAKE_CURRENT_BINARY_DIR}/mesh_dds_server_deadline_generated")

        foreach(_dds_sd_machine brake_dds_server_deadline motor_dds_server_deadline)
            add_custom_command(
                OUTPUT "${MESH_DDS_SERVER_DEADLINE_OUTPUT_DIR}/${_dds_sd_machine}_sm.h"
                       "${MESH_DDS_SERVER_DEADLINE_OUTPUT_DIR}/${_dds_sd_machine}_transport.h"
                COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_DDS_SERVER_DEADLINE_OUTPUT_DIR}"
                COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/${_dds_sd_machine}.scxml" -l cpp
                        -o "${MESH_DDS_SERVER_DEADLINE_OUTPUT_DIR}"
                        --deploy "${MESH_TEST_DIR}/deploy_dds_server_deadline.yaml"
                        --write-deps "${MESH_DDS_SERVER_DEADLINE_OUTPUT_DIR}/${_dds_sd_machine}_sm.h.d"
                DEPENDS "${MESH_TEST_DIR}/${_dds_sd_machine}.scxml"
                        "${MESH_TEST_DIR}/deploy_dds_server_deadline.yaml"
                        "${MESH_TEST_DIR}/brake_dds_server_deadline.scxml"
                        "${MESH_TEST_DIR}/motor_dds_server_deadline.scxml"
                        "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                DEPFILE "${MESH_DDS_SERVER_DEADLINE_OUTPUT_DIR}/${_dds_sd_machine}_sm.h.d"
                COMMENT "Mesh dds server deadline: Generating ${_dds_sd_machine}_sm.h + _transport.h"
                VERBATIM
            )
        endforeach()

        add_executable(mesh_dds_server_deadline_test
            mesh/test_mesh_dds_server_deadline.cpp
            "${MESH_DDS_SERVER_DEADLINE_OUTPUT_DIR}/brake_dds_server_deadline_sm.h"
            "${MESH_DDS_SERVER_DEADLINE_OUTPUT_DIR}/brake_dds_server_deadline_transport.h"
            "${MESH_DDS_SERVER_DEADLINE_OUTPUT_DIR}/motor_dds_server_deadline_sm.h"
            "${MESH_DDS_SERVER_DEADLINE_OUTPUT_DIR}/motor_dds_server_deadline_transport.h"
        )
        target_include_directories(mesh_dds_server_deadline_test
            PRIVATE "${MESH_DDS_SERVER_DEADLINE_OUTPUT_DIR}" "${MESH_TEST_DIR}" "${CMAKE_CURRENT_SOURCE_DIR}"
        )
        target_link_libraries(mesh_dds_server_deadline_test
            PRIVATE sce_mesh_common sce_base sce_mesh_dds_types CycloneDDS-CXX::ddscxx
        )
        target_compile_features(mesh_dds_server_deadline_test PRIVATE cxx_std_17)
        apply_sce_test_policies(mesh_dds_server_deadline_test)
        add_test(NAME mesh_dds_server_deadline_verification COMMAND mesh_dds_server_deadline_test)
        set_tests_properties(mesh_dds_server_deadline_verification PROPERTIES
            TIMEOUT 90
        )

        # ── DDS machine-lifetime subscription (SCE Mesh §13) ──
        # The DDS arm of the machine-lifetime axis, sibling of
        # mesh_tcp_machine_lifetime_subscribe_runtime. What it adds over the
        # custom_tcp arm is the teardown half: on DDS a subscription is a
        # notification reader the router destroys while the participant stays
        # up, so a publish issued after `shutdown()` is still a real publish
        # and whether it arrives is a fact about the reader. A control
        # subscriber that is never shut down keeps the negative attributable.
        set(MESH_DDS_MACHINE_LIFETIME_OUTPUT_DIR
            "${CMAKE_CURRENT_BINARY_DIR}/mesh_dds_machine_lifetime_generated")

        foreach(_dds_mls_machine brake_dds_machine_lifetime_subscribe motor_dds_machine_lifetime_subscribe)
            add_custom_command(
                OUTPUT "${MESH_DDS_MACHINE_LIFETIME_OUTPUT_DIR}/${_dds_mls_machine}_sm.h"
                       "${MESH_DDS_MACHINE_LIFETIME_OUTPUT_DIR}/${_dds_mls_machine}_transport.h"
                COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_DDS_MACHINE_LIFETIME_OUTPUT_DIR}"
                COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/${_dds_mls_machine}.scxml" -l cpp
                        -o "${MESH_DDS_MACHINE_LIFETIME_OUTPUT_DIR}"
                        --deploy "${MESH_TEST_DIR}/deploy_dds_machine_lifetime_subscribe.yaml"
                        --write-deps "${MESH_DDS_MACHINE_LIFETIME_OUTPUT_DIR}/${_dds_mls_machine}_sm.h.d"
                DEPENDS "${MESH_TEST_DIR}/${_dds_mls_machine}.scxml"
                        "${MESH_TEST_DIR}/deploy_dds_machine_lifetime_subscribe.yaml"
                        "${MESH_TEST_DIR}/brake_dds_machine_lifetime_subscribe.scxml"
                        "${MESH_TEST_DIR}/motor_dds_machine_lifetime_subscribe.scxml"
                        "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                DEPFILE "${MESH_DDS_MACHINE_LIFETIME_OUTPUT_DIR}/${_dds_mls_machine}_sm.h.d"
                COMMENT "Mesh dds machine-lifetime: Generating ${_dds_mls_machine}_sm.h + _transport.h"
                VERBATIM
            )
        endforeach()

        add_executable(mesh_dds_machine_lifetime_subscribe_test
            mesh/test_mesh_dds_machine_lifetime_subscribe.cpp
            "${MESH_DDS_MACHINE_LIFETIME_OUTPUT_DIR}/brake_dds_machine_lifetime_subscribe_sm.h"
            "${MESH_DDS_MACHINE_LIFETIME_OUTPUT_DIR}/brake_dds_machine_lifetime_subscribe_transport.h"
            "${MESH_DDS_MACHINE_LIFETIME_OUTPUT_DIR}/motor_dds_machine_lifetime_subscribe_sm.h"
            "${MESH_DDS_MACHINE_LIFETIME_OUTPUT_DIR}/motor_dds_machine_lifetime_subscribe_transport.h"
        )
        target_include_directories(mesh_dds_machine_lifetime_subscribe_test
            PRIVATE "${MESH_DDS_MACHINE_LIFETIME_OUTPUT_DIR}" "${MESH_TEST_DIR}" "${CMAKE_CURRENT_SOURCE_DIR}"
        )
        target_link_libraries(mesh_dds_machine_lifetime_subscribe_test
            PRIVATE sce_mesh_common sce_base sce_mesh_dds_types CycloneDDS-CXX::ddscxx
        )
        target_compile_features(mesh_dds_machine_lifetime_subscribe_test PRIVATE cxx_std_17)
        apply_sce_test_policies(mesh_dds_machine_lifetime_subscribe_test)
        add_test(NAME mesh_dds_machine_lifetime_subscribe_runtime
                 COMMAND mesh_dds_machine_lifetime_subscribe_test)
        set_tests_properties(mesh_dds_machine_lifetime_subscribe_runtime PROPERTIES
            TIMEOUT 90
        )

        # ── DDS bounded pool dispatch (SCE Mesh §14.4) ──
        # The runtime half of the DDS pool arm. The §14.4 compile fixture
        # (deploy_pool.yaml) proves the three carriers render together; this
        # one proves the runtime `<param>` value selects WHICH declared
        # member endpoint a send lands on, and that a value outside the
        # declared set is refused rather than dispatched through an endpoint
        # that was never built.
        #
        # The observers are raw `Dds::Server`s rather than generated routers
        # because a generated router binds one topic and the claim is about
        # telling two apart.
        set(MESH_DDS_POOL_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/mesh_dds_pool_generated")

        add_custom_command(
            OUTPUT "${MESH_DDS_POOL_OUTPUT_DIR}/pool_sender_dds_sm.h"
                   "${MESH_DDS_POOL_OUTPUT_DIR}/pool_sender_dds_transport.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_DDS_POOL_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/pool_sender_dds.scxml" -l cpp
                    -o "${MESH_DDS_POOL_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_dds_pool.yaml"
                    --write-deps "${MESH_DDS_POOL_OUTPUT_DIR}/pool_sender_dds_sm.h.d"
            DEPENDS "${MESH_TEST_DIR}/pool_sender_dds.scxml"
                    "${MESH_TEST_DIR}/deploy_dds_pool.yaml"
                    "${MESH_TEST_DIR}/sensor_dds_pool.scxml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
            DEPFILE "${MESH_DDS_POOL_OUTPUT_DIR}/pool_sender_dds_sm.h.d"
            COMMENT "Mesh dds pool: Generating pool_sender_dds_sm.h + _transport.h"
            VERBATIM
        )

        add_executable(mesh_dds_pool_test
            mesh/test_mesh_dds_pool.cpp
            "${MESH_DDS_POOL_OUTPUT_DIR}/pool_sender_dds_sm.h"
            "${MESH_DDS_POOL_OUTPUT_DIR}/pool_sender_dds_transport.h"
        )
        target_include_directories(mesh_dds_pool_test
            PRIVATE "${MESH_DDS_POOL_OUTPUT_DIR}" "${MESH_TEST_DIR}" "${CMAKE_CURRENT_SOURCE_DIR}"
        )
        target_link_libraries(mesh_dds_pool_test
            PRIVATE sce_mesh_common sce_base sce_mesh_dds_types CycloneDDS-CXX::ddscxx
        )
        target_compile_features(mesh_dds_pool_test PRIVATE cxx_std_17)
        apply_sce_test_policies(mesh_dds_pool_test)
        add_test(NAME mesh_dds_pool_runtime COMMAND mesh_dds_pool_test)
        set_tests_properties(mesh_dds_pool_runtime PROPERTIES
            TIMEOUT 90
        )
    endif()
endif()

# ============================================================================
# Note: Code generation handled by sce-codegen (Rust binary from sce-build crate)
# Integration tests run via StaticCodegenIntegrationTest (in integration_tests)
# ============================================================================

# ============================================================================
# CTest execution order optimization (fast feedback → comprehensive validation → performance measurement)
# ============================================================================

# Priority Level 1: Fast unit tests (quick feedback during development, <1s)
add_test(NAME CoreParserTests COMMAND core_parser_tests)
add_test(NAME StatesTests COMMAND states_tests)
add_test(NAME HistoryTests COMMAND history_tests)
add_test(NAME HttpBridgeTests COMMAND http_bridge_tests)
add_test(NAME SessionTargetRoutingTests COMMAND session_target_routing_tests)

# Priority Level 2: Core component tests (basic functionality validation, ~1-5s)
add_test(NAME StateMachineTests COMMAND state_machine_tests)
add_test(NAME JSEngineTests COMMAND js_engine_tests)
add_test(NAME ActionTests COMMAND action_tests)
add_test(NAME SCESCXMLTests COMMAND sce_scxml_tests)
add_test(NAME ReadySCXMLEngineTests COMMAND ready_scxml_engine_tests)
add_test(NAME TimerManagerTests COMMAND timer_manager_tests)
add_test(NAME GameLoopTimerTests COMMAND game_loop_timer_tests)
add_test(NAME ComprehensiveInteractiveTests COMMAND comprehensive_interactive_test)

# Priority Level 3: Integration and concurrency tests (system-level validation, ~5-15s)
add_test(NAME TXMLConverterTests COMMAND txml_converter_tests)
add_test(NAME ConcurrencyTests COMMAND concurrency_tests)
add_test(NAME IntegrationTests COMMAND integration_tests)

# Priority Level 4: W3C SCXML compliance validation (comprehensive validation, ~40s)
# WASM: Uses pre-generated AOT test headers (HTTP tests excluded)
# Native: Full test suite including HTTP tests with txml-converter
if(EMSCRIPTEN)
    add_test(NAME W3CFullComplianceTests COMMAND node w3c_test_cli.js)
else()
    add_test(NAME W3CFullComplianceTests COMMAND w3c_test_cli)
endif()

# ============================================================================
# CTest execution order control: Set priority using COST property
# CTest executes tests with higher COST values first,
# so assign higher values to main tests than benchmarks (COST=1000) to ensure proper order
# ============================================================================

# Priority Level 1: COST 5000 (execute first)
set_tests_properties(CoreParserTests PROPERTIES COST 5000)
set_tests_properties(StatesTests PROPERTIES COST 5000)
set_tests_properties(HistoryTests PROPERTIES COST 5000)

# Priority Level 2: COST 4000
set_tests_properties(StateMachineTests PROPERTIES COST 4000)
set_tests_properties(JSEngineTests PROPERTIES COST 4000)
set_tests_properties(ActionTests PROPERTIES COST 4000)
set_tests_properties(SCESCXMLTests PROPERTIES COST 4000)
set_tests_properties(ComprehensiveInteractiveTests PROPERTIES COST 4000)

# Priority Level 3: COST 3000
set_tests_properties(TXMLConverterTests PROPERTIES COST 3000)
set_tests_properties(ConcurrencyTests PROPERTIES COST 3000)
set_tests_properties(IntegrationTests PROPERTIES COST 3000)

# Priority Level 4: COST 2500 (W3C SCXML compliance validation, higher priority than benchmarks)
set_tests_properties(W3CFullComplianceTests PROPERTIES COST 2500)

# SSOT byte-equivalence harness — Rust vs C++ `sce:template`
# expansion parity. Skipped on WASM because the harness spawns
# `sce-codegen expand` as a subprocess via popen, which is not
# available in the Emscripten runtime.
if(NOT EMSCRIPTEN)
    add_subdirectory(w3c_template_parity)
endif()

# Parsing unit-test family (PositionMap, expanders, the diagnostic
# wire surface). Runs everywhere except the cross-producer parity
# suite, which spawns `sce-codegen` and guards itself on EMSCRIPTEN.
add_subdirectory(parsing)

# Priority Level 5: Performance benchmarks (measure performance after functional validation, execute last)
# Benchmark subdirectory must be included AFTER all main tests to ensure proper execution order
# WASM builds: Benchmarks excluded (Native-only performance measurement)
option(BUILD_BENCHMARKS "Build performance benchmarks" ON)
if(BUILD_BENCHMARKS AND NOT EMSCRIPTEN)
    message(STATUS "Building performance benchmarks")
    add_subdirectory(benchmarks)
endif()

# ============================================================================
# Test dependency and priority configuration
# ============================================================================

# Advanced tests run only after core tests pass
set_tests_properties(ActionTests PROPERTIES DEPENDS "JSEngineTests;StateMachineTests")
set_tests_properties(HistoryTests PROPERTIES DEPENDS "StateMachineTests")
set_tests_properties(StatesTests PROPERTIES DEPENDS "StateMachineTests")

# Integration tests run after core components pass
set_tests_properties(IntegrationTests PROPERTIES DEPENDS "JSEngineTests;StateMachineTests;ActionTests")
set_tests_properties(ConcurrencyTests PROPERTIES DEPENDS "StateMachineTests;StatesTests")

# W3C tests run after all basic functionality is validated
set_tests_properties(TXMLConverterTests PROPERTIES DEPENDS "CoreParserTests")
set_tests_properties(W3CFullComplianceTests PROPERTIES DEPENDS "IntegrationTests;TXMLConverterTests")

# Test timeout configuration (in seconds)
set_tests_properties(SCESCXMLTests PROPERTIES TIMEOUT 30)
set_tests_properties(ReadySCXMLEngineTests PROPERTIES TIMEOUT 30)
set_tests_properties(TimerManagerTests PROPERTIES TIMEOUT 60)
set_tests_properties(GameLoopTimerTests PROPERTIES TIMEOUT 30)
set_tests_properties(JSEngineTests CoreParserTests StateMachineTests PROPERTIES TIMEOUT 60)
set_tests_properties(ActionTests HistoryTests StatesTests PROPERTIES TIMEOUT 90)
set_tests_properties(IntegrationTests ConcurrencyTests PROPERTIES TIMEOUT 180)
set_tests_properties(TXMLConverterTests PROPERTIES TIMEOUT 120)
set_tests_properties(W3CFullComplianceTests PROPERTIES TIMEOUT 300)
set_tests_properties(ComprehensiveInteractiveTests PROPERTIES TIMEOUT 600)

# Limit parallel execution (resource-intensive tests)
set_tests_properties(W3CFullComplianceTests PROPERTIES RUN_SERIAL TRUE)
set_tests_properties(ConcurrencyTests PROPERTIES RUN_SERIAL TRUE)

# Apply SCE policy to all test targets
foreach(test_target ${SCE_TEST_TARGETS})
    if(TARGET ${test_target})
        apply_sce_test_policies(${test_target})
        message(STATUS "✅ Applied SCE policies (cout prohibition) to ${test_target}")
    endif()
endforeach()

message(STATUS "✅ SCE SCXML tests enabled")
message(STATUS "✅ JavaScript Engine tests enabled")
message(STATUS "✅ Core Parser tests enabled")
message(STATUS "✅ Integration tests enabled")
message(STATUS "🚫 std::cout usage prohibited in all test targets")

# ============================================================================
# Interactive SCXML Visualizer (WASM-only)
# ============================================================================
if(EMSCRIPTEN)
    message(STATUS "🎨 Building Interactive SCXML Visualizer for WASM")

    # Visualizer WASM module
    add_executable(visualizer
        w3c/wasm_bindings.cpp
        w3c/InteractiveTestRunner.cpp
        ${CMAKE_SOURCE_DIR}/sce/src/runtime/StateSnapshot.cpp
    )

    target_include_directories(visualizer PRIVATE
        ${CMAKE_SOURCE_DIR}/sce/include
        ${CMAKE_SOURCE_DIR}/sce/src
        ${CMAKE_SOURCE_DIR}/third_party/quickjs
        ${CMAKE_CURRENT_SOURCE_DIR}/w3c
    )

    target_link_libraries(visualizer
        PRIVATE
            sce_runtime
            spdlog::spdlog
    )

    # WASM-specific compile flags (NO pthread for browser compatibility)
    # Removed pthread to avoid SharedArrayBuffer/Cross-Origin-Isolation requirement

    # WASM-specific link flags for visualizer
    target_link_options(visualizer PRIVATE
        # Emscripten bindings
        --bind

        # Module configuration
        -sMODULARIZE=1
        -sEXPORT_NAME=createVisualizer

        # Memory configuration (browser-friendly, no file embedding)
        -sALLOW_MEMORY_GROWTH=0
        -sINITIAL_MEMORY=67108864  # 64MB initial (browser max ~2GB, grows as needed)

        # Exported functions and runtime methods
        "-sEXPORTED_FUNCTIONS=[_malloc,_free]"
        "-sEXPORTED_RUNTIME_METHODS=[cwrap,ccall,UTF8ToString,FS]"

        # File system (for fetch API)
        -sFORCE_FILESYSTEM=1

        # Fetch API (for HTTP requests)
        -sFETCH=1

        # Environment (browser and Node.js)
        -sENVIRONMENT=web,node

        # Exception handling
        -sDISABLE_EXCEPTION_CATCHING=0

        # Assertions for debugging
        -sASSERTIONS=1

        # Asyncify (for async operations)
        -sASYNCIFY
    )

    # Auto-deploy WASM files to web/visualizer/ directory after build
    add_custom_command(TARGET visualizer POST_BUILD
        COMMAND ${CMAKE_COMMAND} -E copy_if_different
                ${CMAKE_CURRENT_BINARY_DIR}/visualizer.wasm
                ${CMAKE_SOURCE_DIR}/web/visualizer/visualizer.wasm
        COMMAND ${CMAKE_COMMAND} -E copy_if_different
                ${CMAKE_CURRENT_BINARY_DIR}/visualizer.js
                ${CMAKE_SOURCE_DIR}/web/visualizer/visualizer.js
        COMMENT "Auto-deploying WASM files to web/visualizer/"
        VERBATIM
    )
    message(STATUS "   - WASM files will be automatically deployed after build")

    message(STATUS "✅ Interactive SCXML Visualizer configured")
    message(STATUS "   - Output: visualizer.wasm + visualizer.js")
    message(STATUS "   - SCXML loading: Dynamic fetch (no embedding)")
    message(STATUS "   - JavaScript API: createVisualizer() module")
endif()

# ----------------------------------------------------------------------------
# Install-tree consumer smoke test (opt-in).
#
# Not part of the regular ctest run — it performs an `install` to a scratch
# prefix and then configures/builds three standalone probe projects against
# that prefix via find_package(SCE COMPONENTS base|scripting|runtime). Opt
# in with `-DSCE_BUILD_INSTALL_CONSUMER_TEST=ON`, then
# `ctest -L install-consumer` runs the end-to-end flow. Keeping it gated
# avoids running the heavy install pipeline on every developer ctest.
# ----------------------------------------------------------------------------
option(SCE_BUILD_INSTALL_CONSUMER_TEST
    "Add tests/install_tree_consumer smoke test (ctest label install-consumer)"
    OFF)

if(SCE_BUILD_INSTALL_CONSUMER_TEST)
    add_test(
        NAME install_tree_consumer
        COMMAND
            "${CMAKE_CURRENT_SOURCE_DIR}/install_tree_consumer/run.sh"
            "${CMAKE_SOURCE_DIR}"
            "${CMAKE_BINARY_DIR}"
    )
    set_tests_properties(install_tree_consumer PROPERTIES
        LABELS "install-consumer"
        # Install produces a scratch prefix under $TMPDIR; run.sh cleans up
        # via `trap`, but avoid interleaving with other tests writing to /tmp.
        RUN_SERIAL TRUE
    )
endif()

# ----------------------------------------------------------------------------
# Staleness guard: no test may run against a binary older than its sources.
#
# CTest has no build dependency — `ctest` (and `ninja test`) run whatever
# executables already exist. A tree edited after the last build therefore
# reports the PREVIOUS build's verdict, which reads as a pass. That failure
# mode is silent and directional: it only ever turns red into green.
#
# The fix is a setup fixture that rebuilds, plus a blanket
# FIXTURES_REQUIRED applied to every test registered in this directory.
# Two properties make it hard to bypass:
#
#   * `-R` / `-L` filters do not exclude a required fixture — a fixture is
#     pulled in by dependency, not by name match, so `ctest -R one_test`
#     builds first as well.
#   * when the build fails the dependent tests are reported NOT RUN rather
#     than passed, so a broken tree cannot produce a green summary.
#
# The blanket application is deliberate: enumerating targets by hand would
# make coverage depend on remembering to opt each new test in, which is the
# same class of mistake this guard exists to remove. A no-op rebuild costs
# about a second.
#
# Not covered: running a test executable directly (`./tests/foo_test`)
# bypasses CTest entirely. That is a deliberate scope limit — pinning a
# source digest into every binary is disproportionate, and the direct-run
# case is normally "the target I just built", not the stale-sibling case
# this guard addresses.
add_test(NAME sce_build_is_current
    COMMAND "${CMAKE_COMMAND}" --build "${CMAKE_BINARY_DIR}" --parallel
)
set_tests_properties(sce_build_is_current PROPERTIES
    FIXTURES_SETUP sce_built
    # A cold or heavily-invalidated tree can take a while; the steady state
    # is a no-op ninja.
    TIMEOUT 3000
    # Serial so parallel ctest does not start N concurrent builds.
    RUN_SERIAL TRUE
)

get_property(_sce_registered_tests DIRECTORY PROPERTY TESTS)
list(REMOVE_ITEM _sce_registered_tests sce_build_is_current)
if(_sce_registered_tests)
    set_tests_properties(${_sce_registered_tests} PROPERTIES
        FIXTURES_REQUIRED sce_built
    )
endif()

# ============================================================================
# C++17 ceiling coverage gate
# ============================================================================
# A compile-only mesh target exists to prove generated code builds at the
# standard the runtime advertises. Adding one without `sce_pin_cxx17` leaves
# it compiling at the global `CMAKE_CXX_STANDARD`, where it proves nothing —
# and does so silently, because the target still builds and its declared
# `cxx_std_17` still reads as a promise. That is how a C++20-only
# `std::string_view::starts_with` reached generated output and stayed there.
#
# Configuration is the right place to fail: the answer is known before a
# compiler runs, and a missing ceiling is a gap in verification rather than
# a broken build.
get_directory_property(_sce_dir_targets BUILDSYSTEM_TARGETS)
foreach(_sce_t IN LISTS _sce_dir_targets)
    if(_sce_t MATCHES "^mesh_.*compile_test$")
        get_target_property(_sce_std ${_sce_t} CXX_STANDARD)
        if(NOT _sce_std STREQUAL "17")
            message(FATAL_ERROR
                "${_sce_t} is a compile-only mesh target but does not pin the "
                "standard (CXX_STANDARD='${_sce_std}'). Call "
                "sce_pin_cxx17(${_sce_t}) after its target_compile_features(); "
                "without it the target compiles at the global standard and "
                "verifies nothing about the C++17 contract it declares.")
        endif()
    endif()
endforeach()
