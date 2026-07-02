// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * @file sce_doom_hooks.c
 * @brief SCE integration hooks implementation for DOOM
 *
 * Bridges DOOM's C code to the SCE C++ wrapper via extern "C" functions.
 * Provides helper functions to map DOOM types to SCE state machine events.
 */

#include "sce_doom_hooks.h"
#include "d_player.h"
#include "doomstat.h"
#include "info.h"
#include "p_local.h"
#include "p_tick.h"
#include "r_main.h"
#include "sce_secret_hint.h"
#include "sce_wrapper.h"
#include <emscripten.h>
#include <string.h>

/* Conditional debug logging (matches sce_sm_internal.h) */
#ifdef SCE_DEBUG
#define SCE_LOG(fmt, ...) printf(fmt, ##__VA_ARGS__)
#else
#define SCE_LOG(fmt, ...) ((void)0)
#endif

/* Forward declaration for P_MobjThinker */
void P_MobjThinker(mobj_t *mobj);

/* Monster type names for SCE enemy tracking */
static const char *SCE_GetMonsterTypeName(mobjtype_t type) {
    switch (type) {
    case MT_POSSESSED:
        return "ZOMBIEMAN";
    case MT_SHOTGUY:
        return "SHOTGUNGUY";
    case MT_VILE:
        return "ARCHVILE";
    case MT_FIRE:
        return "FIRE";
    case MT_UNDEAD:
        return "REVENANT";
    case MT_TRACER:
        return "TRACER";
    case MT_SMOKE:
        return "SMOKE";
    case MT_FATSO:
        return "MANCUBUS";
    case MT_FATSHOT:
        return "FATSHOT";
    case MT_CHAINGUY:
        return "CHAINGUNNER";
    case MT_TROOP:
        return "IMP";
    case MT_SERGEANT:
        return "DEMON";
    case MT_SHADOWS:
        return "SPECTRE";
    case MT_HEAD:
        return "CACODEMON";
    case MT_BRUISER:
        return "BARON";
    case MT_BRUISERSHOT:
        return "BARONBALL";
    case MT_KNIGHT:
        return "HELLKNIGHT";
    case MT_SKULL:
        return "LOSTSOUL";
    case MT_SPIDER:
        return "SPIDERMASTERMIND";
    case MT_BABY:
        return "ARACHNOTRON";
    case MT_CYBORG:
        return "CYBERDEMON";
    case MT_PAIN:
        return "PAINELEMENTAL";
    case MT_WOLFSS:
        return "WOLFSS";
    case MT_KEEN:
        return "COMMANDERKEEN";
    case MT_BOSSBRAIN:
        return "BOSSBRAIN";
    case MT_BOSSSPIT:
        return "BOSSSPIT";
    case MT_BOSSTARGET:
        return "BOSSTARGET";
    case MT_SPAWNSHOT:
        return "SPAWNSHOT";
    case MT_SPAWNFIRE:
        return "SPAWNFIRE";
    default:
        return "UNKNOWN";
    }
}

boolean SCE_IsMonster(mobj_t *mobj) {
    return mobj && (mobj->flags & MF_COUNTKILL);
}

void SCE_Init(void) {
    sce_init();
    SCE_HUD_Init();
}

/* Game state events */
void SCE_GameNewGame(void) {
    sce_game_event_newgame();
}

void SCE_GameLoadGame(void) {
    sce_game_event_loadgame();
}

void SCE_GameCompleted(void) {
    sce_game_event_completed();
}

void SCE_GameWorldDone(void) {
    sce_game_event_worlddone();
}

void SCE_GameFinale(void) {
    sce_game_event_finale();
}

void SCE_GameDemoStart(void) {
    sce_game_event_demostart();
}

/* Player state events */
void SCE_PlayerKilled(void) {
    sce_player_event_killed();
}

void SCE_PlayerRespawn(void) {
    sce_player_event_respawn();
}

void SCE_PlayerSpawnComplete(void) {
    sce_player_event_spawn_complete();
}

void SCE_PlayerGodModeOn(void) {
    sce_player_event_god_mode_on();
}

void SCE_PlayerGodModeOff(void) {
    sce_player_event_god_mode_off();
}

/* Weapon state events */
void SCE_WeaponFire(void) {
    sce_weapon_event_fire();
}

void SCE_WeaponSwitch(void) {
    sce_weapon_event_switch_weapon();
}

void SCE_WeaponLowerComplete(void) {
    sce_weapon_event_lower_complete();
}

void SCE_WeaponRaiseComplete(void) {
    sce_weapon_event_raise_complete();
}

void SCE_WeaponFireComplete(void) {
    sce_weapon_event_fire_complete();
    // Aim assist: Return to idle when weapon fire animation completes
    if (SCE_AimAssistIsEnabled()) {
        SCE_AimEventShotComplete();
    }
}

