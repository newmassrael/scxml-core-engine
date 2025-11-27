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

/**
 * Clear all enemy tracking (call before loading saved game)
 */
void SCE_EnemyClearAll(void);

/**
 * Re-scan all thinkers and register live monsters (call after loading saved game)
 */
void SCE_EnemyRescanAll(void);

/**
 * Secret hint events
 */
void SCE_SecretHintEnable(void);
void SCE_SecretHintDisable(void);
void SCE_SecretHintToggle(void);        /* H key: toggle hint system on/off */
void SCE_SecretHintRequest(void);       /* Request path to current target */
void SCE_SecretHintPrevious(void);      /* G key: select previous target */
void SCE_SecretHintCancel(void);        /* Cancel current hint display */
void SCE_SecretLevelChange(void);       /* New level loaded */

/**
 * Called each frame to check if player reached target secret
 */
void SCE_SecretCheckReached(void);

/**
 * Get current secret hint state name
 */
const char* SCE_SecretGetStateName(void);

/**
 * DOOM Level Statistics - for UI display
 */
int SCE_GetLevelTotalKills(void);
int SCE_GetPlayerKillCount(void);

/**
 * Aim Assist events
 */
void SCE_AimAssistToggle(void);           /* Toggle aim assist on/off */
boolean SCE_AimAssistIsEnabled(void);     /* Check if aim assist is currently enabled */
void SCE_AimAssistTick(void);             /* Called every game tick to update lock-on */
void SCE_AimAssistSetTarget(mobj_t *target);  /* Set lock-on target */
void SCE_AimAssistClearTarget(void);      /* Clear lock-on target */
mobj_t* SCE_AimAssistGetTarget(void);     /* Get current lock-on target (NULL if none) */

/* SCXML state machine events */
void SCE_AimEventShotFired(void);         /* Player fired weapon */
void SCE_AimEventTargetAcquired(void);    /* Target found and locked */
void SCE_AimEventTargetLost(void);        /* Target died or out of range */
void SCE_AimEventNoTarget(void);          /* Search completed, no target found */
void SCE_AimEventShotComplete(void);      /* Shot processing finished, return to idle */

#ifdef __cplusplus
}
#endif

#endif /* SCE_DOOM_HOOKS_H */
