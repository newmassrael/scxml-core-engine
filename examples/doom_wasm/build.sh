#!/bin/bash
#
# DOOM WASM with SCE State Machines - Build Script
#
# This script:
#   1. Clones doomgeneric (minimal DOOM port)
#   2. Integrates SCE state machines (Game + Player + Weapon)
#   3. Builds for WebAssembly using Emscripten
#
# Prerequisites:
#   - Emscripten SDK (emsdk) installed and activated
#   - wget for downloading doom1.wad
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BUILD_DIR="$SCRIPT_DIR/build"
DOOMGENERIC_DIR="$BUILD_DIR/doomgeneric"

echo "============================================"
echo "  DOOM WASM + SCE State Machines Builder"
echo "============================================"
echo ""

# Check for Emscripten
if ! command -v emcc &> /dev/null; then
    echo "Error: Emscripten (emcc) not found in PATH"
    echo "Please install and activate emsdk first:"
    echo "  source /path/to/emsdk/emsdk_env.sh"
    exit 1
fi

echo "Using Emscripten: $(emcc --version | head -1)"
echo ""

# Create build directory
mkdir -p "$BUILD_DIR"

# Clone doomgeneric if not present
if [ ! -d "$DOOMGENERIC_DIR" ]; then
    echo "Cloning doomgeneric..."
    git clone --depth 1 https://github.com/ozkl/doomgeneric.git "$DOOMGENERIC_DIR"
fi

cd "$DOOMGENERIC_DIR/doomgeneric"

# Copy SCE state machine headers
echo "Integrating SCE state machines..."
cp "$SCRIPT_DIR/generated/sce_game_state.h" .
cp "$SCRIPT_DIR/generated/sce_player_state.h" .
cp "$SCRIPT_DIR/generated/sce_weapon_state.h" .

# Modify headers to use extern for global context variables (needed for JS exports)
echo "Modifying headers for extern linkage..."
# Game state: change static context to extern (definition in sce_exports.c)
# Note: actual type name is sce_context_t, not sce_game_ctx_t
sed -i 's/^static sce_context_t g_sce_ctx = {0};/extern sce_context_t g_sce_ctx;/' sce_game_state.h

# Player state
sed -i 's/^static sce_player_ctx_t g_sce_player = {0};/extern sce_player_ctx_t g_sce_player;/' sce_player_state.h

# Weapon state
sed -i 's/^static sce_weapon_ctx_t g_sce_weapon = {0};/extern sce_weapon_ctx_t g_sce_weapon;/' sce_weapon_state.h

# Create sce_manager.h with flat includes (for same directory)
cat > sce_manager.h << 'MANAGER_EOF'
/**
 * @file sce_manager.h
 * @brief SCE State Machine Manager for DOOM
 *
 * Unified manager for all DOOM state machines:
 * - Game State (DEMOSCREEN, LEVEL, INTERMISSION, FINALE)
 * - Player State (ALIVE, DEAD, REBORN)
 * - Weapon State (READY, RAISING, LOWERING, FIRING)
 */
#ifndef SCE_MANAGER_H
#define SCE_MANAGER_H

