# GLib Event Dispatcher CMake Configuration
# Provides optional GLib event dispatcher for SCE

option(SCE_DISPATCHER_GLIB "Build GLib event dispatcher" OFF)

if(SCE_DISPATCHER_GLIB)
    # Find GLib using pkg-config
    find_package(PkgConfig REQUIRED)
    pkg_check_modules(GLIB REQUIRED glib-2.0)

    message(STATUS "SCE: GLib event dispatcher enabled")
    message(STATUS "  GLib include dirs: ${GLIB_INCLUDE_DIRS}")
    message(STATUS "  GLib libraries: ${GLIB_LIBRARIES}")

    # Create GLib dispatcher library
    add_library(sce_glib_dispatcher
        ${CMAKE_SOURCE_DIR}/sce/src/dispatchers/GLibDispatcher.cpp
    )

    target_include_directories(sce_glib_dispatcher
        PUBLIC
            ${CMAKE_SOURCE_DIR}/sce/include
            ${GLIB_INCLUDE_DIRS}
    )

    target_link_libraries(sce_glib_dispatcher
        PUBLIC
            sce_unified
            ${GLIB_LIBRARIES}
    )

    # Export for dependent targets
    set(SCE_GLIB_DISPATCHER_AVAILABLE TRUE CACHE INTERNAL "GLib dispatcher is available")
endif()
