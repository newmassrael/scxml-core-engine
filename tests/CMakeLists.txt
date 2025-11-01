cmake_minimum_required(VERSION 3.14)

# GoogleTest setup - FetchContent approach
find_package(GTest QUIET)

if(NOT GTest_FOUND)
    message(STATUS "GTest not found on system, using FetchContent to download")

    include(FetchContent)
    FetchContent_Declare(
        googletest
        GIT_REPOSITORY https://github.com/google/googletest.git
        GIT_TAG release-1.12.1
        GIT_SHALLOW TRUE
        GIT_PROGRESS TRUE
    )

    # GoogleTest options (reduce build time)
    set(gtest_force_shared_crt ON CACHE BOOL "" FORCE)
    set(BUILD_GMOCK ON CACHE BOOL "" FORCE)
    set(BUILD_GTEST ON CACHE BOOL "" FORCE)
    set(INSTALL_GTEST OFF CACHE BOOL "" FORCE)

    FetchContent_MakeAvailable(googletest)

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
# RSM test configuration: Disable std::cout usage
# ============================================================================
function(apply_rsm_test_policies target_name)
    # Force include DisableStdOut.h to all test targets
    target_compile_options(${target_name}
        PRIVATE
            -include ${CMAKE_SOURCE_DIR}/rsm/include/common/DisableStdOut.h
    )

    # RSM test policy definitions
    target_compile_definitions(${target_name}
        PRIVATE
            RSM_TEST_TARGET=1  # For test target identification
    )
endfunction()

# ============================================================================
# Performance Benchmarks (Optional)
# ============================================================================
# List of all test target names
set(RSM_TEST_TARGETS
    rsm_scxml_tests
    js_engine_tests
    state_machine_tests
    states_tests
    action_tests
    history_tests
    core_parser_tests
    integration_tests
    concurrency_tests
    txml_converter_tests
    w3c_test_cli
    w3c_full_runner
    w3c_simple_tests
    # static_codegen_tests  # Disabled: C++ codegen replaced by Python
    # simple_scxml_engine_tests  # Temporarily disabled
)

# ============================================================================
# 1. RSM SCXML Builtin Tests (currently working tests)
# ============================================================================
add_executable(rsm_scxml_tests
    simple_scxml_test.cpp
    core/TypeRegistryTest.cpp
    concurrency/TypeRegistryThreadSafetyTest.cpp
    common/EventDataHelperTest.cpp
)

target_include_directories(rsm_scxml_tests
    PRIVATE
        ${CMAKE_SOURCE_DIR}
        ${CMAKE_SOURCE_DIR}/rsm/include
        ${CMAKE_SOURCE_DIR}/tests
)

target_link_libraries(rsm_scxml_tests
    PRIVATE
        rsm_unified
        GTest::gtest_main
        pthread
)

# Apply RSM test policy (disable cout, etc.)
apply_rsm_test_policies(rsm_scxml_tests)

# CTest registration is handled below in optimized order

# ============================================================================
# 2. JavaScript Engine Tests (fully updated to RSM)
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
        ${CMAKE_SOURCE_DIR}/rsm/include
        ${CMAKE_SOURCE_DIR}/tests
)

target_link_libraries(js_engine_tests
    PRIVATE
        rsm_unified
        GTest::gtest_main
        pthread
)

# Apply RSM test policy
apply_rsm_test_policies(js_engine_tests)

# ============================================================================
# 3. StateMachine Tests
# ============================================================================
add_executable(state_machine_tests
    engine/StateMachineTest.cpp
)

target_include_directories(state_machine_tests
    PRIVATE
        ${CMAKE_SOURCE_DIR}
        ${CMAKE_SOURCE_DIR}/rsm/include
        ${CMAKE_SOURCE_DIR}/tests
)

target_link_libraries(state_machine_tests
    PRIVATE
        rsm_unified
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
        ${CMAKE_SOURCE_DIR}/rsm/include
        ${CMAKE_SOURCE_DIR}/tests
)

target_link_libraries(states_tests
    PRIVATE
        rsm_unified
        GTest::gtest_main
        pthread
)

# ============================================================================
# 4. Action System Tests (SCXML Executable Content)
# ============================================================================

add_executable(action_tests
    actions/ScriptActionTest.cpp
    actions/AssignActionTest.cpp
    runtime/ActionExecutorImplTest.cpp
    mocks/MockActionExecutor.cpp
    mocks/MockEventRaiser.cpp
)

target_include_directories(action_tests
    PRIVATE
        ${CMAKE_SOURCE_DIR}
        ${CMAKE_SOURCE_DIR}/rsm/include
        ${CMAKE_SOURCE_DIR}/tests
)

target_link_libraries(action_tests
    PRIVATE
        rsm_unified
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
        ${CMAKE_SOURCE_DIR}/rsm/include
        ${CMAKE_SOURCE_DIR}/tests
)

target_link_libraries(history_tests
    PRIVATE
        rsm_unified
        GTest::gtest_main
        pthread
)

# ============================================================================
# 4.6. ReadySCXMLEngine API Tests (High-level Facade) - TEMPORARILY DISABLED
# ============================================================================

