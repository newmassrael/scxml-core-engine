# SCE Mesh two-host test registration helper.
#
# Sister to two_process_test.cmake — wraps tests/mesh/run_two_host_fixture.sh
# so §9.6 cross-device fixtures whose transport requires a real network
# stack (SOME/IP-SD multicast, Zenoh peer-mesh discovery) can register
# without each test reimplementing the netns / handshake orchestration.
#
# Prerequisites (one-time per machine reboot, must run before ctest):
#   sudo tests/mesh/setup_crossdev_netns.sh
#
# For sudoless ctest runs (recommended, narrow scope matching the
# tc8-harness style on the same dev box), configure passwordless sudo
# once for the §9.6 scripts only:
#   sudo visudo
#   # Add (one line):
#   <user> ALL=(ALL) NOPASSWD: <repo>/tests/mesh/run_two_host_fixture.sh, \
#                              <repo>/tests/mesh/setup_crossdev_netns.sh, \
#                              <repo>/tests/mesh/cleanup_crossdev_netns.sh
# After that, plain `ctest` runs as the regular user — the orchestrator
# probes `sudo -ln <self>` and self-execs under sudo when allowed. Without
# either NOPASSWD coverage or a root invocation (`sudo ctest`), it exits
# 77, which ctest reports as `Skipped` via the SKIP_RETURN_CODE property
# below — so a fresh non-root checkout never sees a Failed result.
#
# Usage:
#   include(${PROJECT_SOURCE_DIR}/tests/cmake/two_host_test.cmake)
#   sce_register_two_host_mesh_test(
#       NAME    my_cross_device_fixture
#       WORKER  my_worker_test_binary
#       PARENT  my_parent_test_binary
#       [PARENT_NETNS sce-mesh-parent]
#       [WORKER_NETNS sce-mesh-worker]
#       [TIMEOUT 30])
#
# Parameters:
#   NAME           Test name as registered with ctest.
#   WORKER         Existing CMake target. Worker binary must `init()` its
#                  transport, write "LISTEN_READY\n" to stderr, then loop
#                  pumpScxmlInvokeRequests() until SIGTERM.
#   PARENT         Existing CMake target. Parent binary runs to natural
#                  completion (Pass / Fail) — its exit code is the test
#                  result.
#   PARENT_NETNS   netns the parent runs in (default: sce-mesh-parent —
#                  must match setup_crossdev_netns.sh).
#   WORKER_NETNS   netns the worker runs in (default: sce-mesh-worker).
#   TIMEOUT        ctest TIMEOUT override in seconds (default: 60).
function(sce_register_two_host_mesh_test)
    set(options)
    set(oneValueArgs NAME WORKER PARENT PARENT_NETNS WORKER_NETNS TIMEOUT)
    set(multiValueArgs)
    cmake_parse_arguments(THT "${options}" "${oneValueArgs}" "${multiValueArgs}" ${ARGN})

    if(NOT THT_NAME)
        message(FATAL_ERROR "sce_register_two_host_mesh_test: NAME is required")
    endif()
    if(NOT THT_WORKER)
        message(FATAL_ERROR "sce_register_two_host_mesh_test: WORKER target is required")
    endif()
    if(NOT THT_PARENT)
        message(FATAL_ERROR "sce_register_two_host_mesh_test: PARENT target is required")
    endif()
    if(NOT THT_PARENT_NETNS)
        set(THT_PARENT_NETNS "sce-mesh-parent")
    endif()
    if(NOT THT_WORKER_NETNS)
        set(THT_WORKER_NETNS "sce-mesh-worker")
    endif()

    add_test(
        NAME ${THT_NAME}
        COMMAND bash
                "${CMAKE_CURRENT_SOURCE_DIR}/mesh/run_two_host_fixture.sh"
                ${THT_PARENT_NETNS}
                ${THT_WORKER_NETNS}
                $<TARGET_FILE:${THT_WORKER}>
                $<TARGET_FILE:${THT_PARENT}>)

    if(THT_TIMEOUT)
        set_tests_properties(${THT_NAME} PROPERTIES TIMEOUT ${THT_TIMEOUT})
    else()
        set_tests_properties(${THT_NAME} PROPERTIES TIMEOUT 60)
    endif()

    # 77 maps to Skipped in ctest. The orchestrator emits it when running
    # as non-root or when the netns are absent — keeps `ctest` in a fresh
    # checkout from failing red before setup_crossdev_netns.sh has run.
    set_tests_properties(${THT_NAME} PROPERTIES SKIP_RETURN_CODE 77)
endfunction()
