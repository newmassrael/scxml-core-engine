# DOOM WASM + SCE State Machines

DOOM compiled to WebAssembly with **7 real-time SCXML state machines** and browser-based visualization.

A showcase for the [SCE (SCXML Core Engine)](../../README.md) demonstrating W3C SCXML integration in a real game: flat FSMs, compound states (W3C 3.4), multi-instance state machines, and zero-overhead Named Context callbacks.

## State Machines

| State Machine | States | W3C Pattern | Purpose |
|---------------|--------|-------------|---------|
| **Game** | demoscreen, level, intermission, finale | Flat FSM | Game flow lifecycle |
| **Player** | alive, invulnerable, dead, reborn | Flat FSM | Player survival state |
| **Weapon** | ready, lowering, raising, firing | Flat FSM | Weapon action cycle |
| **Enemy** | dormant, alert, chasing, attacking, pain, dead | Multi-instance | Per-enemy AI tracking (max 64) |
| **Combo** | idle, active { normal, berserk } | Compound (3.4) | Kill combo + berserk mode |
| **Aim Assist** | disabled, enabled { idle, searching, locked } | Compound (3.4) | Per-shot target acquisition |
| **Secret Hint** | disabled, enabled { calculating, showing, found, no_path } | Compound (3.4) | BFS pathfinding to secrets/triggers |

## Features

- **Real-time SCXML visualization**: State diagrams highlighted live as you play
- **7 state machines**: From simple FSMs to compound hierarchical states
- **Kill combo + berserk mode**: 5-kill streak triggers berserk with damage multiplier
- **Aim assist**: Toggle-able auto-aim with searching/locked state tracking
- **Secret hint system**: BFS pathfinding to secrets, doors, lifts, switches, enemies
- **Mobile touch controls**: Virtual joystick, fire button, touch-to-look rotation
- **In-game HUD**: Combo counter and berserk overlay rendered to DOOM's framebuffer
- **Pausable timers**: Combo/berserk timers pause when menu opens
- **External timer architecture**: Game-managed timers (not SCXML internal scheduler)
- **Zero overhead**: AOT code generation with Named Context callbacks

## Architecture

### Layer Diagram

```
Browser (index.html)          JS Visualizer / SCXML Diagrams / Touch UI
        |                         window.onSce*() callbacks
        v
sce_sm_*.cpp (C++ modules)    Named Context callbacks + EM_ASM bridge
        |
        v
sce_doom_hooks.c (C bridge)   DOOM-facing API (SCE_Xxx functions)
        |
        v
DOOM Engine (doomgeneric)     Original C codebase (minimal patches)
        |
        v
SCE Generated (*_sm.h)        AOT code-generated state machines
```

### Modular Source Structure

```
src/
  sce_js_notify.h       Centralized JS notification layer (17 inline functions)
  sce_timer.h           PausableTimer class (pause/resume/isExpired)
  sce_sm_internal.h     Cross-module interface + SCE_LOG macro
  sce_sm_core.cpp       Game / Player / Weapon SMs (flat lifecycle)
  sce_sm_enemy.cpp      Enemy multi-instance SM (O(1) hash map lookup)
  sce_sm_combo.cpp      Combo / Berserk SM + PausableTimer
  sce_sm_aim.cpp        Aim Assist SM (compound states)
  sce_sm_secret.cpp     Secret Hint SM + BFS integration
  sce_wrapper.cpp       Init orchestrator (calls each module's init)
  sce_wrapper.h         Public C API (stable interface for DOOM code)
  sce_doom_hooks.c/h    DOOM C bridge (maps mobj_t to SCE events)
  sce_hud.c/h           In-game HUD (combo counter, berserk overlay)
  sce_secret_hint.c/h   BFS pathfinding through sector adjacency
scxml/
  game_state.scxml      Game flow definition
  player_state.scxml    Player lifecycle definition
  weapon_state.scxml    Weapon action definition
  enemy_state.scxml     Enemy AI definition
  combo_state.scxml     Kill combo + berserk definition
  aim_assist_state.scxml  Aim assist definition
  secret_hint_state.scxml Secret hint definition
```

### Direct Event Injection

Events are injected at exact points in DOOM's source code, not polled:

| DOOM Source | Function | SCE Event |
|-------------|----------|-----------|
| `p_pspr.c` | P_FireWeapon | weapon: fire |
| `p_pspr.c` | A_Lower / A_Raise | weapon: lower_complete / raise_complete |
| `p_inter.c` | P_KillMobj (player) | player: killed |
| `p_inter.c` | P_KillMobj (monster) | enemy: killed + combo: kill |
| `p_enemy.c` | A_Look / A_Chase | enemy: see_player / chase |
| `g_game.c` | G_DoNewGame | Reset all state machines |
| `g_game.c` | G_Ticker | sce_process_tic (timer check at 35Hz) |

### Named Context Pattern

SCXML `<cpp>` tags call C++ callbacks directly at compile time (zero vtable overhead):

```xml
<!-- In combo_state.scxml -->
<state id="berserk">
    <onentry>
        <script><cpp>berserk.onActive()</cpp></script>
    </onentry>
</state>
```

```cpp
// Generated C++ calls: this->berserk_->onActive()
// Resolved at compile time to: BerserkCallbacks::onActive()
```

## Prerequisites

- **Emscripten SDK** (3.x+)
  ```bash
  git clone https://github.com/emscripten-core/emsdk.git
  cd emsdk && ./emsdk install latest && ./emsdk activate latest
  source emsdk_env.sh
  ```
- **CMake** 3.16+
- **Python 3** (for code generation and index.html generation)

## Building

```bash
cd examples/doom_wasm
chmod +x build.sh
./build.sh
```

The build:
1. Generates C++ state machine headers from SCXML files via `sce_add_state_machines_from_dir()`
2. Compiles DOOM + SCE modules to WebAssembly
3. Downloads `doom1.wad` (shareware) if not present
4. Downloads FreePats soundfont for MIDI music
5. Generates `index.html` with embedded SCXML diagrams

### Full Game Support

Place `DOOMU.WAD` (Ultimate DOOM) in `doomgeneric/doomgeneric/` before building to unlock all episodes. Falls back to shareware (Episode 1 only) automatically.

## Running

```bash
cd build
python3 -m http.server 8080
# Open http://localhost:8080/
```

Or from the project root:
```bash
./start-server.sh
# Open http://localhost:8000/visualizer/doom/
```

## Debug Logging

All SCE state machine logging is compiled out by default. To enable verbose console output:

```cmake
target_compile_definitions(doom_sce PRIVATE SCE_DEBUG)
```

## Key Bindings

| Key | Action |
|-----|--------|
| Arrow keys / WASD | Move |
| Space | Use / Open doors |
| Ctrl | Fire |
| 1-7 | Select weapon |
| H | Toggle secret hint system |
| T | Toggle aim assist |
| Tab | Show automap |
| Esc | Menu |

## License

- DOOM source code (doomgeneric): GNU GPL v2
- SCE engine and state machines: LGPL-2.1 (generated code is MIT)