# Temporarily disabled - will be re-enabled when converting existing tests
# add_executable(simple_scxml_engine_tests
#     api/ReadySCXMLEngineTest.cpp
# )
#
# target_include_directories(simple_scxml_engine_tests
#     PRIVATE
#         ${CMAKE_SOURCE_DIR}
#         ${CMAKE_SOURCE_DIR}/rsm/include
#         ${CMAKE_SOURCE_DIR}/tests
# )
#
# target_link_libraries(simple_scxml_engine_tests
#     PRIVATE
#         rsm_unified
#         GTest::gtest_main
#         pthread
# )

# ============================================================================
# 5. Core Parser Tests (RSM namespace updated)
# ============================================================================

# Use same libxml++ configuration as RSM
set(LIBXMLXX_INSTALL_DIR ${CMAKE_BINARY_DIR}/libxmlxx_install)
set(LIBXMLXX_INCLUDE_DIRS
    ${LIBXMLXX_INSTALL_DIR}/include/libxml++-5.0
    ${LIBXMLXX_INSTALL_DIR}/lib/libxml++-5.0/include
)
set(LIBXMLXX_LIBRARIES ${LIBXMLXX_INSTALL_DIR}/lib/libxml++-5.0.so)

add_executable(core_parser_tests
    core/SCXMLParserBasicTest.cpp
)

add_dependencies(core_parser_tests libxmlxx_external rsm_unified)

target_include_directories(core_parser_tests
    PRIVATE
        ${CMAKE_SOURCE_DIR}
        ${CMAKE_SOURCE_DIR}/rsm/include
        ${CMAKE_SOURCE_DIR}/tests
        ${LIBXMLXX_INCLUDE_DIRS}
        /usr/include/libxml2
)

target_link_libraries(core_parser_tests
    PRIVATE
        rsm_unified
        GTest::gtest_main
        ${LIBXMLXX_LIBRARIES}
        xml2
        pthread
)

# ============================================================================
# 5. Concurrent States Foundation Tests
# ============================================================================
# Disabled - test files removed
# add_executable(concurrent_foundation_tests
#     states/concurrent/foundation/ConcurrentStateFoundationTest.cpp
#     mocks/MockActionExecutor.cpp
# )

# ============================================================================
# 6. Concurrent States Orchestration Tests
# ============================================================================
# add_executable(concurrent_orchestration_tests
#     states/concurrent/orchestration/ParallelRegionOrchestratorTest.cpp
#     mocks/MockActionExecutor.cpp
# )

# ============================================================================
# 7. Concurrent Event Broadcasting Tests
# ============================================================================
# add_executable(concurrent_event_tests
#     states/concurrent/events/ConcurrentEventBroadcasterTest.cpp
#     mocks/MockActionExecutor.cpp
# )

# ============================================================================
# 8. Concurrent Completion Monitoring Tests
# ============================================================================
# add_executable(concurrent_monitoring_tests
#     states/concurrent/monitoring/ConcurrentCompletionMonitorTest.cpp
#     mocks/MockActionExecutor.cpp
# )

# ============================================================================
# 9. External Transition Handling Tests
# ============================================================================
# add_executable(external_transition_tests
#     states/concurrent/transitions/ExternalTransitionHandlerTest.cpp
#     mocks/MockActionExecutor.cpp
# )

# Disabled - test configurations removed

# ============================================================================
# 6. Integration Tests (RSM architecture)
# ============================================================================
add_executable(integration_tests
    integration/StateMachineIntegrationTest.cpp
    integration/ActionIntegrationTest.cpp
    integration/SCXMLEventTest.cpp
    integration/EventSchedulingTest.cpp
    integration/HttpEventTargetTest.cpp
    integration/SimpleMockHttpServer.cpp
    integration/ParallelStateIntegrationTest.cpp
    integration/SCXMLParallelParsingTest.cpp
    integration/ParallelStateEndToEndTest.cpp
    integration/SCXMLParallelComplianceTest.cpp
    integration/ConcurrentCompletionMonitoringTest.cpp
    integration/SCXMLForeachIntegrationTest.cpp
    integration/HistoryStateIntegrationTest.cpp
    integration/StaticCodegenIntegrationTest.cpp
    w3c/W3CHttpTestServer.cpp
    mocks/MockActionExecutor.cpp
    mocks/MockEventRaiser.cpp
)

add_dependencies(integration_tests libxmlxx_external rsm_unified)

target_include_directories(integration_tests
    PRIVATE
        ${CMAKE_SOURCE_DIR}
        ${CMAKE_SOURCE_DIR}/rsm/include
        ${CMAKE_SOURCE_DIR}/tests
        ${LIBXMLXX_INCLUDE_DIRS}
        /usr/include/libxml2
)

target_link_libraries(integration_tests
    PRIVATE
        rsm_unified
        GTest::gtest_main
        GTest::gmock
        ${LIBXMLXX_LIBRARIES}
        xml2
        pthread
)

# ============================================================================
# 6. Concurrency and Deadlock Tests
# ============================================================================
add_executable(concurrency_tests
    concurrency/DeadlockReproductionTest.cpp
    mocks/MockEventRaiser.cpp
)

target_include_directories(concurrency_tests
    PRIVATE
        ${CMAKE_SOURCE_DIR}
        ${CMAKE_SOURCE_DIR}/rsm/include
        ${CMAKE_SOURCE_DIR}/tests
)

