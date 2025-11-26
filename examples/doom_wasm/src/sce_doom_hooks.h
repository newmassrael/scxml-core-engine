/**
 * @file sce_doom_hooks.h
 * @brief SCE integration hooks for DOOM
 *
 * Provides C-callable functions to integrate DOOM with SCE state machines.
 * Include this header in DOOM source files that need to trigger SCE events.
 */

#ifndef SCE_DOOM_HOOKS_H
#define SCE_DOOM_HOOKS_H

#include "doomtype.h"
#include "p_mobj.h"

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Initialize SCE state machines. Call once at startup.
 */
void SCE_Init(void);

/**
 * Game state events
 */
void SCE_GameNewGame(void);
void SCE_GameLoadGame(void);
void SCE_GameCompleted(void);
void SCE_GameWorldDone(void);
void SCE_GameFinale(void);
void SCE_GameDemoStart(void);  /* Reset weapon/enemy for new demo playback */

/**
 * Player state events
 */
void SCE_PlayerKilled(void);
void SCE_PlayerRespawn(void);
void SCE_PlayerSpawnComplete(void);
void SCE_PlayerGodModeOn(void);
void SCE_PlayerGodModeOff(void);

/**
 * Weapon state events
 */
void SCE_WeaponFire(void);
void SCE_WeaponSwitch(void);
void SCE_WeaponLowerComplete(void);
void SCE_WeaponRaiseComplete(void);
void SCE_WeaponFireComplete(void);

/**
 * Enemy lifecycle events
 * @param mobj Pointer to the monster's mobj_t
 */
void SCE_EnemySpawned(mobj_t *mobj);
void SCE_EnemyAlert(mobj_t *mobj);
void SCE_EnemyChasing(mobj_t *mobj);
void SCE_EnemyAttacking(mobj_t *mobj);
void SCE_EnemyPain(mobj_t *mobj);
void SCE_EnemyKilled(mobj_t *mobj);
void SCE_EnemyRemoved(mobj_t *mobj);

/**
 * Check if an mobj is a monster (has MF_COUNTKILL flag)
 */
boolean SCE_IsMonster(mobj_t *mobj);

#ifdef __cplusplus
}
#endif

#endif /* SCE_DOOM_HOOKS_H */
