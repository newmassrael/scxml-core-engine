/**
 * @file sce_wrapper.cpp
 * @brief C++ to C wrapper for SCE state machines in DOOM WASM
 *
 * Provides extern "C" functions for DOOM's C code and
 * 100% synchronized JavaScript callbacks via EM_ASM.
 */

#include <array>
#include <chrono>
#include <cstdio>
#include <cstring>
#include <memory>
#include <string>

#ifdef __EMSCRIPTEN__
#include <emscripten.h>
#else
#define EMSCRIPTEN_KEEPALIVE
#define EM_ASM(...)
#define EM_ASM_INT(...) 0
#endif

// SCE Generated State Machines
#include "aim_assist_state_sm.h"
#include "combo_state_sm.h"
#include "enemy_state_sm.h"
#include "game_state_sm.h"
#include "player_state_sm.h"
#include "secret_hint_state_sm.h"
#include "weapon_state_sm.h"

// External Pausable Timer Architecture:
// - Game manages timers externally (not SCXML internal scheduler)
// - Timers pause when menu opens, resume when menu closes
// - Callbacks notify game to start/cancel timers
// - Game raises timeout events when timer expires

// Secret hint pathfinding
#include "sce_secret_hint.h"

// DOOM hooks (C functions)
extern "C" {
void SCE_AimAssistClearTarget(void);
}

// ============================================
// Constants
// ============================================
static constexpr int MAX_ENEMIES = 64;

// ============================================
// Forward declarations for JS callbacks
// ============================================
static inline void js_notify_state_change(const char *machine, const char *state);
static inline void js_notify_secret_path(int num_arrows, int remaining_secrets, bool is_partial);
static inline void js_notify_secret_arrow(int index, int x, int y, int angle);
static inline void js_notify_target_info(const char *type_name, const char *name, int index, int total,
                                         int trigger_x, int trigger_y, int sector_x, int sector_y, int sector_idx,
                                         const char *open_method, int is_hidden, int linked_secret);

// Forward declaration for secret state machine (extern "C" for EMSCRIPTEN_KEEPALIVE exports)
extern "C" {
void sce_secret_path_found(void);
void sce_secret_no_path(void);
void sce_secret_recalculate(void);
const char *sce_secret_get_state(void);

// DOOM level statistics helper functions (defined in sce_doom_hooks.c)
int SCE_GetLevelTotalKills(void);
int SCE_GetPlayerKillCount(void);
}

// ============================================
// Aim Assist SCXML UserContext: C++ Callback Integration
// ============================================

// Global flag for DOOM to check aim assist status
static bool g_aim_assist_enabled = false;

// Forward declaration for JS callbacks
static inline void js_notify_aim_assist_state(bool enabled);
static inline void js_notify_aim_callback(const char *callback_type);
static inline void js_notify_combo_update(int count, bool active);
static inline void js_notify_combo_timer(double remaining_ms, double total_ms);

/**
 * @brief Callback handler for aim assist state machine
 *
 * Implements onentry actions for each state as defined in aim_assist_state.scxml.
 * The generated code calls user_->aim.onXxx() when entering each state.
 */
struct AimCallbacks {
    void onDisabled() {
        printf("[AIM SCXML] onDisabled callback\n");
        js_notify_aim_callback("disabled");
        g_aim_assist_enabled = false;
        SCE_AimAssistClearTarget();  // Clear any lingering lock-on target
        js_notify_aim_assist_state(false);
        js_notify_state_change("aim", "DISABLED");
    }

    void onEnabled() {
        printf("[AIM SCXML] onEnabled callback\n");
        js_notify_aim_callback("enabled");
        g_aim_assist_enabled = true;
        js_notify_aim_assist_state(true);
        // Note: onEnabled is called when entering the compound state,
        // sub-state callbacks (onIdle, etc.) will be called immediately after
    }

    void onIdle() {
        printf("[AIM SCXML] onIdle callback\n");
        js_notify_aim_callback("idle");
        // Aim assist enabled but no target locked - clear lock-on
        SCE_AimAssistClearTarget();
        js_notify_state_change("aim", "IDLE");
    }

    void onSearching() {
        printf("[AIM SCXML] onSearching callback\n");
        js_notify_aim_callback("searching");
        // Actively searching for target during shot
        js_notify_state_change("aim", "SEARCHING");
    }

    void onLocked() {
        printf("[AIM SCXML] onLocked callback\n");
        js_notify_aim_callback("locked");
        // Target acquired and locked on
        js_notify_state_change("aim", "LOCKED");
    }
};

/**
 * @brief UserContext for aim assist state machine
 *
 * The generated code expects UserContext with an 'aim' member.
 * Calls to aim.onXxx() in SCXML become user_->aim.onXxx() in C++.
 */
struct AimContext {
    AimCallbacks aim;
};

// Global aim context instance
static AimContext g_aim_context;

// ============================================
// Combo SCXML UserContext: C++ Callback Integration
// ============================================

// Forward declaration for JS callbacks
static inline void js_notify_combo_callback(const char *callback_type);
static inline void js_notify_berserk_update(int intensity, bool active);

// Forward declaration for DOOM hooks
extern "C" {
void SCE_BerserkHealPlayer(void);
void SCE_BerserkSetMultiplier(float multiplier);
void SCE_BerserkReset(void);
}

// Berserk constants
static constexpr int BERSERK_TRIGGER_COMBO = 5;       // Combo count to trigger berserk
static constexpr int BERSERK_MAX_INTENSITY = 10;      // Maximum berserk intensity level (for damage cap)
// Damage multiplier = intensity level (x1 at intensity 1, x10 at intensity 10)

// Timer durations (external management, not SCXML internal)
static constexpr double COMBO_TIMEOUT_MS = 5000.0;    // 5s for normal mode
static constexpr double BERSERK_TIMEOUT_MS = 10000.0; // 10s for berserk mode

// External Pausable Timer State
// Timers are managed by game, not SCXML internal scheduler
// This allows pausing when menu opens
enum class ComboTimerType { NONE, COMBO, BERSERK };
static ComboTimerType g_active_timer_type = ComboTimerType::NONE;
static std::chrono::steady_clock::time_point g_timer_start;
static double g_timer_duration_ms = 0;
static double g_timer_elapsed_before_pause_ms = 0;  // Accumulated time before pause
static bool g_timer_paused = false;
static bool g_timer_pause_ui_sent = false;  // Prevent redundant UI updates in paused state

// Helper function to clear external timer state (deduplication)
static void clear_external_timer() {
    g_active_timer_type = ComboTimerType::NONE;
    g_timer_paused = false;
    g_timer_elapsed_before_pause_ms = 0;
    g_timer_pause_ui_sent = false;
}

// Berserk tracking state
static int g_berserk_intensity = 0;

/**
 * @brief Callback handler for combo state machine (normal mode)
 *
 * Implements onentry actions for each state as defined in combo_state.scxml.
 * The generated code calls user_->combo.onXxx() when entering each state.
 */
struct ComboCallbacks {
    void onIdle() {
        printf("[COMBO SCXML] onIdle callback\n");

        // Cancel external timer
        clear_external_timer();

        // Reset berserk intensity when combo ends (entering idle)
        // This is the true "end of combo" reset point
        if (g_berserk_intensity > 0) {
            printf("[COMBO] Combo ended - resetting berserk intensity from %d to 0\n", g_berserk_intensity);
            g_berserk_intensity = 0;
            js_notify_berserk_update(0, false);
        }

        js_notify_combo_callback("idle");
        js_notify_state_change("combo", "IDLE");
    }

    void onActive() {
        printf("[COMBO SCXML] onActive (normal) callback\n");
        js_notify_combo_callback("active");
        js_notify_state_change("combo", "NORMAL");

        // Start external 5s timer (pausable)
        g_active_timer_type = ComboTimerType::COMBO;
        g_timer_start = std::chrono::steady_clock::now();
        g_timer_duration_ms = COMBO_TIMEOUT_MS;
        g_timer_elapsed_before_pause_ms = 0;
        g_timer_paused = false;

        printf("[COMBO] External timer started: %.0fms (pausable)\n", COMBO_TIMEOUT_MS);
    }
};

/**
 * @brief Callback handler for berserk mode (compound sub-state)
 *
 * Implements onentry/onexit actions for berserk state.
 * The generated code calls user_->berserk.onXxx() when entering/exiting berserk.
 */
