# mesh_https.cmake
# The C++ tests of the Mesh https binding (docs/adr/0107, D2). Included from
# tests/CMakeLists.txt inside its SCE_ENABLE_MESH block, where MESH_TEST_DIR
# and apply_sce_test_policies are defined.
#
#   mesh_https_client_verification
#       The client (mesh/transports/HttpsClient.h) against a mailbox in this
#       process and against scripted answers: no network, every platform.
#   mesh_https_curl_adapter_conformance
#       The libcurl adapter held to the conformance kit, when the adapter is
#       built (cmake/SCEMeshHttps.cmake) and OpenSSL, the kit's TLS, is found.
#   mesh_https_winhttp_adapter_conformance
#       The WinHTTP adapter held to the kit, on Windows, on the same terms.
#       Its cases write the local machine's Root store, so it runs as an
#       administrator (a hosted Windows runner is one).

if(NOT TARGET Threads::Threads)
    find_package(Threads REQUIRED)
endif()

# The wire's other half in this process: the mailbox a server keeps for one
# client, and the port answered by it with no socket.
add_library(sce_mesh_https_mailbox STATIC
    ${MESH_TEST_DIR}/https_kit/Mailbox.cpp
)
target_include_directories(sce_mesh_https_mailbox
    PUBLIC
        ${MESH_TEST_DIR}
        ${CMAKE_SOURCE_DIR}/sce/include
)
target_link_libraries(sce_mesh_https_mailbox
    PUBLIC
        sce_mesh_common
        Threads::Threads
)
target_compile_features(sce_mesh_https_mailbox PUBLIC cxx_std_17)

add_executable(mesh_https_client_test
    ${MESH_TEST_DIR}/HttpsClientTest.cpp
)
target_link_libraries(mesh_https_client_test
    PRIVATE
        sce_mesh_https_mailbox
        GTest::gtest_main
)
apply_sce_test_policies(mesh_https_client_test)
add_test(NAME mesh_https_client_verification COMMAND mesh_https_client_test)

# The conformance kit: a TLS server of the wire, an explicit CONNECT proxy and
# a TLS-inspecting proxy, on OpenSSL. Built only where an adapter is there to
# hold to it.
if(TARGET sce_mesh_https_curl OR TARGET sce_mesh_https_winhttp)
    find_package(OpenSSL 1.1.1 QUIET)
    if(OPENSSL_FOUND)
        add_library(sce_mesh_https_kit STATIC
            ${MESH_TEST_DIR}/https_kit/HttpsKit.cpp
        )
        target_link_libraries(sce_mesh_https_kit
            PUBLIC
                sce_mesh_https_mailbox
                OpenSSL::SSL
                OpenSSL::Crypto
        )
        if(WIN32)
            target_link_libraries(sce_mesh_https_kit PUBLIC ws2_32)
        endif()

        if(TARGET sce_mesh_https_curl)
            add_executable(mesh_https_curl_adapter_test
                ${MESH_TEST_DIR}/HttpsCurlAdapterTest.cpp
            )
            target_link_libraries(mesh_https_curl_adapter_test
                PRIVATE
                    sce_mesh_https_kit
                    sce_mesh_https_curl
                    GTest::gtest_main
            )
            apply_sce_test_policies(mesh_https_curl_adapter_test)
            add_test(NAME mesh_https_curl_adapter_conformance COMMAND mesh_https_curl_adapter_test)
        endif()

        if(TARGET sce_mesh_https_winhttp)
            add_executable(mesh_https_winhttp_adapter_test
                ${MESH_TEST_DIR}/HttpsWinHttpAdapterTest.cpp
            )
            target_link_libraries(mesh_https_winhttp_adapter_test
                PRIVATE
                    sce_mesh_https_kit
                    sce_mesh_https_winhttp
                    crypt32
                    GTest::gtest_main
            )
            apply_sce_test_policies(mesh_https_winhttp_adapter_test)
            add_test(NAME mesh_https_winhttp_adapter_conformance COMMAND mesh_https_winhttp_adapter_test)
        endif()
    else()
        message(STATUS "SCE: OpenSSL was not found; the Mesh https adapters' conformance kit is not built, "
                       "so no adapter conformance test is registered")
    endif()
endif()