#ifdef __cplusplus
extern "C" {
#endif

#include "sce_game_state.h"
#include "sce_player_state.h"
#include "sce_weapon_state.h"

#ifdef __EMSCRIPTEN__
#include <emscripten.h>
#else
#define EMSCRIPTEN_KEEPALIVE
#endif

typedef enum {
    SCE_SM_GAME = 0,
    SCE_SM_PLAYER,
    SCE_SM_WEAPON,
    SCE_SM_COUNT
} sce_machine_id_t;

typedef struct {
    int initialized;
    int tick_count;
} sce_manager_ctx_t;

static sce_manager_ctx_t g_sce_manager = {0};

static inline void sce_manager_init(void) {
    printf("[SCE:Manager] ========================================\n");
    printf("[SCE:Manager] Initializing DOOM State Machines\n");
    printf("[SCE:Manager] ========================================\n");
    sce_init();
    sce_player_init();
    sce_weapon_init();
    g_sce_manager.initialized = 1;
    g_sce_manager.tick_count = 0;
    printf("[SCE:Manager] All state machines initialized (%d total)\n", SCE_SM_COUNT);
    printf("[SCE:Manager] ========================================\n");
}

/* Note: Weapon state is NOT synchronized here - it uses direct event injection
 * from p_pspr.c for 100% accurate state tracking. The weapon parameters are
 * kept for API compatibility but are ignored.
 */
static inline void sce_manager_sync_full(int gamestate, int playerstate,
                                         int readyweapon, int pendingweapon,
                                         int attackdown) {
    (void)gamestate;      /* Handled by direct injection */
    (void)playerstate;    /* Handled by direct injection */
    (void)readyweapon;    /* Handled by direct injection in p_pspr.c */
    (void)pendingweapon;  /* Handled by direct injection in p_pspr.c */
    (void)attackdown;     /* Handled by direct injection in p_pspr.c */
    if (!g_sce_manager.initialized) sce_manager_init();
    g_sce_manager.tick_count++;
    /* All syncs removed - events/states injected directly from DOOM code:
     * - Game: g_game.c, f_finale.c, d_main.c
     * - Player: p_inter.c, p_mobj.c, g_game.c
     * - Weapon: p_pspr.c
     */
}

static inline void sce_manager_sync(int gamestate, int playerstate) {
    (void)gamestate;      /* Handled by direct injection */
    (void)playerstate;    /* Handled by direct injection */
    if (!g_sce_manager.initialized) sce_manager_init();
    g_sce_manager.tick_count++;
    /* All syncs removed - see sce_manager_sync_full for details */
}

static inline void sce_manager_dump(void) {
    printf("\n[SCE:Manager] === State Dump (tick %d) ===\n", g_sce_manager.tick_count);
    printf("[SCE:Manager] Game:   %s\n", sce_get_state_name());
    printf("[SCE:Manager] Player: %s (deaths: %d)\n",
           sce_player_get_state_name(), sce_player_get_deaths());
    printf("[SCE:Manager] Weapon: %s [%s] (shots: %d)\n",
           sce_weapon_get_action_name(), sce_weapon_get_weapon_name(),
           sce_weapon_get_shots());
    printf("[SCE:Manager] ====================================\n\n");
}

#ifdef __cplusplus
}
#endif

#endif /* SCE_MANAGER_H */
MANAGER_EOF

# Create sce_exports.c with global variable definitions and EMSCRIPTEN_KEEPALIVE wrapper functions
cat > sce_exports.c << 'EXPORTS_EOF'
/**
 * @file sce_exports.c
 * @brief Global variable definitions and Emscripten-exported wrapper functions for SCE state machines
 *
 * This file:
 * 1. Defines all global context variables declared as extern in the headers
 * 2. Exports functions to JavaScript for real-time state visualization
 *
 * Note: Name arrays (sce_state_names, etc.) are static in headers and accessed locally.
 */

#include <stdio.h>

#ifdef __EMSCRIPTEN__
#include <emscripten.h>
#else
#define EMSCRIPTEN_KEEPALIVE
#endif

/* Include headers for type definitions and static name arrays */
#include "sce_game_state.h"
#include "sce_player_state.h"
#include "sce_weapon_state.h"

/* ========== Global Context Variable Definitions ========== */
/* These are declared as extern in the headers (modified by sed) */

/* Game State Context */
sce_context_t g_sce_ctx = {0};

/* Player State Context */
sce_player_ctx_t g_sce_player = {0};

/* Weapon State Context */
sce_weapon_ctx_t g_sce_weapon = {0};

/* ========== Exported Functions (sce_js_* prefix to avoid conflicts with header static inline functions) ========== */

/* Game State Exports */
EMSCRIPTEN_KEEPALIVE
const char* sce_js_get_state_name(void) {
    return sce_state_names[g_sce_ctx.current_state];
}

EMSCRIPTEN_KEEPALIVE
int sce_js_get_transition_count(void) {
    return g_sce_ctx.transition_count;
}

/* Player State Exports */
EMSCRIPTEN_KEEPALIVE
const char* sce_js_player_get_state_name(void) {
    return sce_player_state_names[g_sce_player.state];
}

