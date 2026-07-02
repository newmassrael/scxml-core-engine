// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * @file sce_secret_hint.h
 * @brief Secret and trigger hint system with BFS pathfinding for DOOM
 *
 * Provides functions to find paths to secrets and interactive triggers
 * (doors, lifts, switches, teleporters) using BFS through sector adjacency.
 */

#ifndef SCE_SECRET_HINT_H
#define SCE_SECRET_HINT_H

#include "doomtype.h"
#include "r_defs.h"

#ifdef __cplusplus
extern "C" {
#endif

/* Maximum path length (number of sectors in path) */
#define SECRET_MAX_PATH_LENGTH 256

/* Maximum arrows to render */
#define SECRET_MAX_ARROWS 64

/* Maximum targets per category */
#define SECRET_MAX_TARGETS 128

/* Target types */
/* Door activation methods */
typedef enum {
    DOOR_OPEN_USE,        /* Press Use key (Space) */
    DOOR_OPEN_WALK,       /* Walk through to trigger */
    DOOR_OPEN_SHOOT,      /* Shoot to open */
    DOOR_OPEN_SWITCH,     /* Activate nearby switch */
    DOOR_OPEN_BLUE_KEY,   /* Requires blue key */
    DOOR_OPEN_RED_KEY,    /* Requires red key */
    DOOR_OPEN_YELLOW_KEY, /* Requires yellow key */
    DOOR_OPEN_UNKNOWN     /* Unknown activation method */
} door_open_method_t;

typedef enum {
    TARGET_SECRET,     /* Secret sector (special == 9) */
    TARGET_DOOR,       /* Door linedef */
    TARGET_LIFT,       /* Lift/platform linedef */
    TARGET_SWITCH,     /* Switch linedef */
    TARGET_TELEPORTER, /* Teleporter linedef */
    TARGET_EXIT,       /* Exit switch/linedef */
    TARGET_KEY_DOOR,   /* Key-locked door */
    TARGET_ENEMY,      /* Live enemy (mobj_t with MF_COUNTKILL) */
    TARGET_TYPE_COUNT
} target_type_t;

/* Forward declaration for mobj_t */
struct mobj_s;

/* Target information structure */
/* NOTE: Use int instead of boolean for discovered/reachable to ensure
 * consistent struct layout between C (boolean=enum=4bytes) and C++ (boolean=bool=1byte) */
typedef struct {
    target_type_t type;  /* Type of target */
    int index;           /* Sector index (secrets) or linedef index (triggers) */
    fixed_t x;           /* Trigger X position (linedef center) */
    fixed_t y;           /* Trigger Y position (linedef center) */
    const char *name;    /* Display name (e.g., "Secret 1", "Blue Door") */
    int discovered;      /* For secrets: already found? (0=false, 1=true) */
    int reachable;       /* Can player reach this target? (0=false, 1=true) */
    struct mobj_s *mobj; /* For TARGET_ENEMY: pointer to enemy mobj */
    /* Sector destination (for lifts/doors with tagged sectors) */
    fixed_t sector_x; /* Tagged sector center X */
    fixed_t sector_y; /* Tagged sector center Y */
    int sector_index; /* Tagged sector index (-1 if none) */
    /* Door-specific information */
    door_open_method_t open_method; /* How to open this door */
    int is_hidden;                  /* Is this a hidden door? (0=false, 1=true) */
    int linked_secret;              /* Secret index this door leads to (-1 if none) */
} target_info_t;

/* Arrow waypoint structure */
typedef struct {
    fixed_t x;
    fixed_t y;
    fixed_t z;     /* Floor height at this point */
    angle_t angle; /* Direction to next waypoint */
} secret_arrow_t;

/* Path result structure */
typedef struct {
    boolean valid;                            /* Is the path valid? */
    int target_sector;                        /* Target secret sector index */
    int path_length;                          /* Number of sectors in path */
    int path[SECRET_MAX_PATH_LENGTH];         /* Sector indices from player to secret */
    int num_arrows;                           /* Number of floor arrows to render */
    secret_arrow_t arrows[SECRET_MAX_ARROWS]; /* Arrow positions and directions */
    fixed_t target_x;                         /* Final target X position */
    fixed_t target_y;                         /* Final target Y position */
} secret_path_t;

/**
 * Initialize the secret hint system.
 * Call once during game startup.
 */
void Secret_Init(void);

/**
 * Called when a new level is loaded.
 * Rebuilds the sector adjacency graph.
 */
void Secret_OnLevelLoad(void);

/**
 * Mark adjacency graph as needing rebuild.
 * Call when doors, lifts, or platforms change state.
 */
void Secret_InvalidateAdjacency(void);

/**
 * Cleanup the secret hint system.
 * Call when level unloads.
 */
void Secret_Cleanup(void);

/**
 * Find path to nearest unrevealed secret.
 * Uses BFS through sector adjacency graph.
 *
 * @param out_path Output path information
 * @return true if path found, false if no secrets remaining
 */
boolean Secret_FindPath(secret_path_t *out_path);

/**
 * Get current active path (if any).
 * @return Pointer to current path, or NULL if not showing
 */
const secret_path_t *Secret_GetCurrentPath(void);

/**
 * Clear the current path.
 */
void Secret_ClearPath(void);

/**
 * Check if player has reached the target secret.
 * @return true if player entered target secret sector
 */
boolean Secret_CheckReached(void);

/**
 * Get the number of remaining secrets on current level.
 * @return Number of secret sectors not yet discovered
 */
int Secret_GetRemainingCount(void);

/**
 * Update arrow positions (call each frame when showing hints).
 * Updates arrow visibility based on player position.
 */
void Secret_UpdateArrows(void);

/**
 * Render floor arrows.
 * Call during sprite rendering phase.
 */
void Secret_RenderArrows(void);

/**
 * Check if hint system is enabled.
 */
boolean Secret_IsEnabled(void);

/**
 * Remove hint sprites before save game (sprites not serializable).
 * Call before P_ArchiveThinkers().
 */
void Secret_PrepareForSave(void);

/**
 * Respawn hint sprites after save game.
 * Call after P_ArchiveThinkers().
 */
void Secret_RestoreAfterSave(void);

/**
 * Enable/disable hint system.
 */
void Secret_SetEnabled(boolean enabled);

/**
 * Check if current path leads to a hidden door.
 * If true, player needs to activate the wall/door to access secret.
 * @return true if path ends at hidden door, false if direct path to secret
 */
boolean Secret_IsPathToHiddenDoor(void);

/* ============================================
 * Target Selection API
 * ============================================ */

/**
 * Scan level and build list of all targets (secrets + triggers).
 * Called automatically on level load.
 */
void Secret_ScanTargets(void);

/**
 * Get total number of targets in current category.
 * @param type Target type to count, or TARGET_TYPE_COUNT for all
 * @return Number of targets
 */
int Secret_GetTargetCount(target_type_t type);

/**
 * Check if a secret has been discovered.
 * @param index Secret index (0-based)
 * @return true if secret has been found, false if not yet discovered
 */
boolean Secret_IsDiscovered(int index);

/**
 * Select next target (H key).
 * Cycles through: secrets -> doors -> lifts -> switches -> teleporters -> exits
 * @return true if selection changed
 */
boolean Secret_SelectNextTarget(void);

/**
 * Select previous target (G key).
 * @return true if selection changed
 */
boolean Secret_SelectPrevTarget(void);

/**
 * Select a specific target by type and index.
 * @param type Target type (TARGET_SECRET, TARGET_DOOR, etc.)
 * @param index Index within that type (0-based)
 * @return true if valid target selected
 */
boolean Secret_SelectTarget(target_type_t type, int index);

/**
 * Get current selected target info.
 * @param out_info Output target information
 * @return true if valid target selected
 */
boolean Secret_GetCurrentTarget(target_info_t *out_info);

/**
 * Find path to currently selected target.
 * @param out_path Output path information
 * @return true if path found
 */
boolean Secret_FindPathToCurrentTarget(secret_path_t *out_path);

/**
 * Get name string for target type.
 * @param type Target type
 * @return Static string name
 */
const char *Secret_GetTargetTypeName(target_type_t type);

/**
 * Get name string for door open method.
 * @param method Door open method
 * @return Static string name (e.g., "Use", "Walk", "Blue Key")
 */
const char *Secret_GetDoorOpenMethodName(door_open_method_t method);

/**
 * Get current selection indices.
 * @param out_type Current target type
 * @param out_index Index within type
 * @param out_total Total targets of this type
 */
void Secret_GetSelectionInfo(target_type_t *out_type, int *out_index, int *out_total);

/**
 * Refresh enemy targets list.
 * Scans all thinkers for live enemies and updates TARGET_ENEMY list.
 * Call periodically or when enemy count changes.
 */
void Secret_RefreshEnemyTargets(void);

/**
 * Update enemy target positions.
 * Updates x/y coordinates for all TARGET_ENEMY entries based on current mobj positions.
 * Call each frame when showing path to enemy.
 */
void Secret_UpdateEnemyPositions(void);

/**
 * Check if an enemy target is still valid (alive).
 * @param index Enemy target index
 * @return true if enemy is still alive
 */
boolean Secret_IsEnemyAlive(int index);

/* ============================================
 * Path Destination Mode API
 * ============================================ */

/* Destination mode for targets with tagged sectors (lifts, doors) */
typedef enum {
    DEST_TRIGGER, /* Path to trigger/switch position (default) */
    DEST_SECTOR   /* Path to tagged sector (lift platform, door sector) */
} path_dest_mode_t;

/**
 * Set path destination mode for current target.
 * Only affects targets with tagged sectors (lifts, doors, key doors).
 * @param mode DEST_TRIGGER or DEST_SECTOR
 */
void Secret_SetDestinationMode(path_dest_mode_t mode);

/**
 * Get current path destination mode.
 * @return Current destination mode
 */
path_dest_mode_t Secret_GetDestinationMode(void);

/**
 * Check if current target has a tagged sector.
 * @return true if current target has sector_index >= 0
 */
boolean Secret_CurrentTargetHasSector(void);

/* External functions from sce_wrapper.cpp (SCXML state machine integration) */
void sce_secret_recalculate(void);
const char *sce_secret_get_state(void);
void sce_secret_event_toggle(void);
void sce_secret_event_enable(void);
void sce_secret_event_disable(void);
void sce_secret_event_request(void);
void sce_secret_event_select(void);
void sce_secret_event_cancel(void);
void sce_secret_event_level_change(void);
void sce_secret_event_reached(void);

#ifdef __cplusplus
}
#endif

#endif /* SCE_SECRET_HINT_H */
