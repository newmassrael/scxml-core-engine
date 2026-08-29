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

## Scope & Composition

SCE is the trust boundary in the NL→SCXML→code pipeline. It consumes
SCXML and produces validated, typed, target-language source code. The
`sce:*` namespace is bounded by measured benefit — each primitive earns
its place through demonstrated use and layer fit.

**SCE owns:**
- W3C SCXML conformance — parser, IR, runtime semantics
- W3C XInclude — byte-identical fragment composition at parse time
- `sce:*` extensions:
  - **Runtime/semantic**: mesh, context, import, field
  - **Composition**: `sce:template` / `sce:use` / `sce:param` for
    parameterised XML expansion (RFC at
    `claudedocs/rfc-sce-template-sce-param.md`)
- Forge typed expression pipeline — data fields, inline-eligibility
- Cross-language byte-equivalence across N codegen backends

**Why SCE owns templating** (rather than delegating to producer-side
preprocessors):
- *Native source-mapping* — template diagnostics point at author intent
  (template file row/col), not at expanded bytes. External preprocessors
  require a sidecar convention that the ecosystem has not converged on.
- *Forge-typed parameters* — typed `<sce:param>` integrates with Forge
  inline-eligibility, allowing template instances to feed const-fold
  analysis directly. External expansion loses the template-instance
  semantic link.
- *Single toolchain UX* — `sce-build` performs expansion + parse + type
  + codegen + diagnostic in one stream. No external dependency for
  consumers to wire into their build pipelines.
- *Bounded marginal cost* — ~1000 LOC + 7 diagnostic codes, modelled on
  the existing XInclude pattern (sce-build expander + optional C++
  runtime parity per RFC §6.5).

**Producer-side preprocessing (still valid)** — producers MAY emit
canonical SCXML before SCE consumes it (LLM prompt layers, DSL→SCXML
compilers, Jinja2/m4 preprocessors). SCE's in-tree composition is
recommended when the source is human-authored SCXML; producer-side is
natural when SCXML is one of several formats the producer generates.

**Charter discipline:** further `sce:*` primitives must demonstrate
(a) a use case not covered by existing primitives, (b) layer fit (Stage 0
composition / Stage 2 typing / Stage 4 runtime), and (c) cross-language
portability. Turing-complete templating, conditional inclusion, computed
attributes, and parameter entities are NOT accepted by default —
`sce:template` is intentionally a *minimal* lexical substitution
primitive (see RFC §2 non-goals).

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
- `DataModelReadHelper` — W3C SCXML 5.3 typed reads back out of the session. A
  `<data>` with an initializer is owned by the script engine for the life of
  the session, so a generated machine reads it here rather than shadowing it in
  a member that `<assign>` would leave stale. Rust `helpers::datamodel_read`,
  Go `ReadDatamodel*`, Kotlin `DatamodelRead` and Python `datamodel_read` are
  the same three coercions, so every backend's accessor answers alike.
- `DOMBinding` / `LuaDOMBinding` — DOM access for script engines
- `PlatformExecutionHelper` — Platform abstraction (Native pthread vs WASM synchronous)

**Link target**: Static Hybrid AOT (JSEngine-embedded expressions) and Interpreter consumers.

**Dependency**: `sce_base` + `qjs` (QuickJS) and/or `lua54` (Lua 5.4).

**CMake options**:
- `SCE_ENABLE_QUICKJS` (default: ON)
- `SCE_ENABLE_LUA` (default: ON)
- `SCE_SCRIPT_ENGINE` (default: `quickjs`) — Selects default engine for `ScriptEngineProvider`

### Tier 4: sce_runtime (Full Interpreter)

**Purpose**: Complete SCXML interpreter with parser, state machine, actions, and events.

**Contents**:
- **Model**: `SCXMLModel`, `StateNode`, `TransitionNode`, `GuardNode`, `InvokeNode`
- **Runtime**: `StateMachine`, `ActionExecutorImpl`, `StateMachineBuilder`
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

### C11 Backend Layering

The C11 backend mirrors the 4-tier shape with a small adaptation. Generated C11 state machines carry the runtime engine inline (per `c11_design_decisions.md` T3 inline-only lock-in), so the "tier 4" slot holds platform-specific implementations rather than a runtime-engine library. Consumers compose tiers like cpp.

| C11 Tier | Library | Kind | Contents |
|----------|---------|------|----------|
| Tier 1 (Core) | `sce_c_runtime` | INTERFACE | Public headers (`sce/clock.h`, `sce/dom.h`, `sce/http_client.h`, `sce/lua_dom_binding.h`, `sce/http_lua_binding.h`); no .c sources, no external link |
| Tier 2 (Base) | `sce_c_base` | STATIC | Freestanding-C helpers — DOM parser (`base/dom.c`); links only against `libc` |
| Tier 3 (Scripting) | `sce_c_scripting` | STATIC, optional | Lua-bound bridges over `sce_c_base` + platform impl; gated by `SCE_ENABLE_LUA` |
| Tier 4 (Platform) | `sce_c_runtime_posix` | STATIC, optional | POSIX reference impl (`posix/clock.c`, `posix/http_client.c`); gated by `SCE_C_RUNTIME_POSIX` (default ON for host) |

