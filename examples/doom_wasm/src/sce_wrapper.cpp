/**
 * @file sce_wrapper.cpp
 * @brief C++ to C wrapper for SCE state machines in DOOM WASM
 *
 * Provides extern "C" functions for DOOM's C code and
 * 100% synchronized JavaScript callbacks via EM_ASM.
 */

#include <array>
#include <cstdio>
#include <cstring>
#include <memory>
#include <string>

#ifdef __EMSCRIPTEN__
#include <emscripten.h>
#else
#define EMSCRIPTEN_KEEPALIVE
#define EM_ASM(...)
#define EM_ASM_INT(...) 0
#endif

// SCE Generated State Machines
#include "enemy_state_sm.h"
#include "game_state_sm.h"
#include "player_state_sm.h"
#include "secret_hint_state_sm.h"
#include "weapon_state_sm.h"

// Secret hint pathfinding
#include "sce_secret_hint.h"

// ============================================
// Constants
// ============================================
static constexpr int MAX_ENEMIES = 64;

// ============================================
// Forward declarations for JS callbacks
// ============================================
static inline void js_notify_state_change(const char *machine, const char *state);
static inline void js_notify_secret_path(int num_arrows, int remaining_secrets);
static inline void js_notify_secret_arrow(int index, int x, int y, int angle);
static inline void js_notify_target_info(const char *type_name, const char *name, int index, int total);

// Forward declaration for secret state machine (extern "C" for EMSCRIPTEN_KEEPALIVE exports)
extern "C" {
void sce_secret_path_found(void);
void sce_secret_no_path(void);
void sce_secret_recalculate(void);
const char *sce_secret_get_state(void);

// DOOM level statistics helper functions (defined in sce_doom_hooks.c)
int SCE_GetLevelTotalKills(void);
int SCE_GetPlayerKillCount(void);
}

// ============================================
// Secret SCXML UserContext: C++ Callback Integration
// ============================================

/**
 * @brief Callback handler for secret hint state machine
 *
 * Implements onentry actions for each state as defined in secret_hint_state.scxml.
 * The generated code calls user_->secret.onXxx() when entering each state.
 */
struct SecretCallbacks {
    void onDisabled() {
        // Disabled state: hint system is off
        Secret_ClearPath();
        js_notify_state_change("secret", sce_secret_get_state());
        js_notify_secret_path(0, Secret_GetRemainingCount());
    }

    void onCalculating() {
        // Calculating state: run BFS and raise path_found/no_path event
        js_notify_state_change("secret", sce_secret_get_state());

        // Note: Previous path is cleared by onexit of 'showing' state (SCXML)

        // Always notify target info for button highlighting (even if no path)
        target_info_t info;
        if (Secret_GetCurrentTarget(&info)) {
            target_type_t type;
            int index, total;
            Secret_GetSelectionInfo(&type, &index, &total);
#ifdef __EMSCRIPTEN__
            EM_ASM({
                console.debug('[SCE:Calculating] Target found: type=' + $0 + ' index=' + $1 + ' total=' + $2);
            }, (int)type, index, total);
#endif
            js_notify_target_info(Secret_GetTargetTypeName(type), info.name, index, total);
        } else {
#ifdef __EMSCRIPTEN__
            EM_ASM({
                console.debug('[SCE:Calculating] NO TARGET SET - Secret_GetCurrentTarget returned false');
            });
#endif
        }

        secret_path_t path = {};  // Zero-initialize to avoid stale values
        bool bfs_result = Secret_FindPathToCurrentTarget(&path);
#ifdef __EMSCRIPTEN__
        EM_ASM({
            console.debug('[SCE:Calculating] BFS result=' + $0 + ' num_arrows=' + $1);
        }, bfs_result ? 1 : 0, path.num_arrows);
#endif
        if (bfs_result && path.num_arrows > 0) {
            // Path found with arrows - notify JS with arrow data
            js_notify_secret_path(path.num_arrows, Secret_GetRemainingCount());
            for (int i = 0; i < path.num_arrows; i++) {
                js_notify_secret_arrow(i,
                                       path.arrows[i].x >> 16,
                                       path.arrows[i].y >> 16,
                                       path.arrows[i].angle >> 16);
            }
            sce_secret_path_found();
        } else {
            // No path or path has no arrows (target too close or unreachable)
#ifdef __EMSCRIPTEN__
            EM_ASM({
                console.debug('[SCE:Calculating] Going to no_path state');
            });
#endif
            sce_secret_no_path();
        }
    }

