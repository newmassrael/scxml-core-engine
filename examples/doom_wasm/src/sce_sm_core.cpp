// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * @file sce_sm_core.cpp
 * @brief Game, Player, and Weapon state machine modules
 *
 * Contains the three simple lifecycle state machines that model
 * DOOM's core game flow. These SMs have flat topologies (no compound
 * states) and minimal cross-module dependencies.
 *
 * Dependencies:
 * - sce_sm_combo (sce_combo_reset for level transitions / player death)
 * - sce_sm_enemy (sce_sm_reset_all_enemies for new/load game)
 */

#include "sce_sm_internal.h"

#include "game_state_sm.h"
#include "player_state_sm.h"
#include "weapon_state_sm.h"

#include <memory>

// ============================================
// Game SCXML Named Context
// ============================================

struct GameCallbacks {
    void onDemoScreen() {
        SCE_LOG("[GAME SCXML] onDemoScreen callback\n");
        js_notify_game_callback("demo");
        js_notify_state_change("game", "DEMOSCREEN");
    }

    void onLevelStart() {
        SCE_LOG("[GAME SCXML] onLevelStart callback\n");
        js_notify_game_callback("level_start");
        js_notify_state_change("game", "LEVEL");
    }

    void onLevelEnd() {
        SCE_LOG("[GAME SCXML] onLevelEnd callback\n");
        js_notify_game_callback("level_end");
        sce_combo_reset();
    }

    void onIntermission() {
        SCE_LOG("[GAME SCXML] onIntermission callback\n");
        js_notify_game_callback("intermission");
        js_notify_state_change("game", "INTERMISSION");
        sce_combo_reset();
    }

    void onFinale() {
        SCE_LOG("[GAME SCXML] onFinale callback\n");
        js_notify_game_callback("finale");
        js_notify_state_change("game", "FINALE");
        sce_combo_reset();
    }
};

// ============================================
// Player SCXML Named Context
// ============================================

struct PlayerCallbacks {
    void onAlive() {
        SCE_LOG("[PLAYER SCXML] onAlive callback\n");
        js_notify_player_callback("alive");
        js_notify_state_change("player", "ALIVE");
    }

    void onInvulnerable() {
        SCE_LOG("[PLAYER SCXML] onInvulnerable callback\n");
        js_notify_player_callback("invulnerable");
        js_notify_state_change("player", "INVULNERABLE");
    }

    void onDead() {
        SCE_LOG("[PLAYER SCXML] onDead callback\n");
        sce_combo_reset();
        js_notify_player_callback("dead");
        js_notify_state_change("player", "DEAD");
    }

    void onReborn() {
        SCE_LOG("[PLAYER SCXML] onReborn callback\n");
        js_notify_player_callback("reborn");
        js_notify_state_change("player", "REBORN");
    }
};

// ============================================
// Weapon SCXML Named Context
// ============================================

struct WeaponCallbacks {
    void onReady() {
        SCE_LOG("[WEAPON SCXML] onReady callback\n");
        js_notify_weapon_callback("ready");
        js_notify_state_change("weapon", "READY");
    }

    void onLowering() {
        SCE_LOG("[WEAPON SCXML] onLowering callback\n");
        js_notify_weapon_callback("lowering");
        js_notify_state_change("weapon", "LOWERING");
    }

    void onRaising() {
        SCE_LOG("[WEAPON SCXML] onRaising callback\n");
        js_notify_weapon_callback("raising");
        js_notify_state_change("weapon", "RAISING");
    }

    void onFiring() {
        SCE_LOG("[WEAPON SCXML] onFiring callback\n");
        js_notify_weapon_callback("firing");
        js_notify_state_change("weapon", "FIRING");
    }
};

// ============================================
// Type Aliases and Instances
// ============================================

using GameSM = SCE::Generated::game_state::game_state<GameCallbacks>;
using GameEvent = SCE::Generated::game_state::Event;

using PlayerSM = SCE::Generated::player_state::player_state<PlayerCallbacks>;
using PlayerEvent = SCE::Generated::player_state::Event;

using WeaponSM = SCE::Generated::weapon_state::weapon_state<WeaponCallbacks>;
using WeaponEvent = SCE::Generated::weapon_state::Event;

static GameCallbacks g_game_callbacks;
static PlayerCallbacks g_player_callbacks;
static WeaponCallbacks g_weapon_callbacks;

static std::unique_ptr<GameSM> g_game_sm;
static std::unique_ptr<PlayerSM> g_player_sm;
static std::unique_ptr<WeaponSM> g_weapon_sm;

// ============================================
// Cross-Module: Reset Player and Weapon SMs
// ============================================

void sce_sm_reset_player_weapon(void) {
    g_player_sm = std::make_unique<PlayerSM>(g_player_callbacks);
    g_player_sm->initialize();

    g_weapon_sm = std::make_unique<WeaponSM>(g_weapon_callbacks);
    g_weapon_sm->initialize();
}

// ============================================
// Module Initialization
// ============================================

void sce_sm_core_init(void) {
    g_game_sm = std::make_unique<GameSM>(g_game_callbacks);
    g_player_sm = std::make_unique<PlayerSM>(g_player_callbacks);
    g_weapon_sm = std::make_unique<WeaponSM>(g_weapon_callbacks);

    g_game_sm->initialize();
    g_player_sm->initialize();
    g_weapon_sm->initialize();
}

