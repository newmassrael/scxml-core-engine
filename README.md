# Reactive State Machine (RSM)

A high-performance C++ state machine library implementing W3C SCXML 1.0 specification with intelligent code generation.

[![W3C Tests](https://github.com/newmassrael/reactive-state-machine/actions/workflows/w3c-tests.yml/badge.svg)](https://github.com/newmassrael/reactive-state-machine/actions/workflows/w3c-tests.yml)

**Online Tools**:
- [Code Generator](https://newmassrael.github.io/reactive-state-machine/codegen/) - Generate C++ code from SCXML online
- [Test Results](https://newmassrael.github.io/reactive-state-machine/test-results/test-results.html) - W3C SCXML compliance report

---

## Key Features

### 🎯 Zero-Overhead Static Code Generation

Generate optimized C++ state machines from SCXML with minimal memory footprint - suitable for embedded systems.

- **Compile-time state machines**: State and event enums, no runtime overhead
- **No virtual functions**: Template-based design for complete inlining
- **Minimal memory**: 8 bytes for simple state machines
- **Your code stays intact**: Generated code calls your C++ functions without intrusion

### 🔄 Natural Integration with Your C++ Code

SCXML state logic cleanly separated from your business logic:

```xml
<!-- SCXML: State machine logic -->
<transition event="sensor_update"
            cond="hardware.isCritical()"
            target="emergency">
  <script>hardware.shutdownSystem()</script>
</transition>
```

```cpp
// Your C++ code: Business logic
class Hardware {
public:
    bool isCritical() { return sensor.readTemp() > 85; }
    void shutdownSystem() { emergencyStop(); }
};
```

The generated state machine calls your existing code - no framework lock-in, no base class requirements.

### 📐 W3C SCXML 1.0 Compliance

Full specification compliance with intelligent code generation strategy:

- **Static Engine**: Pure compile-time state machines (zero runtime overhead)
- **Static Hybrid Engine**: Static structure + ECMAScript expressions (minimal overhead)
- **Interpreter Engine**: Full runtime flexibility for dynamic features
- **Automatic selection**: Code generator chooses optimal strategy per SCXML file
- **Complete W3C compliance**: All test categories passing

---

## Quick Start

### Try Online (No Installation)

Experiment with SCXML code generation in your browser:
- [Online Code Generator](https://newmassrael.github.io/reactive-state-machine/codegen/)

### Installation

```bash
git clone --recursive https://github.com/newmassrael/reactive-state-machine.git
cd reactive-state-machine
mkdir build && cd build
cmake .. -DBUILD_TESTS=ON
cmake --build . -j$(nproc)
```

**Requirements**: CMake 3.14+, C++20 compiler

---

## Integration Methods

RSM provides flexible integration options for different project needs:

### Method 1: CMake Integration (Recommended)

**Best for**: Production projects using CMake

Automatic code generation with dependency tracking:

```cmake
# Add RSM to your project (git submodule or FetchContent)
add_subdirectory(external/reactive-state-machine)

# Create your executable
add_executable(my_app main.cpp)

# Auto-generate state machine code from SCXML
rsm_add_state_machine(
    TARGET my_app
    SCXML_FILE traffic_light.scxml
)

# Link RSM library
target_link_libraries(my_app PRIVATE rsm_unified)
```

**Benefits**:
- Automatic code regeneration when SCXML changes
- CMake dependency tracking built-in
- No manual build steps

See [examples/cmake_function](examples/cmake_function) for complete example.

### Method 2: FetchContent (Modern CMake)

**Best for**: Projects wanting automatic dependency management

```cmake
include(FetchContent)
FetchContent_Declare(
    rsm
    GIT_REPOSITORY https://github.com/newmassrael/reactive-state-machine.git
    GIT_TAG main
)
FetchContent_MakeAvailable(rsm)

add_executable(my_app main.cpp)
rsm_add_state_machine(TARGET my_app SCXML_FILE traffic_light.scxml)
target_link_libraries(my_app PRIVATE rsm_unified)
```

### Method 3: Standalone (Learning & Testing)

**Best for**: Quick experiments and understanding the workflow

Manual code generation:

```bash
python3 tools/codegen/codegen.py traffic_light.scxml -o ./generated/
```

See [Usage Example](#usage-example) below for complete standalone workflow.

---

## Usage Example

**Note**: This example shows the standalone workflow for learning. For production use, see [CMake Integration](#method-1-cmake-integration-recommended) above.

### 1. Write SCXML

```xml
<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       name="TrafficLight" initial="red">
  <state id="red">
    <transition event="timer" target="green"/>
  </state>

  <state id="green">
    <transition event="timer" target="yellow"/>
  </state>

  <state id="yellow">
    <transition event="timer" target="red"/>
  </state>
</scxml>
```

### 2. Generate C++ Code

```bash
python3 tools/codegen/codegen.py traffic_light.scxml -o ./generated/
```

**Note**: With CMake integration, this step is automatic - see [Method 1](#method-1-cmake-integration-recommended).

### 3. Use the State Machine

**Easy API** (Recommended for beginners):

```cpp
#include "traffic_light_sm.h"
#include "wrappers/AutoProcessStateMachine.h"

int main() {
    using namespace RSM::Generated::traffic_light;

    // Auto-processing wrapper
    RSM::Wrappers::AutoProcessStateMachine<traffic_light> light;

    light.initialize();
    light.processEvent(Event::Timer);  // Auto-processes event queue!

    return 0;
}
```

**Low-level API** (For advanced users needing fine control):

```cpp
#include "traffic_light_sm.h"

int main() {
    using namespace RSM::Generated::traffic_light;

    traffic_light light;

    light.initialize();
    light.raiseExternal(Event::Timer);  // Queue event
    light.step();                       // Process queue explicitly

    return 0;
}
```

**Compile and run:**
```bash
cd build
cmake .. -DBUILD_EXAMPLES=ON
cmake --build . --target traffic_light_example
env SPDLOG_LEVEL=off ./examples/traffic_light/traffic_light_example
```

**Output:**
```
Initial: Red
After timer: Green
After timer: Yellow
After timer: Red
```

---

## Architecture

### Design Goals

1. **Non-intrusive**: Your C++ code remains independent of the framework
2. **Zero-overhead**: No runtime cost for static state machines
3. **Embedded-friendly**: Minimal memory footprint, no dynamic allocation for simple cases
4. **Standard compliance**: Full W3C SCXML 1.0 support

### Project Structure

```
rsm/
├── include/
│   ├── common/          # Shared helpers (SendHelper, EventDataHelper)
│   ├── runtime/         # Interpreter engine
│   ├── scripting/       # QuickJS integration
│   └── static/          # AOT engine base
└── src/

tools/codegen/           # Code generator (Python + Jinja2)
├── codegen.py
├── scxml_parser.py
└── templates/           # C++ generation templates

tests/
├── w3c/                 # W3C conformance tests (202 tests)
└── engine/              # Engine tests
```

### Code Generator

**Input**: SCXML file
**Output**: Self-contained C++ header file

**Generation Strategy**:
- Analyze SCXML features (static vs dynamic)
- Choose optimal engine: Static, Static Hybrid, or Interpreter
- Generate minimal code with zero dependencies on framework internals
- Your functions are called via simple function pointers or template callbacks

**Engine Selection**:
- **Static**: All states, events, targets known at compile-time
- **Static Hybrid**: Static structure + ECMAScript expressions (e.g., `cond="x > 5"`)
- **Interpreter**: Dynamic features (e.g., `targetexpr`, `srcexpr`, runtime metadata)

---

## Advanced Features

### API Wrapper for Simplified Usage

RSM provides an optional convenience wrapper for automatic event queue processing:

```cpp
#include "wrappers/AutoProcessStateMachine.h"

// Easy wrapper
RSM::Wrappers::AutoProcessStateMachine<MyStateMachine> sm;
sm.processEvent(Event::Start);  // Automatically calls step()

// Low-level API still available for power users
sm.raiseExternal(Event::Stop);  // Queue only
sm.step();                      // Manual processing
```

**When to use**:
- **Easy API**: Most applications, simpler code, fewer mistakes
- **Low-level API**: Batch processing, asynchronous systems, precise control

### Parent-Child State Machines

```xml
<state id="parent">
  <invoke type="http://www.w3.org/TR/scxml/#SCXMLEventProcessor">
    <content>
      <!-- Inline child SCXML -->
      <scxml initial="child_state">
        <state id="child_state">
          <send event="done" target="#_parent"/>
        </state>
      </scxml>
    </content>
  </invoke>

  <transition event="done" target="next"/>
</state>
```

Both parent and child get static code generation - no runtime overhead.

### Event Data Passing

```cpp
// Send event with JSON data
sm.raiseExternal(Event::Update, R"({"temp": 25, "pressure": 1013})");
```

```xml
<!-- Access in SCXML -->
<transition event="update" cond="_event.data.temp &gt; 30">
  <log expr="_event.data.pressure"/>
</transition>
```

### ECMAScript Datamodel

Full ECMAScript support via QuickJS for complex expressions:

```xml
<datamodel>
  <data id="values" expr="[1, 2, 3, 4, 5]"/>
  <data id="sum" expr="values.reduce((a,b) => a+b, 0)"/>
</datamodel>
```

---

## Testing

### W3C SCXML Conformance

```bash
cd build
./tests/w3c_test_cli           # All tests
./tests/w3c_test_cli 336       # Single test
./tests/w3c_test_cli 144 147   # Multiple tests
```

**Test Strategy**:
- **Static tests**: Pure compile-time state machines (e.g., test144, test276)
- **Static Hybrid tests**: Static structure + ECMAScript (e.g., test509, test510, test513)
- **Interpreter tests**: Dynamic features requiring runtime (e.g., test187, test189)

**Compliance**:
- All W3C SCXML test categories passing ✅
- Full specification compliance across all engine types
- Automated test suite with CI/CD integration

---

## Documentation

- **[ARCHITECTURE.md](ARCHITECTURE.md)** - Detailed design and principles
- **[W3C SCXML Spec](https://www.w3.org/TR/scxml/)** - Official specification

---

## License

RSM uses a **Dual License** model:

### Generated Code: MIT (No Restrictions)

All code generated from your SCXML files is **MIT licensed** and owned by you.

```bash
python3 tools/codegen/codegen.py your_machine.scxml -o output/
# Generated: your_machine_sm.h (MIT License, unrestricted use)
```

See [LICENSE-GENERATED.md](LICENSE-GENERATED.md) for details.

### Runtime Engine: LGPL-2.1 or Commercial

**Option 1: LGPL-2.1 (FREE)**
- Use unmodified engine in any project (open source or proprietary)
- Modify engine and share modifications under LGPL-2.1

**Option 2: Commercial ($100 Individual / $500 Enterprise)**
- Modify engine and keep changes proprietary
- Create derivative products/SDKs
- Avoid LGPL compliance requirements

See [LICENSE](LICENSE) and [LICENSE-COMMERCIAL.md](LICENSE-COMMERCIAL.md) for details.

### Quick Decision

| Your Situation | License | Cost |
|---------------|---------|------|
| Use unmodified engine | LGPL-2.1 | FREE |
| Modify engine + share changes | LGPL-2.1 | FREE |
| Modify engine + keep private | Commercial | $100/$500 |
| Your generated code (always) | MIT | FREE |

**Contact:** newmassrael@gmail.com

---

## Contributing

Issues and pull requests welcome. Please ensure tests pass before submitting.

```bash
# Run tests
cd build
ctest --output-on-failure
```
