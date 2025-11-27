/**
 * @file sce_wrapper.h
 * @brief C API for SCE state machines in DOOM
 *
 * Include this header in DOOM's C code to access the state machines.
 */

#ifndef SCE_WRAPPER_H
#define SCE_WRAPPER_H

#ifdef __cplusplus
extern "C" {
#endif

/* Initialization */
void sce_init(void);

/* Game State Machine */
const char *sce_get_game_state(void);
void sce_game_event_newgame(void);
void sce_game_event_loadgame(void);
void sce_game_event_completed(void);
void sce_game_event_worlddone(void);
void sce_game_event_finale(void);
void sce_game_event_demostart(void);  /* Reset weapon/enemy for demo playback */

/* Player State Machine */
const char *sce_get_player_state(void);
void sce_player_event_killed(void);
void sce_player_event_respawn(void);
void sce_player_event_spawn_complete(void);
void sce_player_event_god_mode_on(void);
void sce_player_event_god_mode_off(void);

/* Weapon State Machine */
const char *sce_get_weapon_state(void);
void sce_weapon_event_fire(void);
void sce_weapon_event_switch_weapon(void);
void sce_weapon_event_lower_complete(void);
void sce_weapon_event_raise_complete(void);
void sce_weapon_event_fire_complete(void);

/* Enemy State Machine (Multi-Instance with 100% Sync) */
int sce_get_enemy_count(void);
int sce_get_enemy_killed(void);
int sce_get_max_enemies(void);
const char *sce_get_enemy_info(int slot);

/* DOOM Level Statistics */
int sce_get_level_total_kills(void);     /* Total monsters in level */
int sce_get_player_kill_count(void);     /* Player's kill count */
int sce_get_enemies_remaining(void);     /* Remaining = total - killed */

/* Enemy lifecycle functions - call from DOOM code */
void sce_enemy_spawn(void *mobj, const char *type_name);
void sce_enemy_set_state(void *mobj, const char *state_name);
void sce_enemy_killed(void *mobj);
void sce_enemy_remove(void *mobj);
void sce_enemy_clear_all(void);  /* Clear all tracking (for save/load) */

/* Secret Hint State Machine */
const char *sce_secret_get_state(void);
void sce_secret_event_enable(void);
void sce_secret_event_disable(void);
void sce_secret_event_request(void);      /* Backwards compat - same as next_target */
void sce_secret_event_next_target(void);  /* H key - toggle path visibility */
void sce_secret_event_prev_target(void);  /* G key - deprecated, no-op */
void sce_select_target(int type, int index);  /* Select specific target by type and index, show path */
void sce_secret_event_cancel(void);
void sce_secret_event_level_change(void);
void sce_secret_event_reached(void);

/* Secret path finding result callback */
void sce_secret_path_found(void);
void sce_secret_no_path(void);

/* Target counts for UI - returns count for each target type */
int sce_get_target_count_secret(void);
int sce_get_target_count_door(void);
int sce_get_target_count_lift(void);
int sce_get_target_count_switch(void);
int sce_get_target_count_teleporter(void);
int sce_get_target_count_exit(void);
int sce_get_target_count_keydoor(void);

/* Check if a secret has been discovered (0 = not found, 1 = found) */
int sce_is_secret_discovered(int index);

/* Aim Assist State Machine */
const char *sce_aim_get_state(void);
void sce_aim_event_toggle(void);
int sce_aim_is_enabled(void);  /* Returns 1 if aim assist is enabled, 0 otherwise */

#ifdef __cplusplus
}
#endif

#endif /* SCE_WRAPPER_H */