Bare-metal / RTOS consumers opt out of `sce_c_runtime_posix` and supply their own `sce_c_runtime_<target>` library providing the same symbol contract (`_sce_clock_now_ms` for the W3C 6.2 scheduler; HTTP impl optional). The interface contract in `backends/c/runtime/include/sce/` is the single source of truth — the test runner (`backends/c/tests`) is the contract-test consumer that pins the API against drift.

**Generated C11 code** consumes these tiers via stable headers (`#include <sce/clock.h>` etc.), never via host-relative paths — the headers and their impls are decoupled by the `sce_c_runtime` INTERFACE include path.

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

### Distribution Models

SCE ships in two independent distribution forms. Downstream consumers pick whichever matches their toolchain constraints; the two paths are isolated and neither depends on the other at runtime.

#### A. System install — `cmake --install` + `find_package(SCE)`

**Audience**: Consumers who can install SCE system-wide (Rust toolchain required at install time, no restriction at consumer build time).

**Producer**: `cmake --install build --component <Tier>` after a normal CMake build. Components mirror the 4-tier split (`Core`, `Scripting`, `Runtime`) plus `Codegen` for the codegen utilities. See `CMakeLists.txt:317` block.

**Layout** (default `${CMAKE_INSTALL_PREFIX}`):
```
lib/libsce_base.a, libsce_scripting.a, libsce_runtime.a
lib/cmake/SCE/         — SCEConfig.cmake, SCECodegen.cmake, SCEClangFormat.cmake
bin/sce-codegen        — Codegen binary
share/sce/codegen/     — default.clang-format + templates/ tree
include/               — Headers partitioned by tier
```

**Consumer integration**:
```cmake
find_package(SCE REQUIRED COMPONENTS Core Codegen)
sce_add_state_machine(TARGET my_app SCXML_FILE state.scxml)
target_link_libraries(my_app PRIVATE SCE::sce_base)
```

#### B. Embed vendor — `scripts/package_embed.sh` + `add_subdirectory()`

**Audience**: Consumers who vendor SCE source into `third_party/` and build it in-tree alongside their own code (no Rust toolchain, no git, no network at consumer build time — only a C++17 compiler). Representative downstream: `tc8-harness`.

**Producer**: `./scripts/package_embed.sh [-o OUTPUT_DIR]` emits a self-contained tree. The in-tree `embed/` directory is a gitignored artifact (only `embed/MANIFEST.json` is checked in — `verify_embed_manifest.sh` diffs it as a drift guard).

**Layout** (`embed/` root):
```
CMakeLists.txt         — Consumer-facing entry (from scripts/embed_CMakeLists.txt)
BUILD.bazel            — Bazel entry (auto-generated)
VERSION                — Git-describe of the source tree at packaging time
MANIFEST.json          — Public-header symbol surface (checked in for drift guard)
include/               — Headers (partitioned by SCE_BASE_INCLUDE_DIRS)
src/                   — sce_base sources (SCE_BASE_SOURCES SSOT)
third_party/           — nlohmann_json, pugixml, optional spdlog
sce_base_sources.cmake — SSOT copy for in-place builds
SCECodegen.cmake       — sce_add_state_machine() function
SCEClangFormat.cmake   — clang-format post-processor
tools/codegen/
  ├─ default.clang-format
  └─ templates/          — Jinja2 templates (cpp/rust/kotlin/go/python)
```

**Consumer integration**:
```cmake
add_subdirectory(third_party/sce)
include(${CMAKE_CURRENT_SOURCE_DIR}/third_party/sce/SCECodegen.cmake)
sce_add_state_machine(TARGET my_app SCXML_FILE state.scxml)
target_link_libraries(my_app PRIVATE sce_base)
```

The embed payload ships source + cmake utilities + codegen templates, but **not** the `sce-codegen` binary (platform-specific). Consumers place `sce-codegen` on `PATH`; `SCECodegen.cmake` resolves it via `find_program` and auto-detects `SCE_TEMPLATE_DIR` relative to the shipped `tools/codegen/templates/` — no manual configuration required.

#### Single source of truth: `sce/sce_codegen_assets.cmake`

Codegen-component files are declared once and consumed by both paths:

