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
void sce_game_event_victory(void);
void sce_game_event_died(void);
void sce_game_event_quit(void);
void sce_game_event_worlddone(void);
void sce_game_event_finale(void);
void sce_game_event_done(void);
void sce_game_event_cast(void);

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

/* Enemy lifecycle functions - call from DOOM code */
void sce_enemy_spawn(void *mobj, const char *type_name);
void sce_enemy_set_state(void *mobj, const char *state_name);
void sce_enemy_killed(void *mobj);
void sce_enemy_remove(void *mobj);

#ifdef __cplusplus
}
#endif

#endif /* SCE_WRAPPER_H */