/* Enemy lifecycle events */
void SCE_EnemySpawned(mobj_t *mobj) {
    if (!SCE_IsMonster(mobj)) {
        return;
    }
    sce_enemy_spawn(mobj, SCE_GetMonsterTypeName(mobj->type));
}

void SCE_EnemyAlert(mobj_t *mobj) {
    if (!SCE_IsMonster(mobj)) {
        return;
    }
    sce_enemy_set_state(mobj, "ALERT");
}

void SCE_EnemyChasing(mobj_t *mobj) {
    if (!SCE_IsMonster(mobj)) {
        return;
    }
    sce_enemy_set_state(mobj, "CHASING");
}

void SCE_EnemyAttacking(mobj_t *mobj) {
    if (!SCE_IsMonster(mobj)) {
        return;
    }
    sce_enemy_set_state(mobj, "ATTACKING");
}

void SCE_EnemyPain(mobj_t *mobj) {
    if (!SCE_IsMonster(mobj)) {
        return;
    }
    sce_enemy_set_state(mobj, "PAIN");
}

void SCE_EnemyKilled(mobj_t *mobj) {
    if (!SCE_IsMonster(mobj)) {
        return;
    }
    sce_enemy_killed(mobj);
}

void SCE_EnemyRemoved(mobj_t *mobj) {
    if (!SCE_IsMonster(mobj)) {
        return;
    }
    sce_enemy_remove(mobj);
}

void SCE_EnemyClearAll(void) {
    sce_enemy_clear_all();
}

void SCE_EnemyRescanAll(void) {
    /* Iterate through all thinkers and re-register live monsters */
    extern thinker_t thinkercap;
    thinker_t *th;

    for (th = thinkercap.next; th != &thinkercap; th = th->next) {
        /* Check if this is a mobj thinker */
        if (th->function.acp1 == (actionf_p1)P_MobjThinker) {
            mobj_t *mobj = (mobj_t *)th;
            /* Only register live monsters (not corpses) */
            if (SCE_IsMonster(mobj) && mobj->health > 0) {
                sce_enemy_spawn(mobj, SCE_GetMonsterTypeName(mobj->type));
            }
        }
    }
}

/* Secret hint events */
void SCE_SecretHintEnable(void) {
    Secret_SetEnabled(true);
    sce_secret_event_enable();
}

void SCE_SecretHintDisable(void) {
    Secret_SetEnabled(false);
    sce_secret_event_disable();
}

void SCE_SecretHintToggle(void) {
    /* H key: Toggle hint system on/off using SCXML toggle event */
    const char *state = sce_secret_get_state();
    if (state && strcmp(state, "disabled") == 0) {
        /* Enable: send toggle event to transition from disabled → calculating */
        Secret_SetEnabled(true);
    } else {
        /* Disable: send toggle event to transition from any enabled state → disabled */
        Secret_ClearPath();
        Secret_SetEnabled(false);
    }
    /* The SCXML state machine handles the toggle event to switch states */
    sce_secret_event_toggle();
}

void SCE_SecretHintRequest(void) {
    /* Request path to current target */
    Secret_ClearPath(); /* Clear old path first */
    sce_secret_event_request();
}

void SCE_SecretHintPrevious(void) {
    /* G key: Select previous target */
    if (Secret_SelectPrevTarget()) {
        Secret_ClearPath();
        sce_secret_event_select();
    }
}

void SCE_SecretHintCancel(void) {
    Secret_ClearPath();
    sce_secret_event_cancel();
}

void SCE_SecretLevelChange(void) {
    Secret_OnLevelLoad();
    sce_secret_event_level_change();
    sce_secret_update_count(); /* Update secret count display after level load */
}

void SCE_SecretCheckReached(void) {
    const char *state = sce_secret_get_state();

    /* Update arrows in showing, found, or no_path states.
     * - showing: update arrow positions as player moves
     * - found: keep showing path after reaching target
     * - no_path: keep trying to find path as player moves */
    if (state && (strcmp(state, "showing") == 0 || strcmp(state, "found") == 0 || strcmp(state, "no_path") == 0)) {
        Secret_UpdateArrows();
    }

    if (Secret_CheckReached()) {
        sce_secret_event_reached();
    }
}

const char *SCE_SecretGetStateName(void) {
    return sce_secret_get_state();
}

/* DOOM Level Statistics */
int SCE_GetLevelTotalKills(void) {
    return totalkills;
}

int SCE_GetPlayerKillCount(void) {
    return players[0].killcount;
}

/* Aim Assist */
static mobj_t *g_aim_target = NULL; /* Current lock-on target */

void SCE_AimAssistToggle(void) {
    sce_aim_event_toggle();
    /* Clear target when toggling off */
    if (!sce_aim_is_enabled()) {
        g_aim_target = NULL;
    }
}