struct BerserkCallbacks {
    void onActive() {
        printf("[BERSERK SCXML] onActive callback - intensity=%d\n", g_berserk_intensity);
        js_notify_combo_callback("berserk");
        js_notify_state_change("combo", "BERSERK");

        // Increase intensity (no cap for display, damage multiplier caps separately)
        g_berserk_intensity++;

        // Damage multiplier = intensity level (x1 at intensity 1, x10 max)
        int capped_intensity = (g_berserk_intensity > BERSERK_MAX_INTENSITY)
                               ? BERSERK_MAX_INTENSITY : g_berserk_intensity;
        float multiplier = (float)capped_intensity;  // x1, x2, x3, ... x10

        // Apply berserk effects
        SCE_BerserkSetMultiplier(multiplier);
        SCE_BerserkHealPlayer();  // Heal on activation/re-activation

        // Notify UI with intensity (uncapped) and active state
        js_notify_berserk_update(g_berserk_intensity, true);

        // Start external 10s timer (pausable)
        g_active_timer_type = ComboTimerType::BERSERK;
        g_timer_start = std::chrono::steady_clock::now();
        g_timer_duration_ms = BERSERK_TIMEOUT_MS;
        g_timer_elapsed_before_pause_ms = 0;
        g_timer_paused = false;

        printf("[BERSERK] Activated! Intensity=%d, Multiplier=x%.0f, Timer=%.0fms (pausable)\n",
               g_berserk_intensity, multiplier, BERSERK_TIMEOUT_MS);
    }

    void onExit() {
        printf("[BERSERK SCXML] onExit callback\n");

        // Reset berserk visual effects (damage multiplier)
        // NOTE: Do NOT reset g_berserk_intensity here!
        // Self-transitions (kill while berserk) trigger exit+entry,
        // and we want intensity to accumulate across kills.
        // Intensity is reset when entering idle state (combo ends).
        SCE_BerserkReset();

        // Notify UI (intensity preserved for potential re-entry)
        js_notify_berserk_update(g_berserk_intensity, false);

        printf("[BERSERK] Exiting state (intensity=%d preserved)\n", g_berserk_intensity);
    }
};

/**
 * @brief UserContext for combo state machine (W3C SCXML 3.4 compound states)
 *
 * The generated code expects UserContext with 'combo' and 'berserk' members.
 * Calls to combo.onXxx() in SCXML become user_->combo.onXxx() in C++.
 * Calls to berserk.onXxx() in SCXML become user_->berserk.onXxx() in C++.
 */
struct ComboContext {
    ComboCallbacks combo;
    BerserkCallbacks berserk;
};

// Global combo context instance
static ComboContext g_combo_context;

// ============================================
// Game SCXML UserContext: C++ Callback Integration
// ============================================

// Forward declaration for JS callback
static inline void js_notify_game_callback(const char *callback_type);

// Forward declaration for combo reset (used by game callbacks)
extern "C" void sce_combo_reset(void);

/**
 * @brief Callback handler for game state machine
 *
 * Implements onentry/onexit actions for each state as defined in game_state.scxml.
 * The generated code calls user_->game.onXxx() when entering/exiting each state.
 */
struct GameCallbacks {
    void onDemoScreen() {
        printf("[GAME SCXML] onDemoScreen callback\n");
        js_notify_game_callback("demo");
        js_notify_state_change("game", "DEMOSCREEN");
    }

    void onLevelStart() {
        printf("[GAME SCXML] onLevelStart callback\n");
        js_notify_game_callback("level_start");
        js_notify_state_change("game", "LEVEL");
    }

    void onLevelEnd() {
        printf("[GAME SCXML] onLevelEnd callback\n");
        js_notify_game_callback("level_end");
        sce_combo_reset();  // Reset combo/berserk when level ends
    }

    void onIntermission() {
        printf("[GAME SCXML] onIntermission callback\n");
        js_notify_game_callback("intermission");
        js_notify_state_change("game", "INTERMISSION");
        sce_combo_reset();  // Reset combo/berserk for intermission screen
    }

    void onFinale() {
        printf("[GAME SCXML] onFinale callback\n");
        js_notify_game_callback("finale");
        js_notify_state_change("game", "FINALE");
        sce_combo_reset();  // Reset combo/berserk for finale
    }
};

/**
 * @brief UserContext for game state machine
 *
 * The generated code expects UserContext with a 'game' member.
 * Calls to game.onXxx() in SCXML become user_->game.onXxx() in C++.
 */
struct GameContext {
    GameCallbacks game;
};

// Global game context instance
static GameContext g_game_context;

// ============================================
// Player SCXML UserContext: C++ Callback Integration
// ============================================

// Forward declaration for JS callback
static inline void js_notify_player_callback(const char *callback_type);

/**
 * @brief Callback handler for player state machine
 *
 * Implements onentry actions for each state as defined in player_state.scxml.
 * The generated code calls user_->player.onXxx() when entering each state.
 */
struct PlayerCallbacks {
    void onAlive() {
        printf("[PLAYER SCXML] onAlive callback\n");
        js_notify_player_callback("alive");
        js_notify_state_change("player", "ALIVE");
    }

    void onInvulnerable() {
        printf("[PLAYER SCXML] onInvulnerable callback\n");
        js_notify_player_callback("invulnerable");
        js_notify_state_change("player", "INVULNERABLE");
    }

    void onDead() {
        printf("[PLAYER SCXML] onDead callback\n");

        // Full combo/berserk reset when player dies
        // (clears timer, combo count, berserk UI, timer bar)
        sce_combo_reset();

        js_notify_player_callback("dead");
        js_notify_state_change("player", "DEAD");
    }

    void onReborn() {
        printf("[PLAYER SCXML] onReborn callback\n");
        js_notify_player_callback("reborn");
        js_notify_state_change("player", "REBORN");
    }
};

/**
 * @brief UserContext for player state machine
 *
 * The generated code expects UserContext with a 'player' member.
 * Calls to player.onXxx() in SCXML become user_->player.onXxx() in C++.
 */
struct PlayerContext {
    PlayerCallbacks player;
};

// Global player context instance
static PlayerContext g_player_context;

// ============================================
// Weapon SCXML UserContext: C++ Callback Integration
// ============================================

// Forward declaration for JS callback
static inline void js_notify_weapon_callback(const char *callback_type);

/**
 * @brief Callback handler for weapon state machine
 *
 * Implements onentry actions for each state as defined in weapon_state.scxml.
 * The generated code calls user_->weapon.onXxx() when entering each state.
 */
struct WeaponCallbacks {
    void onReady() {
        printf("[WEAPON SCXML] onReady callback\n");
        js_notify_weapon_callback("ready");
        js_notify_state_change("weapon", "READY");
    }

    void onLowering() {
        printf("[WEAPON SCXML] onLowering callback\n");
        js_notify_weapon_callback("lowering");
        js_notify_state_change("weapon", "LOWERING");
    }

    void onRaising() {
        printf("[WEAPON SCXML] onRaising callback\n");
        js_notify_weapon_callback("raising");
        js_notify_state_change("weapon", "RAISING");
    }

    void onFiring() {
        printf("[WEAPON SCXML] onFiring callback\n");
        js_notify_weapon_callback("firing");
        js_notify_state_change("weapon", "FIRING");
    }
};

/**
 * @brief UserContext for weapon state machine
 *
 * The generated code expects UserContext with a 'weapon' member.
 * Calls to weapon.onXxx() in SCXML become user_->weapon.onXxx() in C++.
 */
struct WeaponContext {
    WeaponCallbacks weapon;
};

// Global weapon context instance
static WeaponContext g_weapon_context;

// ============================================
// Secret SCXML UserContext: C++ Callback Integration
// ============================================

/**
 * @brief Callback handler for secret hint state machine
 *
 * Implements onentry actions for each state as defined in secret_hint_state.scxml.
 * The generated code calls user_->secret.onXxx() when entering each state.
 */
// BFS result storage for clean event separation
// Set by onCalculating callback, processed by caller after step() returns
enum class BfsResult { NONE, PATH_FOUND, NO_PATH };
static BfsResult g_bfs_result = BfsResult::NONE;

// Forward declaration for BFS result processing
static void sce_process_bfs_result(void);

// Forward declaration for JS callback
static inline void js_notify_secret_callback(const char *callback_type);

