# Architecture: Static (AOT) + Dynamic (Interpreter) SCXML Engine

## Vision

**Goal**: W3C SCXML 1.0 100% compliance through intelligent code generation.

**Philosophy**: "You don't pay for what you don't use" — automatically choose optimal execution strategy (Pure AOT, Static Hybrid, or Interpreter) based on SCXML features.

**Hybrid Strategy**: Code generator analyzes SCXML and chooses execution approach per component:
- **Pure AOT (Static)**: When all features are compile-time known — generates optimized C++ code (8-100 bytes)
- **Static Hybrid**: Static state machine structure + runtime ECMAScript evaluation via JSEngine/LuaEngine
- **Interpreter (Dynamic)**: When core structure requires runtime resolution — uses proven Interpreter engine
- **Granular Decision**: Made per-component (parent vs child), not all-or-nothing for entire SCXML

---

## 4-Tier Library Architecture

The engine is structured as four layered libraries with strict dependency hierarchy. Each tier adds capabilities while maintaining backward compatibility. Consumers link only the tier they need.

```
sce_core          (INTERFACE, header-only)
   ↓
sce_base          (STATIC, compiled utilities)
   ↓
sce_scripting     (STATIC, optional — requires QuickJS or Lua)
   ↓
sce_runtime       (STATIC, full interpreter — umbrella target)
```

### Tier 1: sce_core (Header-Only)

**Purpose**: Zero-dependency core for pure static AOT consumers.

**Contents**:
- `StaticExecutionEngine` — CRTP-based AOT execution engine
- `core/` — W3C algorithm helpers (state entry/exit, event processing, conflict resolution, parallel orchestration)
- `common/` — Header-only validators and shared computation (AssignHelper, SendHelper, ForeachValidator, etc.)
- `core/LogMacros.h` — Conditional logging (no-ops when standalone)
- C++20 concepts and interfaces (`StatePolicyConcepts.h`, `EventQueueConcept.h`) — guarded by `__cpp_concepts`, degrades to `void_t` traits on C++17

**Link target**: Pure static AOT generated code with no scripting needs.

**Dependency**: C++ stdlib only.

### Tier 2: sce_base (Compiled Utilities)

**Purpose**: Engine-agnostic compiled utilities needed by AOT generated code.

**Contents**:
- `Logger` + backends (spdlog optional) — Runtime logging infrastructure
- `UniqueIdGenerator` — Thread-safe unique ID generation
- `ScriptResultUtils`, `EventDataHelper`, `GuardUtils` — Value manipulation
- `EcmaScriptToLuaTransformer` — ECMAScript-to-Lua syntax transformation
- `XMLDOMWrapper`, `SessionRegistry` — DOM and session utilities
- `TypeRegistry`, `JsonUtils` — Runtime type and JSON support

**Link target**: AOT consumers needing logging, ID generation, or value utilities.

**Dependency**: `sce_core` (transitive).

### Tier 3: sce_scripting (Script Engines, Optional)

**Purpose**: Script engine implementations and engine-dependent helpers.

**Contents**:
- `JSEngine` (QuickJS) — ECMAScript 2020 evaluation engine
- `LuaEngine` (Lua 5.4) — Lua evaluation engine with ECMAScript compatibility layer
- `ScriptEngineProvider` — Compile-time engine selection (`SCE_SCRIPT_ENGINE=lua|quickjs`)
- `DataModelInitHelper` — W3C SCXML 5.3 datamodel initialization
- `DOMBinding` / `LuaDOMBinding` — DOM access for script engines
- `PlatformExecutionHelper` — Platform abstraction (Native pthread vs WASM synchronous)

**Link target**: Static Hybrid AOT (JSEngine-embedded expressions) and Interpreter consumers.

**Dependency**: `sce_base` + `qjs` (QuickJS) and/or `lua54` (Lua 5.4).

**CMake options**:
- `SCE_ENABLE_QUICKJS` (default: ON)
- `SCE_ENABLE_LUA` (default: ON)
- `SCE_SCRIPT_ENGINE` (default: `lua`) — Selects default engine for `ScriptEngineProvider`

### Tier 4: sce_runtime (Full Interpreter)

**Purpose**: Complete SCXML interpreter with parser, state machine, actions, and events.