    void onShowing() {
        // Showing state: arrows already sent during calculating
        js_notify_state_change("secret", sce_secret_get_state());
    }

    void onExitShowing() {
        // Exiting showing state: clear path and sprites from map
        Secret_ClearPath();
    }

    void onFound() {
        // Found state: player reached target, keep showing path
        // Path will be recalculated by Secret_UpdateArrows
        js_notify_state_change("secret", sce_secret_get_state());
    }

    void onExitFound() {
        // Exiting found state: clear path and sprites from map
        Secret_ClearPath();
    }

    void onNoPath() {
        // No path state: keep display as-is, notify JS
        js_notify_state_change("secret", sce_secret_get_state());
    }
};

/**
 * @brief UserContext for secret hint state machine
 *
 * The generated code expects UserContext with a 'secret' member.
 * Calls to secret.onXxx() in SCXML become user_->secret.onXxx() in C++.
 */
struct SecretContext {
    SecretCallbacks secret;
};

// Global secret context instance
static SecretContext g_secret_context;

// Type aliases for enemy state machine
using EnemySM = SCE::Generated::enemy_state::enemy_state;
using EnemyState = SCE::Generated::enemy_state::State;
using EnemyEvent = SCE::Generated::enemy_state::Event;

// ============================================
// Enemy Instance Tracking (with SCXML state machine)
// ============================================
struct EnemyInstance {
    void *mobj_ptr = nullptr;
    int instance_id = 0;
    const char *type_name = "UNKNOWN";
    std::unique_ptr<EnemySM> sm;  // SCXML state machine instance
    bool active = false;
};

// Get state name from enemy state machine
static const char *get_enemy_state_name(EnemyState state) {
    switch (state) {
    case EnemyState::Dormant:
        return "DORMANT";
    case EnemyState::Alert:
        return "ALERT";
    case EnemyState::Chasing:
        return "CHASING";
    case EnemyState::Attacking:
        return "ATTACKING";
    case EnemyState::Pain:
        return "PAIN";
    case EnemyState::Dead:
        return "DEAD";
    default:
        return "UNKNOWN";
    }
}

// Map DOOM state string to SCXML event
static EnemyEvent doom_state_to_event(const char *state_name) {
    if (strcmp(state_name, "ALERT") == 0) {
        return EnemyEvent::See_player;
    }
    if (strcmp(state_name, "CHASING") == 0) {
        return EnemyEvent::Chase;
    }
    if (strcmp(state_name, "ATTACKING") == 0) {
        return EnemyEvent::Attack;
    }
    if (strcmp(state_name, "PAIN") == 0) {
        return EnemyEvent::Pain;
    }
    // Note: ATTACK_COMPLETE and PAIN_COMPLETE not needed - chase event handles these transitions
    return EnemyEvent::NONE;
}

static std::array<EnemyInstance, MAX_ENEMIES> g_enemies;
static int g_enemy_count = 0;
static int g_enemy_killed = 0;
static int g_next_instance_id = 1;

// ============================================
// State Machine Instances
// ============================================
static std::unique_ptr<SCE::Generated::game_state::game_state> g_game_sm;
static std::unique_ptr<SCE::Generated::player_state::player_state> g_player_sm;
static std::unique_ptr<SCE::Generated::weapon_state::weapon_state> g_weapon_sm;
// Secret state machine with UserContext for C++ callbacks
using SecretSM = SCE::Generated::secret_hint_state::secret_hint_state<SecretContext>;
using SecretEvent = SCE::Generated::secret_hint_state::Event;
static std::unique_ptr<SecretSM> g_secret_sm;