struct SecretCallbacks {
    void onDisabled() {
        printf("[SECRET SCXML] onDisabled callback\n");
        js_notify_secret_callback("disabled");
        // Disabled state: hint system is off
        Secret_ClearPath();
        js_notify_state_change("secret", "DISABLED");
        int count = Secret_GetRemainingCount();
        js_notify_secret_path(0, count, false);
        // Clear button highlight when disabled
        js_notify_target_info("", "", -1, 0, 0, 0, 0, 0, -1, "", 0, -1);
    }

    void onCalculating() {
        printf("[SECRET SCXML] onCalculating callback\n");
        js_notify_secret_callback("calculating");
        // Calculating state: run BFS and store result
        // NOTE: Do NOT raise events here - we're inside step() call
        // The result will be processed by the caller after step() returns
        js_notify_state_change("secret", "CALCULATING");

        // Reset BFS result
        g_bfs_result = BfsResult::NONE;

        // Always notify target info for button highlighting (with trigger and sector coordinates)
        target_info_t info;
        if (Secret_GetCurrentTarget(&info)) {
            target_type_t type;
            int index, total;
            Secret_GetSelectionInfo(&type, &index, &total);

            js_notify_target_info(Secret_GetTargetTypeName(type), info.name, index, total,
                                  info.x >> 16, info.y >> 16,
                                  info.sector_x >> 16, info.sector_y >> 16, info.sector_index,
                                  Secret_GetDoorOpenMethodName(info.open_method), info.is_hidden, info.linked_secret);
        }

        // Run BFS and store result (no event raising)
        secret_path_t path = {};
        bool bfs_success = Secret_FindPathToCurrentTarget(&path);
        if (bfs_success && path.num_arrows > 0) {
            // Check if path is partial (target unreachable)
            // For secrets: compare path.target_sector with info.index (sector index)
            // For triggers: we trust the BFS result
            bool is_partial = false;
            if (info.type == TARGET_SECRET && path.target_sector != info.index) {
                is_partial = true;
            }
            // Path found - notify JS with arrow data and partial flag
            js_notify_secret_path(path.num_arrows, Secret_GetRemainingCount(), is_partial);
            for (int i = 0; i < path.num_arrows; i++) {
                js_notify_secret_arrow(i,
                                       path.arrows[i].x >> 16,
                                       path.arrows[i].y >> 16,
                                       path.arrows[i].angle >> 16);
            }
            g_bfs_result = BfsResult::PATH_FOUND;
        } else {
            g_bfs_result = BfsResult::NO_PATH;
        }
        // Result stored - caller will process after step() returns
    }

    void onShowing() {
        printf("[SECRET SCXML] onShowing callback\n");
        js_notify_secret_callback("showing");
        // Showing state: arrows already sent during calculating
        js_notify_state_change("secret", "SHOWING");
    }

    void onExitShowing() {
        printf("[SECRET SCXML] onExitShowing callback\n");
        // Exiting showing state: clear path and sprites from map
        Secret_ClearPath();
    }

    void onFound() {
        printf("[SECRET SCXML] onFound callback\n");
        js_notify_secret_callback("found");
        // Found state: player reached target, keep showing path
        // Path will be recalculated by Secret_UpdateArrows
        js_notify_state_change("secret", "FOUND");
    }

    void onExitFound() {
        printf("[SECRET SCXML] onExitFound callback\n");
        // Exiting found state: clear path and sprites from map
        Secret_ClearPath();
    }

    void onNoPath() {
        printf("[SECRET SCXML] onNoPath callback\n");
        js_notify_secret_callback("no_path");
        // No path state: keep display as-is, notify JS
        js_notify_state_change("secret", "NO_PATH");
    }
};

/**
 * @brief UserContext for secret hint state machine
 *
 * The generated code expects UserContext with a 'secret' member.
 * Calls to secret.onXxx() in SCXML become user_->secret.onXxx() in C++.
 */
struct SecretContext {
    SecretCallbacks secret;
};

// Global secret context instance
static SecretContext g_secret_context;

// ============================================
// Enemy SCXML UserContext: C++ Callback Integration (Multi-Instance)
// ============================================

// Forward declarations for enemy callbacks
static inline void js_notify_enemy_callback(int slot, const char *callback_type, const char *type_name, int instance_id);
static inline void js_notify_enemy_update(int slot, const char *type, const char *state, int instance_id, bool active);

// Forward declaration for EnemyState enum
using EnemyState = SCE::Generated::enemy_state::State;

// Get state name from enemy state machine
static const char *get_enemy_state_name(EnemyState state);

/**
 * @brief Callback handler for enemy state machine (per-instance)
 *
 * Implements onentry actions for each state as defined in enemy_state.scxml.
 * Each enemy instance has its own EnemyCallbacks with slot/instance info.
 * The generated code calls user_->enemy.onXxx() when entering each state.
 */
struct EnemyCallbacks {
    int slot = -1;
    int instance_id = 0;
    const char *type_name = "UNKNOWN";

    void onDormant() {
        js_notify_enemy_callback(slot, "dormant", type_name, instance_id);
        js_notify_enemy_update(slot, type_name, "DORMANT", instance_id, true);
    }

    void onAlert() {
        js_notify_enemy_callback(slot, "alert", type_name, instance_id);
        js_notify_enemy_update(slot, type_name, "ALERT", instance_id, true);
    }

    void onChasing() {
        js_notify_enemy_callback(slot, "chasing", type_name, instance_id);
        js_notify_enemy_update(slot, type_name, "CHASING", instance_id, true);
    }

    void onAttacking() {
        js_notify_enemy_callback(slot, "attacking", type_name, instance_id);
        js_notify_enemy_update(slot, type_name, "ATTACKING", instance_id, true);
    }

    void onPain() {
        js_notify_enemy_callback(slot, "pain", type_name, instance_id);
        js_notify_enemy_update(slot, type_name, "PAIN", instance_id, true);
    }

    void onDead() {
        js_notify_enemy_callback(slot, "dead", type_name, instance_id);
        js_notify_enemy_update(slot, type_name, "DEAD", instance_id, true);
    }
};

/**
 * @brief UserContext for enemy state machine (per-instance)
 *
 * The generated code expects UserContext with an 'enemy' member.
 * Calls to enemy.onXxx() in SCXML become user_->enemy.onXxx() in C++.
 * Each enemy instance has its own EnemyContext with slot info.
 */
struct EnemyContext {
    EnemyCallbacks enemy;
};

// Type aliases for enemy state machine with UserContext
using EnemySM = SCE::Generated::enemy_state::enemy_state<EnemyContext>;
using EnemyEvent = SCE::Generated::enemy_state::Event;

// ============================================
// Enemy Instance Tracking (with SCXML state machine)
// ============================================
struct EnemyInstance {
    void *mobj_ptr = nullptr;
    int instance_id = 0;
    const char *type_name = "UNKNOWN";
    EnemyContext context;  // Per-instance context for callbacks
    std::unique_ptr<EnemySM> sm;  // SCXML state machine instance
    bool active = false;
};

// Get state name from enemy state machine
static const char *get_enemy_state_name(EnemyState state) {
    switch (state) {
    case EnemyState::Dormant:
        return "DORMANT";
    case EnemyState::Alert:
        return "ALERT";
    case EnemyState::Chasing:
        return "CHASING";
    case EnemyState::Attacking:
        return "ATTACKING";
    case EnemyState::Pain:
        return "PAIN";
    case EnemyState::Dead:
        return "DEAD";
    default:
        return "UNKNOWN";
    }
}

// Map DOOM state string to SCXML event
static EnemyEvent doom_state_to_event(const char *state_name) {
    if (strcmp(state_name, "ALERT") == 0) {
        return EnemyEvent::See_player;
    }
    if (strcmp(state_name, "CHASING") == 0) {
        return EnemyEvent::Chase;
    }
    if (strcmp(state_name, "ATTACKING") == 0) {
        return EnemyEvent::Attack;
    }
    if (strcmp(state_name, "PAIN") == 0) {
        return EnemyEvent::Pain;
    }
    // Note: ATTACK_COMPLETE and PAIN_COMPLETE not needed - chase event handles these transitions
    return EnemyEvent::NONE;
}

static std::array<EnemyInstance, MAX_ENEMIES> g_enemies;
static int g_enemy_count = 0;
static int g_enemy_killed = 0;
static int g_next_instance_id = 1;

// ============================================
// State Machine Instances
// ============================================
// Game state machine with UserContext for C++ callbacks
using GameSM = SCE::Generated::game_state::game_state<GameContext>;
using GameEvent = SCE::Generated::game_state::Event;
static std::unique_ptr<GameSM> g_game_sm;