**Contents**:
- **Model**: `SCXMLModel`, `StateNode`, `TransitionNode`, `GuardNode`, `InvokeNode`
- **Runtime**: `StateMachine`, `ActionExecutorImpl`, `StateMachineBuilder`, `SessionManagerImpl`
- **Actions**: `ScriptAction`, `AssignAction`, `SendAction`, `IfAction`, `ForeachAction`, `CancelAction`
- **Events**: `EventSchedulerImpl`, `EventDispatcherImpl`, `EventTargetFactoryImpl`, HTTP infrastructure
- **States**: `ConcurrentStateNode`, `ParallelRegionOrchestrator`, `ConcurrentEventBroadcaster`
- **Parsing**: `SCXMLParser`, `StateNodeParser`, `TransitionParser`, `ActionParser`
- **History**: `HistoryManager`, `HistoryStateAutoRegistrar`, `HistoryValidator`

**Link target**: Interpreter consumers and test executables. Umbrella target — linking `sce_runtime` gives everything.

**Dependency**: `sce_scripting` (transitive — provides all lower tiers).

### Consumer Linkage Guide

| Use Case | Link Target | Gets You |
|----------|-------------|----------|
| Pure static AOT (no scripting) | `sce_base` | Headers + logging + utilities |
| Static Hybrid AOT (JSEngine expressions) | `sce_scripting` | + Script engines |
| Interpreter / Full runtime | `sce_runtime` | + Parser, StateMachine, everything |
| Header-only (embedded, minimal) | `sce_core` | Templates and concepts only |

### C++ Standard Compatibility

`sce_core` and `sce_base` target C++17 minimum for cross-compilation to constrained toolchains (e.g., QNX GCC 8.3). C++20 features are conditionally enabled at compile time.

| Tier | C++ Minimum | C++20 Behavior | Compatibility Mechanism |
|------|-------------|----------------|------------------------|
| `sce_core` | C++17 | Concepts enabled, zero-cost constraints | `__cpp_concepts >= 202002L` guard; falls back to `void_t` type traits |
| `sce_base` | C++17 | `std::source_location`, `std::format` | `SCE::source_location` shim (`SourceLocation.h`); `fmt::format` → plain string fallback (`LogMacros.h`) |
| `sce_scripting` | C++20 | Full C++20 features | QuickJS/Lua engines require modern standard library |
| `sce_runtime` | C++20 | Full C++20 features | Interpreter infrastructure requires C++20 |

**Key shims**:
- `SCE::source_location` — aliases `std::source_location` on C++20, provides stub on C++17
- `LogMacros.h` — `std::format` → `fmt::format` (spdlog bundled) → plain string fallback chain
- `SendHelper.h` — `SCE::detail::starts_with()` dual-mode helper (C++20 `std::string::starts_with` or manual)
- `StatePolicyConcepts.h` — `void_t` type traits always available; concepts aliased on C++20, `constexpr bool` on C++17

---

## Code Generator: sce-codegen (Rust + minijinja)

**Tool**: `sce-codegen` — Rust binary from `sce-build` crate (replaces legacy Python codegen).
**Build**: `cargo build --bin sce-codegen --features cli --release -p sce-build`

**Architecture**:
- **Parser**: `sce-build/src/lib.rs` — Parses SCXML files via roxmltree into intermediate model
- **Generator**: `sce-build/src/generator.rs` — Multi-language code generation engine
- **Filters**: `sce-build/src/filters.rs` — minijinja filters for all languages
- **Templates**: `tools/codegen/templates/`
  - `state_machine.jinja2` / `state_machine_inl.jinja2` — Main structure (header + inline implementation)
  - `actions/*.jinja2` — Individual action handlers (send, assign, if, foreach, cancel, etc.)
  - `entry_exit_actions.jinja2` — State entry/exit action generation
  - `invoke_methods.jinja2` — W3C SCXML 6.4 invoke lifecycle (execute pending, tick children, autoforward, finalize)
  - `process_transition.jinja2` — Transition processing logic
  - `conflict_resolution.jinja2` — W3C D.2 optimal transition set
  - `scriptengine_helpers.jinja2` — Script engine lazy initialization
  - `utility_methods.jinja2` — Helper methods (getEventName, etc.)
  - `rust/*.rs.jinja2` — Rust backend (1:1 port of C++ templates)
  - `kotlin/*.kt.jinja2` — Kotlin backend (sealed interfaces + coroutine-based)
  - `go/*.go.jinja2` — Go backend (generics + iota const patterns)
