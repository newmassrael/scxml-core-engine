# Standalone consumer project — invoked by run.sh after SCE is installed to
# a temporary prefix. NOT added to the parent tree via add_subdirectory; it
# deliberately drives the install-tree find_package(SCE) codepath so the
# probes exercise the Config.cmake scripts end to end.

cmake_minimum_required(VERSION 3.16)
project(sce_install_tree_consumer CXX)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

if(NOT DEFINED SCE_PROBE_TIER)
    message(FATAL_ERROR "run.sh must pass -DSCE_PROBE_TIER=base|scripting|runtime")
endif()

if(SCE_PROBE_TIER STREQUAL "base")
    find_package(SCE REQUIRED COMPONENTS base)
    add_executable(probe_base probe_base.cpp)
    target_link_libraries(probe_base PRIVATE SCE::sce_base)
elseif(SCE_PROBE_TIER STREQUAL "scripting")
    find_package(SCE REQUIRED COMPONENTS scripting)
    add_executable(probe_scripting probe_scripting.cpp)
    target_link_libraries(probe_scripting PRIVATE SCE::sce_scripting)
elseif(SCE_PROBE_TIER STREQUAL "runtime")
    find_package(SCE REQUIRED COMPONENTS runtime)
    add_executable(probe_runtime probe_runtime.cpp)
    target_link_libraries(probe_runtime PRIVATE SCE::sce_runtime)
else()
    message(FATAL_ERROR "Unknown SCE_PROBE_TIER: ${SCE_PROBE_TIER}")
endif()
