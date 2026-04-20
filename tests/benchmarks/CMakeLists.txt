cmake_minimum_required(VERSION 3.14)

# ============================================================================
# Dependencies
# ============================================================================
find_package(Threads REQUIRED)

# ============================================================================
# Google Benchmark Setup
# ============================================================================
find_package(benchmark QUIET)

if(NOT benchmark_FOUND)
    message(STATUS "Google Benchmark not found, using FetchContent to download")

    include(FetchContent)
    FetchContent_Declare(
        googlebenchmark
        GIT_REPOSITORY https://github.com/google/benchmark.git
        GIT_TAG v1.8.3
        GIT_SHALLOW TRUE
        GIT_PROGRESS TRUE
    )

    # Benchmark options
    set(BENCHMARK_ENABLE_TESTING OFF CACHE BOOL "" FORCE)
    set(BENCHMARK_ENABLE_INSTALL OFF CACHE BOOL "" FORCE)
    set(BENCHMARK_ENABLE_GTEST_TESTS OFF CACHE BOOL "" FORCE)
    set(BENCHMARK_DOWNLOAD_DEPENDENCIES ON CACHE BOOL "" FORCE)

    FetchContent_MakeAvailable(googlebenchmark)
else()
    message(STATUS "Using system-installed Google Benchmark")
endif()

# ============================================================================
# Benchmark Executable
# ============================================================================

# EventScheduler Benchmarks
add_executable(benchmark_event_scheduler
    EventSchedulerBenchmark.cpp
)
target_include_directories(benchmark_event_scheduler
    PRIVATE
        ${CMAKE_SOURCE_DIR}/sce/src
)
target_link_libraries(benchmark_event_scheduler
    PRIVATE
        sce_runtime
        benchmark::benchmark
        benchmark::benchmark_main
        Threads::Threads
)

# JSEngine Benchmarks
add_executable(benchmark_jsengine
    JSEngineBenchmark.cpp
)
target_link_libraries(benchmark_jsengine
    PRIVATE
        sce_runtime
        benchmark::benchmark
        benchmark::benchmark_main
        Threads::Threads
)

# StateMachine Benchmarks
add_executable(benchmark_statemachine
    StateMachineBenchmark.cpp
)
target_link_libraries(benchmark_statemachine
    PRIVATE
        sce_runtime
        benchmark::benchmark
        benchmark::benchmark_main
        Threads::Threads
)

# Async Dispatcher Benchmarks
add_executable(benchmark_async_dispatcher
    AsyncDispatcherBenchmark.cpp
)
target_link_libraries(benchmark_async_dispatcher
    PRIVATE
        sce_runtime
        benchmark::benchmark
        benchmark::benchmark_main
        Threads::Threads
)

# Timer Accuracy Benchmarks
add_executable(benchmark_timer_accuracy
    TimerAccuracyBenchmark.cpp
)
target_link_libraries(benchmark_timer_accuracy
    PRIVATE
        sce_runtime
        benchmark::benchmark
        benchmark::benchmark_main
        Threads::Threads
)

# LuaEngine Benchmarks
add_executable(benchmark_luaengine
    LuaEngineBenchmark.cpp
)
target_link_libraries(benchmark_luaengine
    PRIVATE
        sce_runtime
        benchmark::benchmark
        benchmark::benchmark_main
        Threads::Threads
)

# Memory Footprint Benchmarks
add_executable(benchmark_memory_footprint
    MemoryFootprintBenchmark.cpp
)
target_link_libraries(benchmark_memory_footprint
    PRIVATE
        sce_runtime
        benchmark::benchmark
        benchmark::benchmark_main
        Threads::Threads
)

# ParallelCompletionTracker Benchmarks (SCE_MESH.md §16.5 std::function
# footprint audit — justifies or refutes a CRTP/template refactor of
# the tracker's callback members).
if(SCE_ENABLE_MESH)
    add_executable(benchmark_parallel_completion_tracker
        ParallelCompletionTrackerBenchmark.cpp
    )
    target_link_libraries(benchmark_parallel_completion_tracker
        PRIVATE
            sce_mesh_common
            benchmark::benchmark
            benchmark::benchmark_main
            Threads::Threads
    )
