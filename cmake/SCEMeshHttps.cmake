# SCEMeshHttps.cmake
# The reference adapters of the Mesh https binding's port (docs/adr/0107, D2;
# sce/include/mesh/transports/HttpsTransport.h). Each is a target of its own,
# so the binding's core (the client in sce_mesh_common) links no network
# library, and a host with an HTTP stack of its own links none of them.
#
#   SCE_MESH_HTTPS_CURL     AUTO (the default): built when SCE_ENABLE_MESH is
#                           on and the system's libcurl is found
#                           ON:   required; the configure stops without it
#                           OFF:  not built, and libcurl not looked for
#   SCE_MESH_HTTPS_WINHTTP  AUTO (the default): built on Windows when
#                           SCE_ENABLE_MESH is on
#                           ON:   required; the configure stops off Windows
#                           OFF:  not built
#
# Targets, when built:
#   sce_mesh_https_curl     (alias SCE::mesh_https_curl): CurlHttpsTransport
#                           on the libcurl find_package(CURL) found. SCE
#                           vendors no libcurl: the distribution's, with its
#                           CA bundle, TLS library and security fixes, is the
#                           platform's HTTP client this adapter stands for.
#   sce_mesh_https_winhttp  (alias SCE::mesh_https_winhttp):
#                           WinHttpHttpsTransport on winhttp and crypt32.
#
# Each states SCE_MESH_HTTPS_CURL=1 or SCE_MESH_HTTPS_WINHTTP=1 to what links
# it. Neither is installed: their headers stay out of the installed mesh
# headers (the root CMakeLists excludes `transports/https`), and a host on an
# installed SCE implements the port on its own stack or builds SCE as a
# subproject.
include_guard(GLOBAL)

set(SCE_MESH_HTTPS_CURL "AUTO" CACHE STRING
    "Build the https binding's adapter on the system's libcurl: AUTO (when found), ON (required) or OFF")
set_property(CACHE SCE_MESH_HTTPS_CURL PROPERTY STRINGS AUTO ON OFF)
set(SCE_MESH_HTTPS_WINHTTP "AUTO" CACHE STRING
    "Build the https binding's adapter on WinHTTP: AUTO (on Windows), ON (required) or OFF")
set_property(CACHE SCE_MESH_HTTPS_WINHTTP PROPERTY STRINGS AUTO ON OFF)

# A mode is AUTO, OFF, or anything CMake reads as true (ON).
function(_sce_mesh_https_mode variable out_mode)
    string(TOUPPER "${${variable}}" _mode)
    if(_mode STREQUAL "AUTO")
        set(${out_mode} "AUTO" PARENT_SCOPE)
    elseif(${variable})
        set(${out_mode} "ON" PARENT_SCOPE)
    else()
        set(${out_mode} "OFF" PARENT_SCOPE)
    endif()
endfunction()

_sce_mesh_https_mode(SCE_MESH_HTTPS_CURL _sce_https_curl_mode)
_sce_mesh_https_mode(SCE_MESH_HTTPS_WINHTTP _sce_https_winhttp_mode)

if(NOT SCE_ENABLE_MESH)
    if(_sce_https_curl_mode STREQUAL "ON" OR _sce_https_winhttp_mode STREQUAL "ON")
        message(FATAL_ERROR
            "SCE_MESH_HTTPS_CURL=ON or SCE_MESH_HTTPS_WINHTTP=ON asks for an adapter of the Mesh https "
            "binding, which needs the Mesh layer: configure with -DSCE_ENABLE_MESH=ON.")
    endif()
    return()
endif()

# ── libcurl ──────────────────────────────────────────────────────────────────
if(NOT _sce_https_curl_mode STREQUAL "OFF")
    # 7.68 is the oldest libcurl of a supported distribution (Ubuntu 20.04);
    # every option the adapter sets is older than it.
    find_package(CURL 7.68 QUIET)
    if(TARGET CURL::libcurl)
        add_library(sce_mesh_https_curl STATIC
            "${PROJECT_SOURCE_DIR}/sce/src/mesh/https/CurlHttpsTransport.cpp"
        )
        add_library(SCE::mesh_https_curl ALIAS sce_mesh_https_curl)
        target_include_directories(sce_mesh_https_curl PUBLIC
            "$<BUILD_INTERFACE:${PROJECT_SOURCE_DIR}/sce/include>"
        )
        target_link_libraries(sce_mesh_https_curl PRIVATE CURL::libcurl)
        target_compile_definitions(sce_mesh_https_curl INTERFACE SCE_MESH_HTTPS_CURL=1)
        target_compile_features(sce_mesh_https_curl PUBLIC cxx_std_17)
        message(STATUS "SCE: the Mesh https adapter on libcurl ${CURL_VERSION_STRING} is built "
                       "(sce_mesh_https_curl)")
    elseif(_sce_https_curl_mode STREQUAL "ON")
        message(FATAL_ERROR
            "SCE_MESH_HTTPS_CURL=ON asks for the Mesh https adapter on the system's libcurl, and libcurl "
            "7.68 or later was not found. Install it (Debian and Ubuntu: libcurl4-openssl-dev; vcpkg: the "
            "port curl, with the vcpkg toolchain file) or name an installation with -DCURL_ROOT=<prefix>; "
            "or configure with -DSCE_MESH_HTTPS_CURL=AUTO to build without it.")
    else()
        message(STATUS "SCE: libcurl was not found; the Mesh https adapter on it is not built "
                       "(SCE_MESH_HTTPS_CURL=AUTO; ON makes it required)")
    endif()
endif()

# ── WinHTTP ──────────────────────────────────────────────────────────────────
if(NOT _sce_https_winhttp_mode STREQUAL "OFF")
    if(WIN32)
        add_library(sce_mesh_https_winhttp STATIC
            "${PROJECT_SOURCE_DIR}/sce/src/mesh/https/WinHttpHttpsTransport.cpp"
        )
        add_library(SCE::mesh_https_winhttp ALIAS sce_mesh_https_winhttp)
        target_include_directories(sce_mesh_https_winhttp PUBLIC
            "$<BUILD_INTERFACE:${PROJECT_SOURCE_DIR}/sce/include>"
        )
        target_link_libraries(sce_mesh_https_winhttp PRIVATE winhttp crypt32)
        target_compile_definitions(sce_mesh_https_winhttp INTERFACE SCE_MESH_HTTPS_WINHTTP=1)
        # The source states its own UTF-16 calls (the W functions), so the
        # adapter means the same whatever UNICODE the consumer defines.
        target_compile_features(sce_mesh_https_winhttp PUBLIC cxx_std_17)
        message(STATUS "SCE: the Mesh https adapter on WinHTTP is built (sce_mesh_https_winhttp)")
    elseif(_sce_https_winhttp_mode STREQUAL "ON")
        message(FATAL_ERROR
            "SCE_MESH_HTTPS_WINHTTP=ON asks for the Mesh https adapter on WinHTTP, which exists on Windows "
            "only; configure with -DSCE_MESH_HTTPS_WINHTTP=AUTO elsewhere.")
    endif()
endif()

unset(_sce_https_curl_mode)
unset(_sce_https_winhttp_mode)
