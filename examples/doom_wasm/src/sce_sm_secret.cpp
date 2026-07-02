// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * @file sce_sm_secret.cpp
 * @brief Secret Hint state machine module
 *
 * W3C SCXML 3.4 compound state machine:
 *   disabled / enabled { calculating, showing, found, no_path }
 *
 * Integrates BFS pathfinding (sce_secret_hint.c) with the SCXML state machine.
 * Uses deferred BFS result processing to avoid nested step() calls:
 *   raiseExternal(Select) -> step() -> onCalculating() stores BFS result
 *   -> process_bfs_result() raises event -> step() transitions state
 *
 * Dependencies:
 * - sce_secret_hint.c (Secret_* pathfinding functions)
 */

#include "sce_sm_internal.h"

#include "sce_secret_hint.h"
#include "secret_hint_state_sm.h"

#include <memory>

// ============================================
// BFS Result Deferred Processing
// ============================================

enum class BfsResult { NONE, PATH_FOUND, NO_PATH };
static BfsResult g_bfs_result = BfsResult::NONE;

// Forward declaration
static void process_bfs_result(void);

// ============================================
// Secret Hint SCXML Named Context
// ============================================

struct SecretCallbacks {
    void onDisabled() {
        SCE_LOG("[SECRET SCXML] onDisabled callback\n");
        js_notify_secret_callback("disabled");
        Secret_ClearPath();
        js_notify_state_change("secret", "DISABLED");
        int count = Secret_GetRemainingCount();
        js_notify_secret_path(0, count, false);
        js_notify_target_info("", "", -1, 0, 0, 0, 0, 0, -1, "", 0, -1);
    }

    void onCalculating() {
        SCE_LOG("[SECRET SCXML] onCalculating callback\n");
        js_notify_secret_callback("calculating");
        js_notify_state_change("secret", "CALCULATING");

        g_bfs_result = BfsResult::NONE;

        // Notify target info for button highlighting
        target_info_t info = {};
        bool has_target = Secret_GetCurrentTarget(&info);
        if (has_target) {
            target_type_t type;
            int index, total;
            Secret_GetSelectionInfo(&type, &index, &total);

            js_notify_target_info(Secret_GetTargetTypeName(type), info.name, index, total, info.x >> 16, info.y >> 16,
                                  info.sector_x >> 16, info.sector_y >> 16, info.sector_index,
                                  Secret_GetDoorOpenMethodName(info.open_method), info.is_hidden, info.linked_secret);
        }

        // Run BFS and store result (no event raising during step)
        secret_path_t path = {};
        bool bfs_success = Secret_FindPathToCurrentTarget(&path);
        if (bfs_success && path.num_arrows > 0) {
            bool is_partial = false;
            if (has_target && info.type == TARGET_SECRET && path.target_sector != info.index) {
                is_partial = true;
            }
            js_notify_secret_path(path.num_arrows, Secret_GetRemainingCount(), is_partial);
            for (int i = 0; i < path.num_arrows; i++) {
                js_notify_secret_arrow(i, path.arrows[i].x >> 16, path.arrows[i].y >> 16, path.arrows[i].angle >> 16);
            }
            g_bfs_result = BfsResult::PATH_FOUND;
        } else {
            g_bfs_result = BfsResult::NO_PATH;
        }
    }

    void onShowing() {
        SCE_LOG("[SECRET SCXML] onShowing callback\n");
        js_notify_secret_callback("showing");
        js_notify_state_change("secret", "SHOWING");
    }

    void onExitShowing() {
        SCE_LOG("[SECRET SCXML] onExitShowing callback\n");
        Secret_ClearPath();
    }

    void onFound() {
        SCE_LOG("[SECRET SCXML] onFound callback\n");
        js_notify_secret_callback("found");
        js_notify_state_change("secret", "FOUND");
    }

    void onExitFound() {
        SCE_LOG("[SECRET SCXML] onExitFound callback\n");
        Secret_ClearPath();
    }

    void onNoPath() {
        SCE_LOG("[SECRET SCXML] onNoPath callback\n");
        js_notify_secret_callback("no_path");
        js_notify_state_change("secret", "NO_PATH");
    }
};

// ============================================
// Type Aliases and Instance
// ============================================

using SecretSM = SCE::Generated::secret_hint_state::secret_hint_state<SecretCallbacks>;
using SecretEvent = SCE::Generated::secret_hint_state::Event;

static SecretCallbacks g_secret_callbacks;
static std::unique_ptr<SecretSM> g_secret_sm;

// ============================================
// BFS Result Processing (deferred after step)
// ============================================

static void process_bfs_result(void) {
    if (!g_secret_sm) {
        return;
    }

    switch (g_bfs_result) {
    case BfsResult::PATH_FOUND:
        g_secret_sm->raiseExternal(SecretEvent::Path_found);
        g_secret_sm->step();
        break;
    case BfsResult::NO_PATH:
        g_secret_sm->raiseExternal(SecretEvent::No_path);
        g_secret_sm->step();
        break;
    case BfsResult::NONE:
        break;
    }
    g_bfs_result = BfsResult::NONE;
}

// ============================================
// Module Initialization
// ============================================

void sce_sm_secret_init(void) {
    g_secret_sm = std::make_unique<SecretSM>(g_secret_callbacks);
    g_secret_sm->initialize();
}

// ============================================
// extern "C" API
// ============================================