// Forward declarations
extern "C" {
const char *sce_get_player_state(void);
const char *sce_get_weapon_state(void);
}

// Forward declarations for JavaScript callbacks (defined below)
static inline void js_notify_state_change(const char *machine, const char *state);
static inline void js_notify_enemy_update(int slot, const char *type, const char *state, int instance_id, bool active);
static inline void js_notify_stats_update(int enemy_count, int enemy_killed, int enemy_remaining);
static inline void js_notify_secret_path(int num_arrows, int remaining_secrets);
static inline void js_notify_secret_arrow(int index, int x, int y, int angle);
static inline void js_notify_target_info(const char *type_name, const char *name, int index, int total);

// ============================================
// Helper Functions
// ============================================

/**
 * @brief Reset player and weapon state machines for new/load game
 *
 * StaticExecutionEngine::initialize() does NOT reset currentState_,
 * so we must recreate the state machine objects to properly reset them.
 */
static void reset_player_and_weapon_state_machines() {
    // Recreate player state machine (initialize() doesn't reset currentState_)
    g_player_sm = std::make_unique<SCE::Generated::player_state::player_state>();
    g_player_sm->initialize();
    js_notify_state_change("player", sce_get_player_state());

    // Recreate weapon state machine
    g_weapon_sm = std::make_unique<SCE::Generated::weapon_state::weapon_state>();
    g_weapon_sm->initialize();
    js_notify_state_change("weapon", sce_get_weapon_state());
}

/**
 * @brief Reset all enemy tracking and notify JavaScript
 * @param notify_dead If true, notify JS with DEAD state before removal
 */
static void reset_all_enemies(bool notify_dead = false) {
    for (int i = 0; i < MAX_ENEMIES; i++) {
        if (g_enemies[i].active) {
            const char *state_name = "UNKNOWN";
            if (g_enemies[i].sm) {
                state_name = notify_dead ? "DEAD" : get_enemy_state_name(g_enemies[i].sm->getCurrentState());
            }
            js_notify_enemy_update(i, g_enemies[i].type_name, state_name, g_enemies[i].instance_id, false);
            g_enemies[i].sm.reset();
        }
        g_enemies[i] = EnemyInstance{};
    }
    g_enemy_count = 0;
    g_enemy_killed = 0;
    g_next_instance_id = 1;
    js_notify_stats_update(0, 0, 0);
}

// ============================================
// JavaScript Callbacks (100% Sync)
// ============================================
static inline void js_notify_state_change(const char *machine, const char *state) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceStateChange === 'function') {
                window.onSceStateChange(UTF8ToString($0), UTF8ToString($1));
            }
        },
        machine, state);
#endif
}

static inline void js_notify_enemy_update(int slot, const char *type, const char *state, int instance_id, bool active) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceEnemyUpdate === 'function') {
                window.onSceEnemyUpdate($0, UTF8ToString($1), UTF8ToString($2), $3, $4);
            } else {
                console.warn('[SCE:C++] window.onSceEnemyUpdate not defined!');
            }
        },
        slot, type, state, instance_id, active ? 1 : 0);
#endif
}

static inline void js_notify_stats_update(int enemy_count, int enemy_killed, int enemy_remaining) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceStatsUpdate === 'function') {
                window.onSceStatsUpdate($0, $1, $2);
            }
        },
        enemy_count, enemy_killed, enemy_remaining);
#endif
}

static inline void js_notify_secret_path(int num_arrows, int remaining_secrets) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceSecretPath === 'function') {
                window.onSceSecretPath($0, $1);
            }
        },
        num_arrows, remaining_secrets);
#endif
}

static inline void js_notify_secret_arrow(int index, int x, int y, int angle) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceSecretArrow === 'function') {
                window.onSceSecretArrow($0, $1, $2, $3);
            }
        },
        index, x, y, angle);
#endif
}

static inline void js_notify_target_info(const char *type_name, const char *name, int index, int total) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceTargetInfo === 'function') {
                window.onSceTargetInfo(UTF8ToString($0), UTF8ToString($1), $2, $3);
            }
        },
        type_name, name, index, total);
#endif
}