- **Flow**: SCXML → Parser → Model → Jinja2 Templates → C++/Rust/Kotlin/Go output

**Key Properties**:
- Always generates working C++ code — never refuses generation
- Automatic optimization: simple features → static, complex → dynamic
- Transparent hybrid: user doesn't choose, generator decides
- Template-based: easy to modify and extend

**ECMAScript Expression Handling** (Static Hybrid):
- Detects ECMAScript features (`typeof`, `_event`, `In()`) automatically
- Generates JSEngine/LuaEngine-embedded code for expression evaluation
- Maintains static state machine structure (enums, switch statements)
- Lazy JSEngine initialization (RAII pattern)

**Automatic Child→Parent Event Collection**:
- W3C SCXML 6.2: Scans child state machines for `<send target="#_parent" event="xxx"/>`
- Auto-adds events to parent Event enum for compile-time type safety
- Implementation: child event collection in `sce-build/src/lib.rs`

### Code Generation Strategy

```
SCXML File
    ↓
Feature Detection (sce-build parser)
    ↓
Generate Hybrid C++ Code (always succeeds)
    ↓
    ├─ Static Components (compile-time)
    │  - State transitions → enum-based switch
    │  - Guards/actions → inline C++ code
    │  - Datamodel (basic types) → member variables
    │  - If/elseif/else → C++ conditionals
    │  - Raise events → internal queue
    │  - Parallel states → inline regions
    │  - History states → std::optional<State> tracking
    │  Performance: Zero-overhead, 8-100 bytes
    │
    └─ Dynamic Components (runtime, lazy-init)
       - ECMAScript expressions → JSEngine/LuaEngine
       - Send with delay → SendSchedulingHelper::SimpleScheduler
       - Invoke (static child) → Generated child classes
       - Invoke (hybrid) → AOT parent + Interpreter child
       - HTTP sends → External HttpEventTarget
       Memory: Only allocated if SCXML uses these features
    ↓
Generated code works for ALL SCXML (W3C 100%)
```

### Policy Generation

Generated state machine policies come in two modes:

1. **Pure Static Policy** — Zero stateful features:
   - All methods are static or template-based
   - No member variables except simple datamodel vars
   - Memory: 8-100 bytes, zero overhead

2. **Stateful Policy** — Any stateful feature present (JSEngine, Invoke, Send delay, Event data):
   - All methods become non-static member functions
   - Policy has member variables: `sessionId_`, `jsEngineInitialized_`, `eventDataMap_`, etc.
   - Memory: Policy size + session data (~1-10KB)

---

## Feature Handling Strategy

### Static vs Dynamic Decision

**Principle**: The decision is based on **logical implementability at compile-time** (Closed World Assumption).

- **Static (Closed World)**: Feature operates on SCXML document content only → all information available at parse time
- **Dynamic (Open World)**: Feature requires external world communication (file I/O, network, runtime data) → needs Interpreter

### Decision Matrix

| SCXML Feature | Static | Hybrid | Interpreter | Reason |
|---------------|--------|--------|-------------|--------|
| `<cancel sendid="foo"/>` | Yes | Yes | Yes | Literal string |
| `<cancel sendidexpr="var"/>` | — | Yes | Yes | JSEngine evaluates at runtime |
| `<send delay="1s"/>` | Yes | Yes | Yes | Literal delay |
| `<send delayexpr="var"/>` | — | Yes | Yes | JSEngine evaluates at runtime |
| `<send target="http://..."/>` | Yes | Yes | Yes | Static URL (W3C C.2) |
| `<send targetexpr="var"/>` | — | Yes | Yes | JSEngine evaluates at runtime |
| `<transition cond="x > 5"/>` | — | Yes | Yes | ECMAScript expression |
| `<invoke src="child.scxml"/>` | Yes | Yes | Yes | Static child, compile-time known |
| `<invoke><content>...</content></invoke>` | Yes | Yes | Yes | Inline SCXML, compile-time known |
| `<invoke contentExpr="expr"/>` | — | Yes | Yes | Runtime content with JSEngine |
| `<invoke srcexpr="pathVar"/>` | — | — | Yes | Dynamic file I/O at runtime |
| `_event.origintype` | — | — | Yes | Runtime metadata |
| `In('state1')` | — | Yes | Yes | W3C predicate, JSEngine evaluation |

