# Standalone consumer project — invoked by run.sh after SCE is installed to
# a temporary prefix. NOT added to the parent tree via add_subdirectory; it
# deliberately drives the install-tree find_package(SCE) codepath so the
# probes exercise the Config.cmake scripts end to end.
#
# The `generated` probe is the one a user of the code generator is: it
# generates a statechart with the installed SCECodegen.cmake and sce-codegen,
# links the tier the documentation names for it and nothing else, and builds
# and runs the result at C++17 with warnings as errors.

# 3.20 because sce_add_state_machine() refuses an older CMake (its rules
# carry a DEPFILE; see the top CMakeLists.txt of SCE).
cmake_minimum_required(VERSION 3.20)
project(sce_install_tree_consumer CXX)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

if(NOT DEFINED SCE_PROBE_TIER)
    message(FATAL_ERROR "run.sh must pass -DSCE_PROBE_TIER=base|scripting|runtime|generated")
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
elseif(SCE_PROBE_TIER STREQUAL "generated")
    if(NOT SCE_PROBE_PREFIX)
        message(FATAL_ERROR "run.sh must pass -DSCE_PROBE_PREFIX=<the scratch install prefix>")
    endif()
    find_package(SCE REQUIRED COMPONENTS base)
    # The installed package must be the one generating, or this would prove
    # nothing about an install: SCEConfig.cmake resolves sce-codegen from the
    # prefix's bin/ and the templates from its share/. A generator found on
    # PATH or in a source tree would pair another tree's templates with this
    # install's headers.
    set(_sce_probe_prefix "${SCE_PROBE_PREFIX}")
    set(_sce_probe_installed_codegen FALSE)
    set(_sce_probe_installed_templates FALSE)
    if(SCE_CODEGEN)
        cmake_path(IS_PREFIX _sce_probe_prefix "${SCE_CODEGEN}" NORMALIZE
            _sce_probe_installed_codegen)
    endif()
    if(SCE_TEMPLATE_DIR)
        cmake_path(IS_PREFIX _sce_probe_prefix "${SCE_TEMPLATE_DIR}" NORMALIZE
            _sce_probe_installed_templates)
    endif()
    if(NOT _sce_probe_installed_codegen OR NOT _sce_probe_installed_templates)
        message(FATAL_ERROR
            "the generated probe must run the sce-codegen and the templates "
            "installed under ${SCE_PROBE_PREFIX}; SCE resolved the generator "
            "'${SCE_CODEGEN}' and the templates '${SCE_TEMPLATE_DIR}'")
    endif()
    add_executable(probe_generated probe_generated.cpp)
    sce_add_state_machine(
        TARGET probe_generated
        SCXML_FILE ${CMAKE_CURRENT_SOURCE_DIR}/probe_generated.scxml
    )
    # What SCECodegen.cmake tells a consumer to link for a machine that needs
    # no script engine. The Forge runtime the generated header includes comes
    # with it, through SCE::sce_core.
    target_link_libraries(probe_generated PRIVATE SCE::sce_base)
    if(MSVC)
        target_compile_options(probe_generated PRIVATE /W4 /WX)
    else()
        target_compile_options(probe_generated PRIVATE -Wall -Wextra -pedantic -Werror)
    endif()
else()
    message(FATAL_ERROR "Unknown SCE_PROBE_TIER: ${SCE_PROBE_TIER}")
endif()