| Variable | Purpose |
|----------|---------|
| `SCE_CODEGEN_CMAKE_FILES` | CMake utility files (SCECodegen.cmake, SCEClangFormat.cmake) |
| `SCE_CODEGEN_TEMPLATE_DIR` | Jinja2 template tree path |
| `SCE_CODEGEN_STYLE_FILE` | clang-format style file path |

Adding a new utility or template group means editing `sce_codegen_assets.cmake` only. `CMakeLists.txt` `install()` rules and `package_embed.sh` both parse this file — drift between the two paths is not possible by construction, and `scripts/smoke_embed_consumer.sh` exercises the full embed→consumer-build pipeline in CI as a secondary guard.

---

## Code Generator: sce-codegen (Rust + minijinja)

**Tool**: `sce-codegen` — Rust binary from `sce-build` crate (replaces legacy Python codegen).
**Build**: `cargo build --bin sce-codegen --features cli -p sce-build`

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

### Stability and Library Use

`sce-build` is published as an `rlib` (`sce-build/Cargo.toml`). Downstream Rust crates depend on it as a regular library — workspace runtime crates and `build.rs` helpers already do so, and `sce-codegen` is one binary that consumes the same library surface. WASM builds that need a `cdylib` form (gated on the `wasm` feature) must pass `--crate-type cdylib` on the `cargo build` invocation explicitly; unconditional `cdylib` in the default crate-type set produced no in-tree consumer but did trip cargo issue #6313 output-path collisions, so it was removed.

**Until SCE 1.0, every `pub` item in `sce-build` is unstable and may change between commits without notice or migration path.** This includes `forge::model`, `forge::parser`, `forge::provenance`, `forge::sourcemap`, `forge::diagnostic`, `forge::xsd_validator`, and `forge::target_plugin`. This is policy, not oversight: 5-backend codegen parity and the v1 diagnostic wire contract are still consolidating, and freezing the surface before they settle would force a back-compat shim later.

A public stability tier (stable / unstable / hidden) will be declared in a future SCE release alongside the 1.0 cut. Until then, downstream consumers should pin a specific SCE commit and treat the parser/IR surface as private-by-policy even though it is `pub` for workspace reasons. The `--error-format=json` NDJSON wire contract (`SCE_ERROR_CONTRACT.md` §8 "Evolution policy") and the `sce-codegen` CLI flags are the only surfaces with their own explicit pre-1.0 governance today.

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

### Final-State Predicates — structural vs. session-ended

Two different W3C SCXML questions live at two different layers, and every backend draws the line in the same place.

| Predicate | Layer | Returns true when... | W3C reference | Used by |
|-----------|-------|---------------------|---------------|---------|
| `StatePolicy::isFinalState(s)` | Policy (generated) | `s` is a `<final>` element — including a region-level `<final>` nested inside a `<parallel>` or a compound state | Appendix D `isFinalState` — a structural question | `done.state.<parent>` emission, parallel-region completion checks |
| `isInFinalState()` | Engine | `currentState_` is a `<final>` **and** has no parent, i.e. its parent is the `<scxml>` element | §3.7 / §6.4 "this session has ended" | `tick()` short-circuit, `processEventQueues()` drain bail-out, `runUntilCompletion()`, `done.invoke` propagation |

**Why the parent check is load-bearing.** Appendix D `enterStates` sets `running = false` for a `<final>` only when `isSCXMLElement(s.parent)`; a nested one queues `done.state.<parent>` and the machine carries on. So when a `<parallel>` region reaches its regional `<final>` ahead of its siblings, `currentState_` transitions to that regional-final leaf while the `<parallel>` itself is still awaiting the other regions. A polling guard keyed on the bare structural predicate would misread this as "machine done" and skip the scheduler pump — starving any `<send delay="…">` scheduled elsewhere in the still-running configuration, including the §16.5 L3500 barrier timer that arms on **first** region completion.

**Do not add a second engine-level predicate.** The engine previously carried both `isInFinalState()` (structural) and `isGlobalFinalState()` (parent-checked). Nothing consumed the structural one — the policy already answers that question — while `runUntilCompletion()` bound to it by name and reported completion for a machine still resting in a nested `<final>`. The two names differed by a word that does not say which is which, and the Rust and Go engines reproduced the same defect by porting the name. One engine predicate, named `isInFinalState` across all six backends, meaning "this session has ended". Pinned by `integration_resources/nested_final_not_terminal/`.

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

The C++ backend emits the author's ECMAScript verbatim — a generated state
machine calls `safeEvaluateGuard(engine, session, "turns + 1 >= max_turns")`
— and the Interpreter does the same. So for C++, the selected engine *is* the
semantics of `datamodel="ecmascript"`, and the two selections are not
interchangeable.