// ============================================
// Enemy Management
// ============================================
static int find_enemy_slot(void *mobj) {
    for (int i = 0; i < MAX_ENEMIES; i++) {
        if (g_enemies[i].active && g_enemies[i].mobj_ptr == mobj) {
            return i;
        }
    }
    return -1;
}

static int find_free_slot() {
    for (int i = 0; i < MAX_ENEMIES; i++) {
        if (!g_enemies[i].active) {
            return i;
        }
    }
    return -1;
}

extern "C" {

// Forward declaration for get game state
const char *sce_get_game_state(void);

// ============================================
// Initialization
// ============================================

// Forward declaration for secret state
const char *sce_secret_get_state(void);

EMSCRIPTEN_KEEPALIVE
void sce_init(void) {
    g_game_sm = std::make_unique<SCE::Generated::game_state::game_state>();
    g_player_sm = std::make_unique<SCE::Generated::player_state::player_state>();
    g_weapon_sm = std::make_unique<SCE::Generated::weapon_state::weapon_state>();
    // Secret state machine: pass UserContext for C++ callback integration
    g_secret_sm = std::make_unique<SecretSM>(g_secret_context);

    g_game_sm->initialize();
    g_player_sm->initialize();
    g_weapon_sm->initialize();
    g_secret_sm->initialize();

    // Reset enemy tracking
    for (auto &e : g_enemies) {
        e = EnemyInstance{};
    }
    g_enemy_count = 0;
    g_enemy_killed = 0;
    g_next_instance_id = 1;

    // Notify initial states
    js_notify_state_change("game", sce_get_game_state());
    js_notify_state_change("player", sce_get_player_state());
    js_notify_state_change("weapon", sce_get_weapon_state());
    js_notify_state_change("secret", sce_secret_get_state());
    js_notify_stats_update(0, 0, 0);
}

// ============================================
// Game State Machine
// ============================================

EMSCRIPTEN_KEEPALIVE
const char *sce_get_game_state(void) {
    if (!g_game_sm) {
        return "UNINITIALIZED";
    }
    auto state = g_game_sm->getCurrentState();
    switch (state) {
    case SCE::Generated::game_state::State::Demoscreen:
        return "DEMOSCREEN";
    case SCE::Generated::game_state::State::Level:
        return "LEVEL";
    case SCE::Generated::game_state::State::Intermission:
        return "INTERMISSION";
    case SCE::Generated::game_state::State::Finale:
        return "FINALE";
    default:
        return "UNKNOWN";
    }
}

#define GAME_EVENT(name, event)                                                                                        \
    EMSCRIPTEN_KEEPALIVE                                                                                               \
    void sce_game_event_##name(void) {                                                                                 \
        if (g_game_sm) {                                                                                               \
            g_game_sm->processEvent(SCE::Generated::game_state::Event::event);                                         \
            js_notify_state_change("game", sce_get_game_state());                                                      \
        }                                                                                                              \
    }

// Special handling for newgame - reset all state machines
EMSCRIPTEN_KEEPALIVE
void sce_game_event_newgame(void) {
    if (g_game_sm) {
        g_game_sm->processEvent(SCE::Generated::game_state::Event::Newgame);
        js_notify_state_change("game", sce_get_game_state());
    }

    // Reset player and weapon state machines
    reset_player_and_weapon_state_machines();

    // Reset all enemy tracking
    reset_all_enemies(false);
}

// Special handling for loadgame - also reset player/weapon states
EMSCRIPTEN_KEEPALIVE
void sce_game_event_loadgame(void) {
    if (g_game_sm) {
        g_game_sm->processEvent(SCE::Generated::game_state::Event::Loadgame);
        js_notify_state_change("game", sce_get_game_state());
    }

    // Reset player and weapon state machines
    reset_player_and_weapon_state_machines();

    // Reset all enemy tracking (enemies will be re-spawned by level load)
    reset_all_enemies(true);
}

