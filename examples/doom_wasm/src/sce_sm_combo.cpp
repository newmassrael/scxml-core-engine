// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * @file sce_sm_combo.cpp
 * @brief Kill Combo and Berserk state machine module
 *
 * W3C SCXML 3.4 compound state machine:
 *   idle / active { normal, berserk }
 *
 * External Pausable Timer Architecture:
 * - Game manages timers externally (not SCXML internal scheduler)
 * - Timers pause when menu opens, resume when menu closes
 * - onentry callbacks start timers, sce_process_tic checks expiry
 * - PausableTimer class encapsulates all timing state
 *
 * Dependencies:
 * - sce_doom_hooks.c (SCE_BerserkHealPlayer, SCE_BerserkSetMultiplier, SCE_BerserkReset)
 * - sce_hud (SCE_HUD_SetCombo, SCE_HUD_SetBerserk, SCE_HUD_SetComboTimer)
 */

#include "sce_sm_internal.h"
#include "sce_timer.h"
#include "sce_hud.h"

#include "combo_state_sm.h"

#include <cstring>
#include <memory>

// ============================================
// DOOM Hooks (defined in sce_doom_hooks.c)
// ============================================

extern "C" {
void SCE_BerserkHealPlayer(void);
void SCE_BerserkSetMultiplier(float multiplier);
void SCE_BerserkReset(void);
}

// ============================================
// Constants
// ============================================

static constexpr int BERSERK_TRIGGER_COMBO = 5;
static constexpr int BERSERK_MAX_INTENSITY = 10;
static constexpr double COMBO_TIMEOUT_MS = 5000.0;
static constexpr double BERSERK_TIMEOUT_MS = 10000.0;

// ============================================
// Module State
// ============================================

static PausableTimer g_timer;
static bool g_timer_pause_ui_sent = false;

static int g_combo_count = 0;
static int g_berserk_intensity = 0;

// ============================================
// Local Notification Helpers (HUD + JS)
// ============================================

static void notify_combo_update(int count, bool active) {
    SCE_HUD_SetCombo(count, active);
    js_notify_combo_update(count, active);
}

static void notify_berserk_update(int intensity, bool active) {
    SCE_HUD_SetBerserk(intensity, active);
    js_notify_berserk_update(intensity, active);
}

static void notify_combo_timer(double remaining_ms, double total_ms) {
    SCE_HUD_SetComboTimer(remaining_ms, total_ms);
    js_notify_combo_timer(remaining_ms, total_ms);
}

// ============================================
// Combo SCXML Named Context
// ============================================

struct ComboCallbacks {
    void onIdle() {
        SCE_LOG("[COMBO SCXML] onIdle callback\n");

        g_timer.cancel();
        g_timer_pause_ui_sent = false;

        if (g_berserk_intensity > 0) {
            SCE_LOG("[COMBO] Combo ended - resetting berserk intensity from %d to 0\n",
                   g_berserk_intensity);
            g_berserk_intensity = 0;
            notify_berserk_update(0, false);
        }

        js_notify_combo_callback("idle");
        js_notify_state_change("combo", "IDLE");
    }

    void onActive() {
        SCE_LOG("[COMBO SCXML] onActive (normal) callback\n");
        js_notify_combo_callback("active");
        js_notify_state_change("combo", "NORMAL");

        g_timer.start(PausableTimer::Type::COMBO, COMBO_TIMEOUT_MS);
        g_timer_pause_ui_sent = false;

        SCE_LOG("[COMBO] External timer started: %.0fms (pausable)\n", COMBO_TIMEOUT_MS);
    }
};

// ============================================
// Berserk SCXML Named Context
// ============================================

struct BerserkCallbacks {
    void onActive() {
        SCE_LOG("[BERSERK SCXML] onActive callback - intensity=%d\n", g_berserk_intensity);
        js_notify_combo_callback("berserk");
        js_notify_state_change("combo", "BERSERK");

        g_berserk_intensity++;

        int capped = (g_berserk_intensity > BERSERK_MAX_INTENSITY)
                     ? BERSERK_MAX_INTENSITY : g_berserk_intensity;
        float multiplier = (float)capped;

        SCE_BerserkSetMultiplier(multiplier);
        SCE_BerserkHealPlayer();

        notify_berserk_update(g_berserk_intensity, true);

        g_timer.start(PausableTimer::Type::BERSERK, BERSERK_TIMEOUT_MS);
        g_timer_pause_ui_sent = false;

        SCE_LOG("[BERSERK] Activated! Intensity=%d, Multiplier=x%.0f, Timer=%.0fms (pausable)\n",
               g_berserk_intensity, multiplier, BERSERK_TIMEOUT_MS);
    }