EMSCRIPTEN_KEEPALIVE
int sce_js_player_get_deaths(void) {
    return g_sce_player.death_count;
}

/* Weapon State Exports */
EMSCRIPTEN_KEEPALIVE
const char* sce_js_weapon_get_action_name(void) {
    return sce_weapon_action_names[g_sce_weapon.action];
}

EMSCRIPTEN_KEEPALIVE
const char* sce_js_weapon_get_weapon_name(void) {
    if (g_sce_weapon.current_weapon < SCE_WPN_COUNT) {
        return sce_weapon_type_names[g_sce_weapon.current_weapon];
    }
    return "UNKNOWN";
}

EMSCRIPTEN_KEEPALIVE
int sce_js_weapon_get_shots(void) {
    return g_sce_weapon.shots_fired;
}

EMSCRIPTEN_KEEPALIVE
int sce_js_weapon_get_switches(void) {
    return g_sce_weapon.weapon_switches;
}
EXPORTS_EOF

# Patch d_main.c
if ! grep -q "sce_manager.h" d_main.c; then
    echo "Patching d_main.c..."

    # Add include after d_main.h
    sed -i '/#include "d_main.h"/a \
\
// SCE State Machine Integration\
#include "sce_manager.h"' d_main.c

    # Add sce_manager_init() before main_loop_started = true
    sed -i '/main_loop_started = true;/i \
    // Initialize SCE state machines (Game + Player + Weapon)\
    sce_manager_init();\
' d_main.c

    # Add sce_manager_sync_full() after TryRunTics in doomgeneric_Tick
    sed -i '/TryRunTics.*will run at least one tic/a \
\
    // Sync SCE state machines with DOOM state\
    sce_manager_sync_full(gamestate, players[consoleplayer].playerstate,\
                          players[consoleplayer].readyweapon,\
                          players[consoleplayer].pendingweapon,\
                          players[consoleplayer].attackdown);' d_main.c
fi

# Download doom1.wad if not present
if [ ! -f "doom1.wad" ]; then
    echo "Downloading doom1.wad (shareware)..."
    wget -q "https://distro.ibiblio.org/slitaz/sources/packages/d/doom1.wad" -O doom1.wad
fi

# Apply p_pspr.c patch for direct weapon state event injection
# This provides 100% accurate weapon state synchronization
echo "Patching p_pspr.c for direct weapon state event injection..."

# Add include for sce_weapon_state.h after p_pspr.h include
if ! grep -q "sce_weapon_state.h" p_pspr.c; then
    sed -i '/#include "p_pspr.h"/a\
\
/* SCE State Machine Integration - Direct event injection for 100% sync */\
#include "sce_weapon_state.h"' p_pspr.c
fi

# Patch P_FireWeapon to inject fire event
if ! grep -q "SCE_WEAPON_EVT_FIRE" p_pspr.c; then
    sed -i '/P_SetMobjState (player->mo, S_PLAY_ATK1);/i\
    /* SCE: Direct fire event - 100% sync with actual weapon fire */\
    sce_weapon_set_current((sce_weapon_type_t)player->readyweapon);\
    sce_weapon_event(SCE_WEAPON_EVT_FIRE);' p_pspr.c
fi

# Patch A_WeaponReady to inject switch event (insert before P_SetPsprite in weapon change block)
if ! grep -q "SCE_WEAPON_EVT_SWITCH_WEAPON" p_pspr.c; then
    # Find the line with downstate and add event injection after it
    sed -i '/newstate = weaponinfo\[player->readyweapon\].downstate;/a\
\
	/* SCE: Direct switch event - 100% sync with actual weapon switch */\
	sce_weapon_set_pending((sce_weapon_type_t)player->pendingweapon);\
	sce_weapon_event(SCE_WEAPON_EVT_SWITCH_WEAPON);' p_pspr.c
fi

# Patch A_Lower to inject lower complete event (insert after readyweapon assignment)
if ! grep -q "SCE_WEAPON_EVT_LOWER_COMPLETE" p_pspr.c; then
    sed -i '/player->readyweapon = player->pendingweapon;/a\
\
    /* SCE: Direct lower complete event - 100% sync with actual weapon lowering */\
    sce_weapon_event(SCE_WEAPON_EVT_LOWER_COMPLETE);' p_pspr.c
