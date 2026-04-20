# SCE Standalone Project Example

This example demonstrates how to use SCE (SCXML Core Engine) in a standalone project
using `find_package(SCE)`.

## Quick Start

### Option 1: Install SCE System-wide

```bash
# Build and install SCE
cd /path/to/scxml-core-engine
mkdir build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
make -j$(nproc)
sudo make install

# Build this example
cd examples/standalone_project
mkdir build && cd build
cmake ..
make
./traffic_light
```

### Option 2: Use SCE from Build Directory

```bash
# Build SCE (don't install)
cd /path/to/scxml-core-engine
mkdir build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
make -j$(nproc)

# Build this example, pointing to SCE build
cd ../examples/standalone_project
mkdir build && cd build
cmake .. -DCMAKE_PREFIX_PATH=/path/to/scxml-core-engine/build
make
./traffic_light
```

Or use SCE_ROOT for explicit source directory:
```bash
cmake .. -DSCE_ROOT=/path/to/scxml-core-engine
```

## Project Structure

```
standalone_project/
├── CMakeLists.txt       # CMake configuration
├── main.cpp             # Application code
├── traffic_light.scxml  # State machine definition
└── README.md            # This file
```

## CMake Usage

```cmake
# Find SCE package
find_package(SCE REQUIRED)

# Create your executable
add_executable(my_app main.cpp)

# Generate state machine from SCXML
sce_add_state_machine(
    TARGET my_app
    SCXML_FILE my_state_machine.scxml
)

# Link the tier that matches your SCXML feature set (ARCHITECTURE.md §4-Tier)
#   SCE::sce_base       — pure static AOT (no <cond>/<expr>/<assign location>)
#   SCE::sce_scripting  — static hybrid AOT with JSEngine  ← this example
#   SCE::sce_runtime    — full interpreter + parser + HTTP
target_link_libraries(my_app PRIVATE SCE::sce_scripting)
```

> **Checking which tier your SCXML needs**: run `sce-codegen generate … -l cpp`
> and inspect the `needs_script_engine` field in the JSON output —
> `false` → `sce_base`, `true` → `sce_scripting`.

## Available CMake Functions

### sce_add_state_machine()

Generates C++ state machine from SCXML and adds to target.

```cmake
sce_add_state_machine(
    TARGET my_app           # Target to add generated code to
    SCXML_FILE state.scxml  # SCXML file path
    [OUTPUT_DIR dir]        # Optional output directory
)
```

### sce_add_state_machines_from_dir()

Generates state machines from all SCXML files in a directory.

```cmake
sce_add_state_machines_from_dir(
    TARGET my_app           # Target to add generated code to
    SCXML_DIR scxml/        # Directory containing SCXML files
    [OUTPUT_DIR dir]        # Optional output directory
)
```

### sce_create_state_machine_library()

Creates a standalone library from SCXML for sharing between targets.

```cmake
sce_create_state_machine_library(
    NAME player_sm          # Library name
    SCXML_FILE player.scxml # SCXML file path
)

target_link_libraries(my_app PRIVATE player_sm SCE::sce_scripting)
```

## Generated Code

The `sce_add_state_machine()` function generates a header file with:

- State machine class with enum-based states and events
- `initialize()` - Enter initial state
- `raiseEvent()` - Queue an event
- `tick()` - Process queued events
- `getCurrentState()` - Get current state enum
- `getCurrentStateString()` - Get current state name

## License

Same as SCE (SCXML Core Engine) - See LICENSE file.