    void onExit() {
        SCE_LOG("[BERSERK SCXML] onExit callback\n");
        SCE_BerserkReset();
        notify_berserk_update(g_berserk_intensity, false);
        SCE_LOG("[BERSERK] Exiting state (intensity=%d preserved)\n", g_berserk_intensity);
    }
};

// ============================================
// Type Aliases and Instance
// ============================================

using ComboSM = SCE::Generated::combo_state::combo_state<ComboCallbacks, BerserkCallbacks>;
using ComboEvent = SCE::Generated::combo_state::Event;
using ComboState = SCE::Generated::combo_state::State;

static ComboCallbacks g_combo_callbacks;
static BerserkCallbacks g_berserk_callbacks;
static std::unique_ptr<ComboSM> g_combo_sm;

// ============================================
// DOOM gameplay state (combo only activates during real gameplay)
// ============================================

extern "C" {
extern bool demoplayback;  // DOOM global: true during demo playback
}

static bool is_real_gameplay(void) {
    const char *state = sce_get_game_state();
    return state && strcmp(state, "LEVEL") == 0 && !demoplayback;
}

// ============================================
// Cross-Module: Combo On Kill
// ============================================

void sce_sm_combo_on_kill(void) {
    if (!g_combo_sm || !is_real_gameplay()) return;

    g_combo_sm->raiseExternal(ComboEvent::Kill);
    g_combo_sm->step();

    ComboState new_state = g_combo_sm->getCurrentState();

    if (new_state == ComboState::Normal) {
        g_combo_count++;

        SCE_LOG("[COMBO] Kill #%d in NORMAL mode - external timer restarted (5s)\n",
               g_combo_count);

        if (g_combo_count >= BERSERK_TRIGGER_COMBO) {
            SCE_LOG("[COMBO] Combo >= %d, triggering BERSERK!\n", BERSERK_TRIGGER_COMBO);
            g_combo_sm->raiseExternal(ComboEvent::Berserk_activate);
            g_combo_sm->step();
        }

        notify_combo_update(g_combo_count, true);

    } else if (new_state == ComboState::Berserk) {
        g_combo_count++;

        SCE_LOG("[BERSERK] Kill #%d - external timer restarted (10s), intensity=%d\n",
               g_combo_count, g_berserk_intensity);

        notify_combo_update(g_combo_count, true);
    }
}

// ============================================
// Module Initialization
// ============================================

void sce_sm_combo_init(void) {
    g_combo_sm = std::make_unique<ComboSM>(g_combo_callbacks, g_berserk_callbacks);
    g_combo_sm->initialize();

    g_timer.cancel();
    g_timer_pause_ui_sent = false;
    g_combo_count = 0;
    g_berserk_intensity = 0;
}

// ============================================
// extern "C" API
// ============================================