// Special handling for demo start - reset player/weapon/enemy without changing game state
EMSCRIPTEN_KEEPALIVE
void sce_game_event_demostart(void) {
    // Game state stays in DEMOSCREEN - no state machine event needed

    // Reset player and weapon state machines for new demo
    reset_player_and_weapon_state_machines();

    // Reset all enemy tracking
    reset_all_enemies(false);
}

GAME_EVENT(completed, Completed)
GAME_EVENT(worlddone, Worlddone)
GAME_EVENT(finale, Finale)

#undef GAME_EVENT

// ============================================
// Player State Machine
// ============================================

EMSCRIPTEN_KEEPALIVE
const char *sce_get_player_state(void) {
    if (!g_player_sm) {
        return "UNINITIALIZED";
    }
    auto state = g_player_sm->getCurrentState();
    switch (state) {
    case SCE::Generated::player_state::State::Alive:
        return "ALIVE";
    case SCE::Generated::player_state::State::Dead:
        return "DEAD";
    case SCE::Generated::player_state::State::Reborn:
        return "REBORN";
    case SCE::Generated::player_state::State::Invulnerable:
        return "INVULNERABLE";
    default:
        return "UNKNOWN";
    }
}

#define PLAYER_EVENT(name, event)                                                                                      \
    EMSCRIPTEN_KEEPALIVE                                                                                               \
    void sce_player_event_##name(void) {                                                                               \
        if (g_player_sm) {                                                                                             \
            g_player_sm->processEvent(SCE::Generated::player_state::Event::event);                                     \
            js_notify_state_change("player", sce_get_player_state());                                                  \
        }                                                                                                              \
    }

PLAYER_EVENT(killed, Killed)
PLAYER_EVENT(respawn, Respawn)
PLAYER_EVENT(spawn_complete, Spawn_complete)
PLAYER_EVENT(god_mode_on, God_mode_on)
PLAYER_EVENT(god_mode_off, God_mode_off)

#undef PLAYER_EVENT

// ============================================
// Weapon State Machine
// ============================================

EMSCRIPTEN_KEEPALIVE
const char *sce_get_weapon_state(void) {
    if (!g_weapon_sm) {
        return "UNINITIALIZED";
    }
    auto state = g_weapon_sm->getCurrentState();
    switch (state) {
    case SCE::Generated::weapon_state::State::Ready:
        return "READY";
    case SCE::Generated::weapon_state::State::Lowering:
        return "LOWERING";
    case SCE::Generated::weapon_state::State::Raising:
        return "RAISING";
    case SCE::Generated::weapon_state::State::Firing:
        return "FIRING";
    default:
        return "UNKNOWN";
    }
}

#define WEAPON_EVENT(name, event)                                                                                      \
    EMSCRIPTEN_KEEPALIVE                                                                                               \
    void sce_weapon_event_##name(void) {                                                                               \
        if (g_weapon_sm) {                                                                                             \
            g_weapon_sm->processEvent(SCE::Generated::weapon_state::Event::event);                                     \
            js_notify_state_change("weapon", sce_get_weapon_state());                                                  \
        }                                                                                                              \
    }

WEAPON_EVENT(fire, Fire)
WEAPON_EVENT(switch_weapon, Switch_weapon)
WEAPON_EVENT(lower_complete, Lower_complete)
WEAPON_EVENT(raise_complete, Raise_complete)
WEAPON_EVENT(fire_complete, Fire_complete)

#undef WEAPON_EVENT

// ============================================
// Enemy State Machine (Multi-Instance)
// 100% Synchronized via JavaScript callbacks
// ============================================

EMSCRIPTEN_KEEPALIVE
int sce_get_enemy_count(void) {
    return g_enemy_count;
}

EMSCRIPTEN_KEEPALIVE
int sce_get_enemy_killed(void) {
    return g_enemy_killed;
}

EMSCRIPTEN_KEEPALIVE
int sce_get_max_enemies(void) {
    return MAX_ENEMIES;
}

// DOOM Level Statistics
EMSCRIPTEN_KEEPALIVE
int sce_get_level_total_kills(void) {
    return SCE_GetLevelTotalKills();
}

EMSCRIPTEN_KEEPALIVE
int sce_get_player_kill_count(void) {
    return SCE_GetPlayerKillCount();
}

