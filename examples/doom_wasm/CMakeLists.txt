# DOOM WASM with SCE State Machines
# CMake-based build using C++ SCE engine
#
# Build:
#   mkdir build && cd build
#   emcmake cmake ..
#   emmake make
#
# This integrates DOOM (C codebase) with SCE (C++ engine) via extern "C" wrappers

cmake_minimum_required(VERSION 3.16)
project(doom_wasm_sce LANGUAGES C CXX)

# Require Emscripten
if(NOT EMSCRIPTEN)
    message(FATAL_ERROR "This project requires Emscripten. Use: emcmake cmake ..")
endif()

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_C_STANDARD 99)

# SCE Engine (parent project)
set(SCE_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../..")
add_subdirectory(${SCE_ROOT}/sce ${CMAKE_CURRENT_BINARY_DIR}/sce)

# Include SCE CMake functions
include(${SCE_ROOT}/cmake/SCECodegen.cmake)

# ============================================
# SCE Integration Sources
# ============================================
set(SCE_WRAPPER_SRC "${CMAKE_CURRENT_SOURCE_DIR}/src/sce_wrapper.cpp")
set(SCE_HOOKS_SRC "${CMAKE_CURRENT_SOURCE_DIR}/src/sce_doom_hooks.c")
set(SCE_SECRET_HINT_SRC "${CMAKE_CURRENT_SOURCE_DIR}/src/sce_secret_hint.c")

# ============================================
# DOOM Sources (cloned from doomgeneric)
# ============================================
set(DOOMGENERIC_DIR "${CMAKE_CURRENT_SOURCE_DIR}/doomgeneric")

# Check if doomgeneric exists
if(NOT EXISTS ${DOOMGENERIC_DIR})
    message(STATUS "Cloning doomgeneric...")
    execute_process(
        COMMAND git clone --depth 1 https://github.com/ozkl/doomgeneric.git ${DOOMGENERIC_DIR}
        WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
    )

    # Apply SCE integration patches using Python script
    message(STATUS "Applying SCE integration patches...")
    execute_process(
        COMMAND python3 ${CMAKE_CURRENT_SOURCE_DIR}/patches/apply_patches.py ${DOOMGENERIC_DIR}
        RESULT_VARIABLE PATCH_RESULT
    )
    if(NOT PATCH_RESULT EQUAL 0)
        message(WARNING "Patch application had issues, check output")
    endif()
endif()

# Download shareware doom1.wad if not present
set(DOOM_WAD "${DOOMGENERIC_DIR}/doomgeneric/doom1.wad")
if(NOT EXISTS ${DOOM_WAD})
    message(STATUS "Downloading doom1.wad (shareware)...")
    file(DOWNLOAD
        "https://distro.ibiblio.org/slitaz/sources/packages/d/doom1.wad"
        ${DOOM_WAD}
        SHOW_PROGRESS
        STATUS DOWNLOAD_STATUS
    )
    list(GET DOWNLOAD_STATUS 0 STATUS_CODE)
    if(NOT STATUS_CODE EQUAL 0)
        message(FATAL_ERROR "Failed to download doom1.wad. Please download manually.")
    endif()
endif()

# DOOM source files (WASM-compatible only, exclude platform-specific)
set(DOOM_SOURCES
    ${DOOMGENERIC_DIR}/doomgeneric/dummy.c
    ${DOOMGENERIC_DIR}/doomgeneric/am_map.c
    ${DOOMGENERIC_DIR}/doomgeneric/doomdef.c
    ${DOOMGENERIC_DIR}/doomgeneric/doomstat.c
    ${DOOMGENERIC_DIR}/doomgeneric/dstrings.c
    ${DOOMGENERIC_DIR}/doomgeneric/d_event.c
    ${DOOMGENERIC_DIR}/doomgeneric/d_items.c
    ${DOOMGENERIC_DIR}/doomgeneric/d_iwad.c
    ${DOOMGENERIC_DIR}/doomgeneric/d_loop.c
    ${DOOMGENERIC_DIR}/doomgeneric/d_main.c
    ${DOOMGENERIC_DIR}/doomgeneric/d_mode.c
    ${DOOMGENERIC_DIR}/doomgeneric/d_net.c
    ${DOOMGENERIC_DIR}/doomgeneric/f_finale.c
    ${DOOMGENERIC_DIR}/doomgeneric/f_wipe.c
    ${DOOMGENERIC_DIR}/doomgeneric/g_game.c
    ${DOOMGENERIC_DIR}/doomgeneric/hu_lib.c
    ${DOOMGENERIC_DIR}/doomgeneric/hu_stuff.c
    ${DOOMGENERIC_DIR}/doomgeneric/info.c
    ${DOOMGENERIC_DIR}/doomgeneric/i_cdmus.c
    ${DOOMGENERIC_DIR}/doomgeneric/i_endoom.c
    ${DOOMGENERIC_DIR}/doomgeneric/i_joystick.c
    ${DOOMGENERIC_DIR}/doomgeneric/i_scale.c
    ${DOOMGENERIC_DIR}/doomgeneric/i_sound.c
    ${DOOMGENERIC_DIR}/doomgeneric/i_system.c
    ${DOOMGENERIC_DIR}/doomgeneric/i_timer.c
    ${DOOMGENERIC_DIR}/doomgeneric/memio.c
    ${DOOMGENERIC_DIR}/doomgeneric/m_argv.c
    ${DOOMGENERIC_DIR}/doomgeneric/m_bbox.c
    ${DOOMGENERIC_DIR}/doomgeneric/m_cheat.c
    ${DOOMGENERIC_DIR}/doomgeneric/m_config.c
    ${DOOMGENERIC_DIR}/doomgeneric/m_controls.c
    ${DOOMGENERIC_DIR}/doomgeneric/m_fixed.c
    ${DOOMGENERIC_DIR}/doomgeneric/m_menu.c
    ${DOOMGENERIC_DIR}/doomgeneric/m_misc.c
    ${DOOMGENERIC_DIR}/doomgeneric/m_random.c
    ${DOOMGENERIC_DIR}/doomgeneric/p_ceilng.c
    ${DOOMGENERIC_DIR}/doomgeneric/p_doors.c
    ${DOOMGENERIC_DIR}/doomgeneric/p_enemy.c
    ${DOOMGENERIC_DIR}/doomgeneric/p_floor.c
    ${DOOMGENERIC_DIR}/doomgeneric/p_inter.c
    ${DOOMGENERIC_DIR}/doomgeneric/p_lights.c
    ${DOOMGENERIC_DIR}/doomgeneric/p_map.c
    ${DOOMGENERIC_DIR}/doomgeneric/p_maputl.c
    ${DOOMGENERIC_DIR}/doomgeneric/p_mobj.c
    ${DOOMGENERIC_DIR}/doomgeneric/p_plats.c
    ${DOOMGENERIC_DIR}/doomgeneric/p_pspr.c
    ${DOOMGENERIC_DIR}/doomgeneric/p_saveg.c
    ${DOOMGENERIC_DIR}/doomgeneric/p_setup.c
    ${DOOMGENERIC_DIR}/doomgeneric/p_sight.c
    ${DOOMGENERIC_DIR}/doomgeneric/p_spec.c
    ${DOOMGENERIC_DIR}/doomgeneric/p_switch.c
    ${DOOMGENERIC_DIR}/doomgeneric/p_telept.c
    ${DOOMGENERIC_DIR}/doomgeneric/p_tick.c
    ${DOOMGENERIC_DIR}/doomgeneric/p_user.c
    ${DOOMGENERIC_DIR}/doomgeneric/r_bsp.c
    ${DOOMGENERIC_DIR}/doomgeneric/r_data.c
    ${DOOMGENERIC_DIR}/doomgeneric/r_draw.c
    ${DOOMGENERIC_DIR}/doomgeneric/r_main.c
    ${DOOMGENERIC_DIR}/doomgeneric/r_plane.c
    ${DOOMGENERIC_DIR}/doomgeneric/r_segs.c
    ${DOOMGENERIC_DIR}/doomgeneric/r_sky.c
    ${DOOMGENERIC_DIR}/doomgeneric/r_things.c
    ${DOOMGENERIC_DIR}/doomgeneric/sha1.c
    ${DOOMGENERIC_DIR}/doomgeneric/sounds.c
    ${DOOMGENERIC_DIR}/doomgeneric/statdump.c
    ${DOOMGENERIC_DIR}/doomgeneric/st_lib.c
    ${DOOMGENERIC_DIR}/doomgeneric/st_stuff.c
    ${DOOMGENERIC_DIR}/doomgeneric/s_sound.c
    ${DOOMGENERIC_DIR}/doomgeneric/tables.c
    ${DOOMGENERIC_DIR}/doomgeneric/v_video.c
    ${DOOMGENERIC_DIR}/doomgeneric/wi_stuff.c
    ${DOOMGENERIC_DIR}/doomgeneric/w_checksum.c
    ${DOOMGENERIC_DIR}/doomgeneric/w_file.c
    ${DOOMGENERIC_DIR}/doomgeneric/w_main.c
    ${DOOMGENERIC_DIR}/doomgeneric/w_wad.c
    ${DOOMGENERIC_DIR}/doomgeneric/z_zone.c
    ${DOOMGENERIC_DIR}/doomgeneric/w_file_stdc.c
    ${DOOMGENERIC_DIR}/doomgeneric/i_input.c
    ${DOOMGENERIC_DIR}/doomgeneric/i_video.c
    ${DOOMGENERIC_DIR}/doomgeneric/doomgeneric.c
    ${DOOMGENERIC_DIR}/doomgeneric/doomgeneric_emscripten.c
    ${DOOMGENERIC_DIR}/doomgeneric/mus2mid.c
    ${DOOMGENERIC_DIR}/doomgeneric/i_sdlmusic.c
    ${DOOMGENERIC_DIR}/doomgeneric/i_sdlsound.c
)

# ============================================
# DOOM WASM Executable
# ============================================
add_executable(doom_sce
    ${DOOM_SOURCES}
    ${SCE_WRAPPER_SRC}
    ${SCE_HOOKS_SRC}
    ${SCE_SECRET_HINT_SRC}
)

# Generate state machines from all SCXML files in scxml/ directory
sce_add_state_machines_from_dir(
    TARGET doom_sce
    SCXML_DIR "${CMAKE_CURRENT_SOURCE_DIR}/scxml"
)

target_include_directories(doom_sce PRIVATE
    ${DOOMGENERIC_DIR}/doomgeneric
    ${SCE_ROOT}/sce/include
    ${CMAKE_CURRENT_SOURCE_DIR}/src
)

target_link_libraries(doom_sce PRIVATE
    sce_unified
)

# Emscripten settings
set_target_properties(doom_sce PROPERTIES
    SUFFIX ".html"
)

target_compile_options(doom_sce PRIVATE
    -O2
    -DFEATURE_SOUND=0
    -sUSE_SDL=2
    -sUSE_SDL_MIXER=2
)

target_link_options(doom_sce PRIVATE
    -sUSE_SDL=2
    -sUSE_SDL_MIXER=2
    -sALLOW_MEMORY_GROWTH=1
    -sFORCE_FILESYSTEM=1
    -sEXPORTED_RUNTIME_METHODS=['ccall','cwrap','FS']
    -sEXPORTED_FUNCTIONS=['_main','_sce_init','_sce_get_game_state','_sce_get_player_state','_sce_get_weapon_state','_sce_get_enemy_count','_sce_get_enemy_killed','_sce_get_max_enemies','_sce_get_enemy_info','_sce_enemy_spawn','_sce_enemy_set_state','_sce_enemy_killed','_sce_enemy_remove','_sce_secret_get_state','_sce_secret_event_enable','_sce_secret_event_disable','_sce_secret_event_request','_sce_secret_event_cancel','_sce_secret_event_level_change','_sce_secret_event_reached','_sce_secret_path_found','_sce_secret_no_path']
    -sNO_EXIT_RUNTIME=1
    --preload-file ${DOOMGENERIC_DIR}/doomgeneric/doom1.wad@doom1.wad
)

# Generate index.html with embedded SCXML Base64
add_custom_command(TARGET doom_sce POST_BUILD
    COMMAND python3 ${CMAKE_CURRENT_SOURCE_DIR}/generate_index.py
        ${CMAKE_CURRENT_SOURCE_DIR}/scxml
        ${CMAKE_CURRENT_BINARY_DIR}
    COMMENT "Generating index.html with SCXML data"
)

# Symlink visualizer to build directory for local development
# Using symlink instead of copy - changes are reflected immediately
add_custom_target(link_visualizer ALL
    COMMAND ${CMAKE_COMMAND} -E rm -rf ${CMAKE_CURRENT_BINARY_DIR}/visualizer
    COMMAND ${CMAKE_COMMAND} -E create_symlink ${SCE_ROOT}/visualizer ${CMAKE_CURRENT_BINARY_DIR}/visualizer
    COMMENT "Creating symlink to visualizer"
)
add_dependencies(doom_sce link_visualizer)

message(STATUS "DOOM WASM + SCE configured")
message(STATUS "  DOOM sources: ${DOOMGENERIC_DIR}")
