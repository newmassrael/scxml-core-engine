# Qt Event Dispatcher CMake Configuration
# Provides optional Qt5/Qt6 event dispatcher for SCE

option(SCE_DISPATCHER_QT "Build Qt event dispatcher" OFF)

if(SCE_DISPATCHER_QT)
    # Enable Qt automoc for signal/slot handling
    set(CMAKE_AUTOMOC ON)
    set(CMAKE_INCLUDE_CURRENT_DIR ON)

    # Try Qt6 first, fall back to Qt5
    find_package(Qt6 COMPONENTS Core QUIET)
    if(Qt6_FOUND)
        message(STATUS "SCE: Using Qt6 for event dispatcher")
        set(QT_LIBRARIES Qt6::Core)
    else()
        find_package(Qt5 COMPONENTS Core REQUIRED)
        message(STATUS "SCE: Using Qt5 for event dispatcher")
        set(QT_LIBRARIES Qt5::Core)
    endif()

    # Create Qt dispatcher library
    add_library(sce_qt_dispatcher
        ${CMAKE_SOURCE_DIR}/sce/src/dispatchers/QtDispatcher.cpp
    )

    target_include_directories(sce_qt_dispatcher
        PUBLIC
            ${CMAKE_SOURCE_DIR}/sce/include
    )

    target_link_libraries(sce_qt_dispatcher
        PUBLIC
            sce_runtime
        PRIVATE
            ${QT_LIBRARIES}
    )

    # Export for dependent targets
    set(SCE_QT_DISPATCHER_AVAILABLE TRUE CACHE INTERNAL "Qt dispatcher is available")
endif()