EMSCRIPTEN_KEEPALIVE
int sce_get_enemies_remaining(void) {
    // Return count of currently tracked live enemies
    return g_enemy_count;
}

// Get enemy info by slot: returns "ptr,type,state,instance_id" or empty
EMSCRIPTEN_KEEPALIVE
const char *sce_get_enemy_info(int slot) {
    static char buffer[128];
    buffer[0] = '\0';

    if (slot < 0 || slot >= MAX_ENEMIES || !g_enemies[slot].active) {
        return buffer;
    }

    // Get state from state machine
    const char *state_name = "UNKNOWN";
    if (g_enemies[slot].sm) {
        state_name = get_enemy_state_name(g_enemies[slot].sm->getCurrentState());
    }

    // Use field width limits to prevent buffer overflow
    // Format: ptr(max 20), type(max 31), state(max 31), id(max 10) + separators
    snprintf(buffer, sizeof(buffer), "%ld,%.31s,%.31s,%d",
             (long)(intptr_t)g_enemies[slot].mobj_ptr,
             g_enemies[slot].type_name ? g_enemies[slot].type_name : "UNKNOWN",
             state_name ? state_name : "UNKNOWN",
             g_enemies[slot].instance_id);
    buffer[sizeof(buffer) - 1] = '\0';  // Ensure null-termination
    return buffer;
}

// Called from DOOM when enemy spawns/wakes up
EMSCRIPTEN_KEEPALIVE
void sce_enemy_spawn(void *mobj, const char *type_name) {
    int slot = find_enemy_slot(mobj);
    if (slot < 0) {
        slot = find_free_slot();
        if (slot < 0) {
            return;  // No free slots
        }
    }

    g_enemies[slot].mobj_ptr = mobj;
    g_enemies[slot].instance_id = g_next_instance_id++;
    g_enemies[slot].type_name = type_name;
    g_enemies[slot].active = true;

    // Create and initialize SCXML state machine for this enemy
    g_enemies[slot].sm = std::make_unique<EnemySM>();
    g_enemies[slot].sm->initialize();

    g_enemy_count++;

    // Get initial state from state machine
    const char *state_name = get_enemy_state_name(g_enemies[slot].sm->getCurrentState());

    // 100% sync callback
    js_notify_enemy_update(slot, type_name, state_name, g_enemies[slot].instance_id, true);
    // Use g_enemy_count directly - DOOM's totalkills lags during spawn phase
    js_notify_stats_update(g_enemy_count, g_enemy_killed, g_enemy_count);
}

// Called from DOOM when enemy state changes
EMSCRIPTEN_KEEPALIVE
void sce_enemy_set_state(void *mobj, const char *doom_state) {
    int slot = find_enemy_slot(mobj);
    if (slot < 0 || !g_enemies[slot].sm) {
        return;
    }

    // Map DOOM state to SCXML event and process it
    EnemyEvent event = doom_state_to_event(doom_state);
    if (event != EnemyEvent::NONE) {
        g_enemies[slot].sm->processEvent(event);
    }

    // Get actual state from state machine (may differ from requested if transition invalid)
    const char *actual_state = get_enemy_state_name(g_enemies[slot].sm->getCurrentState());

    // 100% sync callback
    js_notify_enemy_update(slot, g_enemies[slot].type_name, actual_state, g_enemies[slot].instance_id, true);
}

// Called from DOOM when enemy dies
EMSCRIPTEN_KEEPALIVE
void sce_enemy_killed(void *mobj) {
    int slot = find_enemy_slot(mobj);
    if (slot < 0 || !g_enemies[slot].sm) {
        return;
    }

    // Process Killed event through state machine
    g_enemies[slot].sm->processEvent(EnemyEvent::Killed);
    g_enemy_killed++;

    // Get state from state machine (should be DEAD)
    const char *state_name = get_enemy_state_name(g_enemies[slot].sm->getCurrentState());

    // 100% sync callback - notify dead state first
    js_notify_enemy_update(slot, g_enemies[slot].type_name, state_name, g_enemies[slot].instance_id, true);
    js_notify_stats_update(g_enemy_count, g_enemy_killed, g_enemy_count);
}