// Player state machine with UserContext for C++ callbacks
using PlayerSM = SCE::Generated::player_state::player_state<PlayerContext>;
using PlayerEvent = SCE::Generated::player_state::Event;
static std::unique_ptr<PlayerSM> g_player_sm;

// Weapon state machine with UserContext for C++ callbacks
using WeaponSM = SCE::Generated::weapon_state::weapon_state<WeaponContext>;
using WeaponEvent = SCE::Generated::weapon_state::Event;
static std::unique_ptr<WeaponSM> g_weapon_sm;
// Secret state machine with UserContext for C++ callbacks
using SecretSM = SCE::Generated::secret_hint_state::secret_hint_state<SecretContext>;
using SecretEvent = SCE::Generated::secret_hint_state::Event;
static std::unique_ptr<SecretSM> g_secret_sm;

// Aim assist state machine with UserContext for C++ callbacks
using AimSM = SCE::Generated::aim_assist_state::aim_assist_state<AimContext>;
using AimEvent = SCE::Generated::aim_assist_state::Event;
static std::unique_ptr<AimSM> g_aim_sm;

// Kill combo state machine with external pausable timer management
// Uses external C++ timers (not SCXML <send delay>) for menu pause support
using ComboSM = SCE::Generated::combo_state::combo_state<ComboContext>;
using ComboEvent = SCE::Generated::combo_state::Event;
using ComboState = SCE::Generated::combo_state::State;
static std::unique_ptr<ComboSM> g_combo_sm;

// Combo tracking state
static int g_combo_count = 0;

// Forward declarations
extern "C" {
const char *sce_get_player_state(void);
const char *sce_get_weapon_state(void);
// Note: sce_combo_reset() declared earlier (line ~276) for game callbacks
}

// Forward declarations for JavaScript callbacks (defined below)
static inline void js_notify_state_change(const char *machine, const char *state);
static inline void js_notify_enemy_update(int slot, const char *type, const char *state, int instance_id, bool active);
static inline void js_notify_stats_update(int enemy_count, int enemy_killed, int enemy_remaining);
static inline void js_notify_secret_path(int num_arrows, int remaining_secrets, bool is_partial);
static inline void js_notify_secret_arrow(int index, int x, int y, int angle);
static inline void js_notify_target_info(const char *type_name, const char *name, int index, int total,
                                         int trigger_x, int trigger_y, int sector_x, int sector_y, int sector_idx,
                                         const char *open_method, int is_hidden, int linked_secret);

// ============================================
// Helper Functions
// ============================================

/**
 * @brief Reset player and weapon state machines for new/load game
 *
 * StaticExecutionEngine::initialize() does NOT reset currentState_,
 * so we must recreate the state machine objects to properly reset them.
 */
static void reset_player_and_weapon_state_machines() {
    // Recreate player state machine with UserContext (initialize() doesn't reset currentState_)
    g_player_sm = std::make_unique<PlayerSM>(g_player_context);
    g_player_sm->initialize();
    // Note: onentry callback handles js_notify_state_change

    // Recreate weapon state machine with UserContext (initialize() doesn't reset currentState_)
    g_weapon_sm = std::make_unique<WeaponSM>(g_weapon_context);
    g_weapon_sm->initialize();
    // Note: onentry callback handles js_notify_state_change
}

/**
 * @brief Reset all enemy tracking and notify JavaScript
 * @param notify_dead If true, notify JS with DEAD state before removal
 */
static void reset_all_enemies(bool notify_dead = false) {
    for (int i = 0; i < MAX_ENEMIES; i++) {
        if (g_enemies[i].active) {
            const char *state_name = "UNKNOWN";
            if (g_enemies[i].sm) {
                state_name = notify_dead ? "DEAD" : get_enemy_state_name(g_enemies[i].sm->getCurrentState());
            }
            js_notify_enemy_update(i, g_enemies[i].type_name, state_name, g_enemies[i].instance_id, false);
            g_enemies[i].sm.reset();
        }
        g_enemies[i] = EnemyInstance{};
    }
    g_enemy_count = 0;
    g_enemy_killed = 0;
    g_next_instance_id = 1;
    js_notify_stats_update(0, 0, 0);
}

// ============================================
// JavaScript Callbacks (100% Sync)
// ============================================
static inline void js_notify_state_change(const char *machine, const char *state) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceStateChange === 'function') {
                window.onSceStateChange(UTF8ToString($0), UTF8ToString($1));
            }
        },
        machine, state);
#endif
}

static inline void js_notify_enemy_update(int slot, const char *type, const char *state, int instance_id, bool active) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceEnemyUpdate === 'function') {
                window.onSceEnemyUpdate($0, UTF8ToString($1), UTF8ToString($2), $3, $4);
            } else {
                console.warn('[SCE:C++] window.onSceEnemyUpdate not defined!');
            }
        },
        slot, type, state, instance_id, active ? 1 : 0);
#endif
}

static inline void js_notify_stats_update(int enemy_count, int enemy_killed, int enemy_remaining) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceStatsUpdate === 'function') {
                window.onSceStatsUpdate($0, $1, $2);
            }
        },
        enemy_count, enemy_killed, enemy_remaining);
#endif
}

static inline void js_notify_secret_path(int num_arrows, int remaining_secrets, bool is_partial) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceSecretPath === 'function') {
                window.onSceSecretPath($0, $1, $2);
            }
        },
        num_arrows, remaining_secrets, is_partial ? 1 : 0);
#endif
}

static inline void js_notify_secret_arrow(int index, int x, int y, int angle) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceSecretArrow === 'function') {
                window.onSceSecretArrow($0, $1, $2, $3);
            }
        },
        index, x, y, angle);
#endif
}

static inline void js_notify_target_info(const char *type_name, const char *name, int index, int total,
                                         int trigger_x, int trigger_y, int sector_x, int sector_y, int sector_idx,
                                         const char *open_method, int is_hidden, int linked_secret) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceTargetInfo === 'function') {
                window.onSceTargetInfo(UTF8ToString($0), UTF8ToString($1), $2, $3, $4, $5, $6, $7, $8,
                                       UTF8ToString($9), $10, $11);
            }
        },
        type_name, name, index, total, trigger_x, trigger_y, sector_x, sector_y, sector_idx,
        open_method, is_hidden, linked_secret);
#endif
}

static inline void js_notify_aim_assist_state(bool enabled) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceAimAssistState === 'function') {
                window.onSceAimAssistState($0);
            }
        },
        enabled ? 1 : 0);
#endif
}

static inline void js_notify_combo_update(int count, bool active) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceComboUpdate === 'function') {
                window.onSceComboUpdate($0, $1);
            }
        },
        count, active ? 1 : 0);
#endif
}

static inline void js_notify_game_callback(const char *callback_type) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceGameCallback === 'function') {
                window.onSceGameCallback(UTF8ToString($0));
            }
        },
        callback_type);
#endif
}

static inline void js_notify_player_callback(const char *callback_type) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onScePlayerCallback === 'function') {
                window.onScePlayerCallback(UTF8ToString($0));
            }
        },
        callback_type);
#endif
}

static inline void js_notify_weapon_callback(const char *callback_type) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceWeaponCallback === 'function') {
                window.onSceWeaponCallback(UTF8ToString($0));
            }
        },
        callback_type);
#endif
}

static inline void js_notify_enemy_callback(int slot, const char *callback_type, const char *type_name, int instance_id) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceEnemyCallback === 'function') {
                window.onSceEnemyCallback($0, UTF8ToString($1), UTF8ToString($2), $3);
            }
        },
        slot, callback_type, type_name, instance_id);
#endif
}

static inline void js_notify_secret_callback(const char *callback_type) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceSecretCallback === 'function') {
                window.onSceSecretCallback(UTF8ToString($0));
            }
        },
        callback_type);
#endif
}

static inline void js_notify_aim_callback(const char *callback_type) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceAimCallback === 'function') {
                window.onSceAimCallback(UTF8ToString($0));
            }
        },
        callback_type);
#endif
}

static inline void js_notify_combo_callback(const char *callback_type) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceComboCallback === 'function') {
                window.onSceComboCallback(UTF8ToString($0));
            }
        },
        callback_type);
#endif
}

static inline void js_notify_berserk_update(int intensity, bool active) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceBerserkUpdate === 'function') {
                window.onSceBerserkUpdate($0, $1);
            }
        },
        intensity, active ? 1 : 0);
#endif
}