fi

# Patch A_Raise to inject raise complete event (insert before readystate assignment)
if ! grep -q "SCE_WEAPON_EVT_RAISE_COMPLETE" p_pspr.c; then
    sed -i '/newstate = weaponinfo\[player->readyweapon\].readystate;/i\
    /* SCE: Direct raise complete event - 100% sync with actual weapon raising */\
    sce_weapon_event(SCE_WEAPON_EVT_RAISE_COMPLETE);' p_pspr.c
fi

# Patch A_ReFire to inject fire complete and refire events
if ! grep -q "SCE_WEAPON_EVT_FIRE_COMPLETE" p_pspr.c; then
    # Add refire event after player->refire++
    sed -i '/player->refire++;/a\
	/* SCE: Refire event - continuous firing */\
	sce_weapon_event(SCE_WEAPON_EVT_FIRE_REFIRE);' p_pspr.c
    # Add fire complete event before player->refire = 0
    sed -i '/player->refire = 0;/i\
	/* SCE: Fire complete event - player stopped firing */\
	sce_weapon_event(SCE_WEAPON_EVT_FIRE_COMPLETE);' p_pspr.c
fi

echo "p_pspr.c patched successfully"

# ============================================
# Patch p_inter.c for Player death event injection
# ============================================
echo "Patching p_inter.c for player death event injection..."

# Add include for sce_player_state.h
if ! grep -q "sce_player_state.h" p_inter.c; then
    sed -i '/#include "p_local.h"/a\
\
/* SCE Player State Integration - Direct event injection for 100% sync */\
#include "sce_player_state.h"' p_inter.c
fi

# Inject KILLED event when player dies
if ! grep -q "SCE_PLAYER_EVT_KILLED" p_inter.c; then
    sed -i '/target->player->playerstate = PST_DEAD;/a\
        /* SCE: Direct player death event - 100% sync */\
        sce_player_event(SCE_PLAYER_EVT_KILLED);' p_inter.c
fi

echo "p_inter.c patched successfully"

# ============================================
# Patch p_mobj.c for Player spawn event injection
# ============================================
echo "Patching p_mobj.c for player spawn event injection..."

# Add include for sce_player_state.h
if ! grep -q "sce_player_state.h" p_mobj.c; then
    sed -i '/#include "p_local.h"/a\
\
/* SCE Player State Integration - Direct event injection for 100% sync */\
#include "sce_player_state.h"' p_mobj.c
fi

# Inject SPAWN_COMPLETE event when player spawns
if ! grep -q "SCE_PLAYER_EVT_SPAWN_COMPLETE" p_mobj.c; then
    sed -i '/p->playerstate = PST_LIVE;/a\
    /* SCE: Direct player spawn complete event - 100% sync */\
    sce_player_event(SCE_PLAYER_EVT_SPAWN_COMPLETE);' p_mobj.c
fi

echo "p_mobj.c patched successfully"

# ============================================
# Patch g_game.c for Player respawn and Game state injection
# ============================================
echo "Patching g_game.c for player respawn and game state injection..."

# Add includes for SCE state machines
if ! grep -q "sce_game_state.h" g_game.c; then
    sed -i '/#include "p_saveg.h"/a\
\
/* SCE State Machine Integration - Direct injection for 100% sync */\
#include "sce_game_state.h"\
#include "sce_player_state.h"\
#include "sce_weapon_state.h"' g_game.c
fi

# Inject RESPAWN event at start of G_DoReborn
if ! grep -q "SCE_PLAYER_EVT_RESPAWN" g_game.c; then
    sed -i '/void G_DoReborn (int playernum)/,/^{/ {
        /^{/a\
    /* SCE: Direct respawn event - 100% sync */\
    sce_player_event(SCE_PLAYER_EVT_RESPAWN);
    }' g_game.c
fi

# Inject GS_LEVEL state
if ! grep -q "sce_set_state(SCE_STATE_LEVEL)" g_game.c; then
    sed -i '/gamestate = GS_LEVEL;/a\
    /* SCE: Direct game state set - 100% sync */\
    sce_set_state(SCE_STATE_LEVEL);' g_game.c
