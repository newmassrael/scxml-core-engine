/**
 * @file sce_sm_internal.h
 * @brief Cross-module interface for SCE state machine modules
 *
 * Provides:
 * - Emscripten compatibility macros
 * - Module initialization function declarations
 * - Cross-module function declarations for inter-SM communication
 *
 * Each sce_sm_*.cpp includes this header for shared infrastructure.
 */

#ifndef SCE_SM_INTERNAL_H
#define SCE_SM_INTERNAL_H

#ifdef __EMSCRIPTEN__
#include <emscripten.h>
#else
#ifndef EMSCRIPTEN_KEEPALIVE
#define EMSCRIPTEN_KEEPALIVE
#endif
#endif

// ============================================
// Conditional Debug Logging
// ============================================
// Define SCE_DEBUG before including this header to enable verbose logging.
// In release builds, all SCE_LOG calls compile to nothing.
#ifdef SCE_DEBUG
#include <cstdio>
#define SCE_LOG(fmt, ...) printf(fmt, ##__VA_ARGS__)
#else
#define SCE_LOG(fmt, ...) ((void)0)
#endif

#include "sce_js_notify.h"

// ============================================
// Module Initialization (called from sce_init)
// ============================================
void sce_sm_core_init(void);
void sce_sm_enemy_init(void);
void sce_sm_combo_init(void);
void sce_sm_aim_init(void);
void sce_sm_secret_init(void);

// ============================================
// Cross-Module Functions
// ============================================

/** Reset player and weapon SMs to initial state (for new/load game) */
void sce_sm_reset_player_weapon(void);

/** Reset all enemy tracking (for new/load game) */
void sce_sm_reset_all_enemies(bool notify_dead);

/** Notify combo system of a kill (called from enemy module) */
void sce_sm_combo_on_kill(void);

// ============================================
// State Query Functions (used across modules)
// ============================================
extern "C" {
const char *sce_get_game_state(void);
const char *sce_get_player_state(void);
const char *sce_get_weapon_state(void);
const char *sce_secret_get_state(void);
const char *sce_aim_get_state(void);
const char *sce_combo_get_state(void);
void sce_combo_reset(void);
}

#endif /* SCE_SM_INTERNAL_H */
