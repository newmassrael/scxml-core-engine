# SCE Mesh two-process test registration helper (SCE_MESH.md §16.8.3).
#
# Wraps tests/mesh/run_two_process_fixture.sh so cross-device fixtures
# can declare a worker+parent pair without reimplementing the handshake
# / teardown orchestration each time. The helper is intended for mesh
# harness tests that need a real OS-process boundary between the dialing
# and listening halves of a custom_tcp transport — ephemeral-port based
# via Server::local_endpoint() so the parent connects to the port the
# kernel assigned, not one baked into deploy.yaml.
#
# Usage:
#   include(${PROJECT_SOURCE_DIR}/tests/cmake/two_process_test.cmake)
#   sce_register_two_process_mesh_test(
#       NAME    my_cross_device_fixture
#       WORKER  my_worker_test_binary
#       PARENT  my_parent_test_binary
#       [TIMEOUT 30])
#
# Parameters:
#   NAME     Test name as registered with ctest.
#   WORKER   Existing CMake target that produces the worker binary. The
#            binary MUST bind its Server to "127.0.0.1:0" and emit one
#            line "LISTEN_ENDPOINT=host:port" to stderr after readback,
#            then block until SIGTERM.
#   PARENT   Existing CMake target that produces the parent binary. The
#            binary reads MESH_PEER_ENDPOINT from env (set by the script
#            after reading it off the worker's stderr) and uses it as the
#            connect endpoint — typically via
#            TransportRouter::init(PortOverride{ {{"worker", endpoint}} }).
#   TIMEOUT  Optional ctest TIMEOUT override in seconds. Default = 60.
function(sce_register_two_process_mesh_test)
    set(options)
    set(oneValueArgs NAME WORKER PARENT TIMEOUT)
    set(multiValueArgs)
    cmake_parse_arguments(TPT "${options}" "${oneValueArgs}" "${multiValueArgs}" ${ARGN})

    if(NOT TPT_NAME)
        message(FATAL_ERROR "sce_register_two_process_mesh_test: NAME is required")
    endif()
    if(NOT TPT_WORKER)
        message(FATAL_ERROR "sce_register_two_process_mesh_test: WORKER target is required")
    endif()
    if(NOT TPT_PARENT)
        message(FATAL_ERROR "sce_register_two_process_mesh_test: PARENT target is required")
    endif()

    add_test(
        NAME ${TPT_NAME}
        COMMAND bash
                "${CMAKE_CURRENT_SOURCE_DIR}/mesh/run_two_process_fixture.sh"
                $<TARGET_FILE:${TPT_WORKER}>
                $<TARGET_FILE:${TPT_PARENT}>)

    if(TPT_TIMEOUT)
        set_tests_properties(${TPT_NAME} PROPERTIES TIMEOUT ${TPT_TIMEOUT})
    else()
        set_tests_properties(${TPT_NAME} PROPERTIES TIMEOUT 60)
    endif()
endfunction()