| Engine | Standard | Selection | W3C IRP | ECMA-262 (`tests/ecmascript/ecma262_semantics.json`) |
|--------|----------|-----------|---------|------|
| QuickJS | ECMAScript 2020 | `SCE_SCRIPT_ENGINE=quickjs` (default) | 202/202 | **98/98** |
| Lua 5.4 | Lua 5.4 + ECMAScript compat | `SCE_SCRIPT_ENGINE=lua` | 202/202 | **98/98** |

⚠ **The Lua row is about the RUNTIME REWRITER, and since 2026-08-29 that is no
longer everything the `lua` selection runs.** `sce_add_state_machine` now
derives `--script-engine lua` for a `-DSCE_SCRIPT_ENGINE=lua` tree, so
**generated C++ in such a tree is lowered at build time and answers 98/98** —
it never reaches the rewriter. What still does is the **Interpreter**, which has
no build step, and any artifact generated with `--script-engine ecmascript`
explicitly. So read this row as the score for those, not for a C++ AOT build.

⚠⚠ **And since later the same day the rewriter is no longer everything THAT
path runs either.** The owner decided to link `sce-build`'s frontend into the
engine, so `LuaEngine::loweredTextOf` offers the author's ECMAScript to the
frontend's parser before the rewrite and falls back only when it refuses. The
scope it asked against was empty, which selects exactly the CLOSED expressions
— those naming no variable — so the row moved 75 → 86 without the rewriter
being touched.

⚠⚠⚠ **Then the scope stopped being empty, and the row moved again — 86 → 97.**
A `LuaEngine` session owns a `LoweringScope` and tells it what the session
holds: one `declare` per variable `setVariable` creates, one `declare_chunk`
per ECMAScript `<script>` that ran. An expression naming a declared variable is
therefore parsed rather than rewritten, so `a && b` yields its left operand and
`a == null` equates null with undefined. The rewriter was not touched for this
either — what changed is how much of the table it is still asked.

⚠⚠⚠⚠ **97 → 98, and `tests/ecmascript/lua_engine_divergences.json` is now
EMPTY.** The last case was not an expression: it diverged in a statement
sequence, which reaches the engine through `loweredScriptOf`. That path now
asks `sce_lower_script` first, by the same seam and the same fallback, so
`continue` reaches a real Lua label instead of the rewriter's `_ = continue`.
**An empty list is not a retired rewriter**: `EcmaScriptToLuaTransformer` is
still linked and still answers anything the frontend refuses, and this cell
scores the 98-case shared table rather than every program a consumer can
write. Retirement is a separate claim and needs its own witness.

⚠ **And this cell is the DIRECT route.** The same engine reached through a
generated `--script-engine ecmascript` document is measured separately by
`LoweredEcma262`, which reported `source-wrong=14` on 2026-08-29 and is RED on
it — its answers there include Lua's own `^` and `>>`, so that route appears to
reach neither the frontend nor the rewriter. Until that is understood, do not
read this cell as covering a document-driven artifact; the lane's census line
is the number to ask, and it prints on every run.
A second row for the lowered path is deliberately absent: it would be a cell
about an artifact shape rather than an engine, and this table is what a consumer
reads when choosing an ENGINE. `LoweredEcma262` is where the lowered path's
score lives, and it prints it on every green run.

The ECMA-262 column is derived, not typed. Its denominator is the length of
that table and the Lua row's numerator is that length minus the entries in
`tests/ecmascript/lua_engine_divergences.json` **declared on the
`runtime-rewriter` path**, which is the route this row's consumer takes: C++
codegen hands the engine the author's ECMAScript unless the run asked for
`--script-engine lua`. That list is what `ecmascript_semantics_test` holds the
`lua` selection to in both directions — an undeclared disagreement and a
declared one that has been repaired are both red.

Each entry's `diverges_on` names the paths, because there are two routes into
the same engine and they fail differently. The other one,
`build-time-lowering`, is `sce-build`'s frontend having emitted Lua already,
and `LoweredEcma262` is its contract — also both ways, which is what lets the
list empty rather than only grow. A cell for that path is deliberately absent
from this table: it would be a score for an artifact this repository does not
yet emit by default, and the row a consumer reads must describe the engine they
would actually get.

`sce-build/tests/ecma262_scoreboard_contract.rs` re-derives these two cells,
because the column had been typed once: it read **58/58** and **32/58** after
the shared table had grown to 98 cases, so both engines were being scored out
of a denominator that no longer existed.

The W3C column and the ECMA-262 column measure different things, and the gap
between them is why the second column exists. The IRP suite never writes
`0 && x`, `-7 % 3`, `1 == '1'` or a computed array index, so a full green
there says nothing about whether expressions mean what the language says they
mean. Selecting `lua` answers a whole class of them wrong — silently, with the
whole IRP suite passing — and the divergence list beside the table names every
one of them with the ECMA-262 clause it breaks. The count is whatever that
list holds; this paragraph deliberately does not repeat it, because the number
it used to carry ("26 of 58", measured 2026-08-14) outlived two growths of the
table.

