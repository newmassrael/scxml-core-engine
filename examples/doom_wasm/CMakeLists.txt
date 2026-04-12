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
# Modular architecture: each sce_sm_*.cpp owns one state machine domain
set(SCE_SOURCES
    ${CMAKE_CURRENT_SOURCE_DIR}/src/sce_wrapper.cpp        # Init orchestrator
    ${CMAKE_CURRENT_SOURCE_DIR}/src/sce_sm_core.cpp        # Game / Player / Weapon SMs
    ${CMAKE_CURRENT_SOURCE_DIR}/src/sce_sm_enemy.cpp       # Enemy multi-instance SM
    ${CMAKE_CURRENT_SOURCE_DIR}/src/sce_sm_combo.cpp       # Combo / Berserk SM + timer
    ${CMAKE_CURRENT_SOURCE_DIR}/src/sce_sm_aim.cpp         # Aim Assist SM
    ${CMAKE_CURRENT_SOURCE_DIR}/src/sce_sm_secret.cpp      # Secret Hint SM + BFS
    ${CMAKE_CURRENT_SOURCE_DIR}/src/sce_doom_hooks.c       # DOOM C bridge
    ${CMAKE_CURRENT_SOURCE_DIR}/src/sce_secret_hint.c      # BFS pathfinding
    ${CMAKE_CURRENT_SOURCE_DIR}/src/sce_hud.c              # In-game HUD rendering
)

# ============================================
# DOOM Sources (pre-patched doomgeneric included in repo)
# ============================================
set(DOOMGENERIC_DIR "${CMAKE_CURRENT_SOURCE_DIR}/doomgeneric")

# Verify doomgeneric exists (should be committed to repo)
if(NOT EXISTS ${DOOMGENERIC_DIR}/doomgeneric)
    message(FATAL_ERROR "doomgeneric source not found. This should be included in the repository.")
endif()

# Check for full game WAD (DOOMU.WAD) or fall back to shareware (doom1.wad)
set(DOOM_WAD_FULL "${DOOMGENERIC_DIR}/doomgeneric/DOOMU.WAD")
set(DOOM_WAD_SHAREWARE "${DOOMGENERIC_DIR}/doomgeneric/doom1.wad")

if(EXISTS ${DOOM_WAD_FULL})
    set(DOOM_WAD ${DOOM_WAD_FULL})
    set(DOOM_WAD_NAME "doom.wad")  # Mount as doom.wad so DOOM can find it
    message(STATUS "Using full game: DOOMU.WAD (Ultimate DOOM)")
else()
    set(DOOM_WAD ${DOOM_WAD_SHAREWARE})
    set(DOOM_WAD_NAME "doom1.wad")
    if(NOT EXISTS ${DOOM_WAD})
        message(STATUS "Downloading doom1.wad (shareware)...")
        file(DOWNLOAD
            "https://distro.ibiblio.org/slitaz/sources/packages/d/doom1.wad"
            ${DOOM_WAD}
            STATUS DOWNLOAD_STATUS
            TIMEOUT 60
        )
        list(GET DOWNLOAD_STATUS 0 STATUS_CODE)
        if(NOT STATUS_CODE EQUAL 0)
            message(FATAL_ERROR
                "Failed to download doom1.wad (status: ${DOWNLOAD_STATUS}).\n"
                "Download manually and place at: ${DOOM_WAD}\n"
                "Source: https://distro.ibiblio.org/slitaz/sources/packages/d/doom1.wad")
        endif()
    endif()
    message(STATUS "Using shareware: doom1.wad (Episode 1 only)")
endif()