### Static Hybrid: ECMAScript Expression Handling

**Philosophy**: ECMAScript expressions evaluated at runtime via embedded script engine, while maintaining static state machine structure.

- **Static Structure**: States, events, transitions compiled to C++ enums and switch statements
- **Dynamic Expressions**: Conditionals, guards, assignments evaluated via JSEngine or LuaEngine
- **Lazy Initialization**: Script engine session created only when needed (RAII)
- **Zero Duplication**: Expression evaluation helpers shared between engines

**Detection**: Code generator automatically detects ECMAScript features:
- `typeof` operator, `_event` system variable, `In()` predicate → triggers hybrid generation

### Static History States

W3C SCXML 3.11 history states implemented as hybrid: static structure with runtime history variables.

- **Recording on Exit**: `std::optional<State>` captures active state when exiting compound state
- **Restoration on Transition**: Check recorded value or follow default `<transition>`
- **Zero Overhead When Unused**: Optional variables only allocated when recorded
- Types: shallow (direct children) and deep (nested descendants)

### Invoke Strategy

| Invoke Pattern | Approach | Memory |
|----------------|----------|--------|
| `<invoke src="child.scxml"/>` | Both parent and child AOT | ~300 bytes |
| `<invoke><content><scxml>...</scxml></content></invoke>` | Both parent and child AOT | ~300 bytes |
| `<invoke contentExpr="expr"/>` | AOT parent + Interpreter child | ~100KB |
| `<invoke srcexpr="pathVar"/>` | Interpreter only | ~200KB |

**Hybrid Invoke** (`contentExpr`):
- AOT parent evaluates expression via JSEngine/LuaEngine
- Creates Interpreter child at runtime via `StateMachine::createFromSCXMLString()`
- ~50% memory reduction vs all-Interpreter (~100KB vs ~200KB)
- `done.invoke` event routing via completion callback

### Invoke Template Architecture (Cross-Backend)

Invoke lifecycle methods are extracted into dedicated templates in C++, Rust, and Go, while Kotlin uses a different pattern leveraging runtime lambda closures:

| Aspect | C++ | Rust | Kotlin | Go |
|--------|-----|------|--------|-----|
| Template | `invoke_methods.jinja2` | `rust/invoke_methods.rs.jinja2` | Inline in `entry_exit_actions.kt.jinja2` |
| Dispatch | Template-generated state switch | Template-generated match arms | Runtime `deferInvoke()` lambda closures |
| Lifecycle methods | `executePendingInvokes()`, `tickChildren()`, `forwardToAutoforwardChildren()`, `executeFinalizeForChildEvent()` | `do_execute_pending_invokes()`, `do_tick_children()`, `do_forward_to_autoforward_children()`, `do_execute_finalize_for_child_event()` | `StateMachineEngine` base class (runtime) |

**Why Kotlin differs**: Kotlin's first-class functions allow the invoke lifecycle to be handled at runtime via `deferInvoke(state, id) { ... }` closures. The `StateMachineEngine<S, E>` base class provides `executePendingInvokes()`, `cancelInvoke()`, `startInvoke()` etc. as runtime methods. This is intentionally different from C++/Rust where template-generated state-switch dispatch is required for compile-time type safety.

---

## Scripting Engine Architecture

### Dual Engine Support

Both QuickJS (ECMAScript) and Lua 5.4 engines are supported at 100% W3C parity.

| Engine | Standard | Selection | Status |
|--------|----------|-----------|--------|
| QuickJS | ECMAScript 2020 | `SCE_SCRIPT_ENGINE=quickjs` | 202/202 (100%) |
| Lua 5.4 | Lua 5.4 + ECMAScript compat | `SCE_SCRIPT_ENGINE=lua` (default) | 202/202 (100%) |

**ScriptEngineProvider**: Compile-time engine selection via `SCE_SCRIPT_ENGINE` CMake option. Provides `IScriptEngine` interface for engine-agnostic consumers.