**ScriptEngineProvider**: Compile-time engine selection via `SCE_SCRIPT_ENGINE` CMake option. Provides `IScriptEngine` interface for engine-agnostic consumers.

### EcmaScriptToLuaTransformer

Reached only by `SCE_SCRIPT_ENGINE=lua`, and the reason that is no longer the
default: it rewrites expression *text* rather than parsing it, so the answers
it gets wrong (every entry in `tests/ecmascript/lua_engine_divergences.json`)
are wrong by construction rather
than by omission — `0 && x` reads as Lua truthiness, `-7 % 3` as Lua's
flooring remainder, `5 ^ 3` as Lua's exponentiation. The build-time frontend
in `sce-build/src/ecmascript` is the parsing counterpart that the other five
backends use, and this one has no equivalent.

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

### ECMAScript Semantics (Single Source of Truth)

`sce/include/scripting/ecma_semantics.lua` — the ECMAScript operators Lua
does not share, defined once and loaded by every engine:

| Operator | Why Lua's own is not it |
|----------|------------------------|
| `+` | concatenates when either operand is a string, adds otherwise |
| `==` / `!=` | compare across types after coercion; Lua's `==` is `===` |
| `%` | ECMAScript truncates toward zero, Lua floors |
| `& \| ^ ~ << >> >>>` | operate on ToInt32 of the operands, not on integers |
| `obj[k]` | an Array is stored one-based, an ECMAScript index is zero-based |

| Backend | Embedding Mechanism | When |
|---------|-------------------|------|
| C++ | CMake `EmbedLuaScript.cmake` → `ecma_semantics_lua.h` | Compile-time |
| Rust | `include_str!` in `sce-rust-lua` | Compile-time |
| Kotlin | Gradle `copyEcmaSemantics` → `/scripting/ecma_semantics.lua` | Runtime (lazy) |
| Go | `//go:embed ecma_semantics.lua` in `backends/go/lua/` | Compile-time |
| Python | read from the repository path | Runtime |
| C11 | emitted into the generated engine bootstrap by codegen | Generation-time |

The file is written to Lua 5.2 rules — go-lua has no bitwise operators and
no `string.match`/`string.gsub` — and carries no file-local functions,
because the C11 embed splits it across several `luaL_dostring` calls to stay
under the C99 string-literal limit. `sce-build/tests/shared_lua_assets.rs`
fails if the Go copy drifts or an engine stops loading it.

The producer is `sce-build/src/ecmascript/` — the ECMAScript parser and Lua
emitter that replaced a 25-pass string rewriter whose entry point could not
fail. Do NOT reintroduce per-engine copies of these operators.

### JSON Builtins (Single Source of Truth)

`sce/include/scripting/json_builtins.lua` — canonical `JSON.stringify()` / `JSON.parse()` implementation shared across all three backends:

| Backend | Embedding Mechanism | When |
|---------|-------------------|------|
| C++ | CMake `EmbedLuaScript.cmake` → `json_builtins_lua.h` string literal | Compile-time |
| Rust | `include_str!("../../../../sce/include/scripting/json_builtins.lua")` | Compile-time |
| Kotlin | Gradle `copyJsonBuiltins` → classpath resource `/scripting/json_builtins.lua` | Runtime (lazy) |
| Go | `//go:embed json_builtins.lua` in `backends/go/lua/` | Compile-time |

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
| `ParallelTransitionHelper` (`ExitSetAlgorithms`) | D.2 | `getTransitionDomain` + `computeExitSet` over the configuration |
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
| JSEngine-dependent | GuardHelper, DoneDataHelper, AssignmentExecutionHelper, FinalizeHelper, DataModelInitHelper, DataModelReadHelper | `scripting/IJSExecutionEngine` |
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
backends/python/bindings/
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

### Backend directory layout

Every per-language backend lives under `backends/<lang>/` — the single
source of truth for the runtime + tests + Forge runtime of each language.
The C++ reference implementation (`sce/`) and the code generator
(`sce-build/`) stay at the repo root; they are the compiler components, not
per-language backends.

```
backends/
  c/{runtime, tests, forge-runtime}
  cpp/{forge-runtime}                         # C++ core lives in sce/
  go/{runtime, lua, tests, forge-runtime}
  kotlin/{runtime, lua, quickjs, rhino, tests, benchmark, spring-boot-starter, android-app, forge-runtime}
  python/{bindings, runtime, tests, forge-runtime}
  rust/{runtime, lua, tests, link-runtime, portable-bytes, forge-runtime, probes/*}
```