static inline void js_notify_combo_timer(double remaining_ms, double total_ms) {
#ifdef __EMSCRIPTEN__
    EM_ASM(
        {
            if (typeof window.onSceComboTimer === 'function') {
                window.onSceComboTimer($0, $1);
            }
        },
        remaining_ms, total_ms);
#endif
}

// ============================================
// Enemy Management
// ============================================
static int find_enemy_slot(void *mobj) {
    for (int i = 0; i < MAX_ENEMIES; i++) {
        if (g_enemies[i].active && g_enemies[i].mobj_ptr == mobj) {
            return i;
        }
    }
    return -1;
}

static int find_free_slot() {
    for (int i = 0; i < MAX_ENEMIES; i++) {
        if (!g_enemies[i].active) {
            return i;
        }
    }
    return -1;
}

extern "C" {

// Forward declaration for get game state
const char *sce_get_game_state(void);

// ============================================
// Initialization
// ============================================

// Forward declaration for secret state
const char *sce_secret_get_state(void);

// Forward declaration for aim assist state
const char *sce_aim_get_state(void);

// Forward declaration for combo state
const char *sce_combo_get_state(void);

EMSCRIPTEN_KEEPALIVE
void sce_init(void) {
    // Game state machine: pass UserContext for C++ callback integration
    g_game_sm = std::make_unique<GameSM>(g_game_context);
    // Player state machine: pass UserContext for C++ callback integration
    g_player_sm = std::make_unique<PlayerSM>(g_player_context);
    // Weapon state machine: pass UserContext for C++ callback integration
    g_weapon_sm = std::make_unique<WeaponSM>(g_weapon_context);
    // Secret state machine: pass UserContext for C++ callback integration
    g_secret_sm = std::make_unique<SecretSM>(g_secret_context);
    // Aim assist state machine: pass UserContext for C++ callback integration
    g_aim_sm = std::make_unique<AimSM>(g_aim_context);
    // Kill combo state machine with external pausable timer management
    g_combo_sm = std::make_unique<ComboSM>(g_combo_context);

    g_game_sm->initialize();
    g_player_sm->initialize();
    g_weapon_sm->initialize();
    g_secret_sm->initialize();
    g_aim_sm->initialize();
    g_combo_sm->initialize();

    // Reset enemy tracking
    for (auto &e : g_enemies) {
        e = EnemyInstance{};
    }
    g_enemy_count = 0;
    g_enemy_killed = 0;
    g_next_instance_id = 1;
    g_combo_count = 0;

    // Notify initial states
    js_notify_state_change("game", sce_get_game_state());
    js_notify_state_change("player", sce_get_player_state());
    js_notify_state_change("weapon", sce_get_weapon_state());
    js_notify_state_change("secret", sce_secret_get_state());
    js_notify_state_change("aim", sce_aim_get_state());
    js_notify_state_change("combo", sce_combo_get_state());
    js_notify_stats_update(0, 0, 0);
    js_notify_combo_update(0, false);
}

// ============================================
// Game State Machine
// ============================================

EMSCRIPTEN_KEEPALIVE
const char *sce_get_game_state(void) {
    if (!g_game_sm) {
        return "UNINITIALIZED";
    }
    auto state = g_game_sm->getCurrentState();
    switch (state) {
    case SCE::Generated::game_state::State::Demoscreen:
        return "DEMOSCREEN";
    case SCE::Generated::game_state::State::Level:
        return "LEVEL";
    case SCE::Generated::game_state::State::Intermission:
        return "INTERMISSION";
    case SCE::Generated::game_state::State::Finale:
        return "FINALE";
    default:
        return "UNKNOWN";
    }
}

#define GAME_EVENT(name, event)                                                                                        \
    EMSCRIPTEN_KEEPALIVE                                                                                               \
    void sce_game_event_##name(void) {                                                                                 \
        if (g_game_sm) {                                                                                               \
            g_game_sm->raiseExternal(GameEvent::event);                                                                \
            g_game_sm->step();                                                                                         \
        }                                                                                                              \
    }

// Special handling for newgame - reset all state machines
EMSCRIPTEN_KEEPALIVE
void sce_game_event_newgame(void) {
    if (g_game_sm) {
        // W3C SCXML: Use raiseExternal + step for proper onentry execution
        g_game_sm->raiseExternal(GameEvent::Newgame);
        g_game_sm->step();
        // Note: onentry callback handles js_notify_state_change
    }

    // Reset player and weapon state machines
    reset_player_and_weapon_state_machines();

    // Reset all enemy tracking
    reset_all_enemies(false);
}

// Special handling for loadgame - also reset player/weapon states
EMSCRIPTEN_KEEPALIVE
void sce_game_event_loadgame(void) {
    if (g_game_sm) {
        // W3C SCXML: Use raiseExternal + step for proper onentry execution
        g_game_sm->raiseExternal(GameEvent::Loadgame);
        g_game_sm->step();
        // Note: onentry callback handles js_notify_state_change
    }

    // Reset player and weapon state machines
    reset_player_and_weapon_state_machines();

    // Reset all enemy tracking (enemies will be re-spawned by level load)
    reset_all_enemies(true);
}

// Special handling for demo start - reset player/weapon/enemy without changing game state
EMSCRIPTEN_KEEPALIVE
void sce_game_event_demostart(void) {
    // Game state stays in DEMOSCREEN - no state machine event needed

    // Reset player and weapon state machines for new demo
    reset_player_and_weapon_state_machines();

    // Reset all enemy tracking
    reset_all_enemies(false);

    // Reset combo/berserk state machine for new demo
    sce_combo_reset();
}

GAME_EVENT(completed, Completed)
GAME_EVENT(worlddone, Worlddone)
GAME_EVENT(finale, Finale)

#undef GAME_EVENT

// ============================================
// Player State Machine
// ============================================

EMSCRIPTEN_KEEPALIVE
const char *sce_get_player_state(void) {
    if (!g_player_sm) {
        return "UNINITIALIZED";
    }
    auto state = g_player_sm->getCurrentState();
    switch (state) {
    case SCE::Generated::player_state::State::Alive:
        return "ALIVE";
    case SCE::Generated::player_state::State::Dead:
        return "DEAD";
    case SCE::Generated::player_state::State::Reborn:
        return "REBORN";
    case SCE::Generated::player_state::State::Invulnerable:
        return "INVULNERABLE";
    default:
        return "UNKNOWN";
    }
}

#define PLAYER_EVENT(name, event)                                                                                      \
    EMSCRIPTEN_KEEPALIVE                                                                                               \
    void sce_player_event_##name(void) {                                                                               \
        if (g_player_sm) {                                                                                             \
            g_player_sm->raiseExternal(PlayerEvent::event);                                                            \
            g_player_sm->step();                                                                                       \
        }                                                                                                              \
    }

PLAYER_EVENT(killed, Killed)
PLAYER_EVENT(spawn_complete, Spawn_complete)

// Custom respawn handler - resets enemy tracking
// Note: combo/berserk already reset in onDead() callback
EMSCRIPTEN_KEEPALIVE
void sce_player_event_respawn(void) {
    if (g_player_sm) {
        g_player_sm->raiseExternal(PlayerEvent::Respawn);
        g_player_sm->step();
    }

    // Reset enemy tracking (threats/killed counts) on respawn
    reset_all_enemies(false);

    printf("[PLAYER] Respawn - reset enemies\n");
}
PLAYER_EVENT(god_mode_on, God_mode_on)
PLAYER_EVENT(god_mode_off, God_mode_off)

#undef PLAYER_EVENT

// ============================================
// Weapon State Machine
// ============================================

EMSCRIPTEN_KEEPALIVE
const char *sce_get_weapon_state(void) {
    if (!g_weapon_sm) {
        return "UNINITIALIZED";
    }
    auto state = g_weapon_sm->getCurrentState();
    switch (state) {
    case SCE::Generated::weapon_state::State::Ready:
        return "READY";
    case SCE::Generated::weapon_state::State::Lowering:
        return "LOWERING";
    case SCE::Generated::weapon_state::State::Raising:
        return "RAISING";
    case SCE::Generated::weapon_state::State::Firing:
        return "FIRING";
    default:
        return "UNKNOWN";
    }
}

#define WEAPON_EVENT(name, event)                                                                                      \
    EMSCRIPTEN_KEEPALIVE                                                                                               \
    void sce_weapon_event_##name(void) {                                                                               \
        if (g_weapon_sm) {                                                                                             \
            g_weapon_sm->raiseExternal(WeaponEvent::event);                                                            \
            g_weapon_sm->step();                                                                                       \
        }                                                                                                              \
    }