# Download FreePats soundfont for MIDI music support
set(FREEPATS_DIR "${CMAKE_CURRENT_SOURCE_DIR}/freepats")
set(FREEPATS_CFG "${FREEPATS_DIR}/freepats.cfg")
if(NOT EXISTS ${FREEPATS_CFG})
    file(MAKE_DIRECTORY ${FREEPATS_DIR})
    message(STATUS "Downloading FreePats soundfont for MIDI support...")
    set(FREEPATS_ARCHIVE "${FREEPATS_DIR}/freepats.tar.gz")
    file(DOWNLOAD
        "http://ftp.debian.org/debian/pool/main/f/freepats/freepats_20060219.orig.tar.gz"
        ${FREEPATS_ARCHIVE}
        SHOW_PROGRESS
        STATUS DOWNLOAD_STATUS
    )
    list(GET DOWNLOAD_STATUS 0 STATUS_CODE)
    if(NOT STATUS_CODE EQUAL 0)
        message(WARNING "Failed to download FreePats. MIDI music will not work.")
    else()
        # Extract FreePats (Debian package has nested tar.bz2)
        message(STATUS "Extracting FreePats...")
        execute_process(
            COMMAND tar xzf ${FREEPATS_ARCHIVE}
            WORKING_DIRECTORY ${FREEPATS_DIR}
        )
        # Extract inner tar.bz2
        execute_process(
            COMMAND tar xjf freepats-20060219/upstream/freepats-20060219.tar.bz2
            WORKING_DIRECTORY ${FREEPATS_DIR}
        )
        # Move files from nested directory to freepats root
        execute_process(
            COMMAND ${CMAKE_COMMAND} -E copy_directory
                ${FREEPATS_DIR}/freepats ${FREEPATS_DIR}
            WORKING_DIRECTORY ${FREEPATS_DIR}
        )
        # Cleanup
        file(REMOVE ${FREEPATS_ARCHIVE})
        file(REMOVE_RECURSE "${FREEPATS_DIR}/freepats-20060219")
        file(REMOVE_RECURSE "${FREEPATS_DIR}/freepats")
        # Create timidity.cfg
        file(WRITE "${FREEPATS_DIR}/timidity.cfg" "source freepats.cfg\n")
        message(STATUS "FreePats installed successfully")
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
    ${SCE_SOURCES}
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
    sce_runtime
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
    "-sSDL2_MIXER_FORMATS=[\"mid\"]"
    -sALLOW_MEMORY_GROWTH=1
    -sFORCE_FILESYSTEM=1
    -lidbfs.js  # Enable IndexedDB filesystem for persistent saves
    -sEXPORTED_RUNTIME_METHODS=['ccall','cwrap','FS']
    -sEXPORTED_FUNCTIONS=['_main','_sce_init','_sce_get_game_state','_sce_get_player_state','_sce_get_weapon_state','_sce_get_enemy_count','_sce_get_enemy_killed','_sce_get_max_enemies','_sce_get_enemy_info','_sce_get_enemies_remaining','_sce_get_level_total_kills','_sce_get_player_kill_count','_sce_enemy_spawn','_sce_enemy_set_state','_sce_enemy_killed','_sce_enemy_remove','_sce_secret_get_state','_sce_secret_event_enable','_sce_secret_event_disable','_sce_secret_event_request','_sce_secret_event_cancel','_sce_secret_event_level_change','_sce_secret_event_reached','_sce_secret_path_found','_sce_secret_no_path','_sce_aim_get_state','_sce_aim_event_toggle','_sce_aim_is_enabled']
    -sNO_EXIT_RUNTIME=1
    "SHELL:--preload-file ${DOOM_WAD}@${DOOM_WAD_NAME}"
    "SHELL:--preload-file ${CMAKE_CURRENT_SOURCE_DIR}/freepats@/etc/timidity"
)

# Generate index.html with embedded SCXML Base64
# Use OUTPUT mode with DEPENDS to rebuild when template or SCXML files change
add_custom_command(
    OUTPUT ${CMAKE_CURRENT_BINARY_DIR}/index.html
    COMMAND python3 ${CMAKE_CURRENT_SOURCE_DIR}/generate_index.py
        ${CMAKE_CURRENT_SOURCE_DIR}/scxml
        ${CMAKE_CURRENT_BINARY_DIR}
    DEPENDS
        ${CMAKE_CURRENT_SOURCE_DIR}/index.html.template
        ${CMAKE_CURRENT_SOURCE_DIR}/scxml/game_state.scxml
        ${CMAKE_CURRENT_SOURCE_DIR}/scxml/player_state.scxml
        ${CMAKE_CURRENT_SOURCE_DIR}/scxml/weapon_state.scxml
        ${CMAKE_CURRENT_SOURCE_DIR}/scxml/enemy_state.scxml
        ${CMAKE_CURRENT_SOURCE_DIR}/scxml/secret_hint_state.scxml
        ${CMAKE_CURRENT_SOURCE_DIR}/scxml/aim_assist_state.scxml
        ${CMAKE_CURRENT_SOURCE_DIR}/scxml/combo_state.scxml
    COMMENT "Generating index.html with SCXML data"
)
add_custom_target(generate_index ALL DEPENDS ${CMAKE_CURRENT_BINARY_DIR}/index.html)
add_dependencies(doom_sce generate_index)

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