extern "C" {

EMSCRIPTEN_KEEPALIVE
const char *sce_secret_get_state(void) {
    if (!g_secret_sm) {
        return "UNINITIALIZED";
    }
    switch (g_secret_sm->getCurrentState()) {
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

EMSCRIPTEN_KEEPALIVE
void sce_secret_event_toggle(void) {
    if (g_secret_sm) {
        g_secret_sm->raiseExternal(SecretEvent::Toggle);
        g_secret_sm->step();
    }
}

EMSCRIPTEN_KEEPALIVE
void sce_secret_event_enable(void) {
    sce_secret_event_toggle();
}

EMSCRIPTEN_KEEPALIVE
void sce_secret_event_disable(void) {
    sce_secret_event_toggle();
}

EMSCRIPTEN_KEEPALIVE
void sce_secret_discovered(void) {
    js_notify_secret_path(0, Secret_GetRemainingCount(), false);
}

EMSCRIPTEN_KEEPALIVE
void sce_secret_update_count(void) {
    js_notify_secret_path(0, Secret_GetRemainingCount(), false);
}

EMSCRIPTEN_KEEPALIVE
void sce_secret_event_next_target(void) {
    sce_secret_event_toggle();
}

EMSCRIPTEN_KEEPALIVE
void sce_secret_event_prev_target(void) { /* Deprecated, no-op */ }

EMSCRIPTEN_KEEPALIVE
void sce_select_target(int type, int index) {
    if (!g_secret_sm) {
        return;
    }

    target_type_t curr_type;
    int curr_index, curr_total;
    Secret_GetSelectionInfo(&curr_type, &curr_index, &curr_total);

    bool is_same_target = (curr_type == (target_type_t)type && curr_index == index);

    auto current_state = g_secret_sm->getCurrentState();
    bool in_disabled = (current_state == SCE::Generated::secret_hint_state::State::Disabled);
    bool in_no_path = (current_state == SCE::Generated::secret_hint_state::State::No_path);

    if (Secret_SelectTarget((target_type_t)type, index)) {
        if (in_disabled) {
            g_secret_sm->raiseExternal(SecretEvent::Select);
            g_secret_sm->step();
            process_bfs_result();
        } else if (is_same_target && !in_no_path) {
            g_secret_sm->raiseExternal(SecretEvent::Toggle);
            g_secret_sm->step();
        } else {
            g_secret_sm->raiseExternal(SecretEvent::Select);
            g_secret_sm->step();
            process_bfs_result();
        }
    }
}

EMSCRIPTEN_KEEPALIVE
void sce_secret_event_request(void) {
    sce_secret_event_next_target();
}

EMSCRIPTEN_KEEPALIVE
void sce_secret_event_select(void) {
    if (g_secret_sm) {
        g_secret_sm->raiseExternal(SecretEvent::Select);
        g_secret_sm->step();
        process_bfs_result();
    }
}

EMSCRIPTEN_KEEPALIVE
void sce_secret_event_cancel(void) {
    sce_secret_event_toggle();
}

EMSCRIPTEN_KEEPALIVE
void sce_secret_event_level_change(void) {
    if (g_secret_sm) {
        g_secret_sm->raiseExternal(SecretEvent::Level_change);
        g_secret_sm->step();
    }
    js_notify_secret_path(0, Secret_GetRemainingCount(), false);
}

EMSCRIPTEN_KEEPALIVE
void sce_secret_event_reached(void) {
    if (g_secret_sm) {
        g_secret_sm->raiseExternal(SecretEvent::Secret_reached);
        g_secret_sm->step();
    }
}

EMSCRIPTEN_KEEPALIVE
void sce_secret_path_found(void) {
    if (g_secret_sm) {
        g_secret_sm->raiseExternal(SecretEvent::Path_found);
    }
}

EMSCRIPTEN_KEEPALIVE
void sce_secret_no_path(void) {
    if (g_secret_sm) {
        g_secret_sm->raiseExternal(SecretEvent::No_path);
    }
}

EMSCRIPTEN_KEEPALIVE
void sce_secret_recalculate(void) {
    if (g_secret_sm) {
        g_secret_sm->raiseExternal(SecretEvent::Recalculate);
        g_secret_sm->step();
        process_bfs_result();
    }
}

// ============================================
// Target Count Queries
// ============================================

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
int sce_get_target_count_enemy(void) {
    Secret_RefreshEnemyTargets();
    return Secret_GetTargetCount(TARGET_ENEMY);
}

EMSCRIPTEN_KEEPALIVE
int sce_is_secret_discovered(int index) {
    return Secret_IsDiscovered(index) ? 1 : 0;
}

EMSCRIPTEN_KEEPALIVE
int sce_is_enemy_alive(int index) {
    return Secret_IsEnemyAlive(index) ? 1 : 0;
}

EMSCRIPTEN_KEEPALIVE
void sce_refresh_enemy_targets(void) {
    Secret_RefreshEnemyTargets();
}

// ============================================
// Path Destination Mode
// ============================================

EMSCRIPTEN_KEEPALIVE
void sce_set_dest_mode_trigger(void) {
    Secret_SetDestinationMode(DEST_TRIGGER);
}

EMSCRIPTEN_KEEPALIVE
void sce_set_dest_mode_sector(void) {
    Secret_SetDestinationMode(DEST_SECTOR);
}

EMSCRIPTEN_KEEPALIVE
int sce_get_dest_mode(void) {
    return (int)Secret_GetDestinationMode();
}

EMSCRIPTEN_KEEPALIVE
int sce_current_target_has_sector(void) {
    return Secret_CurrentTargetHasSector() ? 1 : 0;
}

}  // extern "C"