target_link_libraries(concurrency_tests
    PRIVATE
        rsm_unified
        GTest::gtest_main
        GTest::gmock
        pthread
)

# ============================================================================
# 7. TXML Converter Tests (W3C SCXML Compliance Testing)
# ============================================================================
add_executable(txml_converter_tests
    w3c/TXMLConverterTest.cpp
    w3c/impl/TXMLConverter.cpp
)

target_include_directories(txml_converter_tests
    PRIVATE
        ${CMAKE_SOURCE_DIR}
        ${CMAKE_SOURCE_DIR}/rsm/include
        ${CMAKE_SOURCE_DIR}/tests
        ${LIBXMLXX_INCLUDE_DIRS}
        /usr/include/libxml2
)

target_link_libraries(txml_converter_tests
    PRIVATE
        rsm_unified
        GTest::gtest_main
        ${LIBXMLXX_LIBRARIES}
        xml2
        pthread
)

add_dependencies(txml_converter_tests libxmlxx_external rsm_unified)

# ============================================================================
# 8. W3C SCXML Compliance Tests (SOLID Architecture)
# ============================================================================
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
include(${CMAKE_SOURCE_DIR}/cmake/RSMStaticW3CTest.cmake)
rsm_generate_static_w3c_test(144 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 4.2: raise element inserts events at rear of internal queue (FIFO ordering) (Pure Static)
rsm_generate_static_w3c_test(147 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 4.3: if/elseif/else conditional execution (first true clause only) (Static Hybrid)
rsm_generate_static_w3c_test(148 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 4.3: else clause execution when if/elseif both false (Static Hybrid)
rsm_generate_static_w3c_test(149 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 4.3: if/elseif/else - neither clause executes (Static Hybrid)
rsm_generate_static_w3c_test(150 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 4.6: foreach declares new variable if item doesn't exist (Static Hybrid)
rsm_generate_static_w3c_test(151 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 4.6: foreach variable declaration persistence (Static Hybrid)
rsm_generate_static_w3c_test(152 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 4.6: foreach error handling - illegal array/item raises error.execution (Static Hybrid)
rsm_generate_static_w3c_test(153 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 4.6: foreach iteration order correctness (Static Hybrid)
rsm_generate_static_w3c_test(155 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 4.6: foreach executes content once per item (Static Hybrid)
rsm_generate_static_w3c_test(156 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 4.6: foreach stops execution on child action error (Static Hybrid)
rsm_generate_static_w3c_test(158 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.12.1: executable content executes in document order (Pure Static)
rsm_generate_static_w3c_test(159 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.12.1: error causes subsequent elements to be skipped (Static Hybrid)
rsm_generate_static_w3c_test(172 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.2.2: send eventexpr uses current value at evaluation time (Static Hybrid)
rsm_generate_static_w3c_test(173 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.2.3: send targetexpr uses current value at evaluation time (Static Hybrid)
rsm_generate_aot_test_header(174 "SIMPLE")  # Generate Test174.h from metadata.txt (Single Source of Truth)
rsm_generate_static_w3c_test(174 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.2.4: send typeexpr uses current value at evaluation time (Static Hybrid)
rsm_generate_static_w3c_test(175 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.2.5: send delayexpr uses current value at evaluation time (Static Hybrid)
rsm_generate_static_w3c_test(176 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.2.1: param uses current value at evaluation time (Static Hybrid)
rsm_generate_static_w3c_test(178 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.2.1: param - multiple key/value pairs with same keys (Static Hybrid)
rsm_generate_static_w3c_test(179 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.2.6: content element populates message body (Pure Static)
rsm_generate_static_w3c_test(183 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.2.7: send idlocation stores sendid value (Static Hybrid)
rsm_generate_static_w3c_test(185 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.2.5: send respects delay specification (Pure Static)
rsm_generate_static_w3c_test(186 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.2: send evaluates args at evaluation time, not send time (Static Hybrid)
rsm_generate_static_w3c_test(187 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.4.5: Delayed send cancellation on session termination (Pure Static)
rsm_generate_static_w3c_test(189 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10.1: Internal queue priority via target="#_internal" (Pure Static)
rsm_generate_static_w3c_test(190 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10.1: External queue priority via targetexpr (Static Hybrid)
rsm_generate_static_w3c_test(191 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.4.1: Inline <content> invoke with #_parent target (Pure Static)
rsm_generate_static_w3c_test(192 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.4: Inline <content> invoke with #_<invokeid> target (Pure Static)
rsm_generate_static_w3c_test(193 ${STATIC_W3C_OUTPUT_DIR})  # type attribute for queue routing (W3C SCXML 6.2.4)
rsm_generate_static_w3c_test(194 ${STATIC_W3C_OUTPUT_DIR})  # invalid target raises error.execution (W3C SCXML 6.2)
rsm_generate_static_w3c_test(198 ${STATIC_W3C_OUTPUT_DIR})  # default event processor type (W3C SCXML 6.2)
rsm_generate_static_w3c_test(199 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.2: unsupported send type raises error.execution (Pure Static)
rsm_generate_static_w3c_test(200 ${STATIC_W3C_OUTPUT_DIR})  # SCXML event processor support (W3C SCXML 6.2)
rsm_generate_static_w3c_test(201 ${STATIC_W3C_OUTPUT_DIR})  # BasicHTTP event processor (optional - W3C SCXML 6.2)
rsm_generate_static_w3c_test(205 ${STATIC_W3C_OUTPUT_DIR})  # inline <content>
rsm_generate_static_w3c_test(207 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.3: cancel tag - cannot cancel events in different sessions (invoke + delayed send)
rsm_generate_static_w3c_test(208 ${STATIC_W3C_OUTPUT_DIR})  # <cancel sendid="..."/> for delayed send cancellation (W3C SCXML 6.3)
rsm_generate_static_w3c_test(210 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.3: sendidexpr for dynamic cancel (Static Hybrid)
rsm_generate_static_w3c_test(215 ${STATIC_W3C_OUTPUT_DIR})  # typeexpr + inline <content>
rsm_generate_static_w3c_test(216 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.4: srcexpr runtime evaluation (Static Hybrid)
rsm_generate_static_w3c_test(220 ${STATIC_W3C_OUTPUT_DIR})  # inline <content>
rsm_generate_static_w3c_test(223 ${STATIC_W3C_OUTPUT_DIR})  # inline <content>
rsm_generate_static_w3c_test(224 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.3.1: invoke with idlocation (Static Hybrid)
rsm_generate_static_w3c_test(225 ${STATIC_W3C_OUTPUT_DIR})  # inline <content>
rsm_generate_static_w3c_test(226 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.2/6.4: auto-discovers test226sub1.txml
rsm_generate_static_w3c_test(228 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.3.1: invoke ID in done.invoke event (Static Hybrid)
rsm_generate_static_w3c_test(229 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.4.6: autoforward with inline content (Pure Static)
rsm_generate_static_w3c_test(230 ${STATIC_W3C_OUTPUT_DIR})  # inline <content> + autoforward (manual)
rsm_generate_static_w3c_test(232 ${STATIC_W3C_OUTPUT_DIR})  # inline <content>
rsm_generate_static_w3c_test(233 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.5: finalize before done.invoke event processing (Static Hybrid)
rsm_generate_static_w3c_test(234 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.4.6: finalize only in invoking state (Static Hybrid)
rsm_generate_static_w3c_test(235 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.4: done.invoke.id event with inline content child (Pure Static)
rsm_generate_static_w3c_test(236 ${STATIC_W3C_OUTPUT_DIR})  # inline <content>
rsm_generate_static_w3c_test(237 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.4: invoke cancellation on state exit with inline content child
rsm_generate_static_w3c_test(239 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.4: invoke with src attribute and inline content (Pure Static)
rsm_generate_static_w3c_test(240 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.4: invoke with namelist and param (Static Hybrid)
rsm_generate_static_w3c_test(241 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.4.3: invoke namelist + param consistency validation (Static Hybrid)
rsm_generate_static_w3c_test(242 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.4: invoke src + inline content consistency (Pure Static)
rsm_generate_static_w3c_test(243 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.4: invoke inline content + param with child→parent event sending (Pure Static)
rsm_generate_static_w3c_test(244 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.3.2: invoke inline content + namelist datamodel value passing (Static Hybrid)
rsm_generate_static_w3c_test(245 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.3.2: invoke namelist with non-existent variable handling (Static Hybrid)
rsm_generate_static_w3c_test(247 ${STATIC_W3C_OUTPUT_DIR})  # inline <content> + done.invoke
rsm_generate_static_w3c_test(250 ${STATIC_W3C_OUTPUT_DIR})  # inline <content> + cancellation (manual test)
rsm_generate_static_w3c_test(252 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.4: invoke cancellation ignores events from cancelled child (Pure Static)
rsm_generate_static_w3c_test(253 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML C.1: SCXML Event I/O Processor bidirectional communication with inline content child (Static Hybrid)
rsm_generate_static_w3c_test(276 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.2/6.4: auto-discovers test276sub1.txml
rsm_generate_static_w3c_test(277 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.3: datamodel init error.execution
rsm_generate_static_w3c_test(278 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10: datamodel in non-entered state (requires wrapper)
rsm_generate_static_w3c_test(279 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.2.2: early binding variable initialization
rsm_generate_static_w3c_test(280 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.3: late binding variable initialization
rsm_generate_static_w3c_test(286 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.3: error.execution for invalid assignment location
rsm_generate_static_w3c_test(287 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.4: valid assignment to valid location
rsm_generate_static_w3c_test(294 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.7.2: donedata with param and content (Static Hybrid)
rsm_generate_static_w3c_test(298 ${STATIC_W3C_OUTPUT_DIR})  # dynamic donedata (invalid param location requires runtime evaluation)
rsm_generate_static_w3c_test(301 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.8: script element download timeout rejection (manual)
rsm_generate_static_w3c_test(302 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.8: script evaluation at load time
rsm_generate_static_w3c_test(303 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.9: script execution in entry actions
rsm_generate_static_w3c_test(304 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.8: script-declared variables accessible in data model
rsm_generate_static_w3c_test(307 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.2.2: late binding variable access error handling (Static Hybrid)
rsm_generate_static_w3c_test(309 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.9.2: invalid boolean expressions treated as false
rsm_generate_static_w3c_test(310 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.9.1: In() predicate (requires Interpreter wrapper)
rsm_generate_static_w3c_test(311 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.9.2: error.execution for invalid assign location
rsm_generate_static_w3c_test(312 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.9.2: assignment error handling
rsm_generate_static_w3c_test(313 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.9.2: assign illegal expression error.execution
rsm_generate_static_w3c_test(314 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.9.2: delayed evaluation of assign illegal expression
rsm_generate_static_w3c_test(318 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10: _event variable binding and persistence
rsm_generate_static_w3c_test(319 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML B.1: _event system variable initially unbound
rsm_generate_static_w3c_test(321 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10.1: _sessionid system variable binding
rsm_generate_static_w3c_test(322 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML B.2.1: _sessionid immutability
rsm_generate_static_w3c_test(323 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML B.2.1: _name system variable binding
rsm_generate_static_w3c_test(324 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML B.2.2: _name system variable immutability
rsm_generate_static_w3c_test(325 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10: _ioprocessors system variable binding
rsm_generate_static_w3c_test(326 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML B.2.3: _ioprocessors system variable immutability
rsm_generate_static_w3c_test(329 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10: system variables immutability
rsm_generate_static_w3c_test(330 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10: event field binding (internal and external events)
rsm_generate_static_w3c_test(331 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10.1: event type classification (internal, platform, external)
rsm_generate_static_w3c_test(332 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10.1: _event.sendid field binding in error events, W3C SCXML 6.2.4: send idlocation attribute
rsm_generate_static_w3c_test(333 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10.1: _event.sendid field must be blank for non-error events without explicit sendid
rsm_generate_static_w3c_test(335 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10: _event.origin field must be blank for internal events
rsm_generate_static_w3c_test(336 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10.1: _event.origin field bidirectional communication via targetexpr
rsm_generate_static_w3c_test(337 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10.1: _event.origintype field must be blank for internal events
rsm_generate_static_w3c_test(338 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.4: Static invoke with inline content child SCXML
rsm_generate_static_w3c_test(339 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10: _event.invokeid field must be blank for non-invoked events
rsm_generate_static_w3c_test(342 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.2: eventexpr dynamic event name evaluation
rsm_generate_static_w3c_test(343 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10.1: Invalid param error.execution
rsm_generate_static_w3c_test(344 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.9: Invalid cond expression error.execution
rsm_generate_static_w3c_test(346 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.9.1: System variable protection validation
rsm_generate_static_w3c_test(347 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.4: SCXML Event I/O processor parent-child communication
rsm_generate_static_w3c_test(348 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.2: Send event parameter sets event name
rsm_generate_static_w3c_test(349 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10.1: _event.origin system variable
rsm_generate_static_w3c_test(350 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.6: Default initial state (first child in document order)
rsm_generate_static_w3c_test(351 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10.1: _event.sendid field validation
rsm_generate_static_w3c_test(352 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10.1: _event.origintype field validation
rsm_generate_static_w3c_test(354 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10: _event.data with namelist/param/content
rsm_generate_static_w3c_test(355 ${STATIC_W3C_OUTPUT_DIR})  # no initial state (default to first child in document order)
rsm_generate_static_w3c_test(364 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.6/3.4: Default initial states and parallel configurations (Pure Static)
rsm_generate_static_w3c_test(372 ${STATIC_W3C_OUTPUT_DIR})  # no initial state (done.state.parentid after final)
rsm_generate_static_w3c_test(375 ${STATIC_W3C_OUTPUT_DIR})  # no initial state (onentry execution order)
rsm_generate_static_w3c_test(376 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.8: Independent onentry handler execution (Static Hybrid)
rsm_generate_static_w3c_test(377 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.8: Multiple onexit handlers in document order (Pure Static)
rsm_generate_static_w3c_test(378 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.8/3.9: Independent onexit handler execution with error.execution (Static Hybrid)
rsm_generate_static_w3c_test(387 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.11: history states with default transitions
rsm_generate_static_w3c_test(388 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.11: deep and shallow history states
rsm_generate_static_w3c_test(396 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.12.1: event name matching and document order transition selection
rsm_generate_static_w3c_test(399 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.9.3: event name matching with prefix and wildcard
rsm_generate_static_w3c_test(401 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.12.1: internal event queue priority over external events
rsm_generate_static_w3c_test(402 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.12: error events processed like any other event
rsm_generate_static_w3c_test(403a ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.13: optimal enabled transition set - basic case
rsm_generate_static_w3c_test(403b ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.13: optimal enabled transition set as SET - parallel regions (needs wrapper)
rsm_generate_static_w3c_test(403c ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.13: optimal enabled transition set with preemption - parallel regions (needs wrapper)
rsm_generate_static_w3c_test(404 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.13: parallel state exit order with document order tie-breaking
rsm_generate_static_w3c_test(405 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.9/D.2: executable content execution order in transitions
rsm_generate_static_w3c_test(406 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.3: state entry order with parallel regions
rsm_generate_static_w3c_test(407 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.8: onexit handlers with datamodel variable updates
rsm_generate_static_w3c_test(409 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.12.1: state removal from active states during exit
rsm_generate_static_w3c_test(411 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.12.1: state addition to active states during entry
rsm_generate_static_w3c_test(412 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.3.2: initial transition executable content execution order
rsm_generate_static_w3c_test(413 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.13: parallel initial states with space-separated state IDs
rsm_generate_static_w3c_test(415 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.7.1: manual test (no Pass state)
rsm_generate_static_w3c_test(416 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.3.2: done.state.id event generation for compound states
rsm_generate_static_w3c_test(417 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.7.1: done.state event for parallel states
rsm_generate_static_w3c_test(419 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.13: eventless transition precedence over event-driven transitions
rsm_generate_static_w3c_test(421 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10.1: internal event priority over external events
rsm_generate_static_w3c_test(422 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.3: delayed send event scheduling
rsm_generate_static_w3c_test(423 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.9.2: external event queue processing
rsm_generate_static_w3c_test(436 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.9.2: In() predicate in null data model
rsm_generate_static_w3c_test(444 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.2/5.9: ECMAScript datamodel variable in condition
rsm_generate_static_w3c_test(445 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML B.2.2: ECMAScript undefined variable initialization
rsm_generate_static_w3c_test(446 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.2.2: external file loading with src attribute
rsm_generate_static_w3c_test(448 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML B.2: ECMAScript single global scope for all variables
rsm_generate_static_w3c_test(449 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML B.2: ECMAScript boolean conversion for string literals
rsm_generate_static_w3c_test(451 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.9.2: In() predicate in parallel states
rsm_generate_static_w3c_test(452 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.3/5.4: datamodel substructure assignment (object properties)
rsm_generate_static_w3c_test(453 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML B.2/3.12.1: ECMAScript function expression evaluation
rsm_generate_static_w3c_test(456 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML B.2/5.9: ECMAScript script element execution
rsm_generate_static_w3c_test(457 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.4/D.3.1: foreach error handling with ECMAScript validation
rsm_generate_static_w3c_test(459 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.4: foreach iteration order and index validation
rsm_generate_static_w3c_test(460 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 4.6: foreach shallow copy semantics with array modification
rsm_generate_static_w3c_test(525 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 4.6: foreach shallow copy semantics (minimal)
rsm_generate_static_w3c_test(487 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.3/5.4: illegal assignment error.execution event
rsm_generate_static_w3c_test(488 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.7: error handling in donedata param expressions
rsm_generate_static_w3c_test(527 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.5: donedata content expr evaluation
rsm_generate_static_w3c_test(528 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.5: donedata content expr error.execution
rsm_generate_static_w3c_test(529 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.5: donedata content integer literal
rsm_generate_static_w3c_test(530 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.4: Hybrid invoke contentexpr evaluation (Static Hybrid)
rsm_generate_static_w3c_test(495 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.2: SCXML Event I/O Processor internal vs external queue handling
rsm_generate_static_w3c_test(496 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML C.1: error.communication for unreachable target (Static Hybrid)
rsm_generate_static_w3c_test(500 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.3.1: SCXML Event I/O processor location field access (Static Hybrid)
rsm_generate_static_w3c_test(501 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML C.1: SCXML Event I/O Processor location field as send target (Static Hybrid)
rsm_generate_static_w3c_test(503 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.13: targetless transitions do not exit/re-enter source state (Static Hybrid)
rsm_generate_static_w3c_test(504 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.13: external transition exit sets (LCCA) (Static Hybrid)
rsm_generate_static_w3c_test(505 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.13: internal transition does not exit source state (Static Hybrid)
rsm_generate_static_w3c_test(506 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.9.2: internal transition with non-descendant target behaves as external (Static Hybrid)
rsm_generate_static_w3c_test(509 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML C.2: BasicHTTP Event I/O Processor POST method validation (Static Hybrid)
rsm_generate_static_w3c_test(510 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML C.2: BasicHTTP messages go to external queue (Static Hybrid)
rsm_generate_static_w3c_test(513 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML C.2: manual test - BasicHTTP success response (Integration test only)
rsm_generate_static_w3c_test(518 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML C.2: BasicHTTP namelist encoding as POST parameters (Static Hybrid)
rsm_generate_static_w3c_test(519 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML C.2: BasicHTTP param encoding as POST parameters (Static)
rsm_generate_static_w3c_test(520 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML C.2: BasicHTTP content element as message body (Static)
rsm_generate_static_w3c_test(521 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.2.4/6.2.5: error.communication on invalid targetexpr (Static Hybrid)
rsm_generate_static_w3c_test(522 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML C.2: BasicHTTP location field usage (Pure Static)
rsm_generate_static_w3c_test(531 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML C.2: _scxmleventname param overrides event name (Pure Static)
rsm_generate_static_w3c_test(532 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML C.2: HTTP method name fallback for event naming (Pure Static)
rsm_generate_static_w3c_test(533 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.13: internal transition exit set for non-compound states (Static Hybrid)
rsm_generate_static_w3c_test(534 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML C.2: BasicHTTP _scxmleventname parameter transmission (Static Hybrid)
rsm_generate_static_w3c_test(550 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.2.2: early binding with expr attribute for data variable (Static Hybrid)
rsm_generate_static_w3c_test(551 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.2.2: early binding with inline content (Static Hybrid)
rsm_generate_static_w3c_test(552 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.2.2: external data loading via src attribute (Static Hybrid)
rsm_generate_static_w3c_test(553 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.2.4 & 5.11: send namelist error handling (Static Hybrid)
rsm_generate_static_w3c_test(554 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 6.4 & B.1: invoke with invalid namelist error handling (Pure Static)
rsm_generate_static_w3c_test(557 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML B.2: ECMAScript XML DOM assignment with inline and file content (Static Hybrid)
rsm_generate_static_w3c_test(558 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML B.2: ECMAScript whitespace normalization for non-XML content (Static Hybrid)
rsm_generate_static_w3c_test(560 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.10: _event.data param structure in ECMAScript datamodel (Static Hybrid)
rsm_generate_static_w3c_test(561 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.9.2: ECMAScript DOM object for XML event data (Static Hybrid)
rsm_generate_static_w3c_test(562 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.9.2: ECMAScript content space normalization (Static Hybrid)
rsm_generate_static_w3c_test(567 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML C.2: BasicHTTP param encoding in _event.data (Static Hybrid)
rsm_generate_static_w3c_test(569 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.8: SCXML Event I/O processor location field (Static Hybrid)
rsm_generate_static_w3c_test(570 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.12.1: Parallel state completion with done.state.id (Static Hybrid)
rsm_generate_static_w3c_test(576 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.13: Parallel initial with space-separated states
rsm_generate_static_w3c_test(577 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML C.2: error.communication on BasicHTTP send with no target
rsm_generate_static_w3c_test(578 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.9/5.10: ECMAScript _event.data JSON object (Static Hybrid)
rsm_generate_static_w3c_test(579 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.8: history default content execution (Static Hybrid)
rsm_generate_static_w3c_test(580 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 3.11: history state not in configuration (parallel)

# ============================================================================
# W3C Test Registry: Single Source of Truth for AOT vs Interpreter Classification
# ============================================================================
#
# ARCHITECTURE: Automatic Test Classification System
#
# Benefits:
# - Single point of control (this file only)
# - Automatic file management (no manual Test*.h, AllAotTests.h edits)
# - Easy test migration (move number between lists)
# ===========================================================================
# W3C Test Registry Architecture
# ===========================================================================
# SINGLE SOURCE OF TRUTH: This section defines all test classifications.
# - W3C_AOT_TESTS: Tests that can run on Pure Static or Static Hybrid AOT engine
# - W3C_INTERPRETER_ONLY_TESTS: Tests requiring Interpreter (dynamic features)
#
# WORKFLOW:
# 1. To add AOT test: Add test number to W3C_AOT_TESTS + create Test*.h in aot_tests/
# 2. To migrate test: Move test number between lists, re-run cmake
# 3. Validate test: Run /validate-test-execution [test-number] to verify correctness
#    - Checks test intent (resources/*/metadata.txt, *.txml)
#    - Compares Interpreter vs AOT execution logs
#    - Detects false positives in both engines
#    - See .claude/commands/validate-test-execution.md for details
# 4. W3CTestRegistry.cpp auto-generates from these lists
#
# FILES GENERATED:
# - build/tests/w3c/W3CTestRegistry.cpp (from W3CTestRegistry.cpp.in template)
# - tests/w3c/runners/W3CTestRunner_Test*.cpp (CMake generates list, files pre-committed)
# ===========================================================================

# AOT Tests: Pure Static or Static Hybrid (only tests with runner files)
set(W3C_AOT_TESTS
    144 147 148 149 150 151 152 153 155 156 158 159
    172 173 174 175 176 178 179 183 185 186 187 189 190 191 192 193 194 198 199 200 201 205 207 208 210
    215 216 220 223 224 225 226 228 229 230 232 233 234 235 236 237 239 240 241 242 243 244 245 247 250
    276 277 278 279 280 286 287 294 298
    301 302 303 304 307 309 310 311 312 313 314 318 319
    321 322 323 324 325 326 329 330 331 332 333 335 336 337 338 339
    342 343 344 346 347 348 349 350 351 352 354 355 364 372 375 376 377 378
    387 388 396 399
    401 402 403a 403b 403c 404 405 406 407 409 411 412 413 415 416 417 419 421 422 423
    436 444 445 446 448 449 451 452 453 456 457 459 460
    487 488 495 496 500 501 503 504 505 506 509 510 513 518 519 520 521 522 525 527 528 529 530 531 532 533 534
    550 551 552 553 554 557 558 560 561 562 567 569 570 576 577 578 579 580
    252 253
)

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

# Full W3C Test Runner for all W3C SCXML 1.0 compliance tests
add_executable(w3c_test_cli
    w3c/W3CTestCLI.cpp
    w3c/W3CTestRunner.cpp
    ${CMAKE_CURRENT_BINARY_DIR}/w3c/W3CTestRegistry.cpp  # Auto-generated test registry
    ${W3C_AOT_RUNNER_SOURCES}  # Explicit AOT test runners (from W3C_AOT_TESTS list)
    w3c/impl/TXMLConverter.cpp
    w3c/impl/TestMetadataParser.cpp
    w3c/W3CHttpTestServer.cpp
    w3c/impl/W3CTestEventDispatcher.cpp
    mocks/MockEventRaiser.cpp
    ${GENERATED_W3C_HEADERS}  # Trigger generation of static test headers for AOT engine
)

target_link_libraries(w3c_test_cli
    PRIVATE
        rsm_unified
        ${LIBXMLXX_LIBRARIES}
        xml2
        pthread
)

target_include_directories(w3c_test_cli
    PRIVATE
        ${CMAKE_SOURCE_DIR}
        ${CMAKE_SOURCE_DIR}/rsm/include
        ${CMAKE_SOURCE_DIR}/tests
        ${CMAKE_SOURCE_DIR}/tests/w3c  # AOT test registry headers
        ${CMAKE_CURRENT_BINARY_DIR}/w3c  # Generated W3CTestRegistry.cpp
        ${STATIC_W3C_OUTPUT_DIR}  # Include generated static test headers
        ${LIBXMLXX_INCLUDE_DIRS}
        /usr/include/libxml2
)

add_dependencies(w3c_test_cli libxmlxx_external rsm_unified)

# ============================================================================
# 9. Static Code Generation Tests (TDD for Static Compiler)
# ============================================================================
# NOTE: C++ code generator replaced by Python + Jinja2 implementation
# These tests are disabled as they depend on C++ StaticCodeGenerator
# TODO: Create Python codegen tests
# add_executable(static_codegen_tests
#     codegen/test_static_codegen.cpp
#     ../tools/codegen/StaticCodeGenerator.cpp
# )
#
# target_include_directories(static_codegen_tests
#     PRIVATE
#         ${CMAKE_SOURCE_DIR}
#         ${CMAKE_SOURCE_DIR}/rsm/include
#         ${CMAKE_SOURCE_DIR}/tools/codegen
#         ${CMAKE_SOURCE_DIR}/tests
#         ${LIBXMLXX_INCLUDE_DIRS}
#         /usr/include/libxml2
# )
#
# target_link_libraries(static_codegen_tests
#     PRIVATE
#         rsm_unified
#         GTest::gtest_main
#         ${LIBXMLXX_LIBRARIES}
#         xml2
#         pthread
# )
#
# add_dependencies(static_codegen_tests libxmlxx_external rsm_unified)

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
add_test(NAME RSMSCXMLTests COMMAND rsm_scxml_tests)
# add_test(NAME StaticCodegenTests COMMAND static_codegen_tests)  # Disabled: C++ codegen replaced by Python
# add_test(NAME ReadySCXMLEngineTests COMMAND simple_scxml_engine_tests)  # Temporarily disabled

# Priority Level 3: Integration and concurrency tests (system-level validation, ~5-15s)
add_test(NAME TXMLConverterTests COMMAND txml_converter_tests)
add_test(NAME ConcurrencyTests COMMAND concurrency_tests)
add_test(NAME IntegrationTests COMMAND integration_tests)

# Priority Level 4: W3C SCXML compliance validation (comprehensive validation, ~40s)
add_test(NAME W3CFullComplianceTests COMMAND w3c_test_cli)

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
set_tests_properties(RSMSCXMLTests PROPERTIES COST 4000)
# set_tests_properties(StaticCodegenTests PROPERTIES COST 4000)  # Disabled

# Priority Level 3: COST 3000
set_tests_properties(TXMLConverterTests PROPERTIES COST 3000)
set_tests_properties(ConcurrencyTests PROPERTIES COST 3000)
set_tests_properties(IntegrationTests PROPERTIES COST 3000)

# Priority Level 4: COST 2500 (W3C SCXML compliance validation, higher priority than benchmarks)
set_tests_properties(W3CFullComplianceTests PROPERTIES COST 2500)

# Priority Level 5: Performance benchmarks (measure performance after functional validation, execute last)
# Benchmark subdirectory must be included AFTER all main tests to ensure proper execution order
option(BUILD_BENCHMARKS "Build performance benchmarks" ON)
if(BUILD_BENCHMARKS)
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
set_tests_properties(RSMSCXMLTests PROPERTIES TIMEOUT 30)
# set_tests_properties(StaticCodegenTests PROPERTIES TIMEOUT 30)  # Disabled
# set_tests_properties(ReadySCXMLEngineTests PROPERTIES TIMEOUT 30)  # Temporarily disabled
set_tests_properties(JSEngineTests CoreParserTests StateMachineTests PROPERTIES TIMEOUT 60)
set_tests_properties(ActionTests HistoryTests StatesTests PROPERTIES TIMEOUT 90)
set_tests_properties(IntegrationTests ConcurrencyTests PROPERTIES TIMEOUT 180)
set_tests_properties(TXMLConverterTests PROPERTIES TIMEOUT 120)
set_tests_properties(W3CFullComplianceTests PROPERTIES TIMEOUT 300)

# Limit parallel execution (resource-intensive tests)
set_tests_properties(W3CFullComplianceTests PROPERTIES RUN_SERIAL TRUE)
set_tests_properties(ConcurrencyTests PROPERTIES RUN_SERIAL TRUE)

# Apply RSM policy to all test targets
foreach(test_target ${RSM_TEST_TARGETS})
    if(TARGET ${test_target})
        apply_rsm_test_policies(${test_target})
        message(STATUS "✅ Applied RSM policies (cout prohibition) to ${test_target}")
    endif()
endforeach()

message(STATUS "✅ RSM SCXML tests enabled")
message(STATUS "✅ JavaScript Engine tests enabled")
message(STATUS "✅ Core Parser tests enabled")
message(STATUS "✅ Integration tests enabled")
message(STATUS "🚫 std::cout usage prohibited in all test targets")
