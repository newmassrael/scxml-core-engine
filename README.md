# SCE (SCXML Core Engine)

A high-performance W3C SCXML 1.0 implementation with C++ AOT code generator, Kotlin/JVM runtime, Rust AOT backend, Go AOT backend, and Python bindings. Supports embedded C++, Android (AAOS), Spring Boot server, Rust native, Go native, and Python applications.

[![W3C Tests](https://github.com/newmassrael/scxml-core-engine/actions/workflows/w3c-tests.yml/badge.svg)](https://github.com/newmassrael/scxml-core-engine/actions/workflows/w3c-tests.yml)

**Online Tools**:
- [Code Generator](https://newmassrael.github.io/scxml-core-engine/visualizer/codegen.html) - Full W3C SCXML code generation in browser (Pyodide)
- [Test Results](https://newmassrael.github.io/scxml-core-engine/test-results/test-results.html) - W3C SCXML compliance report

**Live Demo**:
- [DOOM + SCXML](https://newmassrael.github.io/scxml-core-engine/visualizer/doom/) - DOOM WebAssembly with real-time state machine visualization

---

## Key Features

### 🎯 Zero-Overhead Static Code Generation

Generate optimized C++ or Kotlin state machines from SCXML with minimal memory footprint - suitable for embedded systems, Android, and server applications.

- **Compile-time state machines**: State and event enums, no runtime overhead
- **No virtual functions**: Template-based design for complete inlining
- **Minimal memory**: 8 bytes for simple state machines
- **Your code stays intact**: Generated code calls your C++ functions without intrusion

### 🔄 Natural Integration with Your C++ Code

SCXML state logic cleanly separated from your business logic with direct C++ function calls:

```xml
<!-- SCXML: State machine logic with C++ integration -->
<state id="monitoring">
  <transition event="sensor_update"
              cond="cpp:hardware.isCritical()"
              target="emergency">
    <script><cpp>hardware.shutdownSystem()</cpp></script>
  </transition>
</state>
```

```cpp
// Your C++ code: Business logic
class Hardware {
public:
    bool isCritical() { return sensor.readTemp() > 85; }
    void shutdownSystem() { emergencyStop(); }
};

// UserContext: Dependency injection pattern
struct MyContext {
    Hardware hardware;
};

// Generated state machine uses your code
my_machine<MyContext> sm(context);
```

**Key Benefits**:
- Direct C++ function calls (zero overhead, no virtual functions)
- Type-safe dependency injection via UserContext template
- No framework lock-in, no base class requirements
- Your code remains completely independent

### 🐍 Python Bindings

Use SCE from Python with zero-copy pybind11 bindings wrapping the C++ interpreter:

```python
import sce

# Context manager auto-starts and stops the engine
with sce.Engine.from_file("traffic_light.scxml") as engine:
    engine.send_event("timer")
    print(engine.current_state)  # 'green'
```

**Key Features**:
- **W3C SCXML 1.0 compliant**: 202/202 W3C tests passing (including HTTP)
- **Context manager**: Automatic resource cleanup (`with` statement)
- **External events**: `send_external_event()` for HTTP/BasicHTTPEventProcessor
- **Thread-safe**: GIL properly managed for C++ multi-threaded operations
- **Build**: `cmake -DBUILD_PYTHON_BINDINGS=ON` with scikit-build-core for wheels

### 🦀 Rust AOT Backend

Native Rust state machines generated from the same SCXML sources:

```bash
# Generate Rust code from SCXML
sce-codegen generate traffic_light.scxml -o ./generated/ -l rust
```

**Key Features**:
- **W3C SCXML 1.0 compliant**: 202/202 W3C tests passing
- **Lua 5.4 scripting**: Via mlua (vendored build, same as C++ default)
- **Zero duplication**: json_builtins.lua shared with C++/Kotlin via `include_str!`
- **Cargo workspace**: `backends/rust/runtime`, `backends/rust/lua`, `backends/rust/tests`

### 🐹 Go AOT Backend

Native Go state machines with Go 1.22+ generics:

```bash
# Generate Go code from SCXML
sce-codegen generate --scxml traffic_light.scxml --language go --output-dir ./generated/
```

**Key Features**:
- **W3C SCXML 1.0 compliant**: 202/202 W3C tests passing
- **Generics**: `Engine[S comparable, E comparable]` with `StatePolicy[S, E]` interface
- **Pure Go Lua**: Shopify/go-lua (no CGo, no C compiler required)
- **Go modules**: `backends/go/runtime`, `backends/go/lua`, `backends/go/tests`

### ☕ Kotlin/JVM & Spring Boot Support

Use SCE from Java/Kotlin applications with a single dependency:

```gradle
// Spring Boot (auto-configured)
implementation("com.sce:sce-spring-boot-starter:1.0.0")
```

```java
@Autowired ScxmlScriptEngine engine;

engine.createSession("session1");
engine.evaluateCondition("session1", "x > 5");  // W3C SCXML ECMAScript
engine.destroySession("session1");
```

Three script engines available for Kotlin/JVM:
- **Rhino** (default) — Pure JVM, fastest on server, zero JNI
- **Lua 5.4** — Native JNI, fastest on Android (20/29 benchmarks won)
- **QuickJS** — Native JNI, full ES6 compatibility

### 📐 W3C SCXML 1.0 Compliance

Full specification compliance with intelligent AOT code generation:

- **Pure Static**: Compile-time state machines with zero runtime overhead
- **Static Hybrid**: Automatic ECMAScript support via JSEngine (no manual configuration)
- **Interpreter**: Only for runtime SCXML loading (workflow designers, visual editors)
- **Automatic selection**: Code generator detects features and chooses optimal engine
- **Complete W3C compliance**: All test categories passing
- **No Interpreter fallback needed**: AOT handles 99% of use cases automatically

---

## Quick Start

### Try Online (No Installation)

Experiment with SCXML code generation in your browser with full W3C SCXML support:
- [Online Code Generator](https://newmassrael.github.io/scxml-core-engine/visualizer/codegen.html) - Powered by Pyodide (Python in WebAssembly)

### Installation

```bash
git clone --recursive https://github.com/newmassrael/scxml-core-engine.git
cd scxml-core-engine
mkdir build && cd build
cmake .. -DBUILD_TESTS=ON
cmake --build . -j$(nproc)
```

**C++ Requirements**: CMake 3.14+, C++17 compiler (C++20 for full runtime), `libxml2-dev`
**Kotlin/JVM Requirements**: JDK 17+, Gradle 8.11+ (wrapper included)
**Rust Requirements**: Rust 1.75+, Cargo, `libxml2-dev` (build-time dependency of `sce-build` for XSD schema validation of Extended SCXML — see SCE_FORGE.md §7.1)
**Go Requirements**: Go 1.22+, `libxml2` runtime (loaded by the `sce-codegen` binary)
**Python Requirements**: Python 3.9+, pybind11

**Linux install** (Debian/Ubuntu):
```bash
sudo apt-get install -y libxml2-dev pkg-config
```
**macOS install**:
```bash
brew install libxml2 pkg-config
```

### Python Bindings

```bash
# Build Python bindings
mkdir build_python && cd build_python
cmake .. -DBUILD_PYTHON_BINDINGS=ON -DCMAKE_BUILD_TYPE=Release \
         -DBUILD_TESTS=OFF -DBUILD_EXAMPLES=OFF
cmake --build . --target _sce

# Use
PYTHONPATH=build_python/backends/python/bindings:backends/python/bindings/python python3 -c "
import sce
engine = sce.Engine.from_file('traffic_light.scxml')
engine.start()
"
```

### Rust

```bash
# Generate Rust code from SCXML
sce-codegen generate traffic_light.scxml -o ./generated/ -l rust

# Run W3C conformance tests
cargo test --release -p sce-rust-tests
```

### Go

```bash
# Generate Go code from SCXML
sce-codegen generate --scxml traffic_light.scxml --language go --output-dir ./generated/

# Run W3C conformance tests
cd backends/go/tests && go test ./...
```

### Kotlin/JVM & Spring Boot

```bash
# Build Kotlin modules
./gradlew build

# Publish to local Maven (~/.m2)
./gradlew publishToMavenLocal

# Use in your Spring Boot project:
# implementation("com.sce:sce-spring-boot-starter:1.0.0")
```

---

## Integration Methods

SCE provides flexible integration options for different project needs:

### Method 1: find_package (Recommended for Installed SCE)

**Best for**: Production projects with SCE installed system-wide

```cmake
# Find installed SCE package
find_package(SCE REQUIRED)

# Create your executable
add_executable(my_app main.cpp)

# Auto-generate state machine code from SCXML
sce_add_state_machine(
    TARGET my_app
    SCXML_FILE traffic_light.scxml
)

# Link the SCE tier that matches your SCXML feature set (ARCHITECTURE.md §4-Tier):
#   SCE::sce_base      — pure static (no scripting)
#   SCE::sce_scripting — static hybrid with JSEngine (QuickJS/Lua for <cond>/<expr>)
#   SCE::sce_runtime   — full interpreter + parser + HTTP
# Pick the lightest tier that compiles your SCXML (check `needs_script_engine`
# in sce-codegen JSON output).
target_link_libraries(my_app PRIVATE SCE::sce_scripting)
```

**Installation**:
```bash
cd scxml-core-engine/build
cmake .. -DCMAKE_INSTALL_PREFIX=/usr/local
make && sudo make install
```

See [examples/standalone_project](examples/standalone_project) for complete working example.

### Method 2: add_subdirectory (Development)

**Best for**: Projects including SCE as git submodule

```cmake
# Add SCE to your project (git submodule)
add_subdirectory(external/scxml-core-engine)

# Create your executable
add_executable(my_app main.cpp)

# Auto-generate state machine code from SCXML
sce_add_state_machine(
    TARGET my_app
    SCXML_FILE traffic_light.scxml
)

# Link SCE library
target_link_libraries(my_app PRIVATE sce_runtime)
```

**Benefits**:
- Automatic code regeneration when SCXML changes
- CMake dependency tracking built-in
- No manual build steps

See [examples/cmake_function](examples/cmake_function) for complete working example.

### Method 3: FetchContent (Modern CMake)

**Best for**: Projects wanting automatic dependency management

```cmake
include(FetchContent)
FetchContent_Declare(
    sce
    GIT_REPOSITORY https://github.com/newmassrael/scxml-core-engine.git
    GIT_TAG main
)
FetchContent_MakeAvailable(sce)

add_executable(my_app main.cpp)
sce_add_state_machine(TARGET my_app SCXML_FILE traffic_light.scxml)
target_link_libraries(my_app PRIVATE sce_runtime)
```

### Method 4: Standalone (Learning & Testing)

**Best for**: Quick experiments and understanding the workflow

Manual code generation:

```bash
sce-codegen generate traffic_light.scxml -o ./generated/ -l cpp
```

See [Usage Example](#usage-example) below for complete standalone workflow.

### CMake Functions Reference

SCE provides three CMake functions for state machine code generation:

| Function | Purpose |
|----------|---------|
| `sce_add_state_machine()` | Generate from single SCXML, add to target |
| `sce_add_state_machines_from_dir()` | Generate from all SCXML files in directory |
| `sce_create_state_machine_library()` | Create standalone library for sharing |

**sce_add_state_machine** - Add single SCXML to target:
```cmake
sce_add_state_machine(
    TARGET my_app              # Target to add generated code to
    SCXML_FILE player.scxml    # SCXML file path
    [OUTPUT_DIR dir]           # Optional output directory
)
```

**sce_add_state_machines_from_dir** - Batch generation from directory:
```cmake
sce_add_state_machines_from_dir(
    TARGET my_app              # Target to add generated code to
    SCXML_DIR ${CMAKE_SOURCE_DIR}/scxml  # Directory with SCXML files
    [OUTPUT_DIR dir]           # Optional output directory
)
```

**sce_create_state_machine_library** - Create reusable library:
```cmake
# Create standalone library
sce_create_state_machine_library(
    NAME player_sm             # Library name
    SCXML_FILE player.scxml    # SCXML file path
)

# Use in multiple targets. Pick the tier matching the generated SM's needs
# (ARCHITECTURE.md §4-Tier) — sce_base for pure static, sce_scripting for hybrid.
target_link_libraries(game PRIVATE player_sm SCE::sce_scripting)
target_link_libraries(editor PRIVATE player_sm SCE::sce_scripting)
```

---

## Examples

SCE provides seven examples with progressive complexity for learning the framework:

### Learning Path

**Recommended order**: Start simple, then integrate with your build system, add C++ functions, then explore timers and async patterns.

#### 0. [standalone_project](examples/standalone_project/) - find_package Usage

**What you'll learn**:
- Using SCE with `find_package(SCE)` in external projects
- Production-ready CMake integration pattern
- How to use SCE after system-wide installation

**Complexity**: Beginner (Standalone project template)

```cmake
find_package(SCE REQUIRED)
add_executable(my_app main.cpp)
sce_add_state_machine(TARGET my_app SCXML_FILE state.scxml)
# Pick the SCE tier for the generated SM (see ARCHITECTURE.md §4-Tier).
target_link_libraries(my_app PRIVATE SCE::sce_scripting)
```

#### 1. [traffic_light](examples/traffic_light/) - SCXML Basics

**What you'll learn**:
- Pure SCXML state machine definition
- State transitions and event handling
- Two API patterns (Easy vs Low-level)
- Manual build process

**Complexity**: Beginner (Pure SCXML, no C++ functions, no CMake integration)

```bash
# Build and run
cmake --build build --target traffic_light_example
./build/examples/traffic_light/traffic_light_example
```

#### 2. [cmake_function](examples/cmake_function/) - CMake Integration

**What you'll learn**:
- Automatic code generation with `sce_add_state_machine()`
- Build system integration and dependency tracking
- Zero-configuration CMake workflow

**Complexity**: Beginner-Intermediate (CMake integration, Pure SCXML)

```cmake
sce_add_state_machine(TARGET my_app SCXML_FILE my_machine.scxml)
```

#### 3. [smart_light](examples/smart_light/) - C++ Function Integration

**What you'll learn**:
- Direct C++ function calls from SCXML (`<cpp>` tags)
- UserContext dependency injection pattern
- Conditional transitions with C++ predicates (`cond="cpp:..."`)
- Hardware abstraction layer integration
- Production-ready patterns

**Complexity**: Intermediate (Full C++ integration, testable design)

```xml
<state id="off">
  <onentry>
    <script><cpp>hardware.powerOff()</cpp></script>
  </onentry>
  <transition event="switch_on" cond="cpp:hardware.hasPower()" target="on">
    <script><cpp>hardware.powerOn()</cpp></script>
  </transition>
</state>
```

#### 4. [timer](examples/timer/) - TimerManager API

**What you'll learn**:
- High-level timer API for periodic and one-shot timers
- Built on W3C SCXML 6.2 delayed send infrastructure
- Type-safe timer management via TimerID enum
- Timer lifecycle (start/stop/restart)
- Event scheduling and polling patterns

**Complexity**: Intermediate (Timer API, event scheduling)

```cpp
// Register timers with events
timers.registerTimer(TimerID::HEARTBEAT, Event::Heartbeat_tick);
timers.registerTimer(TimerID::TIMEOUT, Event::Operation_timeout);

// Start periodic timer (500ms)
timers.startTimer(TimerID::HEARTBEAT, 500ms, true);

// Start one-shot timer (3000ms)
timers.startTimer(TimerID::TIMEOUT, 3000ms, false);

// Process events with manual tick loop
while (!sm.isInFinalState()) {
    sm.tick();
    timers.processExpiredTimers();  // Re-schedule periodic timers
}
```

#### 5. [async_dispatcher](examples/async_dispatcher/) - Asynchronous Event Processing

**What you'll learn**:
- Thread-safe asynchronous event processing
- Event dispatcher pattern (std::thread backend)
- Multi-threaded event submission
- Background event loop integration
- Safe cross-thread state machine communication

**Complexity**: Advanced (Multi-threading, async patterns)

```cpp
#include "dispatchers/StdThreadDispatcher.h"
#include "wrappers/AsyncStateMachine.h"

// Create dispatcher
auto dispatcher = StdThreadDispatcher::create();
dispatcher->start();

// Wrap state machine for async processing
AsyncStateMachine<traffic_light, Event> sm(dispatcher);
sm.initialize();

// Start event loop in background thread
std::thread eventLoop([dispatcher]() {
    dispatcher->run();
});

// Post events from any thread (thread-safe)
sm.postEvent(Event::Timer);

// From another thread
std::thread worker([&sm]() {
    sm.postEvent(Event::Alert);  // Safe!
});

// Cleanup
dispatcher->stop();
eventLoop.join();
```

**Key Features**:
- Thread-safe event posting from multiple threads
- FIFO task queue with condition variable synchronization
- Zero overhead when not used (header-only templates)
- Extensible: Add Qt/GLib/FreeRTOS dispatchers via IEventDispatcher interface

#### 6. [traffic_light_auto](examples/traffic_light_auto/) - Timer Integration with Async Dispatcher

**What you'll learn**:
- Integration of timer management with async event dispatcher
- Automatic timer-driven state transitions
- One-shot and periodic timer patterns
- Timer callback execution on dispatcher thread
- Complete async timer workflow

**Complexity**: Advanced (Timers + Async Dispatcher)

```cpp
#include "dispatchers/StdThreadDispatcher.h"
#include "wrappers/AsyncStateMachine.h"

// Create dispatcher with timer support
auto dispatcher = StdThreadDispatcher::create();
dispatcher->start();

// Wrap state machine for async processing
AsyncStateMachine<traffic_light, Event> sm(dispatcher);
sm.initialize();

// Start event loop
std::thread eventLoop([dispatcher]() {
    dispatcher->run();
});

// Schedule timer to fire after 3 seconds
dispatcher->startTimer(
    1,      // Timer ID
    3000,   // Delay in milliseconds
    [&sm]() {
        std::cout << "Timer expired! Posting Timer event\n";
        sm.postEvent(Event::Timer);
    },
    false   // One-shot (not periodic)
);

// Wait for timer to fire
std::this_thread::sleep_for(std::chrono::milliseconds(3500));

// Cleanup
dispatcher->stop();
eventLoop.join();
```

**Key Features**:
- Timer callbacks execute on dispatcher thread (thread-safe with state machine)
- Supports both one-shot (`periodic=false`) and periodic (`periodic=true`) timers
- Timer management API: `startTimer()`, `stopTimer()`, `isTimerRunning()`
- Automatic timer cleanup on dispatcher shutdown
- Timer accuracy: ±30ms tolerance for typical delays

Each example includes a detailed README with building instructions and usage patterns.

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
sce-codegen generate traffic_light.scxml -o ./generated/ -l cpp
```

**Note**: With CMake integration, this step is automatic - see [Method 1](#method-1-cmake-integration-recommended).

### 3. Use the State Machine

**Easy API** (Recommended for beginners):

```cpp
#include "traffic_light_sm.h"
#include "wrappers/AutoProcessStateMachine.h"

int main() {
    using namespace SCE::Generated::traffic_light;

    // Auto-processing wrapper
    SCE::Wrappers::AutoProcessStateMachine<traffic_light> light;

    light.initialize();
    light.processEvent(Event::Timer);  // Auto-processes event queue!

    return 0;
}
```

**Low-level API** (For advanced users needing fine control):

```cpp
#include "traffic_light_sm.h"

int main() {
    using namespace SCE::Generated::traffic_light;

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
sce/
├── include/
│   ├── core/            # W3C algorithm helpers (header-only, zero dependencies)
│   ├── common/          # Shared helpers (SendHelper, EventDataHelper)
│   ├── scripting/       # Script engines (QuickJS, Lua 5.4)
│   ├── static/          # AOT engine (StaticExecutionEngine)
│   ├── runtime/         # Interpreter engine
│   └── ...              # model, actions, events, states, parsing
└── src/

# 4-Tier Library Architecture (link only what you need):
#   sce_core      → Header-only (pure static AOT, zero dependencies)
#   sce_base      → + Logger, utilities (AOT with logging)
#   sce_scripting → + QuickJS/Lua engines (static hybrid AOT)
#   sce_runtime   → + Parser, StateMachine (full interpreter)

sce-build/               # Code generator (Rust + minijinja)
├── src/bin/sce_codegen.rs  # CLI binary (generate, generate-w3c, fix-scxml-name)
├── src/lib.rs              # Library API (SCXML parser + model)
├── src/generator.rs        # Multi-language code generation engine
└── src/filters.rs          # minijinja filters for all languages
tools/codegen/templates/ # Shared Jinja2 templates (C++/Kotlin/Rust/Go)

# Python Bindings (pybind11):
backends/python/bindings/              # Python bindings module
├── src/bindings.cpp     # pybind11 wrapper (PyEngine)
├── python/sce/          # Python package (Engine, Statistics)
├── tests/test_w3c.py    # W3C conformance tests (202/202)
└── pyproject.toml       # scikit-build-core wheel config

# Rust Backend (Cargo workspace):
backends/rust/runtime/        # Core runtime (engine, event, policy, invoke)
backends/rust/lua/            # Lua 5.4 script engine (mlua)
backends/rust/tests/          # W3C conformance tests (202/202)

# Kotlin/JVM Modules (Gradle):
backends/kotlin/runtime/      # ScxmlScriptEngine interface (multiplatform)
backends/kotlin/rhino/        # Rhino ECMAScript engine (pure JVM)
backends/kotlin/lua/          # Lua 5.4 engine (JNI native)
backends/kotlin/quickjs/      # QuickJS engine (JNI native)
backends/kotlin/spring-boot-starter/ # Spring Boot auto-configuration
backends/kotlin/tests/        # W3C conformance tests (202/202)
backends/kotlin/benchmark/    # JMH performance benchmarks
backends/kotlin/android-app/         # Android benchmark app (Compose UI)

tests/
├── w3c/                 # W3C conformance tests (202 tests)
└── engine/              # Engine tests
```

### Code Generator

**Input**: SCXML file
**Output**: Self-contained C++ header, Kotlin class, or Rust module

**Generation Strategy**:
- Analyze SCXML features (static vs dynamic)
- Choose optimal engine: Static, Static Hybrid, or Interpreter
- Generate minimal code with zero dependencies on framework internals
- C++: Function pointers or template callbacks
- Kotlin: Generated state machine classes implementing StateMachineEngine interface
- Rust: Generated trait implementations with mlua Lua 5.4 scripting

### AOT Engine Architecture

SCE's AOT (Ahead-of-Time) code generator automatically selects the optimal engine:

```
SCXML Analysis → Feature Detection → Automatic Engine Selection

┌─────────────────────────────────────────────────────────────────┐
│  Pure Static         │  Static Hybrid        │  Interpreter     │
├─────────────────────────────────────────────────────────────────┤
│  No ECMAScript       │  ECMAScript present   │  Runtime SCXML   │
│  enum State/Event    │  enum + JSEngine      │  loading only    │
│  Zero runtime cost   │  Minimal overhead     │  Full flexibility│
└─────────────────────────────────────────────────────────────────┘
         ↑                      ↑                      ↑
    Automatic              Automatic              Manual only
```

**Key Point: AOT handles 99% of use cases automatically**

| SCXML Feature | Engine | Automatic? |
|---------------|--------|------------|
| Basic states/transitions | Pure Static | Yes |
| ECMAScript guards (`cond="x > 5"`) | Static Hybrid | Yes |
| ECMAScript actions (`<script>`) | Static Hybrid | Yes |
| `_event.data` access | Static Hybrid | Yes |
| `In()` function | Static Hybrid | Yes |
| Data model expressions | Static Hybrid | Yes |
| **Runtime SCXML upload** | **Interpreter** | **No** |

**When is Interpreter actually needed?**

Interpreter is **only** required when SCXML content is not known at build time:

```cpp
// This rare scenario requires Interpreter:
// User uploads SCXML at runtime (workflow designer, visual editor)
std::string userUploadedScxml = receiveFromClient();
interpreter.loadFromString(userUploadedScxml);  // Runtime parsing
```

**For normal development**: SCXML files are known at build time, so AOT (Pure Static or Static Hybrid) is always sufficient. The code generator automatically switches to Static Hybrid when ECMAScript features are detected.

**Engine Selection Summary**:
- **Pure Static**: All states, events, targets known at compile-time (maximum performance)
- **Static Hybrid**: Static structure + ECMAScript expressions (automatic when needed)
- **Interpreter**: Runtime SCXML loading only (rare, requires explicit use)

---

## Advanced Features

### API Wrapper for Simplified Usage

SCE provides an optional convenience wrapper for automatic event queue processing:

```cpp
#include "wrappers/AutoProcessStateMachine.h"

// Easy wrapper
SCE::Wrappers::AutoProcessStateMachine<MyStateMachine> sm;
sm.processEvent(Event::Start);  // Automatically calls step()

// Low-level API still available for power users
sm.raiseExternal(Event::Stop);  // Queue only
sm.step();                      // Manual processing
```

**When to use**:
- **Easy API**: Most applications, simpler code, fewer mistakes
- **Low-level API**: Batch processing, asynchronous systems, precise control

### Timer Management API

SCE provides a high-level timer API built on W3C SCXML 6.2 delayed send infrastructure:

```cpp
#include "wrappers/TimerManager.h"

// Define timer IDs for type-safe timer management
enum class TimerID : uint8_t {
    HEARTBEAT,
    TIMEOUT
};

// Extend generated SM with TimerID
struct MyStateMachineSM : public my_state_machine {
    using TimerID = ::TimerID;
    using my_state_machine::my_state_machine;
};

// Create timer manager
MyStateMachineSM sm;
SCE::Wrappers::TimerManager<MyStateMachineSM> timers(sm);

// Register timer-to-event mappings
timers.registerTimer(TimerID::HEARTBEAT, Event::Heartbeat_tick);
timers.registerTimer(TimerID::TIMEOUT, Event::Operation_timeout);

// Start timers
timers.startTimer(TimerID::HEARTBEAT, 500ms, true);   // Periodic
timers.startTimer(TimerID::TIMEOUT, 3000ms, false);   // One-shot

// Process with manual tick loop
while (!sm.isInFinalState()) {
    sm.tick();
    timers.processExpiredTimers();  // Re-schedule periodic timers
}

// Or use automatic runUntilCompletion
sm.runUntilCompletion(5000ms, 10ms);  // Timers handled automatically
```

**Key Features**:
- Type-safe timer identification via TimerID enum
- Periodic and one-shot timer support
- Built on existing EventScheduler infrastructure (zero duplication)
- Automatic event generation on timer expiration
- Timer lifecycle management (start/stop/restart)

**Thread Safety**: NOT thread-safe per W3C SCXML specification (single-threaded event processing). Use separate state machine instances per thread and EventRaiserRegistry for cross-thread communication.

See [timer example](examples/timer/) for complete usage patterns.

### C++ Function Integration

Direct C++ function calls from SCXML with type-safe dependency injection:

**SCXML Definition:**
```xml
<state id="monitoring">
  <onentry>
    <!-- Simple function call - no CDATA needed -->
    <script><cpp>hardware.initialize()</cpp></script>
  </onentry>

  <transition event="check" cond="cpp:hardware.hasPower()" target="active">
    <script><cpp>hardware.setBrightness(100)</cpp></script>
  </transition>

  <transition event="error" target="shutdown">
    <!-- CDATA for complex expressions with << operator -->
    <script><cpp><![CDATA[
      std::cout << "Error: " << hardware.getErrorCode() << "\n";
      hardware.emergencyShutdown();
    ]]></cpp></script>
  </transition>
</state>
```

**C++ Implementation:**
```cpp
// Your business logic
class Hardware {
public:
    bool hasPower() { return checkPowerSupply(); }
    void initialize() { setupHardware(); }
    void setBrightness(int level) { pwmControl(level); }
    int getErrorCode() { return lastError; }
    void emergencyShutdown() { safetyProtocol(); }
private:
    int lastError = 0;
};

// UserContext: Dependency injection container
struct MyContext {
    Hardware hardware;
};

// Generated state machine automatically uses UserContext
my_machine<MyContext> sm(context);
sm.initialize();
```

**Key Features:**
- **Zero-overhead**: Direct calls, no virtual functions
- **Type-safe**: UserContext template enforces correct types
- **Testable**: Inject mock objects for unit testing
- **CDATA support**: Use `<![CDATA[...]]>` for complex C++ expressions with operators (`<<`, `>>`, `<`, `>`)
- **Automatic transformation**: Generator adds `this->user_->` prefix automatically

**When to use CDATA:**
- C++ operators: `<<`, `>>`, `<`, `>`, `&&`, `||`
- Complex expressions with multiple statements
- String literals with special characters

**Most common case** (no CDATA needed):
- Simple function calls: `hardware.powerOn()`
- Method chains: `sensor.read().process()`
- Conditions: `hardware.temperature() > 50`

See [smart_light example](examples/smart_light/) for production-ready implementation.

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

Full ECMAScript support via Lua 5.4 (default), QuickJS, or Rhino for complex expressions:

```xml
<datamodel>
  <data id="values" expr="[1, 2, 3, 4, 5]"/>
  <data id="sum" expr="values.reduce((a,b) => a+b, 0)"/>
</datamodel>
```

---

## Thread Safety

SCE supports optional thread-safe event submission via the `SCE_THREAD_SAFE` build flag.

### When to Use Thread Safety

**Enable thread safety when**:
- Multiple threads need to send events to the state machine
- Sensor threads submit events while main thread processes them
- Asynchronous I/O callbacks trigger state machine events

**Default (disabled) is appropriate when**:
- Single-threaded event processing
- Maximum performance required (zero overhead)
- Event submission happens only from processing thread

### Building with Thread Safety

```bash
# Enable thread-safe event queues
cmake -DSCE_THREAD_SAFE=ON -B build
cmake --build build
```

**Performance Impact**:
- **Disabled (default)**: Zero overhead, no mutex operations
- **Enabled**: Minimal overhead, mutex only during event queue operations

### Usage Example

```cpp
#include "traffic_light_sm.h"
#include <thread>

traffic_light sm;
sm.initialize();

// Sensor thread: Submit events safely
std::thread sensorThread([&sm]() {
    while (running) {
        if (buttonPressed()) {
            sm.raiseExternal(Event::Pedestrian_button);  // Thread-safe ✅
        }
        std::this_thread::sleep_for(100ms);
    }
});

// Timer thread: Submit events safely
std::thread timerThread([&sm]() {
    while (running) {
        std::this_thread::sleep_for(30s);
        sm.raiseExternal(Event::Timer);  // Thread-safe ✅
    }
});

// Main thread: Process events
while (!sm.isInFinalState()) {
    sm.tick();   // Process scheduler
    sm.step();   // Process event queues
    std::this_thread::sleep_for(10ms);
}
```

### Architecture

Thread safety is implemented at the EventQueueManager level:
- **Zero Duplication**: Single implementation shared across all engines (Static AOT, Interpreter)
- **Conditional Compilation**: No overhead when disabled via preprocessor directives
- **W3C Compliance**: Sequential event processing preserved (mutex ensures atomicity)

Thread-safe operations:
- `raiseExternal()` - Queue external events
- `raise()` - Queue internal events (from within actions)
- `tick()` - Process scheduler
- `step()` - Process event queues
- All queue operations (`hasEvents()`, `size()`, `pop()`, `clear()`)

**Note**: Each state machine instance processes events sequentially (W3C SCXML requirement). Thread safety enables safe event *submission* from multiple threads, not concurrent event *processing*.

---

## Event Dispatchers

SCE provides event dispatcher implementations for asynchronous state machine event processing across different platforms.

### Available Dispatchers

| Dispatcher | Platform | Event Loop | Timer Source |
|------------|----------|------------|--------------|
| StdThreadDispatcher | Any (C++11) | std::thread | std::condition_variable |
| QtDispatcher | Qt5/Qt6 | QCoreApplication | QTimer |
| GLibDispatcher | GTK/GNOME | GMainLoop | g_timeout_source |

### StdThreadDispatcher (Default)

Built-in dispatcher using C++ standard library. No additional dependencies.

```cpp
#include "dispatchers/StdThreadDispatcher.h"
#include "wrappers/AsyncStateMachine.h"
#include "traffic_light_sm.h"

using namespace SCE::Dispatchers;
using namespace SCE::Wrappers;

auto dispatcher = StdThreadDispatcher::create();
AsyncStateMachine<traffic_light, Event> sm(dispatcher);

sm.initialize();
dispatcher->start();

// Run event loop in separate thread
std::thread eventLoop([dispatcher]() { dispatcher->run(); });

// Post events from any thread
sm.postEvent(Event::Timer);

// Cleanup
dispatcher->stop();
eventLoop.join();
```

### QtDispatcher (Optional)

Integration with Qt's event loop. Requires Qt5 or Qt6 Core module.

```bash
# Build with Qt dispatcher
cmake -DSCE_DISPATCHER_QT=ON -B build
cmake --build build
```

```cpp
#include "dispatchers/QtDispatcher.h"
#include <QCoreApplication>

int main(int argc, char *argv[]) {
    QCoreApplication app(argc, argv);
    
    auto dispatcher = QtDispatcher::create();
    dispatcher->start();
    
    // Events are processed by Qt's event loop
    dispatcher->enqueue([]() {
        qDebug() << "Task executed on Qt event loop";
    });
    
    return app.exec();  // Or use dispatcher->run()
}
```

### GLibDispatcher (Optional)

Integration with GLib main loop. Requires glib-2.0.

```bash
# Build with GLib dispatcher
cmake -DSCE_DISPATCHER_GLIB=ON -B build
cmake --build build
```

```cpp
#include "dispatchers/GLibDispatcher.h"

int main() {
    auto dispatcher = GLibDispatcher::create();
    dispatcher->start();
    
    dispatcher->enqueue([]() {
        g_print("Task executed on GLib main loop\n");
    });
    
    dispatcher->run();  // Runs GMainLoop
    return 0;
}
```

### Timer Support

All dispatchers support high-precision timers with drift prevention:

```cpp
// One-shot timer (100ms delay)
dispatcher->startTimer(1, 100, []() {
    std::cout << "Timer fired!\n";
}, false);

// Periodic timer (50ms interval)
dispatcher->startTimer(2, 50, []() {
    std::cout << "Periodic tick\n";
}, true);

// Check timer status
if (dispatcher->isTimerRunning(2)) {
    dispatcher->stopTimer(2);
}
```

### Architecture

The dispatcher architecture follows the **Zero Duplication** principle:

```
IEventDispatcher (interface)
       ↓
EventDispatcherBase (shared logic: timers, task queue)
       ↓
┌──────────────────┬─────────────────┬────────────────┐
│ StdThreadDispatcher │ QtDispatcher    │ GLibDispatcher │
│ (std::thread)       │ (QTimer/QEvent) │ (pipe/GSource) │
└──────────────────┴─────────────────┴────────────────┘
```

- **EventDispatcherBase**: Common timer metadata, task queue, and state management
- **Platform-specific**: Native event loop integration and timer mechanisms

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

### Python Binding Tests

```bash
# Run all 202 W3C tests (including HTTP)
SPDLOG_LEVEL=off PYTHONPATH=build_python/backends/python/bindings:backends/python/bindings/python \
    python3 backends/python/bindings/tests/test_w3c.py

# Skip HTTP tests (faster)
SPDLOG_LEVEL=off PYTHONPATH=build_python/backends/python/bindings:backends/python/bindings/python \
    python3 backends/python/bindings/tests/test_w3c.py --skip-http
```

### Rust Tests

```bash
# Run W3C conformance tests (202/202)
cargo test --release -p sce-rust-tests

# Run specific test
cargo test --release -p sce-rust-tests test144
```

### Kotlin/JVM Tests

```bash
# W3C conformance (Rhino, default)
./gradlew :sce-kotlin-tests:test

# W3C conformance (Lua engine)
./gradlew :sce-kotlin-tests:test -Psce.script.engine=lua

# JMH benchmark (quick)
./gradlew :sce-kotlin-benchmark:jmh --no-configuration-cache -Pjmh.f=1 -Pjmh.wi=1 -Pjmh.i=1
```

### Performance Benchmarks

```bash
cd build

# Run quick benchmark tests (part of ctest)
ctest -R benchmark.*quick

# Run individual benchmarks
./tests/benchmarks/benchmark_async_dispatcher
./tests/benchmarks/benchmark_timer_accuracy
./tests/benchmarks/benchmark_memory_footprint
./tests/benchmarks/benchmark_event_scheduler
./tests/benchmarks/benchmark_jsengine
./tests/benchmarks/benchmark_statemachine
```

**Available Benchmarks**:
- **Async Dispatcher**: Event throughput, enqueue latency, concurrent posting, multi-threaded performance
- **Timer Accuracy**: One-shot/periodic drift measurement, callback latency, concurrent timer operations
- **Memory Footprint**: State machine size, dispatcher overhead, queue/registry memory usage, scalability
- **Event Scheduler**: Delayed event scheduling, cancellation, priority handling
- **JSEngine**: Expression evaluation, session management, variable operations
- **State Machine**: Transition performance, parallel states, event processing

**Benchmark Results** (typical values):
- Async enqueue latency: ~90ns (11M events/sec throughput)
- Timer drift: <1ms for 100ms intervals
- Memory: 4-byte minimal state machine, 16-byte async wrapper overhead
- Event processing: <1μs per transition

---

## Documentation

- **[ARCHITECTURE.md](ARCHITECTURE.md)** - Detailed design and principles
- **[W3C SCXML Spec](https://www.w3.org/TR/scxml/)** - Official specification

---

## License

SCE uses a **Dual License** model:

### Generated Code: MIT (No Restrictions)

All code generated from your SCXML files is **MIT licensed** and owned by you.

```bash
sce-codegen generate your_machine.scxml -o output/ -l cpp
# Generated: your_machine_sm.h (MIT License, unrestricted use)
```

See [LICENSE-GENERATED.md](LICENSE-GENERATED.md) for details.

### Runtime Engine: LGPL-2.1 or Commercial

**Option 1: LGPL-2.1 + Static Linking Exception (FREE)**
- Use unmodified engine in any project (open source or proprietary)
- Static or dynamic linking — no obligation to disclose your code
- Modify engine and share modifications under LGPL-2.1

**Option 2: Commercial ($5000 Individual / Enterprise: contact for pricing)**
- Modify engine and keep changes proprietary
- Create derivative products/SDKs
- Avoid LGPL compliance requirements

See [LICENSE](LICENSE) and [LICENSE-COMMERCIAL.md](LICENSE-COMMERCIAL.md) for details.

### Quick Decision

| Your Situation | License | Cost |
|---------------|---------|------|
| Use unmodified engine | LGPL-2.1 + Exception | FREE |
| Modify engine + share changes | LGPL-2.1 | FREE |
| Modify engine + keep private | Commercial | $5000 / Contact |
| Your generated code (always) | MIT | FREE |

**Contact:** newmassrael@gmail.com
