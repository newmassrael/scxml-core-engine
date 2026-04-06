/**
 * @file sce_sm_enemy.cpp
 * @brief Enemy multi-instance state machine module
 *
 * Each enemy (mobj_t with MF_COUNTKILL) gets its own SCXML state machine
 * instance tracking its lifecycle: dormant -> alert -> chasing/attacking -> dead.
 *
 * Optimization: O(1) mobj pointer to slot lookup via unordered_map
 * (replaces O(N) linear scan in the original monolithic implementation).
 *
 * Dependencies:
 * - sce_sm_combo (sce_sm_combo_on_kill for kill combo tracking)
 */

#include "sce_sm_internal.h"

#include "enemy_state_sm.h"

#include <array>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <memory>
#include <unordered_map>

// ============================================
// Constants
// ============================================

static constexpr int MAX_ENEMIES = 64;

// ============================================
// Type Aliases
// ============================================

using EnemyState = SCE::Generated::enemy_state::State;
using EnemyEvent = SCE::Generated::enemy_state::Event;

// ============================================
// Enemy SCXML Named Context
// ============================================

struct EnemyCallbacks {
    int slot = -1;
    int instance_id = 0;
    const char *type_name = "UNKNOWN";

    void onDormant() {
        js_notify_enemy_callback(slot, "dormant", type_name, instance_id);
        js_notify_enemy_update(slot, type_name, "DORMANT", instance_id, true);
    }
    void onAlert() {
        js_notify_enemy_callback(slot, "alert", type_name, instance_id);
        js_notify_enemy_update(slot, type_name, "ALERT", instance_id, true);
    }
    void onChasing() {
        js_notify_enemy_callback(slot, "chasing", type_name, instance_id);
        js_notify_enemy_update(slot, type_name, "CHASING", instance_id, true);
    }
    void onAttacking() {
        js_notify_enemy_callback(slot, "attacking", type_name, instance_id);
        js_notify_enemy_update(slot, type_name, "ATTACKING", instance_id, true);
    }
    void onPain() {
        js_notify_enemy_callback(slot, "pain", type_name, instance_id);
        js_notify_enemy_update(slot, type_name, "PAIN", instance_id, true);
    }
    void onDead() {
        js_notify_enemy_callback(slot, "dead", type_name, instance_id);
        js_notify_enemy_update(slot, type_name, "DEAD", instance_id, true);
    }
};

using EnemySM = SCE::Generated::enemy_state::enemy_state<EnemyCallbacks>;

// ============================================
// Enemy Instance Storage
// ============================================

struct EnemyInstance {
    void *mobj_ptr = nullptr;
    int instance_id = 0;
    const char *type_name = "UNKNOWN";
    EnemyCallbacks enemy;
    std::unique_ptr<EnemySM> sm;
    bool active = false;
};

static std::array<EnemyInstance, MAX_ENEMIES> g_enemies;
static int g_enemy_count = 0;
static int g_enemy_killed = 0;
static int g_next_instance_id = 1;

/** O(1) mobj pointer to slot index lookup */
static std::unordered_map<void *, int> g_enemy_slot_map;

// ============================================
// Helpers
// ============================================

static const char *get_enemy_state_name(EnemyState state) {
    switch (state) {
    case EnemyState::Dormant:   return "DORMANT";
    case EnemyState::Alert:     return "ALERT";
    case EnemyState::Chasing:   return "CHASING";
    case EnemyState::Attacking: return "ATTACKING";
    case EnemyState::Pain:      return "PAIN";
    case EnemyState::Dead:      return "DEAD";
    default:                    return "UNKNOWN";
    }
}

static EnemyEvent doom_state_to_event(const char *state_name) {
    if (strcmp(state_name, "ALERT") == 0)     return EnemyEvent::See_player;
    if (strcmp(state_name, "CHASING") == 0)   return EnemyEvent::Chase;
    if (strcmp(state_name, "ATTACKING") == 0) return EnemyEvent::Attack;
    if (strcmp(state_name, "PAIN") == 0)      return EnemyEvent::Pain;
    return EnemyEvent::NONE;
}

static int find_enemy_slot(void *mobj) {
    auto it = g_enemy_slot_map.find(mobj);
    return (it != g_enemy_slot_map.end()) ? it->second : -1;
}

static int find_free_slot() {
    for (int i = 0; i < MAX_ENEMIES; i++) {
        if (!g_enemies[i].active) return i;
    }
    return -1;
}

// ============================================
// Cross-Module: Reset All Enemies
// ============================================

void sce_sm_reset_all_enemies(bool notify_dead) {
    for (int i = 0; i < MAX_ENEMIES; i++) {
        if (g_enemies[i].active) {
            const char *state_name = "UNKNOWN";
            if (g_enemies[i].sm) {
                state_name = notify_dead
                    ? "DEAD"
                    : get_enemy_state_name(g_enemies[i].sm->getCurrentState());
            }
            js_notify_enemy_update(i, g_enemies[i].type_name, state_name,
                                   g_enemies[i].instance_id, false);
            g_enemies[i].sm.reset();
        }
        g_enemies[i] = EnemyInstance{};
    }
    g_enemy_slot_map.clear();
    g_enemy_count = 0;
    g_enemy_killed = 0;
    g_next_instance_id = 1;
    js_notify_stats_update(0, 0, 0);
}

// ============================================
// Module Initialization
// ============================================

void sce_sm_enemy_init(void) {
    for (auto &e : g_enemies) {
        e = EnemyInstance{};
    }
    g_enemy_slot_map.clear();
    g_enemy_count = 0;
    g_enemy_killed = 0;
    g_next_instance_id = 1;
}