WEAPON_EVENT(fire, Fire)
WEAPON_EVENT(switch_weapon, Switch_weapon)
WEAPON_EVENT(lower_complete, Lower_complete)
WEAPON_EVENT(raise_complete, Raise_complete)
WEAPON_EVENT(fire_complete, Fire_complete)

#undef WEAPON_EVENT

// ============================================
// Enemy State Machine (Multi-Instance)
// 100% Synchronized via JavaScript callbacks
// ============================================

EMSCRIPTEN_KEEPALIVE
int sce_get_enemy_count(void) {
    return g_enemy_count;
}

EMSCRIPTEN_KEEPALIVE
int sce_get_enemy_killed(void) {
    return g_enemy_killed;
}

EMSCRIPTEN_KEEPALIVE
int sce_get_max_enemies(void) {
    return MAX_ENEMIES;
}

// DOOM Level Statistics
EMSCRIPTEN_KEEPALIVE
int sce_get_level_total_kills(void) {
    return SCE_GetLevelTotalKills();
}

EMSCRIPTEN_KEEPALIVE
int sce_get_player_kill_count(void) {
    return SCE_GetPlayerKillCount();
}

EMSCRIPTEN_KEEPALIVE
int sce_get_enemies_remaining(void) {
    // Return count of currently tracked live enemies
    return g_enemy_count;
}

// Get enemy info by slot: returns "ptr,type,state,instance_id" or empty
EMSCRIPTEN_KEEPALIVE
const char *sce_get_enemy_info(int slot) {
    static char buffer[128];
    buffer[0] = '\0';

    if (slot < 0 || slot >= MAX_ENEMIES || !g_enemies[slot].active) {
        return buffer;
    }

    // Get state from state machine
    const char *state_name = "UNKNOWN";
    if (g_enemies[slot].sm) {
        state_name = get_enemy_state_name(g_enemies[slot].sm->getCurrentState());
    }

    // Use field width limits to prevent buffer overflow
    // Format: ptr(max 20), type(max 31), state(max 31), id(max 10) + separators
    snprintf(buffer, sizeof(buffer), "%ld,%.31s,%.31s,%d",
             (long)(intptr_t)g_enemies[slot].mobj_ptr,
             g_enemies[slot].type_name ? g_enemies[slot].type_name : "UNKNOWN",
             state_name ? state_name : "UNKNOWN",
             g_enemies[slot].instance_id);
    buffer[sizeof(buffer) - 1] = '\0';  // Ensure null-termination
    return buffer;
}

// Called from DOOM when enemy spawns/wakes up
EMSCRIPTEN_KEEPALIVE
void sce_enemy_spawn(void *mobj, const char *type_name) {
    int slot = find_enemy_slot(mobj);
    if (slot < 0) {
        slot = find_free_slot();
        if (slot < 0) {
            return;  // No free slots
        }
    }

    g_enemies[slot].mobj_ptr = mobj;
    g_enemies[slot].instance_id = g_next_instance_id++;
    g_enemies[slot].type_name = type_name;
    g_enemies[slot].active = true;

    // Initialize context with slot info for callbacks
    g_enemies[slot].context.enemy.slot = slot;
    g_enemies[slot].context.enemy.instance_id = g_enemies[slot].instance_id;
    g_enemies[slot].context.enemy.type_name = type_name;

    // Create and initialize SCXML state machine for this enemy with UserContext
    g_enemies[slot].sm = std::make_unique<EnemySM>(g_enemies[slot].context);
    g_enemies[slot].sm->initialize();
    // Note: onentry callback (onDormant) handles js_notify_enemy_update

    g_enemy_count++;

    // Stats: total = alive + killed, remaining = alive (g_enemy_count)
    js_notify_stats_update(g_enemy_count + g_enemy_killed, g_enemy_killed, g_enemy_count);
}

// Called from DOOM when enemy state changes
EMSCRIPTEN_KEEPALIVE
void sce_enemy_set_state(void *mobj, const char *doom_state) {
    int slot = find_enemy_slot(mobj);
    if (slot < 0 || !g_enemies[slot].sm) {
        return;
    }

    // Map DOOM state to SCXML event and process it
    // W3C SCXML: Use raiseExternal + step for proper onentry execution
    EnemyEvent event = doom_state_to_event(doom_state);
    if (event != EnemyEvent::NONE) {
        g_enemies[slot].sm->raiseExternal(event);
        g_enemies[slot].sm->step();
    }
    // Note: onentry callback handles js_notify_enemy_update
}

// Called from DOOM when enemy dies
EMSCRIPTEN_KEEPALIVE
void sce_enemy_killed(void *mobj) {
    int slot = find_enemy_slot(mobj);

    // Update enemy state machine if tracked
    if (slot >= 0 && g_enemies[slot].sm) {
        // W3C SCXML: Use raiseExternal + step for proper onentry execution
        g_enemies[slot].sm->raiseExternal(EnemyEvent::Killed);
        g_enemies[slot].sm->step();
        // Note: onentry callback (onDead) handles js_notify_enemy_update

        g_enemy_killed++;
        g_enemy_count--;  // Decrement immediately when killed, not when removed

        js_notify_stats_update(g_enemy_count + g_enemy_killed, g_enemy_killed, g_enemy_count);
    }

    // Kill Combo System: Always process kills even if enemy not tracked
    // External Timer Architecture: Callbacks handle timer restart on kill
    // W3C SCXML 3.4: Compound states with normal/berserk sub-states
    if (g_combo_sm) {
        // Send kill event - onentry callback restarts external timer
        g_combo_sm->raiseExternal(ComboEvent::Kill);
        g_combo_sm->step();
        // Note: onentry callback handles js_notify_state_change

        ComboState new_state = g_combo_sm->getCurrentState();

        // Handle state-specific logic for compound states
        if (new_state == ComboState::Normal) {
            // In normal combo mode (1-4 kills)
            g_combo_count++;

            printf("[COMBO] Kill #%d in NORMAL mode - external timer restarted (5s)\n",
                   g_combo_count);

            // Check if we should trigger berserk (combo >= 5)
            if (g_combo_count >= BERSERK_TRIGGER_COMBO) {
                printf("[COMBO] Combo >= %d, triggering BERSERK!\n", BERSERK_TRIGGER_COMBO);

                // Trigger transition to berserk state
                // SCXML onentry handles: cancel combo_timer, schedule berserk_timer (10s)
                g_combo_sm->raiseExternal(ComboEvent::Berserk_activate);
                g_combo_sm->step();
            }

            // Notify UI with current combo count
            js_notify_combo_update(g_combo_count, true);

        } else if (new_state == ComboState::Berserk) {
            // In berserk mode - kill extends timer (callback restarts on self-transition)
            g_combo_count++;

            printf("[BERSERK] Kill #%d - external timer restarted (10s), intensity=%d\n",
                   g_combo_count, g_berserk_intensity);

            // Notify UI with current combo count (berserk active)
            js_notify_combo_update(g_combo_count, true);
        }
    }
}

// Called from DOOM when enemy is removed from map
EMSCRIPTEN_KEEPALIVE
void sce_enemy_remove(void *mobj) {
    int slot = find_enemy_slot(mobj);
    if (slot < 0) {
        return;
    }

    // Get current state before cleanup
    const char *state_name = "UNKNOWN";
    if (g_enemies[slot].sm) {
        state_name = get_enemy_state_name(g_enemies[slot].sm->getCurrentState());
    }

    // 100% sync callback - notify inactive
    js_notify_enemy_update(slot, g_enemies[slot].type_name, state_name, g_enemies[slot].instance_id, false);

    // Check if this enemy was already counted as killed (DEAD state)
    bool was_killed = g_enemies[slot].sm &&
        g_enemies[slot].sm->getCurrentState() == EnemyState::Dead;

    // Cleanup state machine
    g_enemies[slot].sm.reset();
    g_enemies[slot].active = false;
    g_enemies[slot].mobj_ptr = nullptr;

    // Only decrement if NOT already killed (e.g., removed without dying)
    if (!was_killed) {
        g_enemy_count--;
    }

    js_notify_stats_update(g_enemy_count + g_enemy_killed, g_enemy_killed, g_enemy_count);
}

