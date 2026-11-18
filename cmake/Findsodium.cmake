# Findsodium.cmake
# libsodium, found and not vendored (docs/adr/0107, D7): a cryptographic library
# takes its security fixes from one place, and a program that already links one
# gets no second copy.
#
#   find_package(sodium [REQUIRED] [QUIET])
#
# Sets sodium_FOUND and, where the installation says it, sodium_VERSION, and
# defines the imported target sodium::sodium, which links the library found.
#
# Where it looks, the first that answers:
#   1. a target `sodium` the including project already defines (a source build
#      of libsodium it added itself);
#   2. vcpkg's port, the form libsodium takes on Windows, whose config package
#      `unofficial-sodium` defines `unofficial-sodium::sodium`;
#   3. pkg-config's `libsodium`, where a pkg-config is installed (a Linux or
#      macOS distribution's package);
#   4. the header and the library on the search path, under `sodium_ROOT` first.
#
# `sodium_USE_STATIC_LIBS` asks step 4 for the static library, and states
# SODIUM_STATIC, which libsodium's header needs to link a static library on
# Windows. Steps 1 to 3 carry what their package states.

include(FindPackageHandleStandardArgs)

set(_sodium_link "")

if(TARGET sodium::sodium)
    set(_sodium_link sodium::sodium)
elseif(TARGET sodium)
    set(_sodium_link sodium)
else()
    find_package(unofficial-sodium CONFIG QUIET)
    if(TARGET unofficial-sodium::sodium)
        set(_sodium_link unofficial-sodium::sodium)
        if(DEFINED unofficial-sodium_VERSION)
            set(sodium_VERSION "${unofficial-sodium_VERSION}")
        endif()
    endif()
endif()

if(NOT _sodium_link)
    find_package(PkgConfig QUIET)
    if(PKG_CONFIG_FOUND)
        pkg_check_modules(PC_SODIUM QUIET IMPORTED_TARGET libsodium)
        if(PC_SODIUM_FOUND AND TARGET PkgConfig::PC_SODIUM)
            set(_sodium_link PkgConfig::PC_SODIUM)
            set(sodium_VERSION "${PC_SODIUM_VERSION}")
        endif()
    endif()
endif()

if(NOT _sodium_link)
    find_path(sodium_INCLUDE_DIR NAMES sodium.h)
    if(sodium_USE_STATIC_LIBS)
        set(_sodium_names libsodium.a libsodium.lib sodium)
    else()
        set(_sodium_names sodium libsodium)
    endif()
    find_library(sodium_LIBRARY NAMES ${_sodium_names})
    unset(_sodium_names)
    mark_as_advanced(sodium_INCLUDE_DIR sodium_LIBRARY)
    if(sodium_INCLUDE_DIR AND sodium_LIBRARY)
        if(EXISTS "${sodium_INCLUDE_DIR}/sodium/version.h")
            file(STRINGS "${sodium_INCLUDE_DIR}/sodium/version.h" _sodium_version_line
                 REGEX "^#define[ \t]+SODIUM_VERSION_STRING[ \t]+\"[^\"]*\"")
            string(REGEX REPLACE "^#define[ \t]+SODIUM_VERSION_STRING[ \t]+\"([^\"]*)\".*$" "\\1"
                   sodium_VERSION "${_sodium_version_line}")
            unset(_sodium_version_line)
        endif()
        if(NOT TARGET sodium::library)
            add_library(sodium::library UNKNOWN IMPORTED)
            set_target_properties(sodium::library PROPERTIES
                IMPORTED_LOCATION "${sodium_LIBRARY}"
                INTERFACE_INCLUDE_DIRECTORIES "${sodium_INCLUDE_DIR}")
            if(sodium_USE_STATIC_LIBS)
                set_property(TARGET sodium::library APPEND PROPERTY
                    INTERFACE_COMPILE_DEFINITIONS SODIUM_STATIC=1)
            endif()
        endif()
        set(_sodium_link sodium::library)
    endif()
endif()

set(sodium_LINK_TARGET "${_sodium_link}")
unset(_sodium_link)

find_package_handle_standard_args(sodium
    REQUIRED_VARS sodium_LINK_TARGET
    VERSION_VAR sodium_VERSION)

if(sodium_FOUND AND NOT TARGET sodium::sodium)
    add_library(sodium::sodium INTERFACE IMPORTED)
    set_property(TARGET sodium::sodium PROPERTY INTERFACE_LINK_LIBRARIES "${sodium_LINK_TARGET}")
endif()