// ============================================
// Game State Machine - extern "C" API
// ============================================

extern "C" {

EMSCRIPTEN_KEEPALIVE
const char *sce_get_game_state(void) {
    if (!g_game_sm) return "UNINITIALIZED";
    switch (g_game_sm->getCurrentState()) {
    case SCE::Generated::game_state::State::Demoscreen:   return "DEMOSCREEN";
    case SCE::Generated::game_state::State::Level:        return "LEVEL";
    case SCE::Generated::game_state::State::Intermission: return "INTERMISSION";
    case SCE::Generated::game_state::State::Finale:       return "FINALE";
    default: return "UNKNOWN";
    }
}

EMSCRIPTEN_KEEPALIVE
void sce_game_event_newgame(void) {
    if (g_game_sm) {
        g_game_sm->raiseExternal(GameEvent::Newgame);
        g_game_sm->step();
    }
    sce_sm_reset_player_weapon();
    sce_sm_reset_all_enemies(false);
    sce_combo_reset();
    sce_sm_aim_reset();
}

EMSCRIPTEN_KEEPALIVE
void sce_game_event_loadgame(void) {
    if (g_game_sm) {
        g_game_sm->raiseExternal(GameEvent::Loadgame);
        g_game_sm->step();
    }
    sce_sm_reset_player_weapon();
    sce_sm_reset_all_enemies(true);
    sce_combo_reset();
    sce_sm_aim_reset();
}

EMSCRIPTEN_KEEPALIVE
void sce_game_event_demostart(void) {
    sce_sm_reset_player_weapon();
    sce_sm_reset_all_enemies(false);
    sce_combo_reset();
    sce_sm_aim_reset();
}

#define GAME_EVENT(name, event)                          \
    EMSCRIPTEN_KEEPALIVE                                 \
    void sce_game_event_##name(void) {                   \
        if (g_game_sm) {                                 \
            g_game_sm->raiseExternal(GameEvent::event);  \
            g_game_sm->step();                           \
        }                                                \
    }

GAME_EVENT(completed, Completed)
GAME_EVENT(worlddone, Worlddone)
GAME_EVENT(finale, Finale)

#undef GAME_EVENT

// ============================================
// Player State Machine - extern "C" API
// ============================================

EMSCRIPTEN_KEEPALIVE
const char *sce_get_player_state(void) {
    if (!g_player_sm) return "UNINITIALIZED";
    switch (g_player_sm->getCurrentState()) {
    case SCE::Generated::player_state::State::Alive:        return "ALIVE";
    case SCE::Generated::player_state::State::Dead:         return "DEAD";
    case SCE::Generated::player_state::State::Reborn:       return "REBORN";
    case SCE::Generated::player_state::State::Invulnerable: return "INVULNERABLE";
    default: return "UNKNOWN";
    }
}

#define PLAYER_EVENT(name, event)                              \
    EMSCRIPTEN_KEEPALIVE                                       \
    void sce_player_event_##name(void) {                       \
        if (g_player_sm) {                                     \
            g_player_sm->raiseExternal(PlayerEvent::event);    \
            g_player_sm->step();                               \
        }                                                      \
    }

PLAYER_EVENT(killed, Killed)
PLAYER_EVENT(spawn_complete, Spawn_complete)
PLAYER_EVENT(god_mode_on, God_mode_on)
PLAYER_EVENT(god_mode_off, God_mode_off)

#undef PLAYER_EVENT

EMSCRIPTEN_KEEPALIVE
void sce_player_event_respawn(void) {
    if (g_player_sm) {
        g_player_sm->raiseExternal(PlayerEvent::Respawn);
        g_player_sm->step();
    }
    sce_sm_reset_all_enemies(false);
    SCE_LOG("[PLAYER] Respawn - reset enemies\n");
}

// ============================================
// Weapon State Machine - extern "C" API
// ============================================

EMSCRIPTEN_KEEPALIVE
const char *sce_get_weapon_state(void) {
    if (!g_weapon_sm) return "UNINITIALIZED";
    switch (g_weapon_sm->getCurrentState()) {
    case SCE::Generated::weapon_state::State::Ready:    return "READY";
    case SCE::Generated::weapon_state::State::Lowering: return "LOWERING";
    case SCE::Generated::weapon_state::State::Raising:  return "RAISING";
    case SCE::Generated::weapon_state::State::Firing:   return "FIRING";
    default: return "UNKNOWN";
    }
}

#define WEAPON_EVENT(name, event)                              \
    EMSCRIPTEN_KEEPALIVE                                       \
    void sce_weapon_event_##name(void) {                       \
        if (g_weapon_sm) {                                     \
            g_weapon_sm->raiseExternal(WeaponEvent::event);    \
            g_weapon_sm->step();                               \
        }                                                      \
    }

WEAPON_EVENT(fire, Fire)
WEAPON_EVENT(switch_weapon, Switch_weapon)
WEAPON_EVENT(lower_complete, Lower_complete)
WEAPON_EVENT(raise_complete, Raise_complete)
WEAPON_EVENT(fire_complete, Fire_complete)

#undef WEAPON_EVENT

}  // extern "C"
