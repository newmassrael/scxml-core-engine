/**
 * @file sce_doom_hooks.c
 * @brief SCE integration hooks implementation for DOOM
 *
 * Bridges DOOM's C code to the SCE C++ wrapper via extern "C" functions.
 * Provides helper functions to map DOOM types to SCE state machine events.
 */

#include "sce_doom_hooks.h"
#include "info.h"
#include "sce_wrapper.h"
#include "sce_secret_hint.h"
#include "p_tick.h"
#include "p_local.h"
#include "doomstat.h"
#include "d_player.h"
#include <string.h>

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
    Secret_ClearPath();  /* Clear old path first */
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
}

void SCE_SecretCheckReached(void) {
    const char *state = sce_secret_get_state();

    /* Update arrows in showing, found, or no_path states.
     * - showing: update arrow positions as player moves
     * - found: keep showing path after reaching target
     * - no_path: keep trying to find path as player moves */
    if (state && (strcmp(state, "showing") == 0 ||
                  strcmp(state, "found") == 0 ||
                  strcmp(state, "no_path") == 0)) {
        Secret_UpdateArrows();
    }

    if (Secret_CheckReached()) {
        sce_secret_event_reached();
    }
}

const char* SCE_SecretGetStateName(void) {
    return sce_secret_get_state();
}

/* DOOM Level Statistics */
int SCE_GetLevelTotalKills(void) {
    return totalkills;
}

int SCE_GetPlayerKillCount(void) {
    return players[0].killcount;
}