// Called from DOOM when enemy is removed from map
EMSCRIPTEN_KEEPALIVE
void sce_enemy_remove(void *mobj) {
    int slot = find_enemy_slot(mobj);
    if (slot < 0) {
        return;
    }

    // Get current state before cleanup
    const char *state_name = "UNKNOWN";
    if (g_enemies[slot].sm) {
        state_name = get_enemy_state_name(g_enemies[slot].sm->getCurrentState());
    }

    // 100% sync callback - notify inactive
    js_notify_enemy_update(slot, g_enemies[slot].type_name, state_name, g_enemies[slot].instance_id, false);

    // Cleanup state machine
    g_enemies[slot].sm.reset();
    g_enemies[slot].active = false;
    g_enemies[slot].mobj_ptr = nullptr;
    g_enemy_count--;

    js_notify_stats_update(g_enemy_count, g_enemy_killed, g_enemy_count);
}

// Clear all enemy tracking (for save/load)
EMSCRIPTEN_KEEPALIVE
void sce_enemy_clear_all(void) {
    for (int i = 0; i < MAX_ENEMIES; i++) {
        if (g_enemies[i].active) {
            g_enemies[i].sm.reset();
            g_enemies[i].active = false;
            g_enemies[i].mobj_ptr = nullptr;
        }
    }
    g_enemy_count = 0;
    g_enemy_killed = 0;
    js_notify_stats_update(g_enemy_count, g_enemy_killed, g_enemy_count);
}

// ============================================
// Secret Hint State Machine
// ============================================

// Forward declarations for secret functions
void sce_secret_path_found(void);
void sce_secret_no_path(void);

EMSCRIPTEN_KEEPALIVE
const char *sce_secret_get_state(void) {
    if (!g_secret_sm) {
        return "UNINITIALIZED";
    }
    auto state = g_secret_sm->getCurrentState();
    switch (state) {
    case SCE::Generated::secret_hint_state::State::Disabled:
        return "disabled";
    case SCE::Generated::secret_hint_state::State::Enabled:
        return "enabled";
    case SCE::Generated::secret_hint_state::State::Calculating:
        return "calculating";
    case SCE::Generated::secret_hint_state::State::Showing:
        return "showing";
    case SCE::Generated::secret_hint_state::State::Found:
        return "found";
    case SCE::Generated::secret_hint_state::State::No_path:
        return "no_path";
    default:
        return "UNKNOWN";
    }
}

// SCXML-driven: processEvent() triggers onentry callbacks automatically
EMSCRIPTEN_KEEPALIVE
void sce_secret_event_toggle(void) {
    // H key: Toggle hint system (disabled <-> enabled)
    // SCXML handles state transitions, onentry callbacks do the work
    if (g_secret_sm) {
#ifdef __EMSCRIPTEN__
        EM_ASM({
            console.debug('[SCE:Toggle] H key pressed, current state:', UTF8ToString($0));
        }, sce_secret_get_state());
#endif
        g_secret_sm->processEvent(SecretEvent::Toggle);
#ifdef __EMSCRIPTEN__
        EM_ASM({
            console.debug('[SCE:Toggle] After toggle, new state:', UTF8ToString($0));
        }, sce_secret_get_state());
#endif
    }
}

// Legacy aliases for backwards compatibility
EMSCRIPTEN_KEEPALIVE
void sce_secret_event_enable(void) {
    sce_secret_event_toggle();
}

EMSCRIPTEN_KEEPALIVE
void sce_secret_event_disable(void) {
    sce_secret_event_toggle();
}

EMSCRIPTEN_KEEPALIVE
void sce_secret_event_next_target(void) {
    sce_secret_event_toggle();
}

EMSCRIPTEN_KEEPALIVE
void sce_secret_event_prev_target(void) {
    // G key: No longer used
}