// ============================================
// extern "C" API
// ============================================

extern "C" {

EMSCRIPTEN_KEEPALIVE
int sce_get_enemy_count(void) { return g_enemy_count; }

EMSCRIPTEN_KEEPALIVE
int sce_get_enemy_killed(void) { return g_enemy_killed; }

EMSCRIPTEN_KEEPALIVE
int sce_get_max_enemies(void) { return MAX_ENEMIES; }

EMSCRIPTEN_KEEPALIVE
int sce_get_enemies_remaining(void) { return g_enemy_count; }

// DOOM Level Statistics (defined in sce_doom_hooks.c)
int SCE_GetLevelTotalKills(void);
int SCE_GetPlayerKillCount(void);

EMSCRIPTEN_KEEPALIVE
int sce_get_level_total_kills(void) { return SCE_GetLevelTotalKills(); }

EMSCRIPTEN_KEEPALIVE
int sce_get_player_kill_count(void) { return SCE_GetPlayerKillCount(); }

EMSCRIPTEN_KEEPALIVE
const char *sce_get_enemy_info(int slot) {
    static char buffer[128];
    buffer[0] = '\0';

    if (slot < 0 || slot >= MAX_ENEMIES || !g_enemies[slot].active) {
        return buffer;
    }

    const char *state_name = "UNKNOWN";
    if (g_enemies[slot].sm) {
        state_name = get_enemy_state_name(g_enemies[slot].sm->getCurrentState());
    }

    snprintf(buffer, sizeof(buffer), "%ld,%.31s,%.31s,%d",
             (long)(intptr_t)g_enemies[slot].mobj_ptr,
             g_enemies[slot].type_name ? g_enemies[slot].type_name : "UNKNOWN",
             state_name ? state_name : "UNKNOWN",
             g_enemies[slot].instance_id);
    buffer[sizeof(buffer) - 1] = '\0';
    return buffer;
}

EMSCRIPTEN_KEEPALIVE
void sce_enemy_spawn(void *mobj, const char *type_name) {
    int slot = find_enemy_slot(mobj);
    if (slot < 0) {
        slot = find_free_slot();
        if (slot < 0) return;
    }

    g_enemies[slot].mobj_ptr = mobj;
    g_enemies[slot].instance_id = g_next_instance_id++;
    g_enemies[slot].type_name = type_name;
    g_enemies[slot].active = true;

    g_enemies[slot].enemy.slot = slot;
    g_enemies[slot].enemy.instance_id = g_enemies[slot].instance_id;
    g_enemies[slot].enemy.type_name = type_name;

    g_enemies[slot].sm = std::make_unique<EnemySM>(g_enemies[slot].enemy);
    g_enemies[slot].sm->initialize();

    g_enemy_slot_map[mobj] = slot;
    g_enemy_count++;

    js_notify_stats_update(g_enemy_count + g_enemy_killed, g_enemy_killed, g_enemy_count);
}

EMSCRIPTEN_KEEPALIVE
void sce_enemy_set_state(void *mobj, const char *doom_state) {
    int slot = find_enemy_slot(mobj);
    if (slot < 0 || !g_enemies[slot].sm) return;

    EnemyEvent event = doom_state_to_event(doom_state);
    if (event != EnemyEvent::NONE) {
        g_enemies[slot].sm->raiseExternal(event);
        g_enemies[slot].sm->step();
    }
}

EMSCRIPTEN_KEEPALIVE
void sce_enemy_killed(void *mobj) {
    int slot = find_enemy_slot(mobj);

    if (slot >= 0 && g_enemies[slot].sm) {
        g_enemies[slot].sm->raiseExternal(EnemyEvent::Killed);
        g_enemies[slot].sm->step();

        g_enemy_killed++;
        g_enemy_count--;

        js_notify_stats_update(g_enemy_count + g_enemy_killed, g_enemy_killed, g_enemy_count);
    }

    // Notify combo system (always, even if enemy not tracked)
    sce_sm_combo_on_kill();
}

EMSCRIPTEN_KEEPALIVE
void sce_enemy_remove(void *mobj) {
    int slot = find_enemy_slot(mobj);
    if (slot < 0) return;

    const char *state_name = "UNKNOWN";
    if (g_enemies[slot].sm) {
        state_name = get_enemy_state_name(g_enemies[slot].sm->getCurrentState());
    }

    js_notify_enemy_update(slot, g_enemies[slot].type_name, state_name,
                           g_enemies[slot].instance_id, false);

    bool was_killed = g_enemies[slot].sm &&
        g_enemies[slot].sm->getCurrentState() == EnemyState::Dead;

    g_enemies[slot].sm.reset();
    g_enemies[slot].active = false;
    g_enemy_slot_map.erase(g_enemies[slot].mobj_ptr);
    g_enemies[slot].mobj_ptr = nullptr;

    if (!was_killed) {
        g_enemy_count--;
    }

    js_notify_stats_update(g_enemy_count + g_enemy_killed, g_enemy_killed, g_enemy_count);
}

EMSCRIPTEN_KEEPALIVE
void sce_enemy_clear_all(void) {
    for (int i = 0; i < MAX_ENEMIES; i++) {
        if (g_enemies[i].active) {
            g_enemies[i].sm.reset();
            g_enemies[i].active = false;
            g_enemies[i].mobj_ptr = nullptr;
        }
    }
    g_enemy_slot_map.clear();
    g_enemy_count = 0;
    g_enemy_killed = 0;
    js_notify_stats_update(g_enemy_count, g_enemy_killed, g_enemy_count);
}

}  // extern "C"
