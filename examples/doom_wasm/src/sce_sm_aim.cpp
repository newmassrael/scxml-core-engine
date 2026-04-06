/**
 * @file sce_sm_aim.cpp
 * @brief Aim Assist state machine module
 *
 * W3C SCXML 3.4 compound state machine:
 *   disabled / enabled { idle, searching, locked }
 *
 * Provides per-shot target acquisition: when player fires, the SM
 * transitions through searching -> locked -> idle, tracking whether
 * aim assist found and locked onto a target.
 *
 * Dependencies: None (self-contained module)
 */

#include "sce_sm_internal.h"

#include "aim_assist_state_sm.h"

#include <memory>

// ============================================
// DOOM Hook (defined in sce_doom_hooks.c)
// ============================================

extern "C" {
void SCE_AimAssistClearTarget(void);
}

// ============================================
// Module State
// ============================================

static bool g_aim_assist_enabled = false;

// ============================================
// Aim Assist SCXML Named Context
// ============================================

struct AimCallbacks {
    void onDisabled() {
        SCE_LOG("[AIM SCXML] onDisabled callback\n");
        js_notify_aim_callback("disabled");
        g_aim_assist_enabled = false;
        SCE_AimAssistClearTarget();
        js_notify_aim_assist_state(false);
        js_notify_state_change("aim", "DISABLED");
    }

    void onEnabled() {
        SCE_LOG("[AIM SCXML] onEnabled callback\n");
        js_notify_aim_callback("enabled");
        g_aim_assist_enabled = true;
        js_notify_aim_assist_state(true);
    }

    void onIdle() {
        SCE_LOG("[AIM SCXML] onIdle callback\n");
        js_notify_aim_callback("idle");
        SCE_AimAssistClearTarget();
        js_notify_state_change("aim", "IDLE");
    }

    void onSearching() {
        SCE_LOG("[AIM SCXML] onSearching callback\n");
        js_notify_aim_callback("searching");
        js_notify_state_change("aim", "SEARCHING");
    }

    void onLocked() {
        SCE_LOG("[AIM SCXML] onLocked callback\n");
        js_notify_aim_callback("locked");
        js_notify_state_change("aim", "LOCKED");
    }
};

// ============================================
// Type Aliases and Instance
// ============================================

using AimSM = SCE::Generated::aim_assist_state::aim_assist_state<AimCallbacks>;
using AimEvent = SCE::Generated::aim_assist_state::Event;

static AimCallbacks g_aim_callbacks;
static std::unique_ptr<AimSM> g_aim_sm;

// ============================================
// Module Initialization
// ============================================

void sce_sm_aim_init(void) {
    g_aim_sm = std::make_unique<AimSM>(g_aim_callbacks);
    g_aim_sm->initialize();
    g_aim_assist_enabled = false;
}

// ============================================
// extern "C" API
// ============================================

extern "C" {

EMSCRIPTEN_KEEPALIVE
const char *sce_aim_get_state(void) {
    if (!g_aim_sm) return "UNINITIALIZED";
    switch (g_aim_sm->getCurrentState()) {
    case SCE::Generated::aim_assist_state::State::Disabled:  return "disabled";
    case SCE::Generated::aim_assist_state::State::Enabled:   return "enabled";
    case SCE::Generated::aim_assist_state::State::Idle:      return "idle";
    case SCE::Generated::aim_assist_state::State::Searching: return "searching";
    case SCE::Generated::aim_assist_state::State::Locked:    return "locked";
    default: return "UNKNOWN";
    }
}

EMSCRIPTEN_KEEPALIVE
void sce_aim_event_toggle(void) {
    if (g_aim_sm) {
        g_aim_sm->raiseExternal(AimEvent::Toggle);
        g_aim_sm->step();
    }
}

EMSCRIPTEN_KEEPALIVE
void sce_aim_event_shot_fired(void) {
    SCE_LOG("[AIM EVENT] shot_fired, g_aim_sm=%p\n", (void *)g_aim_sm.get());
    if (g_aim_sm) {
        g_aim_sm->raiseExternal(AimEvent::Shot_fired);
        g_aim_sm->step();
    }
}

EMSCRIPTEN_KEEPALIVE
void sce_aim_event_target_acquired(void) {
    SCE_LOG("[AIM EVENT] target_acquired, g_aim_sm=%p\n", (void *)g_aim_sm.get());
    if (g_aim_sm) {
        g_aim_sm->raiseExternal(AimEvent::Target_acquired);
        g_aim_sm->step();
    }
}

EMSCRIPTEN_KEEPALIVE
void sce_aim_event_target_lost(void) {
    SCE_LOG("[AIM EVENT] target_lost, g_aim_sm=%p\n", (void *)g_aim_sm.get());
    if (g_aim_sm) {
        g_aim_sm->raiseExternal(AimEvent::Target_lost);
        g_aim_sm->step();
    }
}

EMSCRIPTEN_KEEPALIVE
void sce_aim_event_no_target(void) {
    SCE_LOG("[AIM EVENT] no_target, g_aim_sm=%p\n", (void *)g_aim_sm.get());
    if (g_aim_sm) {
        g_aim_sm->raiseExternal(AimEvent::No_target);
        g_aim_sm->step();
    }
}

EMSCRIPTEN_KEEPALIVE
void sce_aim_event_shot_complete(void) {
    SCE_LOG("[AIM EVENT] shot_complete, g_aim_sm=%p\n", (void *)g_aim_sm.get());
    if (g_aim_sm) {
        g_aim_sm->raiseExternal(AimEvent::Shot_complete);
        g_aim_sm->step();
    }
}

EMSCRIPTEN_KEEPALIVE
int sce_aim_is_enabled(void) {
    return g_aim_assist_enabled ? 1 : 0;
}

}  // extern "C"
