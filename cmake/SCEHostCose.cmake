# SCEHostCose.cmake
# The COSE operations of the C++ reference host (docs/adr/0107, D7;
# sce/include/host/CoseHost.h), on libsodium (host/SodiumCoseCrypto.h).
#
#   SCE_HOST_COSE   AUTO (the default): built when libsodium is found
#                   ON:   required; the configure stops when it is not found
#                   OFF:  not built, and libsodium not looked for
#
# What it defines, when built: the header-only target sce_host_cose (alias
# SCE::host_cose), which links the libsodium found (cmake/Findsodium.cmake) and
# states SCE_HOST_COSE=1 to what links it. The rest of the standard host
# (host/StandardHost.h) needs none of it, so a build without libsodium builds
# everything else; its reference host serves none of the COSE operations.
include_guard(GLOBAL)

set(SCE_HOST_COSE "AUTO" CACHE STRING
    "Build the C++ reference host's COSE operations on libsodium: AUTO (when found), ON (required) or OFF")
set_property(CACHE SCE_HOST_COSE PROPERTY STRINGS AUTO ON OFF)

string(TOUPPER "${SCE_HOST_COSE}" _sce_host_cose_mode)
if(_sce_host_cose_mode STREQUAL "AUTO")
    set(_sce_host_cose_required FALSE)
elseif(SCE_HOST_COSE)
    set(_sce_host_cose_required TRUE)
else()
    set(_sce_host_cose_mode "OFF")
endif()

if(NOT _sce_host_cose_mode STREQUAL "OFF")
    # The module path is widened for this one search only, so no other
    # find_package of the build reads a module from cmake/.
    set(_sce_host_cose_module_path "${CMAKE_MODULE_PATH}")
    list(PREPEND CMAKE_MODULE_PATH "${CMAKE_CURRENT_LIST_DIR}")
    find_package(sodium QUIET)
    set(CMAKE_MODULE_PATH "${_sce_host_cose_module_path}")
    unset(_sce_host_cose_module_path)

    if(sodium_FOUND)
        add_library(sce_host_cose INTERFACE)
        add_library(SCE::host_cose ALIAS sce_host_cose)
        target_include_directories(sce_host_cose INTERFACE "${PROJECT_SOURCE_DIR}/sce/include")
        target_link_libraries(sce_host_cose INTERFACE sodium::sodium)
        target_compile_definitions(sce_host_cose INTERFACE SCE_HOST_COSE=1)
        target_compile_features(sce_host_cose INTERFACE cxx_std_17)
        message(STATUS "SCE: the C++ reference host's COSE operations are built on libsodium "
                       "${sodium_VERSION} (${sodium_LINK_TARGET})")
    elseif(_sce_host_cose_required)
        message(FATAL_ERROR
            "SCE_HOST_COSE=ON asks for the C++ reference host's COSE operations, which need libsodium, "
            "and libsodium was not found. Install it (Debian and Ubuntu: libsodium-dev; vcpkg: the port "
            "libsodium, with the vcpkg toolchain file) or name an installation with -Dsodium_ROOT=<prefix>; "
            "or configure with -DSCE_HOST_COSE=AUTO to build without them.")
    else()
        message(STATUS "SCE: libsodium was not found; the C++ reference host is built without its COSE "
                       "operations (SCE_HOST_COSE=AUTO; ON makes it required)")
    endif()
endif()

unset(_sce_host_cose_mode)
unset(_sce_host_cose_required)
