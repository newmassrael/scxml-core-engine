/**
 * @file sce_wrapper.cpp
 * @brief SCE state machine initialization orchestrator
 *
 * Single responsibility: initialize all state machine modules and
 * notify initial states to JavaScript.
 *
 * Each domain-specific module (sce_sm_*.cpp) owns its own state machine
 * instance, callbacks, and extern "C" API. This file only coordinates
 * their initialization order.
 *
 * Module dependency order:
 *   1. core (game/player/weapon) - no deps
 *   2. enemy - no deps
 *   3. combo - no deps (but called by enemy and core)
 *   4. aim - no deps
 *   5. secret - no deps
 */

#include "sce_sm_internal.h"

extern "C" {

EMSCRIPTEN_KEEPALIVE
void sce_init(void) {
    sce_sm_core_init();
    sce_sm_enemy_init();
    sce_sm_combo_init();
    sce_sm_aim_init();
    sce_sm_secret_init();

    // Notify initial states to JavaScript
    js_notify_state_change("game", sce_get_game_state());
    js_notify_state_change("player", sce_get_player_state());
    js_notify_state_change("weapon", sce_get_weapon_state());
    js_notify_state_change("secret", sce_secret_get_state());
    js_notify_state_change("aim", sce_aim_get_state());
    js_notify_state_change("combo", sce_combo_get_state());
    js_notify_stats_update(0, 0, 0);
    js_notify_combo_update(0, false);

    SCE_LOG("[SCE] All state machines initialized\n");
}

}  // extern "C"
