# DOOM WASM + SCE State Machines

DOOM compiled to WebAssembly with real-time SCXML state machine visualization.

## Overview

This example integrates **three SCE state machines** into DOOM:

| State Machine | States | Purpose |
|---------------|--------|---------|
| **Game** | DEMOSCREEN, LEVEL, INTERMISSION, FINALE | Game flow control |
| **Player** | ALIVE, INVULNERABLE, DEAD, REBORN | Player lifecycle |
| **Weapon** | READY, RAISING, LOWERING, FIRING | Weapon action states |

All state machines use **direct event injection** from DOOM's source code for 100% accurate synchronization.

## Features

- **Tab-based visualization**: Switch between Game, Player, Weapon state diagrams
- **Real-time state highlighting**: Current state highlighted in diagram
- **Statistics tracking**: Deaths, shots fired, weapon switches, transitions
- **Zero external dependencies**: Self-contained C headers

## Architecture

### Direct Injection Approach

Unlike polling-based synchronization, events are injected directly at the exact points in DOOM's code where state changes occur:

```
DOOM Source Code                    SCE State Machine
─────────────────                   ─────────────────
p_pspr.c:P_FireWeapon()    ──────►  sce_weapon_event(FIRE)
p_pspr.c:A_Lower()         ──────►  sce_weapon_event(LOWER_COMPLETE)
p_inter.c:P_KillMobj()     ──────►  sce_player_event(KILLED)
g_game.c:gamestate=LEVEL   ──────►  sce_set_state(LEVEL)
```

This provides 100% accurate state tracking vs ~95% with polling.

### Injection Points

| File | Function | Event/State |
|------|----------|-------------|
| `p_pspr.c` | P_FireWeapon | FIRE |
| `p_pspr.c` | A_WeaponReady | SWITCH_WEAPON |
| `p_pspr.c` | A_Lower | LOWER_COMPLETE |
| `p_pspr.c` | A_Raise | RAISE_COMPLETE |
| `p_pspr.c` | A_ReFire | FIRE_COMPLETE, FIRE_REFIRE |
| `p_inter.c` | P_KillMobj | KILLED |
| `p_mobj.c` | P_SpawnPlayer | SPAWN_COMPLETE |
| `g_game.c` | G_DoReborn | RESPAWN |
| `g_game.c` | G_DoNewGame | Reset all state machines |
| `g_game.c` | gamestate= | LEVEL, INTERMISSION |
| `f_finale.c` | gamestate= | FINALE |
| `d_main.c` | gamestate= | DEMOSCREEN |

## Prerequisites

1. **Emscripten SDK**
   ```bash
   git clone https://github.com/emscripten-core/emsdk.git
   cd emsdk && ./emsdk install latest && ./emsdk activate latest
   source emsdk_env.sh
   ```

2. **wget** (for downloading doom1.wad)

## Building

```bash
chmod +x build.sh
./build.sh
```

The build script:
1. Clones doomgeneric (minimal DOOM port)
2. Copies SCE state machine headers
3. Patches DOOM source with direct event injection
4. Compiles to WebAssembly
5. Sets up visualizer integration

## Running

```bash
cd build
python3 -m http.server 8080
# Open http://localhost:8080/
```

## Files

```
doom_wasm/
├── build.sh                    # Build + patch script
├── generated/
│   ├── sce_game_state.h        # Game state machine
│   ├── sce_player_state.h      # Player state machine
│   └── sce_weapon_state.h      # Weapon state machine
├── scxml/
│   ├── game_state.scxml        # Game SCXML definition
│   ├── player_state.scxml      # Player SCXML definition
│   └── weapon_state.scxml      # Weapon SCXML definition
└── README.md
```

## State Machine Diagrams

### Game State
```
DEMOSCREEN ──newgame/loadgame──► LEVEL ──completed──► INTERMISSION
     ▲                            │                        │
     │ died/quit                  │ victory                │ worlddone
     │                            ▼                        ▼
     └────────────────────────── FINALE ◄─────────────────┘
```

### Player State
```
ALIVE ──killed──► DEAD ──respawn/use──► REBORN ──spawn_complete──► ALIVE
  │                                                                   ▲
  │ god_mode_on                                                       │
  ▼                                                                   │
INVULNERABLE ──god_mode_off───────────────────────────────────────────┘
```

### Weapon State
```
READY ──fire──► FIRING ──fire_complete──► READY
  │                │
  │ switch         │ fire_refire (loop)
  ▼                │
LOWERING ──lower_complete──► RAISING ──raise_complete──► READY
```

## API

### Game State
```c
void sce_init(void);
void sce_set_state(sce_state_t state);
const char* sce_get_state_name(void);
```

### Player State
```c
void sce_player_init(void);
int sce_player_event(sce_player_event_t event);
const char* sce_player_get_state_name(void);
int sce_player_get_deaths(void);
```

### Weapon State
```c
void sce_weapon_init(void);
int sce_weapon_event(sce_weapon_event_t event);
const char* sce_weapon_get_action_name(void);
const char* sce_weapon_get_weapon_name(void);
int sce_weapon_get_shots(void);
```

## Console Output

```
[SCE:Manager] Initializing DOOM State Machines
[SCE] Initialized - State: DEMOSCREEN
[SCE:Player] Initialized - State: ALIVE
[SCE:Weapon] Initialized - Action: READY, Weapon: PISTOL

[SCE] DEMOSCREEN --> LEVEL (transition #1)
[SCE:Weapon] READY --FIRE--> FIRING [PISTOL] (shots:1)
[SCE:Weapon] FIRING --FIRE_COMPLETE--> READY [PISTOL]
[SCE:Player] ALIVE --KILLED--> DEAD (deaths:1)
```

## License

- DOOM source code: GNU GPL v2
- SCE state machines: Same license as SCE project