Package/module identities are stable across the move: Cargo crate names
(`sce-rust-runtime`), Gradle modules (`:sce-kotlin-runtime`), and Go module
paths (`github.com/newmassrael/sce-go-runtime`) are unchanged — only the
directory locations moved.

### Rust AOT Backend

Rust backend generates native Rust state machines from the same SCXML sources:

```
backends/rust/runtime/     Core engine (StaticExecutionEngine, event queue, policy traits)
backends/rust/lua/         Lua 5.4 script engine (mlua vendored build)
backends/rust/tests/       W3C conformance tests (202/202, linkme-based registration)
Cargo.toml            Workspace (Rust 1.75+, edition 2021)
```

**Architecture**:
- **Code Generator**: `sce-codegen generate -l rust` + `templates/rust/*.rs.jinja2`
- **Template Parity**: 1:1 port of C++ Jinja2 templates (state_machine, actions, invoke, etc.)
- **Scripting**: Lua 5.4 via `mlua` crate (vendored, same as C++ default engine)
- **JSON Builtins**: `include_str!("../../../../sce/include/scripting/json_builtins.lua")` — shared with C++/Kotlin
- **Test Registration**: `linkme` crate for compile-time test registration (equivalent to C++ `AotTestRegistrar`)

### Kotlin/JVM & Android

Kotlin/JVM modules provide the same W3C SCXML compliance (202/202) on JVM and Android:

```
backends/kotlin/runtime      ScxmlScriptEngine interface (Kotlin Multiplatform)
backends/kotlin/rhino        Rhino ECMAScript engine (pure JVM, fastest on server)
backends/kotlin/lua           Lua 5.4 engine via JNI (fastest on Android)
backends/kotlin/quickjs      QuickJS engine via JNI (full ES6, native)
backends/kotlin/spring-boot-starter Spring Boot auto-configuration (@AutoConfiguration)
backends/kotlin/tests        W3C conformance (202/202, all 3 engines)
backends/kotlin/benchmark    JMH benchmarks (3-engine comparison)
backends/kotlin/android-app         Android real-device benchmark (Compose UI)
```

**Engine selection per platform**:
- **JVM/Spring** (default: Rhino): Zero JNI overhead, JIT-optimized, pure Java
- **Android/AAOS** (default: Lua 5.4): Native C via JNI outperforms Rhino on ART (3-8x faster for guard evaluation)
- **C++** (default: QuickJS): `SCE_SCRIPT_ENGINE=quickjs` in CMake

**Kotlin code generator**: `sce-codegen generate -l kotlin` — generates sealed interface hierarchies + coroutine-based state machines from the same SCXML sources.

### Go Backend

Go backend generates native Go state machines with Go 1.22+ generics:

```
backends/go/runtime      StatePolicy[S,E] interface, Engine[S,E] generic execution engine
backends/go/lua          Lua script engine via Shopify/go-lua (pure Go, no CGo)
backends/go/tests        W3C conformance (202/202)
```

- **Templates**: `tools/codegen/templates/go/*.go.jinja2` — 1:1 port of Rust templates
- **Generics**: `Engine[S comparable, E comparable]` with `StatePolicy[S, E]` interface
- **State/Event**: `type State int` + `const ( StateXxx State = iota )` pattern
- **Scripting**: Lua via Shopify/go-lua (pure Go, no C compiler required)
- **JSON Builtins**: `//go:embed json_builtins.lua` — shared with C++/Rust/Kotlin
- **Test Generation**: `sce-codegen generate-w3c -l go` generates test files per W3C test

### Python AOT Backend

Python AOT generates native Python state machines from the same SCXML sources, mirroring Rust/Go/Kotlin/C11. Distinct from the **Python Bindings (pybind11)** channel above: that path runs the C++ Interpreter at runtime; this path runs Python code emitted at build time.

```
backends/python/runtime    Engine[S,E] generic execution + StatePolicy ABC + IScriptEngine
                      └── sce_runtime/scripting/lua_engine.py   Lua 5.4 via lupa
backends/python/tests      W3C conformance (202/202), pytest harness, in-process HTTP echo server
sce-forge-runtime/    Non-MCU Forge kinds: codec / filter / interpolation / lookup /
  python/             observer / procedure / timer (mirrors `backends/rust/forge-runtime/src/`)
```

**Channel separation (backends/python/runtime/README.md)**:

| Package | Mode | Mechanism |
|---|---|---|
| `sce` (`backends/python/bindings/`) | **Interpreter** | pybind11 → C++ Interpreter parses SCXML at runtime |
| `sce_runtime` (`backends/python/runtime/`) | **AOT** | Generated `*_sm.py` is the SM; this runtime is a generic driver |