fi

# Inject GS_INTERMISSION state
if ! grep -q "sce_set_state(SCE_STATE_INTERMISSION)" g_game.c; then
    sed -i '/gamestate = GS_INTERMISSION;/a\
    /* SCE: Direct game state set - 100% sync */\
    sce_set_state(SCE_STATE_INTERMISSION);' g_game.c
fi

# Reset weapon and player state machines at new game start (G_DoNewGame)
if ! grep -q "sce_weapon_init\|sce_player_init" g_game.c; then
    sed -i '/void G_DoNewGame (void)/,/^{/ {
        /^{/a\
    /* SCE: Reset state machines for new game - clears demo state */\
    sce_weapon_init();\
    sce_player_init();
    }' g_game.c
fi

echo "g_game.c patched successfully"

# ============================================
# Patch f_finale.c for Game finale state injection
# ============================================
echo "Patching f_finale.c for game finale state injection..."

# Add include for sce_game_state.h (must be before gamestate assignment)
if ! grep -q "sce_game_state.h" f_finale.c; then
    sed -i '/#include "doomstat.h"/a\
\
/* SCE Game State Integration - Direct injection for 100% sync */\
#include "sce_game_state.h"' f_finale.c
fi

# Inject GS_FINALE state
if ! grep -q "sce_set_state(SCE_STATE_FINALE)" f_finale.c; then
    sed -i '/gamestate = GS_FINALE;/a\
    /* SCE: Direct game state set - 100% sync */\
    sce_set_state(SCE_STATE_FINALE);' f_finale.c
fi

echo "f_finale.c patched successfully"

# ============================================
# Patch d_main.c for Game demoscreen state injection
# ============================================
echo "Patching d_main.c for game demoscreen state injection..."

# Add include for sce_game_state.h (if not already present from sce_manager.h)
if ! grep -q "sce_game_state.h" d_main.c && ! grep -q "sce_manager.h" d_main.c; then
    sed -i '/#include "doomstat.h"/a\
\
/* SCE Game State Integration - Direct injection for 100% sync */\
#include "sce_game_state.h"' d_main.c
fi

# Inject GS_DEMOSCREEN state (multiple locations)
# Note: Pattern must NOT match 'wipegamestate = GS_DEMOSCREEN' (global var declaration)
if ! grep -q "sce_set_state(SCE_STATE_DEMOSCREEN)" d_main.c; then
    sed -i '/^[[:space:]]*gamestate = GS_DEMOSCREEN;/a\
	/* SCE: Direct game state set - 100% sync */\
	sce_set_state(SCE_STATE_DEMOSCREEN);' d_main.c
fi

echo "d_main.c patched successfully"

echo ""
echo "All SCE state machine patches applied successfully!"
echo "  - Weapon: p_pspr.c (FIRE, SWITCH, LOWER_COMPLETE, RAISE_COMPLETE, FIRE_COMPLETE, FIRE_REFIRE)"
echo "  - Player: p_inter.c (KILLED), p_mobj.c (SPAWN_COMPLETE), g_game.c (RESPAWN)"
echo "  - Game: g_game.c (LEVEL, INTERMISSION), f_finale.c (FINALE), d_main.c (DEMOSCREEN)"

# Build
echo ""
echo "Compiling DOOM with SCE state machines..."
mkdir -p build

SOURCES="
dummy.c am_map.c doomdef.c doomstat.c dstrings.c d_event.c d_items.c d_iwad.c
d_loop.c d_main.c d_mode.c d_net.c f_finale.c f_wipe.c g_game.c hu_lib.c
hu_stuff.c info.c i_cdmus.c i_endoom.c i_joystick.c i_scale.c i_sound.c
i_system.c i_timer.c memio.c m_argv.c m_bbox.c m_cheat.c m_config.c
m_controls.c m_fixed.c m_menu.c m_misc.c m_random.c p_ceilng.c p_doors.c
p_enemy.c p_floor.c p_inter.c p_lights.c p_map.c p_maputl.c p_mobj.c
p_plats.c p_pspr.c p_saveg.c p_setup.c p_sight.c p_spec.c p_switch.c
p_telept.c p_tick.c p_user.c r_bsp.c r_data.c r_draw.c r_main.c r_plane.c
r_segs.c r_sky.c r_things.c sha1.c sounds.c statdump.c st_lib.c st_stuff.c
s_sound.c tables.c v_video.c wi_stuff.c w_checksum.c w_file.c w_main.c
w_wad.c z_zone.c w_file_stdc.c i_input.c i_video.c doomgeneric.c
doomgeneric_emscripten.c mus2mid.c i_sdlmusic.c i_sdlsound.c
sce_exports.c
"

