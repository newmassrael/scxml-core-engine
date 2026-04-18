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
# SCE test configuration: Disable std::cout usage
# ============================================================================
function(apply_sce_test_policies target_name)
    # Force include DisableStdOut.h to all test targets
    target_compile_options(${target_name}
        PRIVATE
            -include ${CMAKE_SOURCE_DIR}/sce/include/common/DisableStdOut.h
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
    concurrency/TypeRegistryThreadSafetyTest.cpp
    common/EventDataHelperTest.cpp
    common/UuidTest.cpp
)

target_include_directories(sce_scxml_tests
    PRIVATE
        ${CMAKE_SOURCE_DIR}
        ${CMAKE_SOURCE_DIR}/sce/include
        ${CMAKE_SOURCE_DIR}/tests
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

# Generate C++ code for W3C tests BEFORE defining executables
include(${CMAKE_SOURCE_DIR}/cmake/SCEStaticW3CTest.cmake)

# Initialize GENERATED_W3C_HEADERS and W3C_AOT_TESTS before any sce_generate_static_w3c_test calls
# W3C Test Registration Automation: W3C_AOT_TESTS is auto-accumulated by sce_generate_static_w3c_test()
# Adding a new test = adding ONE sce_generate_static_w3c_test() call (TestXXX.h + registry auto-generated)
set(GENERATED_W3C_HEADERS "")
set(W3C_AOT_TESTS "")

sce_generate_static_w3c_test(144 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 4.2: raise element inserts events at rear of internal queue (FIFO ordering) (Pure Static)
sce_generate_static_w3c_test(147 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 4.3: if/elseif/else conditional execution (first true clause only) (Static Hybrid)
sce_generate_static_w3c_test(148 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 4.3: else clause execution when if/elseif both false (Static Hybrid)
sce_generate_static_w3c_test(149 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 4.3: if/elseif/else - neither clause executes (Static Hybrid)
sce_generate_static_w3c_test(150 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 4.6: foreach declares new variable if item doesn't exist (Static Hybrid)
sce_generate_static_w3c_test(151 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 4.6: foreach variable declaration persistence (Static Hybrid)
sce_generate_static_w3c_test(152 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 4.6: foreach error handling - illegal array/item raises error.execution (Static Hybrid)
sce_generate_static_w3c_test(153 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 4.6: foreach iteration order correctness (Static Hybrid)
sce_generate_static_w3c_test(155 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 4.6: foreach executes content once per item (Static Hybrid)
sce_generate_static_w3c_test(156 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 4.6: foreach stops execution on child action error (Static Hybrid)
sce_generate_static_w3c_test(158 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.12.1: executable content executes in document order (Pure Static)
sce_generate_static_w3c_test(159 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.12.1: error causes subsequent elements to be skipped (Static Hybrid)
sce_generate_static_w3c_test(172 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.2.2: send eventexpr uses current value at evaluation time (Static Hybrid)
sce_generate_static_w3c_test(173 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.2.3: send targetexpr uses current value at evaluation time (Static Hybrid)
sce_generate_static_w3c_test(174 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.2.4: send typeexpr uses current value at evaluation time (Static Hybrid)
sce_generate_static_w3c_test(175 ${STATIC_W3C_OUTPUT_DIR} TYPE SCHEDULED)  # W3C SCXML 6.2.5: send delayexpr uses current value at evaluation time (Static Hybrid)
sce_generate_static_w3c_test(176 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.2.1: param uses current value at evaluation time (Static Hybrid)
sce_generate_static_w3c_test(178 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.2.1: param - multiple key/value pairs with same keys (Static Hybrid)
sce_generate_static_w3c_test(179 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.2.6: content element populates message body (Pure Static)
sce_generate_static_w3c_test(183 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.2.7: send idlocation stores sendid value (Static Hybrid)
sce_generate_static_w3c_test(185 ${STATIC_W3C_OUTPUT_DIR} TYPE SCHEDULED)  # W3C SCXML 6.2.5: send respects delay specification (Pure Static)
sce_generate_static_w3c_test(186 ${STATIC_W3C_OUTPUT_DIR} TYPE SCHEDULED)  # W3C SCXML 6.2: send evaluates args at evaluation time, not send time (Static Hybrid)
sce_generate_static_w3c_test(187 ${STATIC_W3C_OUTPUT_DIR} TYPE SCHEDULED)  # W3C SCXML 6.4.5: Delayed send cancellation on session termination (Pure Static)
sce_generate_static_w3c_test(189 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10.1: Internal queue priority via target="#_internal" (Pure Static)
sce_generate_static_w3c_test(190 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10.1: External queue priority via targetexpr (Static Hybrid)
sce_generate_static_w3c_test(191 ${STATIC_W3C_OUTPUT_DIR} TYPE SCHEDULED)  # W3C SCXML 6.4.1: Inline <content> invoke with #_parent target (Pure Static)
sce_generate_static_w3c_test(192 ${STATIC_W3C_OUTPUT_DIR} TYPE SCHEDULED)  # W3C SCXML 6.4: Inline <content> invoke with #_<invokeid> target (Pure Static)
sce_generate_static_w3c_test(193 ${STATIC_W3C_OUTPUT_DIR})  # type attribute for queue routing (W3C SCXML 6.2.4)
sce_generate_static_w3c_test(194 ${STATIC_W3C_OUTPUT_DIR})  # invalid target raises error.execution (W3C SCXML 6.2)
sce_generate_static_w3c_test(198 ${STATIC_W3C_OUTPUT_DIR})  # default event processor type (W3C SCXML 6.2)
sce_generate_static_w3c_test(199 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.2: unsupported send type raises error.execution (Pure Static)
sce_generate_static_w3c_test(200 ${STATIC_W3C_OUTPUT_DIR})  # SCXML event processor support (W3C SCXML 6.2)
sce_generate_static_w3c_test(201 ${STATIC_W3C_OUTPUT_DIR} TYPE HTTP)  # BasicHTTP event processor (optional - W3C SCXML 6.2)
sce_generate_static_w3c_test(205 ${STATIC_W3C_OUTPUT_DIR})  # inline <content>
sce_generate_static_w3c_test(207 ${STATIC_W3C_OUTPUT_DIR} TYPE SCHEDULED)  # W3C SCXML 6.3: cancel tag - cannot cancel events in different sessions (invoke + delayed send)
sce_generate_static_w3c_test(208 ${STATIC_W3C_OUTPUT_DIR} TYPE SCHEDULED)  # <cancel sendid="..."/> for delayed send cancellation (W3C SCXML 6.3)
sce_generate_static_w3c_test(210 ${STATIC_W3C_OUTPUT_DIR} TYPE SCHEDULED)  # W3C SCXML 6.3: sendidexpr for dynamic cancel (Static Hybrid)
sce_generate_static_w3c_test(215 ${STATIC_W3C_OUTPUT_DIR})  # typeexpr + inline <content>
sce_generate_static_w3c_test(216 ${STATIC_W3C_OUTPUT_DIR} TYPE SCHEDULED)  # W3C SCXML 6.4: srcexpr runtime evaluation (Static Hybrid)
sce_generate_static_w3c_test(220 ${STATIC_W3C_OUTPUT_DIR})  # inline <content>
sce_generate_static_w3c_test(223 ${STATIC_W3C_OUTPUT_DIR})  # inline <content>
sce_generate_static_w3c_test(224 ${STATIC_W3C_OUTPUT_DIR} TYPE SCHEDULED)  # W3C SCXML 6.3.1: invoke with idlocation (Static Hybrid)
sce_generate_static_w3c_test(225 ${STATIC_W3C_OUTPUT_DIR})  # inline <content>
sce_generate_static_w3c_test(226 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.2/6.4: auto-discovers test226sub1.txml
sce_generate_static_w3c_test(228 ${STATIC_W3C_OUTPUT_DIR} TYPE SCHEDULED)  # W3C SCXML 6.3.1: invoke ID in done.invoke event (Static Hybrid)
sce_generate_static_w3c_test(229 ${STATIC_W3C_OUTPUT_DIR} TYPE SCHEDULED)  # W3C SCXML 6.4.6: autoforward with inline content (Pure Static)
sce_generate_static_w3c_test(230 ${STATIC_W3C_OUTPUT_DIR} TYPE SCHEDULED)  # inline <content> + autoforward (manual)
sce_generate_static_w3c_test(232 ${STATIC_W3C_OUTPUT_DIR})  # inline <content>
sce_generate_static_w3c_test(233 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.5: finalize before done.invoke event processing (Static Hybrid)
sce_generate_static_w3c_test(234 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.4.6: finalize only in invoking state (Static Hybrid)
sce_generate_static_w3c_test(235 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.4: done.invoke.id event with inline content child (Pure Static)
sce_generate_static_w3c_test(236 ${STATIC_W3C_OUTPUT_DIR} TYPE SCHEDULED)  # inline <content>
sce_generate_static_w3c_test(237 ${STATIC_W3C_OUTPUT_DIR} TYPE SCHEDULED)  # W3C SCXML 6.4: invoke cancellation on state exit with inline content child
sce_generate_static_w3c_test(239 ${STATIC_W3C_OUTPUT_DIR} TYPE SCHEDULED)  # W3C SCXML 6.4: invoke with src attribute and inline content (Pure Static)
sce_generate_static_w3c_test(240 ${STATIC_W3C_OUTPUT_DIR} TYPE SCHEDULED)  # W3C SCXML 6.4: invoke with namelist and param (Static Hybrid)
sce_generate_static_w3c_test(241 ${STATIC_W3C_OUTPUT_DIR} TYPE SCHEDULED)  # W3C SCXML 6.4.3: invoke namelist + param consistency validation (Static Hybrid)
sce_generate_static_w3c_test(242 ${STATIC_W3C_OUTPUT_DIR} TYPE SCHEDULED)  # W3C SCXML 6.4: invoke src + inline content consistency (Pure Static)
sce_generate_static_w3c_test(243 ${STATIC_W3C_OUTPUT_DIR} TYPE SCHEDULED)  # W3C SCXML 6.4: invoke inline content + param with child→parent event sending (Pure Static)
sce_generate_static_w3c_test(244 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.3.2: invoke inline content + namelist datamodel value passing (Static Hybrid)
sce_generate_static_w3c_test(245 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.3.2: invoke namelist with non-existent variable handling (Static Hybrid)
sce_generate_static_w3c_test(247 ${STATIC_W3C_OUTPUT_DIR})  # inline <content> + done.invoke
sce_generate_static_w3c_test(250 ${STATIC_W3C_OUTPUT_DIR} TYPE SCHEDULED)  # inline <content> + cancellation (manual test)
sce_generate_static_w3c_test(252 ${STATIC_W3C_OUTPUT_DIR} TYPE SCHEDULED)  # W3C SCXML 6.4: invoke cancellation ignores events from cancelled child (Pure Static)
sce_generate_static_w3c_test(253 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML C.1: SCXML Event I/O Processor bidirectional communication with inline content child (Static Hybrid)
sce_generate_static_w3c_test(276 ${STATIC_W3C_OUTPUT_DIR} TYPE SCHEDULED)  # W3C SCXML 6.2/6.4: auto-discovers test276sub1.txml
sce_generate_static_w3c_test(277 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.3: datamodel init error.execution
sce_generate_static_w3c_test(278 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10: datamodel in non-entered state (requires wrapper)
sce_generate_static_w3c_test(279 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.2.2: early binding variable initialization
sce_generate_static_w3c_test(280 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.3: late binding variable initialization
sce_generate_static_w3c_test(286 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.3: error.execution for invalid assignment location
sce_generate_static_w3c_test(287 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.4: valid assignment to valid location
sce_generate_static_w3c_test(294 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.7.2: donedata with param and content (Static Hybrid)
sce_generate_static_w3c_test(298 ${STATIC_W3C_OUTPUT_DIR})  # dynamic donedata (invalid param location requires runtime evaluation)
sce_generate_static_w3c_test(301 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.8: script element download timeout rejection (manual)
sce_generate_static_w3c_test(302 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.8: script evaluation at load time
sce_generate_static_w3c_test(303 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.9: script execution in entry actions
sce_generate_static_w3c_test(304 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.8: script-declared variables accessible in data model
sce_generate_static_w3c_test(307 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.2.2: late binding variable access error handling (Static Hybrid)
sce_generate_static_w3c_test(309 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.9.2: invalid boolean expressions treated as false
sce_generate_static_w3c_test(310 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.9.1: In() predicate (requires Interpreter wrapper)
sce_generate_static_w3c_test(311 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.9.2: error.execution for invalid assign location
sce_generate_static_w3c_test(312 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.9.2: assignment error handling
sce_generate_static_w3c_test(313 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.9.2: assign illegal expression error.execution
sce_generate_static_w3c_test(314 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.9.2: delayed evaluation of assign illegal expression
sce_generate_static_w3c_test(318 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10: _event variable binding and persistence
sce_generate_static_w3c_test(319 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML B.1: _event system variable initially unbound
sce_generate_static_w3c_test(321 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10.1: _sessionid system variable binding
sce_generate_static_w3c_test(322 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML B.2.1: _sessionid immutability
sce_generate_static_w3c_test(323 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML B.2.1: _name system variable binding
sce_generate_static_w3c_test(324 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML B.2.2: _name system variable immutability
sce_generate_static_w3c_test(325 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10: _ioprocessors system variable binding
sce_generate_static_w3c_test(326 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML B.2.3: _ioprocessors system variable immutability
sce_generate_static_w3c_test(329 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10: system variables immutability
sce_generate_static_w3c_test(330 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10: event field binding (internal and external events)
sce_generate_static_w3c_test(331 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10.1: event type classification (internal, platform, external)
sce_generate_static_w3c_test(332 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10.1: _event.sendid field binding in error events, W3C SCXML 6.2.4: send idlocation attribute
sce_generate_static_w3c_test(333 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10.1: _event.sendid field must be blank for non-error events without explicit sendid
sce_generate_static_w3c_test(335 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10: _event.origin field must be blank for internal events
sce_generate_static_w3c_test(336 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10.1: _event.origin field bidirectional communication via targetexpr
sce_generate_static_w3c_test(337 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10.1: _event.origintype field must be blank for internal events
sce_generate_static_w3c_test(338 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.4: Static invoke with inline content child SCXML
sce_generate_static_w3c_test(339 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10: _event.invokeid field must be blank for non-invoked events
sce_generate_static_w3c_test(342 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.2: eventexpr dynamic event name evaluation
sce_generate_static_w3c_test(343 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10.1: Invalid param error.execution
sce_generate_static_w3c_test(344 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.9: Invalid cond expression error.execution
sce_generate_static_w3c_test(346 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.9.1: System variable protection validation
sce_generate_static_w3c_test(347 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.4: SCXML Event I/O processor parent-child communication
sce_generate_static_w3c_test(348 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.2: Send event parameter sets event name
sce_generate_static_w3c_test(349 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10.1: _event.origin system variable
sce_generate_static_w3c_test(350 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.6: Default initial state (first child in document order)
sce_generate_static_w3c_test(351 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10.1: _event.sendid field validation
sce_generate_static_w3c_test(352 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10.1: _event.origintype field validation
sce_generate_static_w3c_test(354 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10: _event.data with namelist/param/content
sce_generate_static_w3c_test(355 ${STATIC_W3C_OUTPUT_DIR})  # no initial state (default to first child in document order)
sce_generate_static_w3c_test(364 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.6/3.4: Default initial states and parallel configurations (Pure Static)
sce_generate_static_w3c_test(372 ${STATIC_W3C_OUTPUT_DIR})  # no initial state (done.state.parentid after final)
sce_generate_static_w3c_test(375 ${STATIC_W3C_OUTPUT_DIR})  # no initial state (onentry execution order)
sce_generate_static_w3c_test(376 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.8: Independent onentry handler execution (Static Hybrid)
sce_generate_static_w3c_test(377 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.8: Multiple onexit handlers in document order (Pure Static)
sce_generate_static_w3c_test(378 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.8/3.9: Independent onexit handler execution with error.execution (Static Hybrid)
sce_generate_static_w3c_test(387 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.11: history states with default transitions
sce_generate_static_w3c_test(388 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.11: deep and shallow history states
sce_generate_static_w3c_test(396 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.12.1: event name matching and document order transition selection
sce_generate_static_w3c_test(399 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.9.3: event name matching with prefix and wildcard
sce_generate_static_w3c_test(401 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.12.1: internal event queue priority over external events
sce_generate_static_w3c_test(402 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.12: error events processed like any other event
sce_generate_static_w3c_test(403a ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.13: optimal enabled transition set - basic case
sce_generate_static_w3c_test(403b ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.13: optimal enabled transition set as SET - parallel regions (needs wrapper)
sce_generate_static_w3c_test(403c ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.13: optimal enabled transition set with preemption - parallel regions (needs wrapper)
sce_generate_static_w3c_test(404 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.13: parallel state exit order with document order tie-breaking
sce_generate_static_w3c_test(405 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.9/D.2: executable content execution order in transitions
sce_generate_static_w3c_test(406 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.3: state entry order with parallel regions
sce_generate_static_w3c_test(407 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.8: onexit handlers with datamodel variable updates
sce_generate_static_w3c_test(409 ${STATIC_W3C_OUTPUT_DIR} TYPE SCHEDULED)  # W3C SCXML 3.12.1: state removal from active states during exit
sce_generate_static_w3c_test(411 ${STATIC_W3C_OUTPUT_DIR} TYPE SCHEDULED)  # W3C SCXML 3.12.1: state addition to active states during entry
sce_generate_static_w3c_test(412 ${STATIC_W3C_OUTPUT_DIR} TYPE SCHEDULED)  # W3C SCXML 3.3.2: initial transition executable content execution order
sce_generate_static_w3c_test(413 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.13: parallel initial states with space-separated state IDs
sce_generate_static_w3c_test(415 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.7.1: manual test (no Pass state)
sce_generate_static_w3c_test(416 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.3.2: done.state.id event generation for compound states
sce_generate_static_w3c_test(417 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.7.1: done.state event for parallel states
sce_generate_static_w3c_test(419 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.13: eventless transition precedence over event-driven transitions
sce_generate_static_w3c_test(421 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10.1: internal event priority over external events
sce_generate_static_w3c_test(422 ${STATIC_W3C_OUTPUT_DIR} TYPE SCHEDULED)  # W3C SCXML 6.3: delayed send event scheduling
sce_generate_static_w3c_test(423 ${STATIC_W3C_OUTPUT_DIR} TYPE SCHEDULED)  # W3C SCXML 5.9.2: external event queue processing
sce_generate_static_w3c_test(436 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.9.2: In() predicate in null data model
sce_generate_static_w3c_test(444 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.2/5.9: ECMAScript datamodel variable in condition
sce_generate_static_w3c_test(445 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML B.2.2: ECMAScript undefined variable initialization
sce_generate_static_w3c_test(446 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.2.2: external file loading with src attribute
sce_generate_static_w3c_test(448 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML B.2: ECMAScript single global scope for all variables
sce_generate_static_w3c_test(449 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML B.2: ECMAScript boolean conversion for string literals
sce_generate_static_w3c_test(451 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.9.2: In() predicate in parallel states
sce_generate_static_w3c_test(452 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.3/5.4: datamodel substructure assignment (object properties)
sce_generate_static_w3c_test(453 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML B.2/3.12.1: ECMAScript function expression evaluation
sce_generate_static_w3c_test(456 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML B.2/5.9: ECMAScript script element execution
sce_generate_static_w3c_test(457 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.4/D.3.1: foreach error handling with ECMAScript validation
sce_generate_static_w3c_test(459 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.4: foreach iteration order and index validation
sce_generate_static_w3c_test(460 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 4.6: foreach shallow copy semantics with array modification
sce_generate_static_w3c_test(525 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 4.6: foreach shallow copy semantics (minimal)
sce_generate_static_w3c_test(487 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.3/5.4: illegal assignment error.execution event
sce_generate_static_w3c_test(488 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.7: error handling in donedata param expressions
sce_generate_static_w3c_test(527 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.5: donedata content expr evaluation
sce_generate_static_w3c_test(528 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.5: donedata content expr error.execution
sce_generate_static_w3c_test(529 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.5: donedata content integer literal
sce_generate_static_w3c_test(530 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.4: Hybrid invoke contentexpr evaluation (Static Hybrid)
sce_generate_static_w3c_test(495 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.2: SCXML Event I/O Processor internal vs external queue handling
sce_generate_static_w3c_test(496 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML C.1: error.communication for unreachable target (Static Hybrid)
sce_generate_static_w3c_test(500 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.3.1: SCXML Event I/O processor location field access (Static Hybrid)
sce_generate_static_w3c_test(501 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML C.1: SCXML Event I/O Processor location field as send target (Static Hybrid)
sce_generate_static_w3c_test(503 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.13: targetless transitions do not exit/re-enter source state (Static Hybrid)
sce_generate_static_w3c_test(504 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.13: external transition exit sets (LCCA) (Static Hybrid)
sce_generate_static_w3c_test(505 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.13: internal transition does not exit source state (Static Hybrid)
sce_generate_static_w3c_test(506 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.9.2: internal transition with non-descendant target behaves as external (Static Hybrid)

# W3C SCXML C.2: BasicHTTP Event I/O Processor tests (native builds only, not WASM)
if(NOT EMSCRIPTEN)
    sce_generate_static_w3c_test(509 ${STATIC_W3C_OUTPUT_DIR} TYPE HTTP)  # W3C SCXML C.2: BasicHTTP Event I/O Processor POST method validation (Static Hybrid)
    sce_generate_static_w3c_test(510 ${STATIC_W3C_OUTPUT_DIR} TYPE HTTP)  # W3C SCXML C.2: BasicHTTP messages go to external queue (Static Hybrid)
    sce_generate_static_w3c_test(513 ${STATIC_W3C_OUTPUT_DIR} TYPE HTTP)  # W3C SCXML C.2: manual test - BasicHTTP success response (Integration test only)
    sce_generate_static_w3c_test(518 ${STATIC_W3C_OUTPUT_DIR} TYPE HTTP)  # W3C SCXML C.2: BasicHTTP namelist encoding as POST parameters (Static Hybrid)
    sce_generate_static_w3c_test(519 ${STATIC_W3C_OUTPUT_DIR} TYPE HTTP)  # W3C SCXML C.2: BasicHTTP param encoding as POST parameters (Static)
    sce_generate_static_w3c_test(520 ${STATIC_W3C_OUTPUT_DIR} TYPE HTTP)  # W3C SCXML C.2: BasicHTTP content element as message body (Static)
endif()

sce_generate_static_w3c_test(521 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.2.4/6.2.5: error.communication on invalid targetexpr (Static Hybrid)

if(NOT EMSCRIPTEN)
    sce_generate_static_w3c_test(522 ${STATIC_W3C_OUTPUT_DIR} TYPE HTTP)  # W3C SCXML C.2: BasicHTTP location field usage (Pure Static)
endif()

if(NOT EMSCRIPTEN)
    sce_generate_static_w3c_test(531 ${STATIC_W3C_OUTPUT_DIR} TYPE HTTP)  # W3C SCXML C.2: _scxmleventname param overrides event name (Pure Static)
    sce_generate_static_w3c_test(532 ${STATIC_W3C_OUTPUT_DIR} TYPE HTTP)  # W3C SCXML C.2: HTTP method name fallback for event naming (Pure Static)
endif()

sce_generate_static_w3c_test(533 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.13: internal transition exit set for non-compound states (Static Hybrid)

if(NOT EMSCRIPTEN)
    sce_generate_static_w3c_test(534 ${STATIC_W3C_OUTPUT_DIR} TYPE HTTP)  # W3C SCXML C.2: BasicHTTP _scxmleventname parameter transmission (Static Hybrid)
endif()
sce_generate_static_w3c_test(550 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.2.2: early binding with expr attribute for data variable (Static Hybrid)
sce_generate_static_w3c_test(551 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.2.2: early binding with inline content (Static Hybrid)
sce_generate_static_w3c_test(552 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.2.2: external data loading via src attribute (Static Hybrid)
sce_generate_static_w3c_test(553 ${STATIC_W3C_OUTPUT_DIR} TYPE SCHEDULED)  # W3C SCXML 6.2.4 & 5.11: send namelist error handling (Static Hybrid)
sce_generate_static_w3c_test(554 ${STATIC_W3C_OUTPUT_DIR} TYPE SCHEDULED)  # W3C SCXML 6.4 & B.1: invoke with invalid namelist error handling (Pure Static)
sce_generate_static_w3c_test(557 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML B.2: ECMAScript XML DOM assignment with inline and file content (Static Hybrid)
sce_generate_static_w3c_test(558 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML B.2: ECMAScript whitespace normalization for non-XML content (Static Hybrid)
sce_generate_static_w3c_test(560 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10: _event.data param structure in ECMAScript datamodel (Static Hybrid)
sce_generate_static_w3c_test(561 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.9.2: ECMAScript DOM object for XML event data (Static Hybrid)
sce_generate_static_w3c_test(562 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.9.2: ECMAScript content space normalization (Static Hybrid)

if(NOT EMSCRIPTEN)
    sce_generate_static_w3c_test(567 ${STATIC_W3C_OUTPUT_DIR} TYPE HTTP)  # W3C SCXML C.2: BasicHTTP param encoding in _event.data (Static Hybrid)
endif()

sce_generate_static_w3c_test(569 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.8: SCXML Event I/O processor location field (Static Hybrid)
sce_generate_static_w3c_test(570 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.12.1: Parallel state completion with done.state.id (Static Hybrid)
sce_generate_static_w3c_test(576 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.13: Parallel initial with space-separated states

if(NOT EMSCRIPTEN)
    sce_generate_static_w3c_test(577 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML C.2: error.communication on BasicHTTP send with no target
endif()
sce_generate_static_w3c_test(578 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.9/5.10: ECMAScript _event.data JSON object (Static Hybrid)
sce_generate_static_w3c_test(579 ${STATIC_W3C_OUTPUT_DIR} TYPE SCHEDULED)  # W3C SCXML 3.8: history default content execution (Static Hybrid)
sce_generate_static_w3c_test(580 ${STATIC_W3C_OUTPUT_DIR} TYPE SCHEDULED)  # W3C SCXML 3.11: history state not in configuration (parallel)

# ============================================================================
# W3C Test Registration Automation
# ============================================================================
#
# SINGLE SOURCE OF TRUTH: sce_generate_static_w3c_test() calls above.
# W3C_AOT_TESTS is auto-accumulated from these calls (Zero Duplication).
#
# WORKFLOW - Adding a new AOT test (ONE step):
#   Add: sce_generate_static_w3c_test(XXX ${STATIC_W3C_OUTPUT_DIR})
#   Optional TYPE: TYPE SCHEDULED (delayed send/invoke) or TYPE HTTP (BasicHTTP)
#   Everything else is automatic:
#   - TestXXX.h auto-generated from metadata.txt template (if not exists)
#   - W3C_AOT_TESTS auto-accumulated (no manual list to maintain)
#   - W3CTestRunner_TestXXX.cpp auto-generated
#   - W3CTestRegistry.cpp auto-generated
#   - visualizer/test-list.js auto-generated
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
# Auto-generate visualizer/test-list.js from W3C_AOT_TESTS
# ============================================================================
# Zero Duplication: W3C_AOT_TESTS is auto-accumulated from sce_generate_static_w3c_test() calls
find_package(Python3 COMPONENTS Interpreter)

if(Python3_FOUND AND NOT EMSCRIPTEN)
    # Use the auto-generated W3C_AOT_TESTS list file as input for test-list.js generation
    add_custom_command(
        OUTPUT ${CMAKE_SOURCE_DIR}/visualizer/test-list.js
        COMMAND ${Python3_EXECUTABLE}
                ${CMAKE_SOURCE_DIR}/tools/generate_test_list.py
                ${CMAKE_SOURCE_DIR}/build/W3CAotTestList.cmake
                ${CMAKE_SOURCE_DIR}/visualizer/test-list.js
        DEPENDS ${CMAKE_SOURCE_DIR}/build/W3CAotTestList.cmake
                ${CMAKE_SOURCE_DIR}/tools/generate_test_list.py
        COMMENT "Generating visualizer/test-list.js from W3C_AOT_TESTS"
        VERBATIM
    )

    add_custom_target(generate_test_list ALL
        DEPENDS ${CMAKE_SOURCE_DIR}/visualizer/test-list.js
    )

    message(STATUS "Visualizer test list will be auto-generated from W3C_AOT_TESTS")
elseif(NOT Python3_FOUND)
    message(WARNING "Python3 not found - visualizer/test-list.js will not be auto-generated")
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
# SCE Mesh compile verification test (Phase 1)
# ============================================================================
# Validates that generated mesh transport code compiles against generated SM
# headers. Compilation success = test pass (runtime assertions in Phase 2).
if(SCE_ENABLE_MESH AND NOT EMSCRIPTEN)
    set(MESH_TEST_DIR "${CMAKE_CURRENT_SOURCE_DIR}/mesh")
    set(MESH_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/mesh_generated")

    # Jinja2 template consumed by sce-codegen --deploy. Listed in the
    # DEPENDS of every *_transport.h custom_command so template edits
    # trigger re-codegen without manually deleting cached headers.
    set(SCE_MESH_CPP_TEMPLATE
        "${CMAKE_SOURCE_DIR}/tools/codegen/templates/mesh/cpp/mesh_transport.h.jinja2")

    # ── MeshEnvelope codec unit tests (Phase 3.5 Step 3) ──────────
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

    # ── CommunicationError unit tests (SCE_MESH.md §10.7.1 + §16.7) ─
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
            GTest::gtest_main
            pthread
    )

    target_compile_features(mesh_dispatch_test PRIVATE cxx_std_17)

    apply_sce_test_policies(mesh_dispatch_test)

    add_test(NAME mesh_dispatch_verification COMMAND mesh_dispatch_test)

    # Generate brake_sm.h from brake.scxml
    add_custom_command(
        OUTPUT "${MESH_OUTPUT_DIR}/brake_sm.h"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_OUTPUT_DIR}"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake.scxml" -l cpp -o "${MESH_OUTPUT_DIR}"
        DEPENDS "${MESH_TEST_DIR}/brake.scxml" "${SCE_CODEGEN}"
        COMMENT "Mesh: Generating brake_sm.h"
        VERBATIM
    )

    # Generate motor_sm.h from motor.scxml
    add_custom_command(
        OUTPUT "${MESH_OUTPUT_DIR}/motor_sm.h"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor.scxml" -l cpp -o "${MESH_OUTPUT_DIR}"
        DEPENDS "${MESH_TEST_DIR}/motor.scxml" "${SCE_CODEGEN}"
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
        DEPENDS "${MESH_TEST_DIR}/brake.scxml" "${MESH_TEST_DIR}/deploy.yaml"
                "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                "${MESH_OUTPUT_DIR}/brake_sm.h"
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

    apply_sce_test_policies(mesh_compile_test)

    add_test(NAME mesh_compile_verification COMMAND mesh_compile_test)

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
        DEPENDS "${MESH_TEST_DIR}/brake_invoke.scxml" "${SCE_CODEGEN}"
        COMMENT "Mesh: Generating brake_invoke_sm.h (§9.5 invoke)"
        VERBATIM
    )

    add_custom_command(
        OUTPUT "${MESH_INVOKE_OUTPUT_DIR}/motor_invoke_sm.h"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_invoke.scxml" -l cpp -o "${MESH_INVOKE_OUTPUT_DIR}"
        DEPENDS "${MESH_TEST_DIR}/motor_invoke.scxml" "${SCE_CODEGEN}"
        COMMENT "Mesh: Generating motor_invoke_sm.h (§9.5 invoke receiver)"
        VERBATIM
    )

    add_custom_command(
        OUTPUT "${MESH_INVOKE_OUTPUT_DIR}/brake_invoke_transport.h"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_invoke.scxml" -l cpp -o "${MESH_INVOKE_OUTPUT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_invoke.yaml"
        DEPENDS "${MESH_TEST_DIR}/brake_invoke.scxml" "${MESH_TEST_DIR}/deploy_invoke.yaml"
                "${MESH_TEST_DIR}/motor_invoke.scxml"
                "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                "${MESH_INVOKE_OUTPUT_DIR}/brake_invoke_sm.h"
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
        DEPENDS "${MESH_TEST_DIR}/motor_invoke.scxml" "${MESH_TEST_DIR}/deploy_invoke.yaml"
                "${MESH_TEST_DIR}/brake_invoke.scxml"
                "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                "${MESH_INVOKE_OUTPUT_DIR}/motor_invoke_sm.h"
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

    apply_sce_test_policies(mesh_invoke_compile_test)

    add_test(NAME mesh_invoke_compile_verification COMMAND mesh_invoke_compile_test)

    # ── SCE_MESH §9.5 srcexpr compile verification ──────────────
    # Forces the srcexpr `{% if invoke_info.srcexpr %}` branches in
    # entry_exit_actions.jinja2 + state_machine.jinja2 to render and
    # compile. Until this fixture landed, no SCXML in-tree used
    # `<invoke type="sce:mesh-rpc" srcexpr="...">`, so the Phase 5 β
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
        DEPENDS "${MESH_TEST_DIR}/srcexpr_client.scxml" "${SCE_CODEGEN}"
        COMMENT "Mesh srcexpr: Generating srcexpr_client_sm.h (§9.5 srcexpr runtime target)"
        VERBATIM
    )

    add_custom_command(
        OUTPUT "${MESH_SRCEXPR_OUTPUT_DIR}/motor_srcexpr_sm.h"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_srcexpr.scxml" -l cpp -o "${MESH_SRCEXPR_OUTPUT_DIR}"
        DEPENDS "${MESH_TEST_DIR}/motor_srcexpr.scxml" "${SCE_CODEGEN}"
        COMMENT "Mesh srcexpr: Generating motor_srcexpr_sm.h (§9.5 srcexpr receiver)"
        VERBATIM
    )

    add_custom_command(
        OUTPUT "${MESH_SRCEXPR_OUTPUT_DIR}/srcexpr_client_transport.h"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/srcexpr_client.scxml" -l cpp -o "${MESH_SRCEXPR_OUTPUT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_srcexpr.yaml"
        DEPENDS "${MESH_TEST_DIR}/srcexpr_client.scxml" "${MESH_TEST_DIR}/deploy_srcexpr.yaml"
                "${MESH_TEST_DIR}/motor_srcexpr.scxml"
                "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                "${MESH_SRCEXPR_OUTPUT_DIR}/srcexpr_client_sm.h"
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

    apply_sce_test_policies(mesh_srcexpr_compile_test)

    add_test(NAME mesh_srcexpr_compile_verification COMMAND mesh_srcexpr_compile_test)

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
        DEPENDS "${MESH_TEST_DIR}/brake.scxml" "${SCE_CODEGEN}"
        COMMENT "Mesh shm: Generating brake_sm.h"
        VERBATIM
    )

    add_custom_command(
        OUTPUT "${MESH_SHM_OUTPUT_DIR}/motor_sm.h"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor.scxml" -l cpp -o "${MESH_SHM_OUTPUT_DIR}"
        DEPENDS "${MESH_TEST_DIR}/motor.scxml" "${SCE_CODEGEN}"
        COMMENT "Mesh shm: Generating motor_sm.h"
        VERBATIM
    )

    # Generate brake_transport.h with shm transport
    add_custom_command(
        OUTPUT "${MESH_SHM_OUTPUT_DIR}/brake_transport.h"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake.scxml" -l cpp -o "${MESH_SHM_OUTPUT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_shm.yaml"
        DEPENDS "${MESH_TEST_DIR}/brake.scxml" "${MESH_TEST_DIR}/deploy_shm.yaml"
                "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                "${MESH_SHM_OUTPUT_DIR}/brake_sm.h"
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

    apply_sce_test_policies(mesh_shm_compile_test)

    add_test(NAME mesh_shm_compile_verification COMMAND mesh_shm_compile_test)

    # ── Phase 2: local_transport runtime verification ────────────
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

    # ── Phase 2: shm_transport runtime verification (fork-based) ──
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
    set(SCE_TEST_CUSTOM_TCP_PORT "55821" CACHE STRING
        "TCP loopback port for the custom_tcp harness test (IANA dynamic range)")
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
        DEPENDS "${MESH_TEST_DIR}/brake.scxml"
                "${MESH_TEST_DIR}/motor.scxml"
                "${MESH_TCP_OUTPUT_DIR}/deploy_custom_tcp.yaml"
                "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
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
        DEPENDS "${MESH_TEST_DIR}/brake.scxml"
                "${MESH_TEST_DIR}/motor.scxml"
                "${MESH_TCP_OUTPUT_DIR}/deploy_custom_tcp.yaml"
                "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
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
        # bucket from the default 55821 (matters when a developer runs
        # parallel ctest invocations against different ports).
        RESOURCE_LOCK sce_mesh_custom_tcp_${SCE_TEST_CUSTOM_TCP_PORT})

    # ── Phase 3 entry #4d: end-to-end payload verification ─────────
    # Proves <param> data survives: sender → JSON → wire → drain
    # → raiseExternal → _event.data → receiver cond matches.
    set(MESH_SHM_PAYLOAD_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/mesh_shm_payload_generated")

    add_custom_command(
        OUTPUT "${MESH_SHM_PAYLOAD_OUTPUT_DIR}/brake_payload_sm.h"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SHM_PAYLOAD_OUTPUT_DIR}"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_payload.scxml"
                -l cpp -o "${MESH_SHM_PAYLOAD_OUTPUT_DIR}"
        DEPENDS "${MESH_TEST_DIR}/brake_payload.scxml" "${SCE_CODEGEN}"
        COMMENT "Mesh shm payload: Generating brake_payload_sm.h"
        VERBATIM
    )

    add_custom_command(
        OUTPUT "${MESH_SHM_PAYLOAD_OUTPUT_DIR}/motor_payload_sm.h"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SHM_PAYLOAD_OUTPUT_DIR}"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_payload.scxml"
                -l cpp -o "${MESH_SHM_PAYLOAD_OUTPUT_DIR}"
        DEPENDS "${MESH_TEST_DIR}/motor_payload.scxml" "${SCE_CODEGEN}"
        COMMENT "Mesh shm payload: Generating motor_payload_sm.h"
        VERBATIM
    )

    add_custom_command(
        OUTPUT "${MESH_SHM_PAYLOAD_OUTPUT_DIR}/brake_payload_transport.h"
        COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_payload.scxml"
                -l cpp -o "${MESH_SHM_PAYLOAD_OUTPUT_DIR}"
                --deploy "${MESH_TEST_DIR}/deploy_shm_payload.yaml"
        DEPENDS "${MESH_TEST_DIR}/brake_payload.scxml"
                "${MESH_TEST_DIR}/motor_payload.scxml"
                "${MESH_TEST_DIR}/deploy_shm_payload.yaml"
                "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                "${MESH_SHM_PAYLOAD_OUTPUT_DIR}/brake_payload_sm.h"
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

    # ── Phase 3: someip_transport compile verification ─────────
    # Validates that generated SOME/IP transport code compiles against
    # real vsomeip 3.x headers. Gated on find_package(vsomeip3).
    find_package(vsomeip3 QUIET)
    if(vsomeip3_FOUND)
        set(MESH_SOMEIP_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/mesh_someip_generated")

        add_custom_command(
            OUTPUT "${MESH_SOMEIP_OUTPUT_DIR}/brake_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SOMEIP_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake.scxml" -l cpp -o "${MESH_SOMEIP_OUTPUT_DIR}"
            DEPENDS "${MESH_TEST_DIR}/brake.scxml" "${SCE_CODEGEN}"
            COMMENT "Mesh someip: Generating brake_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_SOMEIP_OUTPUT_DIR}/motor_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SOMEIP_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor.scxml" -l cpp -o "${MESH_SOMEIP_OUTPUT_DIR}"
            DEPENDS "${MESH_TEST_DIR}/motor.scxml" "${SCE_CODEGEN}"
            COMMENT "Mesh someip: Generating motor_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_SOMEIP_OUTPUT_DIR}/brake_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake.scxml" -l cpp -o "${MESH_SOMEIP_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_someip.yaml"
            DEPENDS "${MESH_TEST_DIR}/brake.scxml" "${MESH_TEST_DIR}/deploy_someip.yaml"
                    "${MESH_TEST_DIR}/motor.scxml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_SOMEIP_OUTPUT_DIR}/brake_sm.h"
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
            DEPENDS "${MESH_TEST_DIR}/brake_someip_multi.scxml" "${SCE_CODEGEN}"
            COMMENT "Mesh someip multi: Generating brake_someip_multi_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/motor_someip_multi_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SOMEIP_MULTI_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_someip_multi.scxml" -l cpp -o "${MESH_SOMEIP_MULTI_OUTPUT_DIR}"
            DEPENDS "${MESH_TEST_DIR}/motor_someip_multi.scxml" "${SCE_CODEGEN}"
            COMMENT "Mesh someip multi: Generating motor_someip_multi_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/brake_someip_multi_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_someip_multi.scxml" -l cpp -o "${MESH_SOMEIP_MULTI_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_someip_multi.yaml"
            DEPENDS "${MESH_TEST_DIR}/brake_someip_multi.scxml" "${MESH_TEST_DIR}/deploy_someip_multi.yaml"
                    "${MESH_TEST_DIR}/motor_someip_multi.scxml" "${MESH_TEST_DIR}/vsomeip_motor_multi.json"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/brake_someip_multi_sm.h"
            COMMENT "Mesh someip multi: Generating brake_someip_multi_transport.h (multi-pattern)"
            VERBATIM
        )

        # Session E: generate motor (server) transport header
        add_custom_command(
            OUTPUT "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/motor_someip_multi_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_someip_multi.scxml" -l cpp -o "${MESH_SOMEIP_MULTI_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_someip_multi.yaml"
            DEPENDS "${MESH_TEST_DIR}/motor_someip_multi.scxml" "${MESH_TEST_DIR}/deploy_someip_multi.yaml"
                    "${MESH_TEST_DIR}/vsomeip_motor_multi.json"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_SOMEIP_MULTI_OUTPUT_DIR}/motor_someip_multi_sm.h"
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
        apply_sce_test_policies(mesh_someip_multipattern_compile_test)
        add_test(NAME mesh_someip_multipattern_compile_verification
                 COMMAND mesh_someip_multipattern_compile_test)

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
        # vsomeip's routing manager binds a well-known unix socket
        # (/tmp/vsomeip-0 by default). Only one routing manager can own
        # that path at a time, so serialize any test that spins up an
        # internal-routing vsomeip process. RESOURCE_LOCK prevents
        # parallel ctest runs from racing for the socket.
        set_tests_properties(mesh_someip_runtime PROPERTIES
            TIMEOUT 30
            RESOURCE_LOCK vsomeip_routing_socket
            ENVIRONMENT "VSOMEIP_CONFIGURATION=${CMAKE_CURRENT_SOURCE_DIR}/mesh/vsomeip_e2e_test.json;LD_LIBRARY_PATH=${vsomeip3_DIR}:$ENV{LD_LIBRARY_PATH}")

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
            RESOURCE_LOCK vsomeip_routing_socket
            ENVIRONMENT "VSOMEIP_CONFIGURATION=${CMAKE_CURRENT_SOURCE_DIR}/mesh/vsomeip_e2e_test.json;LD_LIBRARY_PATH=${vsomeip3_DIR}:$ENV{LD_LIBRARY_PATH}")

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
            RESOURCE_LOCK vsomeip_routing_socket
            ENVIRONMENT "VSOMEIP_CONFIGURATION=${CMAKE_CURRENT_SOURCE_DIR}/mesh/vsomeip_e2e_test.json;LD_LIBRARY_PATH=${vsomeip3_DIR}:$ENV{LD_LIBRARY_PATH}")

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
            DEPENDS "${MESH_TEST_DIR}/brake_someip_unsub.scxml" "${SCE_CODEGEN}"
            COMMENT "Mesh someip unsub: Generating brake_someip_unsub_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_SOMEIP_UNSUB_OUTPUT_DIR}/motor_someip_unsub_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_SOMEIP_UNSUB_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_someip_unsub.scxml"
                    -l cpp -o "${MESH_SOMEIP_UNSUB_OUTPUT_DIR}"
            DEPENDS "${MESH_TEST_DIR}/motor_someip_unsub.scxml" "${SCE_CODEGEN}"
            COMMENT "Mesh someip unsub: Generating motor_someip_unsub_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_SOMEIP_UNSUB_OUTPUT_DIR}/brake_someip_unsub_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_someip_unsub.scxml"
                    -l cpp -o "${MESH_SOMEIP_UNSUB_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_someip_unsub.yaml"
            DEPENDS "${MESH_TEST_DIR}/brake_someip_unsub.scxml"
                    "${MESH_TEST_DIR}/motor_someip_unsub.scxml"
                    "${MESH_TEST_DIR}/deploy_someip_unsub.yaml"
                    "${MESH_TEST_DIR}/vsomeip_motor_unsub.json"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_SOMEIP_UNSUB_OUTPUT_DIR}/brake_someip_unsub_sm.h"
            COMMENT "Mesh someip unsub: Generating brake_someip_unsub_transport.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_SOMEIP_UNSUB_OUTPUT_DIR}/motor_someip_unsub_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_someip_unsub.scxml"
                    -l cpp -o "${MESH_SOMEIP_UNSUB_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_someip_unsub.yaml"
            DEPENDS "${MESH_TEST_DIR}/motor_someip_unsub.scxml"
                    "${MESH_TEST_DIR}/deploy_someip_unsub.yaml"
                    "${MESH_TEST_DIR}/vsomeip_motor_unsub.json"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_SOMEIP_UNSUB_OUTPUT_DIR}/motor_someip_unsub_sm.h"
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
            RESOURCE_LOCK vsomeip_routing_socket
            ENVIRONMENT "VSOMEIP_CONFIGURATION=${CMAKE_CURRENT_SOURCE_DIR}/mesh/vsomeip_e2e_unsub.json;LD_LIBRARY_PATH=${vsomeip3_DIR}:$ENV{LD_LIBRARY_PATH}")
    else()
        message(STATUS "vsomeip3 not found — skipping mesh_someip_compile_test")
    endif()

    # ── Phase 3: zenoh_transport compile verification ─────────
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
            DEPENDS "${MESH_TEST_DIR}/brake.scxml" "${SCE_CODEGEN}"
            COMMENT "Mesh zenoh: Generating brake_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_ZENOH_OUTPUT_DIR}/motor_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_ZENOH_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor.scxml" -l cpp -o "${MESH_ZENOH_OUTPUT_DIR}"
            DEPENDS "${MESH_TEST_DIR}/motor.scxml" "${SCE_CODEGEN}"
            COMMENT "Mesh zenoh: Generating motor_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_ZENOH_OUTPUT_DIR}/brake_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake.scxml" -l cpp -o "${MESH_ZENOH_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_zenoh.yaml"
            DEPENDS "${MESH_TEST_DIR}/brake.scxml" "${MESH_TEST_DIR}/deploy_zenoh.yaml"
                    "${MESH_TEST_DIR}/motor.scxml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_ZENOH_OUTPUT_DIR}/brake_sm.h"
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
            DEPENDS "${MESH_TEST_DIR}/brake_zenoh_base_subscribe.scxml" "${SCE_CODEGEN}"
            COMMENT "Mesh zenoh base subscribe: Generating brake_zenoh_base_subscribe_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_ZENOH_BASE_SUB_OUTPUT_DIR}/brake_zenoh_base_subscribe_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_zenoh_base_subscribe.scxml"
                    -l cpp -o "${MESH_ZENOH_BASE_SUB_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_zenoh_base_subscribe.yaml"
            DEPENDS "${MESH_TEST_DIR}/brake_zenoh_base_subscribe.scxml"
                    "${MESH_TEST_DIR}/motor_zenoh_base_subscribe.scxml"
                    "${MESH_TEST_DIR}/deploy_zenoh_base_subscribe.yaml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_ZENOH_BASE_SUB_OUTPUT_DIR}/brake_zenoh_base_subscribe_sm.h"
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
            DEPENDS "${MESH_TEST_DIR}/brake_zenoh_liveliness.scxml" "${SCE_CODEGEN}"
            COMMENT "Mesh zenoh liveliness: Generating brake_zenoh_liveliness_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_ZENOH_LIVELINESS_OUTPUT_DIR}/brake_zenoh_liveliness_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_zenoh_liveliness.scxml"
                    -l cpp -o "${MESH_ZENOH_LIVELINESS_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_zenoh_liveliness.yaml"
            DEPENDS "${MESH_TEST_DIR}/brake_zenoh_liveliness.scxml"
                    "${MESH_TEST_DIR}/motor_zenoh_liveliness.scxml"
                    "${MESH_TEST_DIR}/deploy_zenoh_liveliness.yaml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_ZENOH_LIVELINESS_OUTPUT_DIR}/brake_zenoh_liveliness_sm.h"
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
            RESOURCE_LOCK zenoh_liveliness_port_17448
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
            DEPENDS "${MESH_TEST_DIR}/brake_zenoh_on_drop.scxml" "${SCE_CODEGEN}"
            COMMENT "Mesh zenoh on_drop: Generating brake_zenoh_on_drop_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_ZENOH_ON_DROP_OUTPUT_DIR}/brake_zenoh_on_drop_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_zenoh_on_drop.scxml"
                    -l cpp -o "${MESH_ZENOH_ON_DROP_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_zenoh_on_drop.yaml"
            DEPENDS "${MESH_TEST_DIR}/brake_zenoh_on_drop.scxml"
                    "${MESH_TEST_DIR}/motor_zenoh_on_drop.scxml"
                    "${MESH_TEST_DIR}/deploy_zenoh_on_drop.yaml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_ZENOH_ON_DROP_OUTPUT_DIR}/brake_zenoh_on_drop_sm.h"
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
            RESOURCE_LOCK zenoh_on_drop_port_17449
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
            DEPENDS "${MESH_TEST_DIR}/motor_zenoh_server_timeout.scxml" "${SCE_CODEGEN}"
            COMMENT "Mesh zenoh server_timeout: Generating motor_zenoh_server_timeout_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_ZENOH_SERVER_TIMEOUT_OUTPUT_DIR}/motor_zenoh_server_timeout_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_zenoh_server_timeout.scxml"
                    -l cpp -o "${MESH_ZENOH_SERVER_TIMEOUT_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_zenoh_server_timeout.yaml"
            DEPENDS "${MESH_TEST_DIR}/motor_zenoh_server_timeout.scxml"
                    "${MESH_TEST_DIR}/deploy_zenoh_server_timeout.yaml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_ZENOH_SERVER_TIMEOUT_OUTPUT_DIR}/motor_zenoh_server_timeout_sm.h"
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
            DEPENDS "${MESH_TEST_DIR}/brake.scxml" "${SCE_CODEGEN}"
            COMMENT "Mesh ordered: Generating brake_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_ORDERED_OUTPUT_DIR}/brake_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake.scxml" -l cpp -o "${MESH_ORDERED_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_ordered.yaml"
            DEPENDS "${MESH_TEST_DIR}/brake.scxml" "${MESH_TEST_DIR}/deploy_ordered.yaml"
                    "${MESH_TEST_DIR}/motor.scxml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_ORDERED_OUTPUT_DIR}/brake_sm.h"
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
            DEPENDS "${MESH_TEST_DIR}/brake_zenoh_multi.scxml" "${SCE_CODEGEN}"
            COMMENT "Mesh zenoh multi: Generating brake_zenoh_multi_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_ZENOH_MULTI_OUTPUT_DIR}/motor_zenoh_multi_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_ZENOH_MULTI_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_zenoh_multi.scxml" -l cpp -o "${MESH_ZENOH_MULTI_OUTPUT_DIR}"
            DEPENDS "${MESH_TEST_DIR}/motor_zenoh_multi.scxml" "${SCE_CODEGEN}"
            COMMENT "Mesh zenoh multi: Generating motor_zenoh_multi_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_ZENOH_MULTI_OUTPUT_DIR}/brake_zenoh_multi_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/brake_zenoh_multi.scxml" -l cpp -o "${MESH_ZENOH_MULTI_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_zenoh_multi.yaml"
            DEPENDS "${MESH_TEST_DIR}/brake_zenoh_multi.scxml" "${MESH_TEST_DIR}/deploy_zenoh_multi.yaml"
                    "${MESH_TEST_DIR}/motor_zenoh_multi.scxml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_ZENOH_MULTI_OUTPUT_DIR}/brake_zenoh_multi_sm.h"
            COMMENT "Mesh zenoh multi: Generating brake_zenoh_multi_transport.h (multi-pattern)"
            VERBATIM
        )

        # Session E: generate motor (server) transport header
        add_custom_command(
            OUTPUT "${MESH_ZENOH_MULTI_OUTPUT_DIR}/motor_zenoh_multi_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/motor_zenoh_multi.scxml" -l cpp -o "${MESH_ZENOH_MULTI_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_zenoh_multi.yaml"
            DEPENDS "${MESH_TEST_DIR}/motor_zenoh_multi.scxml" "${MESH_TEST_DIR}/deploy_zenoh_multi.yaml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_ZENOH_MULTI_OUTPUT_DIR}/motor_zenoh_multi_sm.h"
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

    # ── SCE_MESH.md §14.4 pool compile verification ──────────────
    # pool_client exercises both pool branches of the mesh_transport.h
    # Jinja2 template in a single generated header:
    #   * Zenoh open pool — runtime KeyExpr assembled from the `id`
    #     <param> and the binding's `sce/player/{id}` key.
    #   * SOME/IP bounded pool — runtime instance id selected from the
    #     `inst` <param>, validated against `instances: [1, 2, 3]`.
    # Gated on both vsomeip3 and zenohcxx being discovered because the
    # fixture's deploy.yaml binds each transport for exactly one target.
    if(vsomeip3_FOUND AND zenohc_FOUND AND zenohcxx_FOUND)
        set(MESH_POOL_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/mesh_pool_generated")

        add_custom_command(
            OUTPUT "${MESH_POOL_OUTPUT_DIR}/pool_client_sm.h"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${MESH_POOL_OUTPUT_DIR}"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/pool_client.scxml" -l cpp -o "${MESH_POOL_OUTPUT_DIR}"
            DEPENDS "${MESH_TEST_DIR}/pool_client.scxml" "${SCE_CODEGEN}"
            COMMENT "Mesh pool: Generating pool_client_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_POOL_OUTPUT_DIR}/pool_client_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/pool_client.scxml" -l cpp -o "${MESH_POOL_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_pool.yaml"
            DEPENDS "${MESH_TEST_DIR}/pool_client.scxml" "${MESH_TEST_DIR}/deploy_pool.yaml"
                    "${MESH_TEST_DIR}/vsomeip_pool_controller.json"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_POOL_OUTPUT_DIR}/pool_client_sm.h"
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
                vsomeip3
                zenohcxx::zenohc
        )

        target_compile_features(mesh_pool_compile_test PRIVATE cxx_std_17)

        apply_sce_test_policies(mesh_pool_compile_test)

        add_test(NAME mesh_pool_compile_verification COMMAND mesh_pool_compile_test)
    else()
        message(STATUS "vsomeip3 / zenohcxx not both found — skipping mesh_pool_compile_test")
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
            DEPENDS "${MESH_TEST_DIR}/pool_zenoh_client.scxml" "${SCE_CODEGEN}"
            COMMENT "Mesh pool Zenoh: Generating pool_zenoh_client_sm.h"
            VERBATIM
        )

        add_custom_command(
            OUTPUT "${MESH_POOL_ZENOH_OUTPUT_DIR}/pool_zenoh_client_transport.h"
            COMMAND "${SCE_CODEGEN}" generate "${MESH_TEST_DIR}/pool_zenoh_client.scxml" -l cpp -o "${MESH_POOL_ZENOH_OUTPUT_DIR}"
                    --deploy "${MESH_TEST_DIR}/deploy_pool_zenoh.yaml"
            DEPENDS "${MESH_TEST_DIR}/pool_zenoh_client.scxml" "${MESH_TEST_DIR}/deploy_pool_zenoh.yaml"
                    "${MESH_TEST_DIR}/gamer_pool.scxml"
                    "${SCE_CODEGEN}" "${SCE_MESH_CPP_TEMPLATE}"
                    "${MESH_POOL_ZENOH_OUTPUT_DIR}/pool_zenoh_client_sm.h"
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

    # ── Phase 3 entry #5: deploy.yaml-only middleware switch demo ──────
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

    # Auto-deploy WASM files to visualizer/ directory after build
    add_custom_command(TARGET visualizer POST_BUILD
        COMMAND ${CMAKE_COMMAND} -E copy_if_different
                ${CMAKE_CURRENT_BINARY_DIR}/visualizer.wasm
                ${CMAKE_SOURCE_DIR}/visualizer/visualizer.wasm
        COMMAND ${CMAKE_COMMAND} -E copy_if_different
                ${CMAKE_CURRENT_BINARY_DIR}/visualizer.js
                ${CMAKE_SOURCE_DIR}/visualizer/visualizer.js
        COMMENT "Auto-deploying WASM files to visualizer/"
        VERBATIM
    )
    message(STATUS "   - WASM files will be automatically deployed after build")

    message(STATUS "✅ Interactive SCXML Visualizer configured")
    message(STATUS "   - Output: visualizer.wasm + visualizer.js")
    message(STATUS "   - SCXML loading: Dynamic fetch (no embedding)")
    message(STATUS "   - JavaScript API: createVisualizer() module")
endif()