**Architecture**:
- **Code Generator**: `sce-codegen generate-w3c -l python` + `tools/codegen/templates/python/*.py.jinja2`
- **Template Parity**: 1:1 port of Rust templates (state_machine / entry_exit_actions / process_transition / scriptengine_helpers / conflict_resolution / invoke_methods). Per-action emission lives in `tools/codegen/templates/python/actions/{assign,cancel,log,raise,script,send}.py.jinja2`; recursive `<if>` / `<foreach>` stay in `_actions.py.jinja2` so nested children can recurse through `emit` without a circular Jinja2 macro import
- **Scripting**: Lua 5.4 via `lupa` (PyPI), same ECMAScript→Lua transformer (`to_lua_expr` / `to_lua_guard` / `to_lua_script`) every other Lua-family backend uses. DOM bridge (`getElementsByTagName` / `getAttribute`) uses `xml.etree.ElementTree` + a thin `_DomElement` wrapper in `lua_engine.py`, mirroring `backends/rust/lua::dom::XmlRef`
- **State/Event**: `IntEnum` members named in UPPER_SNAKE_CASE; identifiers normalised via `to_python_const` filter
- **HTTP**: `backends/python/tests/conftest.py` spawns an in-process `http.server.HTTPServer` on port 8080 mirroring `tests/w3c/standalone_http_server.js` — no Node.js dependency in the AOT CI lane
- **Mesh**: Permanently rejected at codegen via `reject_python_unsupported_features` per C++-first mesh policy (same as Go / Kotlin)
- **Forge kinds**: Same admission as C++/Kotlin/Go — non-MCU kinds (Statechart / Transform / Lookup / Condition / Procedure / Aoi / Stream / BoundedCollection) ship; MCU-class kinds (Link / BufferPool / Worker) are rust+c11-only per `forge/codegen_matrix.rs::kind_class`

**Build & Test**:
- Codegen: `cargo build --bin sce-codegen --features cli -p sce-build && ./target/debug/sce-codegen generate-w3c -l python`
- Install runtime: `pip install -e backends/python/runtime/` (single hard dep: `lupa>=2.0`)
- Run W3C suite: `pytest backends/python/tests/generated/` — 202/202, ~1.5 s wall clock
- CI: `.github/workflows/w3c-tests.yml::test-python` (family-member AOT lane, sibling to `test-rust` / `test-kotlin` / `test-go`); the pybind channel rides `test-python-bindings`

---

## Key Principles