CFLAGS="-O2 -DFEATURE_SOUND=0"

for src in $SOURCES; do
    if [ -f "$src" ]; then
        obj="build/$(basename $src .c).o"
        echo "  Compiling $src"
        emcc $CFLAGS -c "$src" -o "$obj" -s USE_SDL=2 -s USE_SDL_MIXER=2
    fi
done

echo ""
echo "Linking..."
emcc build/*.o -o doom_sce.html \
    -s USE_SDL=2 \
    -s USE_SDL_MIXER=2 \
    -s ALLOW_MEMORY_GROWTH=1 \
    -s FORCE_FILESYSTEM=1 \
    -s EXPORTED_RUNTIME_METHODS='["ccall","cwrap","FS"]' \
    -s EXPORTED_FUNCTIONS='["_main","_sce_js_get_state_name","_sce_js_get_transition_count","_sce_js_player_get_state_name","_sce_js_player_get_deaths","_sce_js_weapon_get_action_name","_sce_js_weapon_get_weapon_name","_sce_js_weapon_get_shots","_sce_js_weapon_get_switches"]' \
    -s NO_EXIT_RUNTIME=1 \
    --preload-file doom1.wad \
    -lm

# Copy output to build directory
cp doom_sce.html doom_sce.js doom_sce.wasm doom_sce.data "$BUILD_DIR/"

# Copy visualizer files
echo "Copying visualizer files..."
cp -r "$SCRIPT_DIR/../../visualizer" "$BUILD_DIR/"

# Encode SCXML files to Base64
echo "Encoding SCXML files to Base64..."
GAME_SCXML=$(base64 -w0 "$SCRIPT_DIR/scxml/game_state.scxml")
PLAYER_SCXML=$(base64 -w0 "$SCRIPT_DIR/scxml/player_state.scxml")
WEAPON_SCXML=$(base64 -w0 "$SCRIPT_DIR/scxml/weapon_state.scxml")

# Create index.html with tab-based visualizer
cat > "$BUILD_DIR/index.html" << 'EOF'
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>DOOM + SCE State Machines</title>
    <style>
        body { margin: 0; padding: 20px; background: #1a1a1a; color: #fff; font-family: monospace; }
        .container { max-width: 1200px; margin: 0 auto; }
        h1 { color: #ff0000; text-align: center; }

        /* Game Section */
        #game-section { text-align: center; margin-bottom: 20px; }
        canvas { border: 2px solid #666; }
        #status { padding: 10px; color: #ffff00; }
        .controls { font-size: 12px; color: #888; margin-top: 10px; }

        /* Tab Section */
        .tab-container { margin-top: 20px; }
        .tab-buttons {
            display: flex;
            justify-content: center;
            gap: 5px;
            margin-bottom: 0;
        }
        .tab-btn {
            padding: 10px 20px;
            background: #333;
            border: 1px solid #555;
            border-bottom: none;
            border-radius: 4px 4px 0 0;
            color: #888;
            cursor: pointer;
            font-family: monospace;
            font-size: 14px;
        }
        .tab-btn:hover { background: #444; }
        .tab-btn.active {
            background: #2a2a2a;
            color: #00ff00;
        }
        .tab-btn .state-indicator {
            display: inline-block;
            margin-left: 8px;
            padding: 2px 6px;
            background: #444;
            border-radius: 3px;
            font-size: 10px;
            color: #0969da;
        }

        /* Visualizer Section - Fill remaining viewport height */
        #viz-section {
            border: 1px solid #555;
            border-radius: 0 0 8px 8px;
            overflow: hidden;
            background: #2a2a2a;
            /* Calculate height: viewport - (header ~50px + game ~480px + tabs ~50px + stats ~60px + padding ~60px) */
            height: calc(100vh - 700px);
            min-height: 300px;
        }
        #visualizer {
            width: 100%;
            height: 100%;
            border: none;
        }

        /* Stats Section - Fixed at bottom */
        #stats {
            display: flex;
            justify-content: center;
            gap: 30px;
            padding: 10px 15px;
            background: #222;
            border-radius: 4px;
            margin-top: 10px;
        }
        #stats span { color: #888; }
        #stats strong { color: #00ff00; }
    </style>