endif()

# ============================================================================
# CTest Integration
# ============================================================================

# Quick benchmark test (default, runs with ctest)
add_test(
    NAME benchmark_event_scheduler_quick
    COMMAND benchmark_event_scheduler
        --benchmark_min_time=0.1s
        --benchmark_filter=Quick
        --benchmark_format=console
)

# Full benchmark test (disabled by default, manual execution only)
add_test(
    NAME benchmark_event_scheduler_full
    COMMAND benchmark_event_scheduler
)

# JSEngine quick benchmark test
add_test(
    NAME benchmark_jsengine_quick
    COMMAND benchmark_jsengine
        --benchmark_min_time=0.1s
        --benchmark_filter="SessionCreation|SessionLookup|SimpleExpression"
        --benchmark_format=console
)

# JSEngine full benchmark test
add_test(
    NAME benchmark_jsengine_full
    COMMAND benchmark_jsengine
)

# StateMachine quick benchmark test
add_test(
    NAME benchmark_statemachine_quick
    COMMAND benchmark_statemachine
        --benchmark_min_time=0.1s
        --benchmark_filter="EventProcessing|SimpleTransition|ParallelStateEvent"
        --benchmark_format=console
)

# StateMachine full benchmark test
add_test(
    NAME benchmark_statemachine_full
    COMMAND benchmark_statemachine
)

# Async Dispatcher quick benchmark test
add_test(
    NAME benchmark_async_dispatcher_quick
    COMMAND benchmark_async_dispatcher
        --benchmark_min_time=0.1s
        --benchmark_filter="SingleThread|PostEventOverhead|EnqueueLatency"
        --benchmark_format=console
)

# Timer Accuracy quick benchmark test
add_test(
    NAME benchmark_timer_accuracy_quick
    COMMAND benchmark_timer_accuracy
        --benchmark_min_time=0.1s
        --benchmark_filter="OneShotAccuracy/10|StartOverhead|StopOverhead"
        --benchmark_format=console
)

# LuaEngine quick benchmark test
add_test(
    NAME benchmark_luaengine_quick
    COMMAND benchmark_luaengine
        --benchmark_min_time=0.1s
        --benchmark_filter="SessionCreation|SessionLookup|SimpleExpression|NativeSimpleExpression"
        --benchmark_format=console
)

# LuaEngine full benchmark test
add_test(
    NAME benchmark_luaengine_full
    COMMAND benchmark_luaengine
)

# Memory Footprint quick benchmark test
add_test(
    NAME benchmark_memory_footprint_quick
    COMMAND benchmark_memory_footprint
        --benchmark_filter="MinimalStateMachine|SmallStateMachine|StdThreadDispatcher"
        --benchmark_format=console
)

# ParallelCompletionTracker quick benchmark test — exercises the
# sizeof report + the narrowest width (2) of the timing paths so a
# developer can sanity-check the numbers in under a second.
if(SCE_ENABLE_MESH)
    add_test(
        NAME benchmark_parallel_completion_tracker_quick
        COMMAND benchmark_parallel_completion_tracker
            --benchmark_min_time=0.1s
            --benchmark_filter=Sizeof|CaptureAudit|/2$
            --benchmark_format=console
    )
    add_test(
        NAME benchmark_parallel_completion_tracker_full
        COMMAND benchmark_parallel_completion_tracker
    )
endif()

# Configure test properties
# CTest executes tests with higher COST first, so quick (1000) runs before full (500)
set_tests_properties(benchmark_event_scheduler_quick PROPERTIES
    LABELS "benchmark;quick;performance"
    TIMEOUT 300
    COST 1000  # Runs before full benchmark
)

# Disable full benchmark from default ctest run
set_tests_properties(benchmark_event_scheduler_full PROPERTIES
    LABELS "benchmark;full;performance;manual"
    TIMEOUT 600
    COST 500  # Lower cost = runs after quick benchmark
    DISABLED TRUE  # Excluded from default ctest execution
)

set_tests_properties(benchmark_jsengine_quick PROPERTIES
    LABELS "benchmark;quick;performance;jsengine"
    TIMEOUT 300
    COST 1000
)

