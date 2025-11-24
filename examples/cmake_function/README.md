# SCE CMake Integration Example

This example demonstrates how to integrate SCE into your CMake project using the `sce_add_state_machine()` function for automatic code generation.

## Overview

The `sce_add_state_machine()` function seamlessly integrates SCXML code generation into your CMake build process:
- **Automatic Generation**: C++ code generated from SCXML files during build
- **Dependency Tracking**: Regenerates code when SCXML files change
- **Zero Configuration**: Automatically adds generated files to your target
- **Clean Organization**: Generated code placed in dedicated output directory

## Usage

```cmake
# Create your executable
add_executable(my_app main.cpp)

# Generate state machine code from SCXML
sce_add_state_machine(
    TARGET my_app
    SCXML_FILE simple_light.scxml
    # OUTPUT_DIR is optional, defaults to ${CMAKE_CURRENT_BINARY_DIR}/generated
)

# Link with SCE library
target_link_libraries(my_app PRIVATE sce_unified)
```

## Function Parameters

- `TARGET` (required): The CMake target to add the generated code to
- `SCXML_FILE` (required): Path to the SCXML file (relative or absolute)
- `OUTPUT_DIR` (optional): Directory for generated files (defaults to `${CMAKE_CURRENT_BINARY_DIR}/generated`)

## Generated Output

For an SCXML file named `simple_light.scxml` with `name="SimpleLight"`:
- Generated file: `simple_light_sm.h` (based on SCXML filename)
- Location: `${OUTPUT_DIR}/simple_light_sm.h`
- Namespace: `SCE::Generated::simple_light` (based on SCXML filename)

## Implementation Pattern

The generated code provides a ready-to-use state machine class with two usage patterns:

```cpp
#include "simple_light_sm.h"
#include "wrappers/AutoProcessStateMachine.h"

using namespace SCE::Generated::simple_light;

int main() {
    // Option 1: Easy API - Auto-processing wrapper (recommended for beginners)
    SCE::Wrappers::AutoProcessStateMachine<simple_light> light;
    light.initialize();
    light.processEvent(Event::Switch_on);  // Automatically processes event queue

    // Option 2: Low-level API - Manual control (for advanced users)
    simple_light lightManual;
    lightManual.initialize();
    lightManual.raiseExternal(Event::Switch_on);
    lightManual.step();  // Explicit queue processing for fine-grained control

    return 0;
}
```

**Key Point**: This example uses Pure SCXML (no C++ function integration) to focus on CMake integration. For C++ function integration examples, see [smart_light](../smart_light/).

## Building This Example

From the project root:

```bash
mkdir -p build && cd build
cmake ..  # BUILD_EXAMPLES is ON by default
cmake --build . --target light_example
./examples/cmake_function/light_example
```

## Benefits

1. **Automatic Generation**: Code regenerates when SCXML changes
2. **Dependency Tracking**: CMake knows when to rebuild
3. **Zero Overhead**: Template-based design, no virtual functions
4. **Type Safety**: Compile-time checks for state machine logic
5. **Clean Integration**: No manual build steps required

## Next Steps

This example focuses on **CMake integration** with Pure SCXML. For advanced features:

- **C++ Function Integration**: See [smart_light](../smart_light/) example for:
  - Direct C++ function calls from SCXML
  - UserContext dependency injection pattern
  - Conditional transitions with C++ predicates
  - Hardware abstraction layer integration

- **Basic Usage**: See [traffic_light](../traffic_light/) example for:
  - Manual build process (no CMake function)
  - Simple state machine patterns
  - API usage comparison

## W3C SCXML Support

The code generator supports W3C SCXML 1.0 features with zero-overhead principles:

- **Atomic and Compound States**: Flat and hierarchical state structures
- **Parallel States**: Multiple active states simultaneously
- **History States**: State restoration (shallow and deep)
- **Final States**: Automatic done event generation
- **Transitions**: Event-based, eventless, internal, and guarded transitions
- **ECMAScript Support**: Dynamic expressions via JSEngine (Static Hybrid approach)
- **Actions**: Entry/exit handlers, transition actions, assign, if, foreach, etc.

See [ARCHITECTURE.md](../../ARCHITECTURE.md) for detailed design philosophy and W3C test compliance status