</head>
<body>
    <div class="container">
        <h1>DOOM + SCE State Machines</h1>

        <!-- Game Section -->
        <div id="game-section">
            <canvas id="canvas" oncontextmenu="event.preventDefault()" tabindex="-1" width="640" height="400"></canvas>
            <div id="status">Loading...</div>
            <div class="controls">
                Arrow keys: Move | CTRL: Fire | Space: Use | Shift: Run | 1-7: Weapons
            </div>
        </div>

        <!-- Tab + Visualizer Section -->
        <div class="tab-container">
            <div class="tab-buttons">
                <button class="tab-btn active" data-machine="game" onclick="switchTab('game')">
                    Game <span class="state-indicator" id="ind-game">DEMOSCREEN</span>
                </button>
                <button class="tab-btn" data-machine="player" onclick="switchTab('player')">
                    Player <span class="state-indicator" id="ind-player">ALIVE</span>
                </button>
                <button class="tab-btn" data-machine="weapon" onclick="switchTab('weapon')">
                    Weapon <span class="state-indicator" id="ind-weapon">READY</span>
                </button>
            </div>
            <div id="viz-section">
                <iframe id="visualizer" src="visualizer/visualizer.html?embed#scxml=GAME_SCXML_BASE64"></iframe>
            </div>
        </div>

        <!-- Stats Section -->
        <div id="stats">
            <span>Deaths: <strong id="stat-deaths">0</strong></span>
            <span>Shots: <strong id="stat-shots">0</strong></span>
            <span>Switches: <strong id="stat-switches">0</strong></span>
            <span>Transitions: <strong id="stat-transitions">0</strong></span>
        </div>
    </div>

    <script>
        // SCXML Base64 data (injected by build script)
        const SCXML_DATA = {
            game: 'GAME_SCXML_BASE64',
            player: 'PLAYER_SCXML_BASE64',
            weapon: 'WEAPON_SCXML_BASE64'
        };

        // State tracking
        const SCE = {
            currentTab: 'game',
            ready: false,
            lastState: { game: null, player: null, weapon: null }
        };

        // Tab switching
        function switchTab(machine) {
            SCE.currentTab = machine;

            // Update tab button styles
            document.querySelectorAll('.tab-btn').forEach(btn => {
                btn.classList.toggle('active', btn.dataset.machine === machine);
            });

            // Load new SCXML in visualizer
            const iframe = document.getElementById('visualizer');
            iframe.src = 'visualizer/visualizer.html?embed&t=' + Date.now() + '#scxml=' + SCXML_DATA[machine];
            SCE.ready = false;
        }

        // Wait for visualizer ready
        window.addEventListener('message', (event) => {
            if (event.data && event.data.type === 'visualizer-ready') {
                SCE.ready = true;
                // Send current state immediately
                updateCurrentTabState();
            }
        });

        // Track WASM runtime initialization
        let wasmReady = false;

        // Get state from C functions (sce_js_* exports)
        function getStateSnapshot() {
            if (!wasmReady || !Module.ccall) return null;
            try {
                return {
                    game: Module.ccall('sce_js_get_state_name', 'string', [], []),
                    player: Module.ccall('sce_js_player_get_state_name', 'string', [], []),
                    weapon: Module.ccall('sce_js_weapon_get_action_name', 'string', [], []),
                    deaths: Module.ccall('sce_js_player_get_deaths', 'number', [], []),
                    shots: Module.ccall('sce_js_weapon_get_shots', 'number', [], []),
                    switches: Module.ccall('sce_js_weapon_get_switches', 'number', [], []),
                    transitions: Module.ccall('sce_js_get_transition_count', 'number', [], [])
                };
            } catch (e) {
                return null;
            }
        }

        // Update current tab's visualizer
        function updateCurrentTabState() {
            const state = getStateSnapshot();
            if (!state) return;

            const currentState = state[SCE.currentTab];
            if (SCE.ready && currentState !== SCE.lastState[SCE.currentTab]) {
                const iframe = document.getElementById('visualizer');
                iframe.contentWindow.postMessage({
                    type: 'highlight-states',
                    stateIds: [currentState.toLowerCase()]
                }, '*');
            }
        }

        // Update all state indicators and stats
        function updateVisualizers() {
            const state = getStateSnapshot();
            if (!state) return;

            // Update tab indicators
            document.getElementById('ind-game').textContent = state.game;
            document.getElementById('ind-player').textContent = state.player;
            document.getElementById('ind-weapon').textContent = state.weapon;

            // Update stats
            document.getElementById('stat-deaths').textContent = state.deaths;
            document.getElementById('stat-shots').textContent = state.shots;
            document.getElementById('stat-switches').textContent = state.switches;
            document.getElementById('stat-transitions').textContent = state.transitions;

            // Update current tab's diagram (only if changed)
            const currentState = state[SCE.currentTab];
            if (SCE.ready && currentState !== SCE.lastState[SCE.currentTab]) {
                const iframe = document.getElementById('visualizer');
                iframe.contentWindow.postMessage({
                    type: 'highlight-states',
                    stateIds: [currentState.toLowerCase()]
                }, '*');
                SCE.lastState[SCE.currentTab] = currentState;
            }

            // Track all states for indicator updates
            SCE.lastState.game = state.game;
            SCE.lastState.player = state.player;
            SCE.lastState.weapon = state.weapon;
        }

        // Poll state at game frame rate
        setInterval(updateVisualizers, 50);

        // Emscripten Module
        var Module = {
            canvas: document.getElementById('canvas'),
            onRuntimeInitialized: function() {
                wasmReady = true;
                console.log('[SCE] WASM runtime initialized - state polling enabled');
            },
            print: function(text) {
                console.log(text);
                if (text.includes('[SCE:')) {
                    console.log('%c' + text, 'color: #00ff00; font-weight: bold;');
                }
            },
            printErr: function(text) { console.error(text); },
            setStatus: function(text) {
                document.getElementById('status').textContent = text || 'Ready';
            },
            totalDependencies: 0,
            monitorRunDependencies: function(left) {
                this.totalDependencies = Math.max(this.totalDependencies, left);
                Module.setStatus(left ? 'Loading...' : '');
            }
        };
    </script>
    <script async src="doom_sce.js"></script>
