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