// Clear all enemy tracking (for save/load)
EMSCRIPTEN_KEEPALIVE
void sce_enemy_clear_all(void) {
    for (int i = 0; i < MAX_ENEMIES; i++) {
        if (g_enemies[i].active) {
            g_enemies[i].sm.reset();
            g_enemies[i].active = false;
            g_enemies[i].mobj_ptr = nullptr;
        }
    }
    g_enemy_count = 0;
    g_enemy_killed = 0;
    js_notify_stats_update(g_enemy_count, g_enemy_killed, g_enemy_count);
}

// ============================================
// Secret Hint State Machine
// ============================================

// Forward declarations for secret functions
void sce_secret_path_found(void);
void sce_secret_no_path(void);

EMSCRIPTEN_KEEPALIVE
const char *sce_secret_get_state(void) {
    if (!g_secret_sm) {
        return "UNINITIALIZED";
    }
    auto state = g_secret_sm->getCurrentState();
    switch (state) {
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

// SCXML-driven: processEvent() triggers onentry callbacks automatically
EMSCRIPTEN_KEEPALIVE
void sce_secret_event_toggle(void) {
    // H key: Toggle hint system (disabled <-> enabled)
    // SCXML handles state transitions, onentry callbacks do the work
    if (g_secret_sm) {
        g_secret_sm->raiseExternal(SecretEvent::Toggle);
        g_secret_sm->step();
    }
}

// Legacy aliases for backwards compatibility
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
    // Called from p_spec.c when player enters a secret sector
    // Update JavaScript UI with new remaining secret count
    js_notify_secret_path(0, Secret_GetRemainingCount(), false);
}

EMSCRIPTEN_KEEPALIVE
void sce_secret_update_count(void) {
    // Called to update the secret count display without changing state
    int count = Secret_GetRemainingCount();
    js_notify_secret_path(0, count, false);
}

EMSCRIPTEN_KEEPALIVE
void sce_secret_event_next_target(void) {
    sce_secret_event_toggle();
}

EMSCRIPTEN_KEEPALIVE
void sce_secret_event_prev_target(void) {
    // G key: No longer used
}

EMSCRIPTEN_KEEPALIVE
void sce_select_target(int type, int index) {
    // Button click: Select target and show path
    // SCXML calculating state onentry will run BFS
    if (g_secret_sm) {
        // Get current selection to check if same target
        target_type_t curr_type;
        int curr_index, curr_total;
        Secret_GetSelectionInfo(&curr_type, &curr_index, &curr_total);

        // Check if selecting the same target
        bool is_same_target = (curr_type == (target_type_t)type && curr_index == index);

        auto current_state = g_secret_sm->getCurrentState();
        bool in_disabled = (current_state == SCE::Generated::secret_hint_state::State::Disabled);
        bool in_no_path = (current_state == SCE::Generated::secret_hint_state::State::No_path);

        if (Secret_SelectTarget((target_type_t)type, index)) {
            if (in_disabled) {
                // From disabled state - go to calculating via Select
                g_secret_sm->raiseExternal(SecretEvent::Select);
                g_secret_sm->step();          // Enter calculating, run BFS, store result
                sce_process_bfs_result();     // Process stored result (clean separation)
            } else if (is_same_target && !in_no_path) {
                // Same target clicked while showing - toggle off (enabled -> disabled)
                g_secret_sm->raiseExternal(SecretEvent::Toggle);
                g_secret_sm->step();
            } else {
                // Different target OR same target in no_path - recalculate
                g_secret_sm->raiseExternal(SecretEvent::Select);
                g_secret_sm->step();          // Enter calculating, run BFS, store result
                sce_process_bfs_result();     // Process stored result (clean separation)
            }
        }
    }
}

EMSCRIPTEN_KEEPALIVE
void sce_secret_event_request(void) {
    // Backwards compatibility - same as toggle
    sce_secret_event_next_target();
}

EMSCRIPTEN_KEEPALIVE
void sce_secret_event_select(void) {
    // Select event: triggers path calculation to currently selected target
    if (g_secret_sm) {
        g_secret_sm->raiseExternal(SecretEvent::Select);
        g_secret_sm->step();          // Enter calculating, run BFS, store result
        sce_process_bfs_result();     // Process stored result
    }
}

EMSCRIPTEN_KEEPALIVE
void sce_secret_event_cancel(void) {
    // Legacy function - cancel is now just toggle (disable)
    sce_secret_event_toggle();
}

EMSCRIPTEN_KEEPALIVE
void sce_secret_event_level_change(void) {
    if (g_secret_sm) {
        g_secret_sm->raiseExternal(SecretEvent::Level_change);
        g_secret_sm->step();
    }
    // Send initial secret count when level starts
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
    // Legacy function - kept for API compatibility
    // Prefer using sce_process_bfs_result() after step()
    if (g_secret_sm) {
        g_secret_sm->raiseExternal(SecretEvent::Path_found);
    }
}

EMSCRIPTEN_KEEPALIVE
void sce_secret_no_path(void) {
    // Legacy function - kept for API compatibility
    // Prefer using sce_process_bfs_result() after step()
    if (g_secret_sm) {
        g_secret_sm->raiseExternal(SecretEvent::No_path);
    }
}

/**
 * @brief Process BFS result after step() returns from calculating state
 *
 * This function should be called after step() returns from entering
 * the calculating state. The onCalculating callback stores the BFS
 * result in g_bfs_result without raising events (to avoid nested step()).
 *
 * Clean design pattern:
 *   raiseExternal(Select) → step() → onCalculating() stores result
 *   → sce_process_bfs_result() raises event → step() transitions state
 */
static void sce_process_bfs_result(void) {
    if (!g_secret_sm) return;

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
            // No BFS was performed (shouldn't happen in normal flow)
            break;
    }
    g_bfs_result = BfsResult::NONE;  // Reset for next calculation
}

EMSCRIPTEN_KEEPALIVE
void sce_secret_recalculate(void) {
    // Called from Secret_UpdateArrows when player moves significantly
    if (g_secret_sm) {
        g_secret_sm->raiseExternal(SecretEvent::Recalculate);
        g_secret_sm->step();          // Enter calculating, run BFS, store result
        sce_process_bfs_result();     // Process stored result
    }
}

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
    // Refresh enemy list to remove dead enemies before returning count
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
// Path Destination Mode API
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

// ============================================
// Aim Assist State Machine
// ============================================

EMSCRIPTEN_KEEPALIVE
const char *sce_aim_get_state(void) {
    if (!g_aim_sm) {
        return "UNINITIALIZED";
    }
    auto state = g_aim_sm->getCurrentState();
    switch (state) {
    case SCE::Generated::aim_assist_state::State::Disabled:
        return "disabled";
    case SCE::Generated::aim_assist_state::State::Enabled:
        return "enabled";
    case SCE::Generated::aim_assist_state::State::Idle:
        return "idle";
    case SCE::Generated::aim_assist_state::State::Searching:
        return "searching";
    case SCE::Generated::aim_assist_state::State::Locked:
        return "locked";
    default:
        return "UNKNOWN";
    }
}

EMSCRIPTEN_KEEPALIVE
void sce_aim_event_toggle(void) {
    if (g_aim_sm) {
        // W3C SCXML: Use raiseExternal + step for proper hierarchical state entry
        // processEvent() bypasses handleHierarchicalTransition, missing parent state entry
        g_aim_sm->raiseExternal(AimEvent::Toggle);
        g_aim_sm->step();
        // Note: onentry callback handles js_notify_state_change
    }
}

EMSCRIPTEN_KEEPALIVE
void sce_aim_event_shot_fired(void) {
    printf("[AIM EVENT] shot_fired, g_aim_sm=%p\n", (void*)g_aim_sm.get());
    if (g_aim_sm) {
        g_aim_sm->raiseExternal(AimEvent::Shot_fired);
        g_aim_sm->step();
    }
}

EMSCRIPTEN_KEEPALIVE
void sce_aim_event_target_acquired(void) {
    printf("[AIM EVENT] target_acquired, g_aim_sm=%p\n", (void*)g_aim_sm.get());
    if (g_aim_sm) {
        g_aim_sm->raiseExternal(AimEvent::Target_acquired);
        g_aim_sm->step();
    }
}

EMSCRIPTEN_KEEPALIVE
void sce_aim_event_target_lost(void) {
    printf("[AIM EVENT] target_lost, g_aim_sm=%p\n", (void*)g_aim_sm.get());
    if (g_aim_sm) {
        g_aim_sm->raiseExternal(AimEvent::Target_lost);
        g_aim_sm->step();
    }
}