set_tests_properties(benchmark_jsengine_full PROPERTIES
    LABELS "benchmark;full;performance;jsengine;manual"
    TIMEOUT 600
    COST 500
    DISABLED TRUE
)

set_tests_properties(benchmark_statemachine_quick PROPERTIES
    LABELS "benchmark;quick;performance;statemachine"
    TIMEOUT 300
    COST 1000
)

set_tests_properties(benchmark_statemachine_full PROPERTIES
    LABELS "benchmark;full;performance;statemachine;manual"
    TIMEOUT 600
    COST 500
    DISABLED TRUE
)

set_tests_properties(benchmark_async_dispatcher_quick PROPERTIES
    LABELS "benchmark;quick;performance;async;dispatcher"
    TIMEOUT 300
    COST 1000
)

set_tests_properties(benchmark_timer_accuracy_quick PROPERTIES
    LABELS "benchmark;quick;performance;timer;accuracy"
    TIMEOUT 300
    COST 1000
)

set_tests_properties(benchmark_luaengine_quick PROPERTIES
    LABELS "benchmark;quick;performance;luaengine"
    TIMEOUT 300
    COST 1000
)

set_tests_properties(benchmark_luaengine_full PROPERTIES
    LABELS "benchmark;full;performance;luaengine;manual"
    TIMEOUT 600
    COST 500
    DISABLED TRUE
)

set_tests_properties(benchmark_memory_footprint_quick PROPERTIES
    LABELS "benchmark;quick;performance;memory"
    TIMEOUT 300
    COST 1000
)

if(SCE_ENABLE_MESH)
    set_tests_properties(benchmark_parallel_completion_tracker_quick PROPERTIES
        LABELS "benchmark;quick;performance;mesh;tracker"
        TIMEOUT 300
        COST 1000
    )
    set_tests_properties(benchmark_parallel_completion_tracker_full PROPERTIES
        LABELS "benchmark;full;performance;mesh;tracker;manual"
        TIMEOUT 600
        COST 500
        DISABLED TRUE
    )
endif()

# ============================================================================
# Custom Benchmark Targets
# ============================================================================

# Run benchmark with standard settings
add_custom_target(run_benchmarks
    COMMAND benchmark_event_scheduler --benchmark_format=console
    DEPENDS benchmark_event_scheduler
    WORKING_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}
)

# Generate JSON results
add_custom_target(benchmark_json
    COMMAND ${CMAKE_COMMAND} -E make_directory ${CMAKE_BINARY_DIR}/benchmark_results
    COMMAND benchmark_event_scheduler --benchmark_format=json > ${CMAKE_BINARY_DIR}/benchmark_results/event_scheduler.json
    DEPENDS benchmark_event_scheduler
    WORKING_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}
)

# Quick performance check (fewer iterations)
add_custom_target(benchmark_quick
    COMMAND benchmark_event_scheduler --benchmark_min_time=0.1s --benchmark_filter=Quick
    DEPENDS benchmark_event_scheduler
    WORKING_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}
)

# QuickJS vs Lua engine comparison
add_custom_target(benchmark_compare_engines
    COMMAND ${CMAKE_COMMAND} -E echo "======== QuickJS (JSEngine) ========"
    COMMAND benchmark_jsengine
        "--benchmark_filter=SimpleExpression$$|VariableOperations$$|ScriptExecutionLatency$$|SessionCreation$$|SessionLookup$$"
        --benchmark_format=console
    COMMAND ${CMAKE_COMMAND} -E echo ""
    COMMAND ${CMAKE_COMMAND} -E echo "======== Lua 5.4 (LuaEngine) ========"
    COMMAND benchmark_luaengine
        "--benchmark_filter=SimpleExpression$$|VariableOperations$$|ScriptExecutionLatency$$|NativeSimpleExpression$$|NativeMathExpression$$|NativeScriptExecutionLatency$$|SessionCreation$$|SessionLookup$$"
        --benchmark_format=console
    DEPENDS benchmark_jsengine benchmark_luaengine
    WORKING_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}
)