EMSCRIPTEN_KEEPALIVE
void sce_select_target(int type, int index) {
    // Button click: Select target and show path
    // SCXML calculating state onentry will run BFS
    if (g_secret_sm) {
        // Get current selection to check if same target
        target_type_t curr_type;
        int curr_index, curr_total;
        Secret_GetSelectionInfo(&curr_type, &curr_index, &curr_total);

        // Check if selecting the same target
        bool is_same_target = (curr_type == (target_type_t)type && curr_index == index);

        if (Secret_SelectTarget((target_type_t)type, index)) {
            // Check current state for same-target behavior
            auto current_state = g_secret_sm->getCurrentState();
            bool in_no_path = (current_state == SCE::Generated::secret_hint_state::State::No_path);

            if (is_same_target && !in_no_path) {
                // Same target clicked while showing - toggle off (enabled -> disabled)
                g_secret_sm->processEvent(SecretEvent::Toggle);
            } else {
                // Different target OR same target in no_path - recalculate
                g_secret_sm->processEvent(SecretEvent::Select);
            }
        }
    }
}

EMSCRIPTEN_KEEPALIVE
void sce_secret_event_request(void) {
    // Backwards compatibility - same as toggle
    sce_secret_event_next_target();
}

EMSCRIPTEN_KEEPALIVE
void sce_secret_event_select(void) {
    // Select event: triggers path calculation to currently selected target
    if (g_secret_sm) {
        g_secret_sm->processEvent(SecretEvent::Select);
    }
}

EMSCRIPTEN_KEEPALIVE
void sce_secret_event_cancel(void) {
    // Legacy function - cancel is now just toggle (disable)
    sce_secret_event_toggle();
}

EMSCRIPTEN_KEEPALIVE
void sce_secret_event_level_change(void) {
    if (g_secret_sm) {
        g_secret_sm->processEvent(SecretEvent::Level_change);
    }
}

EMSCRIPTEN_KEEPALIVE
void sce_secret_event_reached(void) {
    if (g_secret_sm) {
        g_secret_sm->processEvent(SecretEvent::Secret_reached);
    }
}

EMSCRIPTEN_KEEPALIVE
void sce_secret_path_found(void) {
    // Called from onCalculating callback when BFS finds path
    if (g_secret_sm) {
        g_secret_sm->processEvent(SecretEvent::Path_found);
    }
}

EMSCRIPTEN_KEEPALIVE
void sce_secret_no_path(void) {
    // Called from onCalculating callback when BFS fails
    if (g_secret_sm) {
        g_secret_sm->processEvent(SecretEvent::No_path);
    }
}

EMSCRIPTEN_KEEPALIVE
void sce_secret_recalculate(void) {
    // Called from Secret_UpdateArrows when player moves significantly
    if (g_secret_sm) {
        g_secret_sm->processEvent(SecretEvent::Recalculate);
    }
}

EMSCRIPTEN_KEEPALIVE
int sce_get_target_count_secret(void) {
    return Secret_GetTargetCount(TARGET_SECRET);
}

EMSCRIPTEN_KEEPALIVE
int sce_get_target_count_door(void) {
    return Secret_GetTargetCount(TARGET_DOOR);
}

EMSCRIPTEN_KEEPALIVE
int sce_get_target_count_lift(void) {
    return Secret_GetTargetCount(TARGET_LIFT);
}

EMSCRIPTEN_KEEPALIVE
int sce_get_target_count_switch(void) {
    return Secret_GetTargetCount(TARGET_SWITCH);
}

EMSCRIPTEN_KEEPALIVE
int sce_get_target_count_teleporter(void) {
    return Secret_GetTargetCount(TARGET_TELEPORTER);
}

EMSCRIPTEN_KEEPALIVE
int sce_get_target_count_exit(void) {
    return Secret_GetTargetCount(TARGET_EXIT);
}

EMSCRIPTEN_KEEPALIVE
int sce_get_target_count_keydoor(void) {
    return Secret_GetTargetCount(TARGET_KEY_DOOR);
}

EMSCRIPTEN_KEEPALIVE
int sce_is_secret_discovered(int index) {
    return Secret_IsDiscovered(index) ? 1 : 0;
}

}  // extern "C"