EMSCRIPTEN_KEEPALIVE
void sce_aim_event_no_target(void) {
    printf("[AIM EVENT] no_target, g_aim_sm=%p\n", (void*)g_aim_sm.get());
    if (g_aim_sm) {
        g_aim_sm->raiseExternal(AimEvent::No_target);
        g_aim_sm->step();
    }
}

EMSCRIPTEN_KEEPALIVE
void sce_aim_event_shot_complete(void) {
    printf("[AIM EVENT] shot_complete, g_aim_sm=%p\n", (void*)g_aim_sm.get());
    if (g_aim_sm) {
        g_aim_sm->raiseExternal(AimEvent::Shot_complete);
        g_aim_sm->step();
    }
}

EMSCRIPTEN_KEEPALIVE
int sce_aim_is_enabled(void) {
    return g_aim_assist_enabled ? 1 : 0;
}

// ============================================
// Kill Combo State Machine
// Uses external pausable timer (supports menu pause)
// ============================================

EMSCRIPTEN_KEEPALIVE
const char *sce_combo_get_state(void) {
    if (!g_combo_sm) {
        return "UNINITIALIZED";
    }
    auto state = g_combo_sm->getCurrentState();
    switch (state) {
    case ComboState::Idle:
        return "idle";
    case ComboState::Normal:
        return "normal";
    case ComboState::Berserk:
        return "berserk";
    default:
        return "UNKNOWN";
    }
}

EMSCRIPTEN_KEEPALIVE
int sce_combo_get_count(void) {
    return g_combo_count;
}

EMSCRIPTEN_KEEPALIVE
int sce_combo_is_active(void) {
    if (!g_combo_sm) {
        return 0;
    }
    ComboState state = g_combo_sm->getCurrentState();
    return (state == ComboState::Normal || state == ComboState::Berserk) ? 1 : 0;
}

EMSCRIPTEN_KEEPALIVE
int sce_combo_is_berserk(void) {
    if (!g_combo_sm) {
        return 0;
    }
    return (g_combo_sm->getCurrentState() == ComboState::Berserk) ? 1 : 0;
}

EMSCRIPTEN_KEEPALIVE
int sce_berserk_get_intensity(void) {
    return g_berserk_intensity;
}

/**
 * @brief Calculate elapsed time for external timer (supports pause)
 *
 * @return Elapsed milliseconds since timer start, accounting for pauses
 */
static double get_timer_elapsed_ms() {
    if (g_active_timer_type == ComboTimerType::NONE) {
        return 0;
    }

    double elapsed = g_timer_elapsed_before_pause_ms;

    if (g_timer_paused) {
        // Timer is paused - elapsed is frozen at pause point
        return elapsed;
    }

    // Add time since last (re)start
    auto now = std::chrono::steady_clock::now();
    elapsed += std::chrono::duration<double, std::milli>(now - g_timer_start).count();
    return elapsed;
}

/**
 * @brief Process one game tic for timer-based state machines
 *
 * Called from DOOM's game loop (G_Ticker) at 35Hz.
 * External Timer Architecture: Game manages pausable timers, raises events on expiry.
 * Handles combo_timeout (5s) and berserk_timeout (10s) events.
 */
EMSCRIPTEN_KEEPALIVE
void sce_process_tic(void) {
    if (!g_combo_sm) {
        return;
    }

    // Check if external timer expired (only when not paused)
    if (g_active_timer_type != ComboTimerType::NONE && !g_timer_paused) {
        double elapsed = get_timer_elapsed_ms();
        double remaining = g_timer_duration_ms - elapsed;

        if (remaining <= 0) {
            // Timer expired - raise appropriate timeout event
            ComboState prev_state = g_combo_sm->getCurrentState();

            if (g_active_timer_type == ComboTimerType::COMBO) {
                printf("[COMBO] External timer expired (%.0fms) - raising combo_timeout\n", g_timer_duration_ms);
                g_combo_sm->raiseExternal(ComboEvent::Combo_timeout);
            } else if (g_active_timer_type == ComboTimerType::BERSERK) {
                printf("[BERSERK] External timer expired (%.0fms) - raising berserk_timeout\n", g_timer_duration_ms);
                g_combo_sm->raiseExternal(ComboEvent::Berserk_timeout);
            }

            g_combo_sm->step();

            // Timer is now cleared by onIdle callback
            ComboState current_state = g_combo_sm->getCurrentState();

            // If state changed to idle, combo ended
            if (current_state == ComboState::Idle && prev_state != ComboState::Idle) {
                printf("[COMBO/BERSERK] State: %s -> idle, resetting combo from %d to 0\n",
                       prev_state == ComboState::Normal ? "normal" : "berserk",
                       g_combo_count);
                g_combo_count = 0;
                js_notify_combo_update(0, false);
                js_notify_combo_timer(-1, 0);  // Clear timer bar
            }
        } else {
            // Timer still running - update UI with remaining time
            js_notify_combo_timer(remaining, g_timer_duration_ms);
        }
    } else if (g_active_timer_type != ComboTimerType::NONE && g_timer_paused) {
        // Timer is paused - update UI once only (avoid 35Hz redundant calls)
        if (!g_timer_pause_ui_sent) {
            double elapsed = get_timer_elapsed_ms();
            double remaining = g_timer_duration_ms - elapsed;
            js_notify_combo_timer(remaining, g_timer_duration_ms);
            g_timer_pause_ui_sent = true;
        }
    }
}

/**
 * @brief Pause combo/berserk timer (call when menu opens)
 *
 * External Timer Architecture: Pauses timer without losing progress.
 * Resume with sce_combo_timer_resume().
 */
EMSCRIPTEN_KEEPALIVE
void sce_combo_timer_pause(void) {
    printf("[TIMER] sce_combo_timer_pause() CALLED - timer_type=%d, paused=%d\n",
           (int)g_active_timer_type, g_timer_paused);

    if (g_active_timer_type == ComboTimerType::NONE || g_timer_paused) {
        printf("[TIMER] Early return - no timer or already paused\n");
        return;  // No timer or already paused
    }

    // Save elapsed time up to now
    auto now = std::chrono::steady_clock::now();
    g_timer_elapsed_before_pause_ms += std::chrono::duration<double, std::milli>(now - g_timer_start).count();
    g_timer_paused = true;
    g_timer_pause_ui_sent = false;  // Allow one UI update for paused state

    printf("[TIMER] Paused at %.0fms / %.0fms\n",
           g_timer_elapsed_before_pause_ms, g_timer_duration_ms);
}

/**
 * @brief Resume combo/berserk timer (call when menu closes)
 *
 * External Timer Architecture: Continues timer from where it was paused.
 */
EMSCRIPTEN_KEEPALIVE
void sce_combo_timer_resume(void) {
    if (g_active_timer_type == ComboTimerType::NONE || !g_timer_paused) {
        return;  // No timer or not paused
    }

    g_timer_paused = false;
    g_timer_start = std::chrono::steady_clock::now();  // Restart timing from now

    printf("[TIMER] Resumed at %.0fms / %.0fms\n",
           g_timer_elapsed_before_pause_ms, g_timer_duration_ms);
}

/**
 * @brief Check if combo/berserk timer is currently paused
 *
 * @return 1 if paused, 0 if running or no timer
 */
EMSCRIPTEN_KEEPALIVE
int sce_combo_timer_is_paused(void) {
    return (g_active_timer_type != ComboTimerType::NONE && g_timer_paused) ? 1 : 0;
}

/**
 * @brief Reset combo state machine (for new game/level change)
 *
 * Called when starting a new level to reset combo and berserk tracking.
 * External Timer Architecture: Clears timer state when resetting.
 */
EMSCRIPTEN_KEEPALIVE
void sce_combo_reset(void) {
    if (g_combo_sm) {
        // Reset berserk effects in DOOM
        SCE_BerserkReset();

        // Reset external timer state
        clear_external_timer();

        // Recreate state machine to properly reset state with UserContext
        g_combo_sm = std::make_unique<ComboSM>(g_combo_context);
        g_combo_sm->initialize();
        // Note: onentry callback (onIdle) handles js_notify_state_change

        g_combo_count = 0;
        g_berserk_intensity = 0;
        js_notify_combo_update(0, false);
        js_notify_berserk_update(0, false);
        js_notify_combo_timer(-1, 0);  // Clear timer bar
        js_notify_state_change("combo", sce_combo_get_state());
    }
}

}  // extern "C"