boolean SCE_AimAssistIsEnabled(void) {
    return sce_aim_is_enabled() ? true : false;
}

void SCE_AimAssistSetTarget(mobj_t *target) {
    g_aim_target = target;
}

void SCE_AimAssistClearTarget(void) {
    g_aim_target = NULL;
}

mobj_t *SCE_AimAssistGetTarget(void) {
    /* Return target only if valid */
    if (g_aim_target && g_aim_target->health > 0) {
        return g_aim_target;
    }
    g_aim_target = NULL;
    return NULL;
}

void SCE_AimAssistTick(void) {
    player_t *player;
    mobj_t *mo;

    if (!sce_aim_is_enabled() || !g_aim_target) {
        return;
    }

    /* Validate SCXML state: only lock-on when in "locked" state.
     * Prevents dangling pointer dereference after level change. */
    const char *aim_state = sce_aim_get_state();
    if (!aim_state || strcmp(aim_state, "locked") != 0) {
        g_aim_target = NULL;
        return;
    }

    /* Check if target is still valid (alive and exists) */
    if (g_aim_target->health <= 0) {
        g_aim_target = NULL;
        SCE_AimEventTargetLost(); /* SCXML: locked -> idle */
        return;
    }

    /* Get player */
    player = &players[0];
    if (!player->mo) {
        return;
    }
    mo = player->mo;

    /* Lock-on: continuously update player angle to face target */
    mo->angle = R_PointToAngle2(mo->x, mo->y, g_aim_target->x, g_aim_target->y);
}

/* SCXML State Machine Event Wrappers */
extern void sce_aim_event_shot_fired(void);
extern void sce_aim_event_target_acquired(void);
extern void sce_aim_event_target_lost(void);
extern void sce_aim_event_no_target(void);
extern void sce_aim_event_shot_complete(void);

void SCE_AimEventShotFired(void) {
    sce_aim_event_shot_fired();
}

void SCE_AimEventTargetAcquired(void) {
    sce_aim_event_target_acquired();
}

void SCE_AimEventTargetLost(void) {
    sce_aim_event_target_lost();
}

void SCE_AimEventNoTarget(void) {
    sce_aim_event_no_target();
}

void SCE_AimEventShotComplete(void) {
    sce_aim_event_shot_complete();
}

/* ============================================
 * Logical Time Processing (GameLoopTimer)
 * ============================================ */
extern void sce_process_tic(void);

void SCE_ProcessTic(void) {
    sce_process_tic();
}

/* ============================================
 * Kill Combo System
 * ============================================ */
extern void sce_combo_reset(void);
extern int sce_combo_get_count(void);
extern int sce_combo_is_active(void);
extern const char *sce_combo_get_state(void);

void SCE_ComboReset(void) {
    sce_combo_reset();
}

int SCE_ComboGetCount(void) {
    return sce_combo_get_count();
}

boolean SCE_ComboIsActive(void) {
    return sce_combo_is_active() ? true : false;
}

const char *SCE_ComboGetStateName(void) {
    return sce_combo_get_state();
}

/* ============================================
 * Berserk Mode System
 * ============================================ */
extern int sce_combo_is_berserk(void);
extern int sce_berserk_get_intensity(void);

/* Berserk state - managed by SCXML callbacks */
static float g_berserk_multiplier = 1.0f; /* Damage multiplier (1.0 = normal) */

boolean SCE_BerserkIsActive(void) {
    return sce_combo_is_berserk() ? true : false;
}

int SCE_BerserkGetIntensity(void) {
    return sce_berserk_get_intensity();
}

float SCE_BerserkGetMultiplier(void) {
    return g_berserk_multiplier;
}

int SCE_BerserkGetAttackSpeed(void) {
    int intensity = sce_berserk_get_intensity();
    if (intensity <= 1) {
        return 1;
    }
    // Scale: x2-x3 = 2, x4-x5 = 3, x6-x7 = 4, x8-x10 = 5
    int speed = 1 + (intensity - 1) / 2;
    if (speed > 5) {
        speed = 5;
    }
    return speed;
}

void SCE_BerserkHealPlayer(void) {
    player_t *player = &players[0];
    if (player && player->health < 100) {
        player->health = 100;
        if (player->mo) {
            player->mo->health = 100;
        }
        SCE_LOG("[BERSERK] Healed player to 100 HP!\n");
    }
}

void SCE_BerserkSetMultiplier(float mult) {
    g_berserk_multiplier = mult;
    SCE_LOG("[BERSERK] Damage multiplier set to %.2f\n", mult);
}

void SCE_BerserkReset(void) {
    g_berserk_multiplier = 1.0f;
    SCE_LOG("[BERSERK] Reset! Damage multiplier back to 1.0\n");
}
