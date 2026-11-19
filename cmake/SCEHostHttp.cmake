# SCEHostHttp.cmake
# The network transport of the C++ reference host's HTTP operation for a
# server (docs/adr/0109, D4; sce/include/host/CurlTransport.h), on the
# system's libcurl, behind the destination policy of
# sce/include/host/DestinationPolicy.h, which needs no library.
#
#   SCE_HOST_HTTP_CURL  AUTO (the default): built when libcurl is found
#                       ON:   required; the configure stops when it is not found
#                       OFF:  not built, and libcurl not looked for
#
# What it defines, when built: the header-only target sce_host_http_curl
# (alias SCE::host_http_curl), which links the libcurl find_package(CURL)
# found and states SCE_HOST_HTTP_CURL=1 to what links it. SCE vendors no
# libcurl: the distribution's, with its CA bundle, TLS library and security
# fixes, is the HTTP client this transport stands for. The rest of the
# standard host (host/StandardHost.h) needs none of it.
include_guard(GLOBAL)

set(SCE_HOST_HTTP_CURL "AUTO" CACHE STRING
    "Build the C++ reference host's network HTTP transport on libcurl: AUTO (when found), ON (required) or OFF")
set_property(CACHE SCE_HOST_HTTP_CURL PROPERTY STRINGS AUTO ON OFF)

string(TOUPPER "${SCE_HOST_HTTP_CURL}" _sce_host_http_mode)
if(_sce_host_http_mode STREQUAL "AUTO")
    set(_sce_host_http_required FALSE)
elseif(SCE_HOST_HTTP_CURL)
    set(_sce_host_http_required TRUE)
else()
    set(_sce_host_http_mode "OFF")
endif()

if(NOT _sce_host_http_mode STREQUAL "OFF")
    # 7.68 is the oldest libcurl of a supported distribution (Ubuntu 20.04),
    # as cmake/SCEMeshHttps.cmake asks; every option the transport sets is
    # older than it, and CURLOPT_PROTOCOLS_STR is used from 7.85 on.
    find_package(CURL 7.68 QUIET)
    if(TARGET CURL::libcurl)
        add_library(sce_host_http_curl INTERFACE)
        add_library(SCE::host_http_curl ALIAS sce_host_http_curl)
        target_include_directories(sce_host_http_curl INTERFACE "${PROJECT_SOURCE_DIR}/sce/include")
        target_link_libraries(sce_host_http_curl INTERFACE CURL::libcurl)
        if(WIN32)
            # getaddrinfo, for the system resolver beside the transport.
            target_link_libraries(sce_host_http_curl INTERFACE ws2_32)
        endif()
        target_compile_definitions(sce_host_http_curl INTERFACE SCE_HOST_HTTP_CURL=1)
        target_compile_features(sce_host_http_curl INTERFACE cxx_std_17)
        message(STATUS "SCE: the C++ reference host's network HTTP transport is built on libcurl "
                       "${CURL_VERSION_STRING} (sce_host_http_curl)")
    elseif(_sce_host_http_required)
        message(FATAL_ERROR
            "SCE_HOST_HTTP_CURL=ON asks for the C++ reference host's network HTTP transport, which needs "
            "libcurl 7.68 or later, and it was not found. Install it (Debian and Ubuntu: "
            "libcurl4-openssl-dev; vcpkg: the port curl, with the vcpkg toolchain file) or name an "
            "installation with -DCURL_ROOT=<prefix>; or configure with -DSCE_HOST_HTTP_CURL=AUTO to build "
            "without it.")
    else()
        message(STATUS "SCE: libcurl was not found; the C++ reference host is built without its network "
                       "HTTP transport (SCE_HOST_HTTP_CURL=AUTO; ON makes it required)")
    endif()
endif()

unset(_sce_host_http_mode)
unset(_sce_host_http_required)