1. **W3C SCXML Compliance is Non-Negotiable**: every backend's W3C arm must be green — C++, Rust, Kotlin, Go, Python **and C11**, each with a job in `w3c-tests.yml`. The count is not written here because it is not one number: the five listed first run 202 cases, and the C11 arm runs 204 (its lane's own accounting, `test-c11`). This principle said "All 202 W3C tests ... on all backends (C++, Rust, Kotlin, Go, Python)", which omitted a backend that has had a lane since the round that added it and asserted a single total across arms that count differently.
2. **Always Generate Code**: Never refuse generation of W3C SCXML — always produce a working implementation rather than degrading to a runtime fallback. This principle is scoped to the W3C language, and the scoping is load-bearing: an `sce:`-namespace capability that the selected backend has no emission path for is a build-time refusal, not a generation SCE owes the author. Per Principle 8 that gap is a scope rule rather than unfinished work, so `<invoke type="sce:mesh-rpc">` on a backend without a mesh arm is an error that names the backends which have one (`SCE_MESH.md` §9.5), never a declaration the generator accepts and silently does not service. Read unscoped, this principle argues for deleting that gate — which is why the scope is written here rather than left to be inferred.
3. **Automatic Optimization**: Code generator decides AOT vs Interpreter internally per component
4. **Lazy Initialization**: Pay only for features actually used in SCXML
5. **Zero Duplication**: AOT and Interpreter share core W3C logic through Helper functions
6. **You Don't Pay for What You Don't Use**: 4-tier library structure — link only what you need
7. **Multi-Backend Parity**: Same SCXML sources, same codegen pipeline, same W3C compliance across C++/Rust/Kotlin/Go/Python/C11 — six backends, the set `Language::ALL` carries. Principle 8 scopes what parity does NOT cover.
8. **Parity Scope**: Multi-backend parity covers W3C SCXML codegen. Mesh (the SCE-specific distributed capability) is out of that parity obligation: per-backend mesh expansion is case-by-case, gated on explicit demand, and a backend without a mesh arm refuses the construct at build time rather than emitting a machine that ignores it (Principle 2's scoping). **Which** backends carry one is not stated here. This principle used to name the set by hand — "`tools/codegen/templates/mesh/` contains only `cpp/`" — which is a claim about the tree written somewhere the tree cannot correct, and the same hand-written set had already gone stale one layer down (`SCE_MESH.md` §1 listed five non-mesh backends and omitted C11). The set now lives in exactly one place: `SCE_MESH.md` §9.5's table, derived from `tools/codegen/templates/mesh/<dir>/` and held to both the template tree and the CLI's actual answer by `sce-build/tests/mesh_rpc_backend_contract.rs`. The clause that used to read "`SCE_MESH.md` stays agnostic to implementation count" is amended accordingly and deliberately: what it barred was a hand-written count drifting in the contract document, and a table a test regenerates the answer against is not that. The scope RULE still lives here; only the roster moved. The AOT `<parallel>`-final template stays partition-awareness-free for single-process machines; the partition-aware branch lives in `tools/codegen/templates/mesh/cpp/parallel_final.jinja2` (rule 12 designation in SCE_MESH.md §14 — root partition hosts tracker + local `done.state` raise, non-root partition emits wire 21 only) and is the sole coupling point between mesh deploy.yaml and AOT template shape, and lands atomically with the §16.5 `ParallelCompletionTracker` runtime and the rule 12 validator.

## Traceability Ownership Boundary

§5.O traceability (sourcemap JSON + addr2sce + per-symbol SCE-MAP markers) and §6.2.6 generated-source drift detection cover the files SCE emits directly. Files produced by external meta-generators (protoc, bindgen, cbindgen, capnproto), build-system wrappers (CMake / Cargo `build.rs` / Bazel), and hand-authored sources are out-of-scope by design.

**In scope (SCE owns):**

- Every file `sce-codegen` writes: `*_sm.{rs,cpp,h,kt,go,py,c}`, `mod.rs`, per-machine `sce_sourcemap.json` sidecars.
- The §6.2.6 drift header is the canonical SCE-ownership marker. Files carrying a `// SCE-GENERATED — DO NOT EDIT` block with embedded `source-hash` + `template-hash` lines are SCE-traced; files without that header are out-of-scope.

**Out of scope (SCE does not trace):**

- Output of external meta-generators (protoc-generated `.pb.go`, bindgen-generated FFI bindings, etc.).
- Hand-authored sources next to generated files (test harnesses, integration drivers).
- Build-system wrappers and shell scripts.

**Why a boundary, not a recursive ownership chain.** A single-vendor traceability map for a multi-tool pipeline would couple SCE's release cadence to every upstream tool's output format. The textbook answer — established by the JavaScript source-maps spec over twenty years — is that each stage in a toolchain emits its own sourcemap, and integration happens via sourcemap chaining at the consumer side. Single-Responsibility Principle and Bounded Context (DDD) both point the same direction: SCE traces what SCE emits, external tools trace what they emit, and the consumer composes chains as needed.

**addr2sce semantics.** When `sce-codegen addr2sce` cannot find a symbol in the SCE sourcemap (or the embedded `// SCE-MAP:` marker), it returns an explicit not-found rather than silently degrading or guessing. The not-found is the correct answer for symbols that originate in code SCE did not emit; pretending to resolve them would be a silently-broken hook.

**PC resolution.** `--pc` / `--hardfault` add one hop in front of that lookup: an address is mapped to the function symbol containing it by reading the ELF **symbol table**, not a DWARF line program. The SCXML coordinates live in the sourcemap, so a line program would only re-derive the generated-language line the sourcemap supersedes — and `.symtab` survives `--strip-debug`, which is the shape an MCU image ships. Containment is `[st_value, st_value + st_size)`: an address in inter-function padding is a not-found for the same reason an unknown symbol is, because attributing a fault to the nearest preceding function names the wrong state. On ARM the Thumb bit is cleared from `st_value` so a function's own first instruction resolves. `--hardfault` applies this per stack frame and exits non-zero when any frame is unattributable — a triage narrative with a silent hole in it is worse than one that says where it stopped.

**Walker contract.** `forge::sourcemap::validate_emitted_files_have_markers` runs after every `cmd_generate` / `cmd_generate_w3c` success path. It walks `out_dir` recursively, identifies SCE-emitted files by the presence of a §6.2.6 drift header, and verifies each one contains at least one `SCE-MAP:` marker line. Files without a drift header are silently skipped (out-of-scope per this section). A drift-headered file with no `SCE-MAP:` marker fires `traceability/meta-generated-source-line-marker-missing` — a codegen-internal invariant violation indicating a template regressed (added a backend, removed a marker macro call) without anyone noticing.

**Future extension — external sourcemap chaining.** When an SCE consumer eventually wraps SCE output behind an external meta-generator that the consumer wants in one resolution chain, the textbook path is a `deploy.yaml` `external_sourcemaps:` field listing each upstream tool's sourcemap path. `addr2sce` chains them at lookup time rather than merging them at emit time, preserving the boundary above. Until a consumer surfaces with that requirement, the field stays unbuilt per `feedback_planned_not_yagni.md`.