</body>
</html>
EOF

# Replace SCXML Base64 placeholders in index.html
echo "Injecting SCXML data into index.html..."
sed -i "s|GAME_SCXML_BASE64|$GAME_SCXML|g" "$BUILD_DIR/index.html"
sed -i "s|PLAYER_SCXML_BASE64|$PLAYER_SCXML|g" "$BUILD_DIR/index.html"
sed -i "s|WEAPON_SCXML_BASE64|$WEAPON_SCXML|g" "$BUILD_DIR/index.html"

echo ""
echo "============================================"
echo "  Build complete!"
echo "============================================"
echo ""
echo "State Machines Integrated:"
echo "  - Game State (DEMOSCREEN, LEVEL, INTERMISSION, FINALE)"
echo "  - Player State (ALIVE, DEAD, REBORN)"
echo "  - Weapon State (READY, RAISING, LOWERING, FIRING)"
echo ""
echo "Visualization Features:"
echo "  - Tab-based state machine diagram viewer"
echo "  - Real-time state updates via postMessage"
echo "  - Statistics: Deaths, Shots, Switches, Transitions"
echo ""
echo "Output files in: $BUILD_DIR/"
echo ""
echo "To run:"
echo "  cd $BUILD_DIR"
echo "  python3 -m http.server 8080"
echo "  Open http://localhost:8080/"
echo ""