### EcmaScriptToLuaTransformer

Bridges ECMAScript syntax in W3C SCXML `datamodel="ecmascript"` to Lua evaluation:
- Operator translation: `===`→`==`, `!==`→`~=`, `!`→`not`, `&&`→`and`, `||`→`or`
- `typeof` operator, `null`/`undefined` handling, increment/decrement operators
- Object literals, array indexing (0-based JS → 1-based Lua), ternary operator
- Math builtins: `Math.sqrt`→`math.sqrt`, `Math.pow(a,b)`→`(a)^(b)`, `Math.PI`→`math.pi`
- For-in loops: `for (var k in obj) {...}` → `for k, _ in pairs(obj) do ... end`
- Three-layer expression cache + regex elimination for performance:
  - **Layer 1**: Transformer caches JS→Lua results
  - **Layer 2**: LuaSessionContext caches compiled Lua bytecode (registry refs)
  - **Layer 3**: Per-session direct expression→chunk ref mapping (skips Layer 1+2 on repeat)

### JSON Builtins (Single Source of Truth)

`sce/include/scripting/json_builtins.lua` — canonical `JSON.stringify()` / `JSON.parse()` implementation shared across all three backends:

| Backend | Embedding Mechanism | When |
|---------|-------------------|------|
| C++ | CMake `EmbedLuaScript.cmake` → `json_builtins_lua.h` string literal | Compile-time |
| Rust | `include_str!("../../sce/include/scripting/json_builtins.lua")` | Compile-time |
| Kotlin | Gradle `copyJsonBuiltins` → classpath resource `/scripting/json_builtins.lua` | Runtime (lazy) |
| Go | `//go:embed json_builtins.lua` in `sce-go-lua/` | Compile-time |

Do NOT duplicate JSON logic in backend-specific code. All five backends load the same file.

### LuaDOMBinding

Provides JavaScript-compatible DOM API over shared `XMLDOMWrapper`:
- 0-based indexing for JS compatibility (`childNodes[0]`, `item(0)`)
- `getElementsByTagName`, `getAttribute`, `childNodes`, `data` property
- Shared with QuickJS `DOMBinding` via common `XMLDOMWrapper`

---

## Zero Duplication Architecture

### Principle

All W3C SCXML logic shared between AOT and Interpreter engines through helper functions. Bug fixes automatically benefit both engines.

### Shared Helper Organization

Helpers distributed across `sce/include/core/` and `sce/include/common/`:

**`core/`** — W3C algorithm helpers (header-only templates, no external dependencies):

| Helper | W3C Section | Purpose |
|--------|-------------|---------|
| `EventQueueManager` | 3.12.1 | Internal event queue (FIFO) |
| `HierarchicalStateHelper` | 3.7, 3.8, 3.12 | LCA calculation, entry/exit chains |
| `ForeachHelper` | 4.6 | Loop variable declaration and type preservation |
| `InvokeHelper` | 6.4 | Invoke lifecycle (defer/cancel/execute pattern) |
| `TransitionHelper` | 3.13 | Transition selection and execution |
| `ConflictResolutionHelper` | D.2 | Optimal transition set selection |
| `ParallelStateHelper` | 3.4 | Parallel region orchestration |
| `HistoryHelper` | 3.11 | History state recording/restoration |
| `EntryExitHelper` | 3.7, 3.8 | State entry/exit action execution |
| `EventMatchingHelper` | 5.9.3 | Event descriptor prefix matching |
| `StateEntryHelper` | 3.3 | Compound state initial child resolution |

**`common/`** — Action/data primitive helpers:

| Category | Helpers | Dependency |
|----------|---------|------------|
| Pure validators | AssignHelper, ForeachValidator, DatamodelValidationHelper | stdlib only |
| Shared computation | StringUtils, SCXMLConstants, EventTypeHelper, InPredicateHelper, EventMetadataHelper, SendHelper, SendSchedulingHelper, NamelistHelper, LogicalTimeScheduler | stdlib + LogMacros |
| JSEngine-dependent | GuardHelper, DoneDataHelper, AssignmentExecutionHelper, FinalizeHelper, DataModelInitHelper | `scripting/IJSExecutionEngine` |
| Runtime infrastructure | UniqueIdGenerator, UrlEncodingHelper, EventDataHelper, FileLoadingHelper | compiled (.cpp in sce_base/sce_runtime) |
| Logging | Logger, ILoggerBackend, DisableStdOut | compiled (.cpp in sce_base) |