extern "C" {

EMSCRIPTEN_KEEPALIVE
const char *sce_combo_get_state(void) {
    if (!g_combo_sm) return "UNINITIALIZED";
    switch (g_combo_sm->getCurrentState()) {
    case ComboState::Idle:    return "idle";
    case ComboState::Normal:  return "normal";
    case ComboState::Berserk: return "berserk";
    default: return "UNKNOWN";
    }
}

EMSCRIPTEN_KEEPALIVE
int sce_combo_get_count(void) { return g_combo_count; }

EMSCRIPTEN_KEEPALIVE
int sce_combo_is_active(void) {
    if (!g_combo_sm) return 0;
    ComboState state = g_combo_sm->getCurrentState();
    return (state == ComboState::Normal || state == ComboState::Berserk) ? 1 : 0;
}

EMSCRIPTEN_KEEPALIVE
int sce_combo_is_berserk(void) {
    if (!g_combo_sm) return 0;
    return (g_combo_sm->getCurrentState() == ComboState::Berserk) ? 1 : 0;
}

EMSCRIPTEN_KEEPALIVE
int sce_berserk_get_intensity(void) { return g_berserk_intensity; }

// ============================================
// Timer Tick Processing (called at 35Hz from G_Ticker)
// ============================================

EMSCRIPTEN_KEEPALIVE
void sce_process_tic(void) {
    if (!g_combo_sm || !is_real_gameplay()) return;

    if (g_timer.isActive() && !g_timer.isPaused()) {
        if (g_timer.isExpired()) {
            ComboState prev_state = g_combo_sm->getCurrentState();

            if (g_timer.getType() == PausableTimer::Type::COMBO) {
                SCE_LOG("[COMBO] External timer expired (%.0fms) - raising combo_timeout\n",
                       g_timer.getDurationMs());
                g_combo_sm->raiseExternal(ComboEvent::Combo_timeout);
            } else if (g_timer.getType() == PausableTimer::Type::BERSERK) {
                SCE_LOG("[BERSERK] External timer expired (%.0fms) - raising berserk_timeout\n",
                       g_timer.getDurationMs());
                g_combo_sm->raiseExternal(ComboEvent::Berserk_timeout);
            }

            g_combo_sm->step();

            ComboState current_state = g_combo_sm->getCurrentState();
            if (current_state == ComboState::Idle && prev_state != ComboState::Idle) {
                SCE_LOG("[COMBO/BERSERK] State: %s -> idle, resetting combo from %d to 0\n",
                       prev_state == ComboState::Normal ? "normal" : "berserk",
                       g_combo_count);
                g_combo_count = 0;
                notify_combo_update(0, false);
                notify_combo_timer(-1, 0);
            }
        } else {
            double remaining = g_timer.getRemainingMs();
            notify_combo_timer(remaining, g_timer.getDurationMs());
        }
    } else if (g_timer.isActive() && g_timer.isPaused()) {
        if (!g_timer_pause_ui_sent) {
            notify_combo_timer(g_timer.getRemainingMs(), g_timer.getDurationMs());
            g_timer_pause_ui_sent = true;
        }
    }
}

// ============================================
// Timer Pause / Resume API
// ============================================

EMSCRIPTEN_KEEPALIVE
void sce_combo_timer_pause(void) {
    SCE_LOG("[TIMER] sce_combo_timer_pause() CALLED - active=%d, paused=%d\n",
           g_timer.isActive(), g_timer.isPaused());

    if (!g_timer.isActive() || g_timer.isPaused()) {
        SCE_LOG("[TIMER] Early return - no timer or already paused\n");
        return;
    }

    g_timer.pause();
    g_timer_pause_ui_sent = false;

    SCE_LOG("[TIMER] Paused at %.0fms / %.0fms\n",
           g_timer.getDurationMs() - g_timer.getRemainingMs(), g_timer.getDurationMs());
}

EMSCRIPTEN_KEEPALIVE
void sce_combo_timer_resume(void) {
    if (!g_timer.isActive() || !g_timer.isPaused()) return;

    g_timer.resume();

    SCE_LOG("[TIMER] Resumed at %.0fms / %.0fms\n",
           g_timer.getDurationMs() - g_timer.getRemainingMs(), g_timer.getDurationMs());
}

EMSCRIPTEN_KEEPALIVE
int sce_combo_timer_is_paused(void) {
    return (g_timer.isActive() && g_timer.isPaused()) ? 1 : 0;
}

EMSCRIPTEN_KEEPALIVE
void sce_combo_timer_get(double *out_remaining_ms, double *out_total_ms) {
    if (!g_timer.isActive()) {
        *out_remaining_ms = -1;
        *out_total_ms = 0;
        return;
    }
    *out_remaining_ms = g_timer.getRemainingMs();
    *out_total_ms = g_timer.getDurationMs();
}

// ============================================
// Combo Reset (cross-module, called from game/player modules)
// ============================================

EMSCRIPTEN_KEEPALIVE
void sce_combo_reset(void) {
    if (g_combo_sm) {
        SCE_BerserkReset();

        g_timer.cancel();
        g_timer_pause_ui_sent = false;

        g_combo_sm = std::make_unique<ComboSM>(g_combo_callbacks, g_berserk_callbacks);
        g_combo_sm->initialize();

        g_combo_count = 0;
        g_berserk_intensity = 0;
        notify_combo_update(0, false);
        notify_berserk_update(0, false);
        notify_combo_timer(-1, 0);
        js_notify_state_change("combo", sce_combo_get_state());
    }
}

}  // extern "C"
