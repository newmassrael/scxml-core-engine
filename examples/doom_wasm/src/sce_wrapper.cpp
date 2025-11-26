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
#include "weapon_state_sm.h"

// ============================================
// Constants
// ============================================
static constexpr int MAX_ENEMIES = 64;

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
    // Transition completion events
    if (strcmp(state_name, "ATTACK_COMPLETE") == 0) {
        return EnemyEvent::Attack_complete;
    }
    if (strcmp(state_name, "PAIN_COMPLETE") == 0) {
        return EnemyEvent::Pain_complete;
    }
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
            console.log('[SCE:C++] js_notify_enemy_update called:', $0, UTF8ToString($1), UTF8ToString($2), $3, $4);
            if (typeof window.onSceEnemyUpdate === 'function') {
                window.onSceEnemyUpdate($0, UTF8ToString($1), UTF8ToString($2), $3, $4);
            } else {
                console.warn('[SCE:C++] window.onSceEnemyUpdate not defined!');
            }
        },
        slot, type, state, instance_id, active ? 1 : 0);
#endif
}

static inline void js_notify_stats_update(int enemy_count, int enemy_killed) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceStatsUpdate === 'function') {
                window.onSceStatsUpdate($0, $1);
            }
        },
        enemy_count, enemy_killed);
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

// Forward declarations for get state functions
const char *sce_get_game_state(void);
const char *sce_get_player_state(void);
const char *sce_get_weapon_state(void);

// ============================================
// Initialization
// ============================================

EMSCRIPTEN_KEEPALIVE
void sce_init(void) {
    g_game_sm = std::make_unique<SCE::Generated::game_state::game_state>();
    g_player_sm = std::make_unique<SCE::Generated::player_state::player_state>();
    g_weapon_sm = std::make_unique<SCE::Generated::weapon_state::weapon_state>();

    g_game_sm->initialize();
    g_player_sm->initialize();
    g_weapon_sm->initialize();

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
    js_notify_stats_update(0, 0);
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

// Special handling for newgame - reset all enemies
EMSCRIPTEN_KEEPALIVE
void sce_game_event_newgame(void) {
    if (g_game_sm) {
        g_game_sm->processEvent(SCE::Generated::game_state::Event::Newgame);
        js_notify_state_change("game", sce_get_game_state());
    }

    // Reset player state machine for new game
    if (g_player_sm) {
        g_player_sm->initialize();
        js_notify_state_change("player", sce_get_player_state());
    }

    // Reset weapon state machine for new game
    if (g_weapon_sm) {
        g_weapon_sm->initialize();
        js_notify_state_change("weapon", sce_get_weapon_state());
    }

    // Reset all enemy tracking
    for (int i = 0; i < MAX_ENEMIES; i++) {
        if (g_enemies[i].active) {
            // Get state from state machine before cleanup
            const char *state_name = "UNKNOWN";
            if (g_enemies[i].sm) {
                state_name = get_enemy_state_name(g_enemies[i].sm->getCurrentState());
            }
            // Notify JS to remove this enemy
            js_notify_enemy_update(i, g_enemies[i].type_name, state_name, g_enemies[i].instance_id, false);
            // Reset state machine
            g_enemies[i].sm.reset();
        }
        g_enemies[i].mobj_ptr = nullptr;
        g_enemies[i].instance_id = 0;
        g_enemies[i].type_name = "UNKNOWN";
        g_enemies[i].active = false;
    }
    g_enemy_count = 0;
    g_enemy_killed = 0;
    g_next_instance_id = 1;

    // Notify stats reset
    js_notify_stats_update(0, 0);
}

GAME_EVENT(loadgame, Loadgame)
GAME_EVENT(completed, Completed)
GAME_EVENT(victory, Victory)
GAME_EVENT(died, Died)
GAME_EVENT(quit, Quit)
GAME_EVENT(worlddone, Worlddone)
GAME_EVENT(finale, Finale)
GAME_EVENT(done, Done)
GAME_EVENT(cast, Cast)

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

// Get enemy info by slot: returns "ptr,type,state,instance_id" or empty
EMSCRIPTEN_KEEPALIVE
const char *sce_get_enemy_info(int slot) {
    static char buffer[128];
    if (slot < 0 || slot >= MAX_ENEMIES || !g_enemies[slot].active) {
        buffer[0] = '\0';
        return buffer;
    }
    // Get state from state machine
    const char *state_name = "UNKNOWN";
    if (g_enemies[slot].sm) {
        state_name = get_enemy_state_name(g_enemies[slot].sm->getCurrentState());
    }
    snprintf(buffer, sizeof(buffer), "%ld,%s,%s,%d", (long)(intptr_t)g_enemies[slot].mobj_ptr,
             g_enemies[slot].type_name, state_name, g_enemies[slot].instance_id);
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
    js_notify_stats_update(g_enemy_count, g_enemy_killed);
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
    js_notify_stats_update(g_enemy_count, g_enemy_killed);
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

    js_notify_stats_update(g_enemy_count, g_enemy_killed);
}

}  // extern "C"