### Key Helper Patterns

**Error Handling Callback Pattern**: Helpers accept lambda callbacks for engine-specific error.execution raising:
```cpp
// Interpreter
helper.evaluate([this](const std::string& msg) { eventRaiser_->raiseEvent("error.execution", msg); });
// AOT
helper.evaluate([&engine](const std::string& msg) { engine.raise(Event::Error_execution); });
```

**Template + String Adapter Pattern**: Core helpers use templates for AOT (enum State) and string adapters for Interpreter:
```cpp
HierarchicalStateHelper<StatePolicy>       // AOT: compile-time type checking
HierarchicalStateHelperString              // Interpreter: string state IDs
```

**Deferred Error Handling** (W3C SCXML 5.3): `datamodelInitFailed_` flag in AOT for deferred error.execution raising, maintaining correct event priority.

---

## HTTP Infrastructure (W3C SCXML C.2)

BasicHTTP Event I/O Processor support for `<send type="BasicHTTPEventProcessor">`:

- `StaticExecutionEngine.raiseExternal()` detects HTTP target URLs
- `HttpEventTarget` performs real HTTP POST operations
- `HttpAotTest` base class: starts `W3CHttpTestServer` on localhost:8080/test
- Zero Duplication: Reuses Interpreter's `HttpEventTarget`, `W3CHttpTestServer`
- Hybrid Strategy: Pure AOT structure + external HTTP server (not engine mixing)

**CMake**: `SCE_ENABLE_HTTP` option (default: ON), requires cpp-httplib (native only, not WASM).

---

## Platform Support

### Native (Linux, macOS, Windows)
- `QueuedExecutionHelper`: Worker thread with operation queue for QuickJS thread safety
- pthread for `EventDispatcherImpl` and `ConcurrentEventBroadcaster`

### WASM (Emscripten)
- `SynchronousExecutionHelper`: Direct synchronous execution (no pthread for QuickJS)
- Emscripten Fetch API for HTTP requests (`EmscriptenFetchClient`)
- Configurable memory: `WASM_INITIAL_MEMORY`, fixed allocation (no growth)

**Factory**: `PlatformExecutionHelper::createPlatformExecutor()` selects at compile-time (`#ifdef __EMSCRIPTEN__`).

### Python Bindings (pybind11)

Python bindings wrap the C++ `ReadySCXMLEngine` interpreter via pybind11:

```
sce-python/
├── src/bindings.cpp          PyEngine wrapper (GIL management, context manager)
├── python/sce/__init__.py    Python package (Engine, Statistics exports)
├── tests/test_w3c.py         W3C conformance tests (202/202, HTTP included)
└── pyproject.toml            scikit-build-core wheel configuration
```

**Architecture**:
- **PyEngine**: RAII wrapper around `ReadySCXMLEngine` with `__enter__`/`__exit__` context manager
- **GIL Management**: `py::gil_scoped_release` for C++ multi-threaded operations (EventRaiser, EventScheduler)
- **Thread Safety**: Requires `SCE_THREAD_SAFE=ON` for external event queue (`send_external_event()`)
- **HTTP Support**: `W3CHttpTestServer` in Python for BasicHTTPEventProcessor tests (13 tests)
- **Error Propagation**: C++ parser/factory errors → Python `ValueError` with error chain

**API Surface**:
- `Engine.from_file(path)` / `Engine.from_string(scxml)` — Factory methods
- `start()` / `stop()` — Lifecycle management
- `send_event(name, data?)` — Internal event queue
- `send_external_event(name, data?)` — External event queue (W3C SCXML 5.10)
- `current_state` / `active_states` / `running` / `statistics` — Properties

**Build**: `cmake -DBUILD_PYTHON_BINDINGS=ON` (requires Python 3.9+, pybind11 v2.13.6 via FetchContent)

### Rust AOT Backend

Rust backend generates native Rust state machines from the same SCXML sources:

```
sce-rust-runtime/     Core engine (StaticExecutionEngine, event queue, policy traits)
sce-rust-lua/         Lua 5.4 script engine (mlua vendored build)
sce-rust-tests/       W3C conformance tests (202/202, linkme-based registration)
Cargo.toml            Workspace (Rust 1.75+, edition 2021)
```

**Architecture**:
- **Code Generator**: `sce-codegen generate -l rust` + `templates/rust/*.rs.jinja2`
- **Template Parity**: 1:1 port of C++ Jinja2 templates (state_machine, actions, invoke, etc.)
- **Scripting**: Lua 5.4 via `mlua` crate (vendored, same as C++ default engine)
- **JSON Builtins**: `include_str!("../../sce/include/scripting/json_builtins.lua")` — shared with C++/Kotlin
- **Test Registration**: `linkme` crate for compile-time test registration (equivalent to C++ `AotTestRegistrar`)

### Kotlin/JVM & Android

Kotlin/JVM modules provide the same W3C SCXML compliance (202/202) on JVM and Android:

```
sce-kotlin-runtime      ScxmlScriptEngine interface (Kotlin Multiplatform)
sce-kotlin-rhino        Rhino ECMAScript engine (pure JVM, fastest on server)
sce-kotlin-lua           Lua 5.4 engine via JNI (fastest on Android)
sce-kotlin-quickjs      QuickJS engine via JNI (full ES6, native)
sce-spring-boot-starter Spring Boot auto-configuration (@AutoConfiguration)
sce-kotlin-tests        W3C conformance (202/202, all 3 engines)
sce-kotlin-benchmark    JMH benchmarks (3-engine comparison)
sce-android-app         Android real-device benchmark (Compose UI)
```

**Engine selection per platform**:
- **JVM/Spring** (default: Rhino): Zero JNI overhead, JIT-optimized, pure Java
- **Android/AAOS** (default: Lua 5.4): Native C via JNI outperforms Rhino on ART (3-8x faster for guard evaluation)
- **C++** (default: Lua 5.4): `SCE_SCRIPT_ENGINE=lua` in CMake

**Kotlin code generator**: `sce-codegen generate -l kotlin` — generates sealed interface hierarchies + coroutine-based state machines from the same SCXML sources.

### Go Backend

Go backend generates native Go state machines with Go 1.22+ generics:

```
sce-go-runtime      StatePolicy[S,E] interface, Engine[S,E] generic execution engine
sce-go-lua          Lua script engine via Shopify/go-lua (pure Go, no CGo)
sce-go-tests        W3C conformance (202/202)
```

- **Templates**: `tools/codegen/templates/go/*.go.jinja2` — 1:1 port of Rust templates
- **Generics**: `Engine[S comparable, E comparable]` with `StatePolicy[S, E]` interface
- **State/Event**: `type State int` + `const ( StateXxx State = iota )` pattern
- **Scripting**: Lua via Shopify/go-lua (pure Go, no C compiler required)
- **JSON Builtins**: `//go:embed json_builtins.lua` — shared with C++/Rust/Kotlin
- **Test Generation**: `sce-codegen generate-w3c -l go` generates test files per W3C test

---

## Key Principles

1. **W3C SCXML Compliance is Non-Negotiable**: All 202 W3C tests must pass on all backends (C++, Rust, Kotlin, Go, Python)
2. **Always Generate Code**: Never refuse generation, always produce working implementation
3. **Automatic Optimization**: Code generator decides AOT vs Interpreter internally per component
4. **Lazy Initialization**: Pay only for features actually used in SCXML
5. **Zero Duplication**: AOT and Interpreter share core W3C logic through Helper functions
6. **You Don't Pay for What You Don't Use**: 4-tier library structure — link only what you need
7. **Multi-Backend Parity**: Same SCXML sources, same codegen pipeline, same W3C compliance across C++/Rust/Kotlin/Go/Python
8. **Parity Scope**: Multi-backend parity covers W3C SCXML codegen. Mesh (the SCE-specific distributed capability) is implemented in C++ alone — `tools/codegen/templates/mesh/` contains only `cpp/`, and no other backend carries `MeshEnvelope` or `mesh_transport` code. Per-backend mesh expansion is case-by-case, gated on explicit demand, not an implicit parity obligation. `SCE_MESH.md` stays agnostic to implementation count; the scope rule lives at the architecture layer.
