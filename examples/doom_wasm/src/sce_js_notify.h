// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * @file sce_js_notify.h
 * @brief Centralized JavaScript notification layer for SCE state machines
 *
 * All C++ to Browser communication goes through these functions.
 * Single point of truth for the EM_ASM bridge.
 *
 * Design: Pure notification layer - no game logic, no HUD updates.
 * Each function maps 1:1 to a window.onSce*() JavaScript callback.
 */

#ifndef SCE_JS_NOTIFY_H
#define SCE_JS_NOTIFY_H

#ifdef __EMSCRIPTEN__
#include <emscripten.h>
#endif

// ============================================
// State Machine Notifications
// ============================================

/** Notify state change for any state machine */
inline void js_notify_state_change(const char *machine, const char *state) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceStateChange == = 'function') {
                window.onSceStateChange(UTF8ToString($0), UTF8ToString($1));
            }
        },
        machine, state);
#endif
}

// ============================================
// Enemy Notifications
// ============================================

/** Notify enemy state update (per-slot) */
inline void js_notify_enemy_update(int slot, const char *type, const char *state, int instance_id, bool active) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceEnemyUpdate == = 'function') {
                window.onSceEnemyUpdate($0, UTF8ToString($1), UTF8ToString($2), $3, $4);
            }
        },
        slot, type, state, instance_id, active ? 1 : 0);
#endif
}

/** Notify enemy statistics update */
inline void js_notify_stats_update(int enemy_count, int enemy_killed, int enemy_remaining) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceStatsUpdate == = 'function') {
                window.onSceStatsUpdate($0, $1, $2);
            }
        },
        enemy_count, enemy_killed, enemy_remaining);
#endif
}

/** Notify enemy callback event */
inline void js_notify_enemy_callback(int slot, const char *callback_type, const char *type_name, int instance_id) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceEnemyCallback == = 'function') {
                window.onSceEnemyCallback($0, UTF8ToString($1), UTF8ToString($2), $3);
            }
        },
        slot, callback_type, type_name, instance_id);
#endif
}

// ============================================
// Secret Hint Notifications
// ============================================

/** Notify secret path calculation result */
inline void js_notify_secret_path(int num_arrows, int remaining_secrets, bool is_partial) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceSecretPath == = 'function') {
                window.onSceSecretPath($0, $1, $2);
            }
        },
        num_arrows, remaining_secrets, is_partial ? 1 : 0);
#endif
}

/** Notify individual arrow position */
inline void js_notify_secret_arrow(int index, int x, int y, int angle) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceSecretArrow == = 'function') {
                window.onSceSecretArrow($0, $1, $2, $3);
            }
        },
        index, x, y, angle);
#endif
}

/** Notify target selection info */
inline void js_notify_target_info(const char *type_name, const char *name, int index, int total, int trigger_x,
                                  int trigger_y, int sector_x, int sector_y, int sector_idx, const char *open_method,
                                  int is_hidden, int linked_secret) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceTargetInfo == = 'function') {
                window.onSceTargetInfo(UTF8ToString($0), UTF8ToString($1), $2, $3, $4, $5, $6, $7, $8, UTF8ToString($9),
                                       $10, $11);
            }
        },
        type_name, name, index, total, trigger_x, trigger_y, sector_x, sector_y, sector_idx, open_method, is_hidden,
        linked_secret);
#endif
}

/** Notify secret callback event */
inline void js_notify_secret_callback(const char *callback_type) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceSecretCallback == = 'function') {
                window.onSceSecretCallback(UTF8ToString($0));
            }
        },
        callback_type);
#endif
}

// ============================================
// Aim Assist Notifications
// ============================================

/** Notify aim assist enabled/disabled state */
inline void js_notify_aim_assist_state(bool enabled) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceAimAssistState == = 'function') {
                window.onSceAimAssistState($0);
            }
        },
        enabled ? 1 : 0);
#endif
}

/** Notify aim assist callback event */
inline void js_notify_aim_callback(const char *callback_type) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceAimCallback == = 'function') {
                window.onSceAimCallback(UTF8ToString($0));
            }
        },
        callback_type);
#endif
}

// ============================================
// Combo / Berserk Notifications
// ============================================

/** Notify combo count update */
inline void js_notify_combo_update(int count, bool active) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceComboUpdate == = 'function') {
                window.onSceComboUpdate($0, $1);
            }
        },
        count, active ? 1 : 0);
#endif
}

/** Notify combo timer progress */
inline void js_notify_combo_timer(double remaining_ms, double total_ms) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceComboTimer == = 'function') {
                window.onSceComboTimer($0, $1);
            }
        },
        remaining_ms, total_ms);
#endif
}

/** Notify combo callback event */
inline void js_notify_combo_callback(const char *callback_type) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceComboCallback == = 'function') {
                window.onSceComboCallback(UTF8ToString($0));
            }
        },
        callback_type);
#endif
}

/** Notify berserk mode update */
inline void js_notify_berserk_update(int intensity, bool active) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceBerserkUpdate == = 'function') {
                window.onSceBerserkUpdate($0, $1);
            }
        },
        intensity, active ? 1 : 0);
#endif
}

// ============================================
// Game / Player / Weapon Callback Notifications
// ============================================

/** Notify game state callback event */
inline void js_notify_game_callback(const char *callback_type) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceGameCallback == = 'function') {
                window.onSceGameCallback(UTF8ToString($0));
            }
        },
        callback_type);
#endif
}

/** Notify player state callback event */
inline void js_notify_player_callback(const char *callback_type) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onScePlayerCallback == = 'function') {
                window.onScePlayerCallback(UTF8ToString($0));
            }
        },
        callback_type);
#endif
}

/** Notify weapon state callback event */
inline void js_notify_weapon_callback(const char *callback_type) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceWeaponCallback == = 'function') {
                window.onSceWeaponCallback(UTF8ToString($0));
            }
        },
        callback_type);
#endif
}

#endif /* SCE_JS_NOTIFY_H */
