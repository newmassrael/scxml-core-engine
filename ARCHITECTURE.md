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
- `XMLDOMWrapper`, `SessionRegistry` — DOM and session utilities
- `TypeRegistry`, `JsonUtils` — Runtime type and JSON support
- `UrlEncodingHelper` — RFC 3986 percent-encoding, which the Tier 1 headers `IOProcessorHelper.h` and `SendHelper.h` call inline for every generated machine

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
- `EventRaiserService` / `EventRaiserRegistry` — the session → `IEventRaiser` registry both engines route a script's raise through (headers under `events/`)

**Link target**: Static Hybrid AOT (JSEngine-embedded expressions) and Interpreter consumers.

**Dependency**: `sce_base` + `qjs` (QuickJS) and/or `lua54` (Lua 5.4) — and never `sce_runtime`. A symbol this tier calls but `sce_runtime` defines is a cycle between two static archives: lld resolves archive members in any order and hid three such edges (`EventRaiserService`, `Event`'s constructor, `StateMachine::isStateActive`), while GNU ld refused the first consumer that pulled no interpreter object before them. An engine reaches the interpreter only through what the interpreter registers with it, as `In()` reaches a state configuration through `setStateQueryCallback`.

**CMake options**:
- `SCE_ENABLE_QUICKJS` (default: ON)
- `SCE_ENABLE_LUA` (default: ON)
- `SCE_SCRIPT_ENGINE` (default: `quickjs`) — Selects default engine for `ScriptEngineProvider`

### Tier 4: sce_runtime (Full Interpreter)

**Purpose**: Complete SCXML interpreter with parser, state machine, actions, and events.

**Contents**:
- **Model**: `SCXMLModel`, `StateNode`, `TransitionNode`, `GuardNode`, `InvokeNode`
- **Runtime**: `StateMachine`, `InterpreterDocument`, `ActionExecutorImpl`, `StateMachineBuilder`
- **Actions**: `ScriptAction`, `AssignAction`, `SendAction`, `IfAction`, `ForeachAction`, `CancelAction`
- **Events**: `EventSchedulerImpl`, `EventDispatcherImpl`, `EventTargetFactoryImpl`, HTTP infrastructure
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
| Tier 1 (Core) | `sce_c_runtime` | INTERFACE | Public headers (`sce/clock.h`, `sce/dom.h`, `sce/http_client.h`, `sce/lua_dom_binding.h`); no .c sources, no external link |
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
SCEClangFormat.cmake   — how sce-codegen formats generated C++ (pinned clang-format 19)
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
- Template values are encoded for the comment they land in. Every template enters its environment through `generator::register_template`, which reads it in the language it emits (`sce-build/src/template_lexing.rs`) and routes each `{{ … }}` inside a comment through `comment_text` — `\`, CR, LF, the `/` of `*/` and the `*` of `/*` become `\xHH`, and every other character is unchanged. A template therefore writes `// cond="{{ trans.cond }}"`, never `| comment_text` and never a literal escaper (`escape_*`) inside a comment; `sce-build/tests/a_value_written_into_a_comment_is_encoded.rs` refuses both, refuses a template registered any other way, and renders hostile documents through all six backends. A value inside a string literal is not covered by this rule — its encoder is that language's escaper (`docs/SCE_ACCEPTED_SUBSET.md` §2.10). Nor is a value inside a Go `//line` directive: its reader is the Go toolchain, which decodes nothing, so `template_lexing` classifies it as a directive and the door passes the value through untouched, refusing only a line break (`comment_text::directive_guard`). What the encoder writes into a `SCE-MAP:` marker is read back by `forge::sourcemap::read_marker`, and a marker spelled in Rust rather than rendered from a template is written by `forge::sourcemap::marker_payload`.

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

⚠ **This section describes the present state, and Key Principle 9 (ADR 0003) is the decision to shrink it.** What follows is how SCE routes expressions it does not lower ahead of time; it is not the destination. Every cause in `script_engine_causes` that can be decided at build time is to move into the compiler — static `<param>` literals already did, and native `cpp:` / `kt:` conditions never raised a cause at all — leaving the engine for what genuinely exists only at run time. Read this section as the debt being worked down, not as the shape the pipeline is meant to keep.

### Dual Engine Support

The C++ backend emits the author's ECMAScript verbatim — a generated state
machine calls `safeEvaluateGuard(engine, session, "turns + 1 >= max_turns")`
— and the Interpreter does the same. So for C++, the selected engine *is* the
semantics of `datamodel="ecmascript"`, and the two selections are not
interchangeable.

| Engine | Standard | Selection | W3C IRP | ECMA-262 (`tests/ecmascript/ecma262_semantics.json`) |
|--------|----------|-----------|---------|------|
| QuickJS | ECMAScript 2020 | `SCE_SCRIPT_ENGINE=quickjs` (default) | 202/202 | **107/107** |
| Lua 5.4 | Lua 5.4 + ECMAScript compat | `SCE_SCRIPT_ENGINE=lua` | 202/202 | **107/107** |

⚠ **The Lua row is about the RUNTIME REWRITER, and since 2026-08-29 that is no
longer everything the `lua` selection runs.** `sce_add_state_machine` now
derives `--script-engine lua` for a `-DSCE_SCRIPT_ENGINE=lua` tree, so
**generated C++ in such a tree is lowered at build time and answers 107/107** —
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
asks `sce_lower_script`, so `continue` reaches a real Lua label instead of the
rewriter's `_ = continue`.
**An empty list was not a retired rewriter**, and this cell still scores the
107-case shared table rather than every program a consumer can write.
Retirement was the separate claim, and it now has the separate witness it
needed: the seam doc's `retire-rewriter` row sweeps every tracked C++ file
for anything that reaches `EcmaScriptToLuaTransformer` and finds none — the
unit itself has since been deleted, so there is nothing left to reach. What
the frontend refuses is refused (§scxml-5.9.1); nothing answers it instead.

⚠ **This cell was the DIRECT route only, until the same day closed the other
one.** The engine reached through a generated `--script-engine ecmascript`
document is measured separately by `LoweredEcma262`, and it reported
`source-wrong=14` while this cell read 98/98. The gap was one entry point, not
one more path: a `<assign>` whose location is not a bare identifier is run as a
SCRIPT (`AssignmentExecutionHelper`'s complex path), and the seam was on
`loweredTextOf` only — so `5 ^ 3` came back 125, Lua's exponentiation. Putting
the seam on `loweredScriptOf` took that census to `source-wrong=0`. Ask the
lane's census line rather than this cell for the document route; it prints on
every run.
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

### EcmaScriptToLuaTransformer — DELETED, on BOTH backends

**It is gone: header, source, and the benchmark that priced its retirement.**
`LuaEngine` lowers the author's ECMAScript with `sce-build`'s frontend
(`LoweringScope` → `SceLowering.h`) and refuses what that frontend refuses;
the seam doc's `retire-rewriter` row is the gate that holds the tree to it,
and `sce-build/tests/mutations/the_rewriter_is_retired_not_merely_second.cases`
puts a caller back and requires the row to go red. The sweep reads every
tracked C++ file rather than the engine's own directories, because a directory
boundary does not exempt what lies outside it — it hides it; there is now no
exemption of any shape, since nothing may reach a unit that is not there.

⚠ **The name existed TWICE, and the second one left a day later.** The Kotlin
backend carried its own 1175-line port
(`backends/kotlin/lua/.../EcmaScriptToLuaTransformer.kt`), on its own path,
behind its own four call sites. It is deleted too:
`com.sce.scripting.lua.LuaScriptEngine` reaches the same frontend through the
same C surface (`lowering_jni.cpp` → `SceLowering.h`) and refuses what it
refuses. Its witness is a SEPARATE row, `kotlin-retire-rewriter`, over a
separate population of every tracked Kotlin file — one sweep over both
backends would have reported the Kotlin half retired from the day C++ deleted
its own, four rounds before any Kotlin file stopped reaching it. What the two
retirements do share is the shape of the seam: one branch that asks the
language, the frontend behind it, and a refusal behind that.

⚠ **Deleting it deleted the sweep's control, and that is worth knowing before
touching this row.** The unit's own files used to be the proof that the
predicate could still see the name at all; "no file names it" is otherwise
what a sweep answers when it read nothing. What replaces them is the prose —
the comments in this file, in `LuaEngine`, in the engine suites — which the
check requires to stay above a floor. Deleting that history is allowed and
turns the row red on purpose, because it takes the last evidence that the
sweep reads.

Why it was retired: it rewrote expression *text* rather than parsing it, so
the answers it got wrong were wrong by construction rather than by omission —
`0 && x` read as Lua truthiness, `-7 % 3` as Lua's flooring remainder,
`5 ^ 3` as Lua's exponentiation. Every entry
`tests/ecmascript/lua_engine_divergences.json` ever held was one of those, and
the list is empty because the parsing counterpart the other five backends
already used is now reachable at run time too.

What it did, while it did it — ECMAScript in W3C SCXML `datamodel="ecmascript"`
bridged to Lua evaluation:
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

### JSON Text (Single Source of Truth)

A string written into JSON text is written one way on every engine, so the
same value is the same bytes whichever engine — or which peer over Mesh —
wrote it:

| Character | Written as |
|-----------|-----------|
| `"` `\` | `\"` `\\` |
| U+0008 U+000C U+000A U+000D U+0009 | `\b` `\f` `\n` `\r` `\t` |
| every other U+0000–U+001F | `\u00xx`, lowercase hex |
| everything else (U+007F, all non-ASCII) | as it is, in UTF-8 |

It is the form nlohmann's writer produces, which the C++ core already used for
script values and `error.communication` data. RFC 8259 allows other spellings;
SCE allows this one. Measured 2026-09-28, eleven hand-written escapers across
six engines agreed on none of the control characters, five passed U+0001 raw —
which is not JSON — and no test pinned any of them.

`tests/json_text/string_escape.json` holds the cases, and every writer reads it
with its own engine's JSON parser:

| Engine | Writer | Reader of the table |
|--------|--------|---------------------|
| C++ | `SCE::JsonText` (`sce/include/common/JsonText.h`) — `DoneDataHelper`, `HttpEventTarget`, `EventPayloadFields` | `tests/common/JsonTextTest.cpp` |
| Lua (all engines) | `JSON.stringify` in `json_builtins.lua` | `tests/common/JsonTextTest.cpp`, through the engine this build selected |
| Rust | `json::push_escaped` / `json::quote`; `escape_json_string` delegates | `backends/rust/runtime/src/json.rs` |
| Go | `EscapeJSONString`; `PayloadJSON` writes its strings with it | `backends/go/runtime/json_text_test.go` |
| Python | `_json_string`; `json.dumps(..., separators=(",", ":"), ensure_ascii=False)` | `backends/python/tests/json_text/test_string_escape.py` |
| C11 | `sce_payload_quote_span` | `backends/c/tests/unit/json_text_test.c` |
| Kotlin | `Json.quote` | `backends/kotlin/tests/.../runtime/JsonTextTest.kt` |
| codegen | the `escape_json_string` filter | `sce-build/src/filters.rs` |

Do NOT write a new escaper; call the engine's writer above.

### JSON Number Text (Single Source of Truth)

A finite 64-bit float written into JSON text, or into the text of an untyped
`<param>`, is written the way ECMAScript's Number::toString writes it in radix
10 (ECMA-262), so the same value is the same bytes whichever engine — or which
peer over Mesh — wrote it:

| Rule | Example |
|------|---------|
| the fewest digits that read back as the same double; of that many, the nearest to the value | `0.1`, not `0.10000000000000001` |
| decimal notation when `1e-6 <= \|x\| < 1e21`, otherwise `d[.ddd]e[+-]n` | `1e21`, `1.5e-7`, `0.000001` |
| a whole value has no fraction, and no integer type is in between | `5`, not `5.0`; `9223372036854775808` for 2^63 |
| either zero is `0` | `-0` is `0` |
| a value that is not finite is `null` in JSON, and `NaN` / `Infinity` / `-Infinity` in the text of an untyped `<param>` | |

The nearest decimal of the fewest digits is not always one that reads back.
Below a power of two the neighbouring double is half as far, so the nearest
decimal can fall outside the interval that reads as the value while the one
unit above it lies inside: 2^-44 is `5.6843418860808014...e-14`, which reads
back as `5.684341886080802e-14` and not as `...801e-14`. An engine that
rounds to a precision and then checks it writes seventeen digits there.

Measured 2026-10-04, the engines disagreed on the same value. The C++ payload
writer used `%.17g` (`0.1` came out as `0.10000000000000001`), the C++ result
text used the stream's default precision (pi came out as `3.14159`), Kotlin
used `Double.toString` (`1.0E21`, `5.0`), Rust and Go wrote `1e21` as twenty-two
digits and `1e-7` as `0.0000001`, and a whole double went through an `int64`
cast, which is wrong above 2^63. Nothing pinned any of them.

`tests/json_text/real_text.json` holds the cases (each value is its IEEE 754
bits, so no engine's float parser is the one deciding), checked against Node's
`String(x)` and `JSON.stringify`, and every writer is held to it:

| Engine | Writer | Reader of the table |
|--------|--------|---------------------|
| C++ | `SCE::JsonText::numberText` (`sce/include/common/JsonText.h`) — `JsonUtils::toCompactString`, `ScriptResultUtils`, `EventPayloadFields::field` | `tests/common/JsonTextTest.cpp` |
| Lua (all engines) | `JSON._number_text` in `json_builtins.lua`: the first `%e` precision that `tonumber` reads back, or the decimal one unit above it | `tests/common/JsonTextTest.cpp`, through the C++ build's Lua engine |
| Rust | `json::number_text`; `script_value_to_json` and `script_value_to_wire_string` use it | `backends/rust/runtime/src/helpers/event_data.rs` |
| Go | `NumberText` (`number_text.go`); `ScriptValueToJSON`, `ToWireString`, `PayloadJSON` use it | `backends/go/runtime/json_text_test.go` |
| Python | `sce_runtime.number_text.number_text`; `ScriptValue.to_json_literal` and its wire text use it, and a generated machine writes `_event.data` through them | `backends/python/tests/json_text/test_real_text.py` |
| Kotlin | `Json.numberText` over `shortestDigits` (`expect`, `BigDecimal` on the JVM); `valueToJson`, `valueToWireString`, `EventPayload.literal` use it | `backends/kotlin/tests/.../runtime/RealTextTest.kt` |
| C11 | `sce_number_text` (`sce/number_text.h`, in the runtime's include directory so a machine with no `sce-static` datamodel reaches it): the typed payload a machine writes into an event's `data`, and in a `sce-static` machine the pair and the request's param text (`sce/forge/wire.h`); a script-engine machine's own values go through `JSON.stringify` of the shared Lua above | `backends/c/tests/unit/json_text_test.c`; `test_event_schema_real.c` for the payload's text; `test_static_host_params.c` for a machine's wire; the Lua half, loaded in chunks as C11 loads it, by `sce-build/tests/shared_lua_assets.rs` |

What the contract does not fix:

- A `float32` is written as its platform spells it; only the 64-bit form is pinned.
- A real held in a saved machine's state (Rust `saved_real!` in `saved_state.rs`, Kotlin `SavedState.kt`) is written with the platform's own round-trip form and is read back by the same backend.
- A value shown to a person (a log line, a debug message, `getValueAsString`) is not wire text and keeps its engine's spelling.
- A script engine's own `JSON.stringify` is not a writer of SCE's wire text. The C++ test still holds the engine this build selected to the table, with one measured exception: QuickJS writes 2^-44 as `5.6843418860808015e-14`, and a power of two it writes with one digit more is accepted when that text reads back as the same value.

Do NOT write a new float spelling, and do NOT cast a double to an integer type
to drop its fraction; call the engine's writer above.

### JSON Object Key Order (Single Source of Truth)

The members of a JSON object written into `_event.data` come in one order on
every engine: ascending by the key's UTF-8 bytes — which is ascending by
Unicode code point — at every depth. RFC 8259 leaves the order open and SCE
closes it, so the same `<send>` is the same bytes whichever engine wrote it,
and a saved machine's `event_data` (held as text) reads the same on every
backend. An array keeps its order; only an object's members are sorted. A
`<param>` name that repeats collects its values, in document order, into one
array (W3C SCXML test178).

| Order is | Example |
|----------|---------|
| by bytes, not by number | `10` before `2` before `9` |
| by bytes, not by case | `B` before `a` |
| by code point, not by UTF-16 unit | U+FF5E before U+1F600 (UTF-16 would put the surrogate pair first) |

Measured 2026-10-02, the Kotlin writer kept the order a document declared its
`<param>`s in while the C++, Rust, Go and Python writers sorted them, and
nothing pinned either: a document that declared `b` before `a` wrote
`{"b":..,"a":..}` on one backend and `{"a":..,"b":..}` on the rest. A value a
script engine keeps to itself (a QuickJS object) is written by that engine's
`JSON.stringify`, which orders integer-like keys first whatever the text; the
Kotlin runtime reads that text back and writes it again in this order.

`tests/json_text/object_key_order.json` holds the cases, and every writer is
held to it:

| Engine | Writer | Reader of the table |
|--------|--------|---------------------|
| C++ | `EventDataHelper::buildJsonFromTypedParams` (nlohmann's `json`, a sorted map) | `tests/common/JsonTextTest.cpp` |
| Rust | `helpers::event_data::build_json_from_typed_params`, `script_value_to_json` | `backends/rust/runtime/src/helpers/event_data.rs` |
| Go | `BuildJSONFromTypedParams`, `ScriptValueToJSON` | `backends/go/runtime/event_data_key_order_test.go` |
| Python | `ScriptValue.to_json_literal` (no parameter builder yet) | `backends/python/tests/json_text/test_object_key_order.py` |
| Kotlin | `buildJsonFromParams`, `valueToJson`, `Json.writeCanonical`; order is `Json.compareKeys` | `backends/kotlin/tests/.../runtime/EventDataKeyOrderTest.kt` |
| C11 | a script-engine machine reads a payload (`sce_payload_*`) and writes no parameter object; a `sce-static` machine writes a flat object of scalars through `sce_forge_wire_pair` (`sce/forge/wire.h`), whose pairs the generator lists sorted by name once, since a pair a failed value leaves out leaves the others' order as it was. A name that repeats is one array, which it does not write yet and refuses by name | `sce-build/tests/a_c_machines_event_data_follows_the_shared_key_order.rs` (the table's flat cases, read from the generated source) |

Do NOT write a new object writer in a template or a backend; call the engine's
writer above, so a new engine arrives with the order already decided. A runtime
that has none writes one pair at a time and has its generator fix the order, as C11
does, and is added to the table above with the test that holds it to the cases.

### Durations (Single Source of Truth)

A `<send>` delay is read one way on every engine. W3C SCXML 6.2 names the
grammar — `delay` and the value of `delayexpr` must be a valid CSS2 time — and
SCE reads exactly that:

| Part | Rule |
|------|------|
| surroundings | ASCII whitespace before and after is ignored |
| number | digits with an optional fraction of at least one digit (`2`, `2.5`), or a leading `.` and digits (`.5`); no sign, no exponent |
| unit | `ms` or `s`, either case, directly after the number |
| value | exact decimal, truncated to whole milliseconds, at most 2^63−1 |

Anything else is not a time: a bare number (`5`), a space before the unit
(`2 s`), `min`/`h`, a negative. A static `delay` that is not one, and a
`delayexpr` value that is not one, are the argument error of W3C SCXML 6.2 —
`error.execution` with the send id, nothing scheduled, and the block ends —
on the Interpreter at run time and on every generated engine, where the build
decides the static case once (`Action::delay_invalid`) and the send refuses it
before evaluating anything. Measured 2026-09-28, nine readers across seven
channels disagreed on eleven of fifteen inputs: a bare `5` was 5000 ms on
three and 5 ms on four, `1min` was a minute on two and 0 elsewhere, `-1s` went
negative on three, and only the C++ reader had a test.

`tests/durations/css2_time.json` holds the cases, and every reader is held to
it:

| Engine | Reader | Reader of the table |
|--------|--------|---------------------|
| C++ (Interpreter and AOT) | `SendSchedulingHelper::parseDelayString` | `tests/common/DurationTest.cpp` |
| Rust | `helpers::send::parse_delay_to_ms` | `backends/rust/runtime/src/helpers/send.rs` |
| Go | `ParseDelayToMs` | `backends/go/runtime/duration_test.go` |
| Python | `sce_runtime.parse_delay_ms` | `backends/python/tests/durations/test_css2_time.py` |
| C11 | `sce_parse_delay_ms` | `backends/c/tests/unit/duration_test.c` |
| Kotlin | `SendHelper.parseDelayMs` | `backends/kotlin/tests/.../runtime/DurationTest.kt` |
| codegen (static `delay`) | `parser::parse_delay_to_ms` | `sce-build/src/parser.rs` |

Do NOT write a new delay parser; call the engine's reader above.

### Document Stem (Single Source of Truth)

A hybrid `<invoke srcexpr>` that declares `sce:candidates` starts the document
its value names (docs/SCE_ACCEPTED_SUBSET.md §2.13), and the value is free to
spell one document many ways: `file:x.scxml`, `./x.scxml`, an absolute path, a
Windows one. So the value, and each declared candidate, is reduced to a STEM,
and the two are compared. One rule reduces both:

| Part | Rule |
|------|------|
| directory | the text after the last `/` or `\` |
| scheme | a leading `file:` is dropped |
| extension | the text from the last `.` is dropped, unless that `.` opens the name (`.hidden` is a name, not an extension) |

A value that names no document (nothing after the last separator) has the
empty stem, which no candidate has. The build derives each candidate's stem
this way, so a value and the candidate it names cannot disagree about the stem —
as they could when the build read it with the platform's `Path::file_stem`,
which does not split on `\` on a Unix host and keeps a `file:` scheme in the
stem.

`tests/document_stem/document_stem.json` holds the cases, and every reader is
held to it:

| Engine | Reader | Reader of the table |
|--------|--------|---------------------|
| codegen (each declared candidate) | `model::document_stem` | `sce-build/tests/a_candidates_stem_is_the_one_every_engine_reads.rs` |
| C++ (Interpreter and AOT) | `SCE::documentStem` | `tests/common/DocumentStemTest.cpp` |
| Rust | `helpers::invoke_processing::document_stem` | `backends/rust/runtime/src/helpers/invoke_processing.rs` |
| Go | `DocumentStem` | `backends/go/runtime/document_stem_test.go` |
| Python | `sce_runtime.document_stem` | `backends/python/tests/document_stem/test_document_stem.py` |
| C11 | `sce_document_stem` | `backends/c/tests/unit/document_stem_test.c` |
| Kotlin | `DocumentStem.of` | `backends/kotlin/tests/.../runtime/DocumentStemTest.kt` |

Do NOT write a new stem reader; call the engine's reader above.

### External-Event Budget (Single Source of Truth)

A machine can answer an event by sending itself the next one, with no target,
which puts it on the external queue a host delivers to. Every macrostep of such
a machine ends, so `MAX_MACROSTEP_MICROSTEPS` (§scxml-3.13: Appendix D lets a
macrostep fail to end, and says nothing of a chain of macrosteps) never applies,
and the main event loop takes the next external event whenever the queue is not
empty. A host call that drains it does not return.

Measured 2026-10-02 on the Python runtime through the authoring driver, whose
only bound on such a design was its processor-time limit: it ran 25.5 s and was
judged the machine's doing ("another machine may differ"), when a design that
does this does it on every host. Every engine has the same loop shape and none
had a budget on it; the Python runtime is the first to hold the one below. A
budget on the host's side is not possible, since the loop is inside the engine
call and the host has no point to count from.

**The contract.** It is a ceiling this engine chooses and not a rule of W3C
SCXML, the way `MAX_MACROSTEP_MICROSTEPS` is, and so it has to be visible.

1. **What is counted.** The external events one invocation of the main event
   loop takes from the external queue and processes. The host's own event is
   the first of its invocation, whether the engine queues it first (Rust, Go,
   Kotlin, C, Python) or processes it directly (both C++ engines'
   `processEvent`, which handle the host's event and then run the loop: they
   start with one spent). So `spin` below leaves `links == B - 1`.
2. **The budget `B`.** 10,000 unless the host chooses another, settable at any
   time, and never below one: a value below one, or not a whole number, is
   refused and changes nothing, since a budget that takes no event is a machine
   that cannot run and not a stricter one.
3. **A cut.** When an invocation has taken `B` events and the queue still holds
   one, it stops taking events, leaves the queue exactly as it is, and counts
   the cut. The machine keeps running; the next invocation gets a budget of its
   own and goes on from where this one stopped. An invocation that takes exactly
   `B` and empties the queue refused nothing and counts nothing: a long chain
   that ends is not a runaway.
4. **The unit is the invocation, not the host call.** `send`, `process_event`,
   `step` and `initialize` run the loop once. `tick` and `advance_time` run it
   once per due scheduled entry and once more at the end, each invocation with a
   budget of its own. What is unbounded is a drain that refills itself; a clock
   the host moved is bounded by how far it moved, and a heartbeat every
   millisecond across a long jump must not be refused for taking many entries.
5. **A scheduler that can re-deliver at the instant it is popping.** A
   delivery due at the instant being processed (a `delay` of zero that goes
   through the scheduler rather than straight to the queue) can be re-armed by
   its own handler while the same `tick` or `advance_time` is still popping, and
   a budget on the drain alone never trips, since each pass takes one event. What
   can refill itself is only an entry due AT the clock reading the tick runs
   under: a handler arms relative to that reading, so what it arms is due at the
   reading or after it, and an entry due after it is not popped in this tick (a
   heartbeat across a long jump arms its next beat after the reading). An engine
   whose scheduler can do that bounds the pops of entries due at the reading by
   the same `B`, reset when the reading advances, and counts the cut the same
   way. It counts entries and not pops at a reading because an engine may hold
   ONE reading for the whole tick (Rust, Go): a jump over eight earlier instants
   pops eight entries at it, each due before it, and there are only as many of
   those as were armed when the tick began, so they are never the runaway and
   must not be cut by a small `B`. An engine that delivers a zero delay straight
   to the queue (Python, the C++ Interpreter) is already bounded by item 1, and one whose positive
   delays are dated after the instant being processed cannot re-arm into it.
6. **The default's basis.** 10,000 is above the longest invocation measured
   (6 external events, over every design the authoring suite plays) by three
   orders of magnitude, and is a margin and not a proof: the W3C corpus was
   measured on no engine. An engine that adopts the budget runs its own W3C
   lane under it before it lands, and a test that needs more fails there, which
   is the guard.
7. **A jump of a host-owned clock is one late tick, not a replay of the instants
   it jumped.** `advance_time(ms)` sets the reading `ms` later and ticks, so the
   entries due at or before the new reading are popped under that one reading
   (item 5), and what their handlers arm is dated from it. It is therefore NOT
   splitting-invariant: `advance_time(1000)` over a heartbeat that re-arms
   itself every 100 ms pops the beat armed before the jump and arms the next
   for 1100, where ten `advance_time(100)` pops ten beats. That is deliberate,
   and is what a machine on a real clock does when its host stalls and the next
   tick arrives late: one tick under the late reading. A simulation that must
   see every beat steps the clock as finely as the beats are spaced; a harness
   that wants the late tick jumps. Making the jump visit each due instant in
   order would make a simulated stall unlike a real one, and would move every
   engine's scheduler off the one reading item 5's bound is written against.
   What the host's step size must not change is the order entries are
   delivered in, nor whether a `<cancel>` reaches an entry that an earlier
   entry's macrostep runs before it has been delivered (§scxml-6.2): each due
   entry is delivered one macrostep apart.

**Accessors**, mirroring `truncated_macrosteps` and
`last_truncated_macrostep_state` in each runtime's own style, so a host written
for one reads the other:

| Engine | Count | Head of the queue at the last cut | Budget |
|--------|-------|-----------------------------------|--------|
| Python | `truncated_event_chains()` | `last_truncated_event()` | `max_external_events_per_call()`, `set_max_external_events_per_call(n)`, `MAX_EXTERNAL_EVENTS_PER_CALL` |
| Rust | `truncated_event_chains() -> u32` | `last_truncated_event() -> Option<P::Event>` | `max_external_events_per_call()`, `set_max_external_events_per_call(n)` |
| Go | `TruncatedEventChains() uint32` | `LastTruncatedEvent() (E, bool)` | `MaxExternalEventsPerCall()`, `SetMaxExternalEventsPerCall(n)` |
| Kotlin | `truncatedEventChains(): Int` | `lastTruncatedEvent(): E?` | `maxExternalEventsPerCall()`, `setMaxExternalEventsPerCall(n)` |
| C11 | `<prefix>_truncated_event_chains(sm)` | `<prefix>_last_truncated_event(sm, &out) -> bool` | `<prefix>_max_external_events_per_call(sm)`, `<prefix>_set_max_external_events_per_call(sm, n)`, `SCE_MAX_EXTERNAL_EVENTS_PER_CALL` |
| C++ AOT | `truncatedEventChains()` | `lastTruncatedEvent()` (`std::optional`) | `maxExternalEventsPerCall()`, `setMaxExternalEventsPerCall(n)` |
| C++ Interpreter | `Statistics::truncatedEventChains` | `Statistics::lastTruncatedEvent` (name) | `StateMachine::getMaxExternalEventsPerCall()`, `StateMachine::setMaxExternalEventsPerCall(n)` |

Both C++ engines hold one `Core::ExternalEventBudget` (`sce/include/core/ExternalEventBudget.h`),
which is the one place the default, the comparison, the count, the head and the
refusal of a budget below one are written; each engine asks it before it takes
an event and reports through its own accessors above. The Interpreter's loop asks
its raiser to name the head of the external queue without taking it
(`IEventRaiser::peekQueuedEventName`), because a refusal must leave the queue as
the host left it, and taking an event and putting it back would append it behind
everything queued after it.

Where the engine already gates its truncation diagnostics out of a small build
(Rust's `no_macrostep_diagnostics`), the BOUND stays in and only the count and
the head may be compiled out, as with `truncated_macrosteps`: the flag that
enforces the ceiling is not a diagnostic. C11's external queue is a fixed ring
(`SCE_MAX_EVENTS`) that drops on overflow, which is a different bound and not
this one; it holds only one event at a time in a chain that sends the next.

**Out of scope, and why.** Kotlin's coroutine mode (`start(scope)`) never
returns to the host, so there is no call to hand back, and a cut there would
need a different signal. The C++ Interpreter has no `tick` or `advance_time`:
its scheduler delivers from a thread, so only `processEvent` and `start` are
bound by it. Measured 2026-10-02 on the Interpreter: a delay that is zero,
written out or evaluated from an expression, is delivered straight to the
external queue and never reaches that scheduler, so a chain through one is cut by
item 1 and the call returns (`zero` and `zero_expr` hold there); a chain that
bounces between a parent and the child it invoked is cut on the parent's budget,
since each session runs a loop of its own and the parent takes every event the
child sends it. What stays outside is a chain through a POSITIVE delay, on that
thread: it is the heartbeat item 5 describes, paced by the clock and not a call
that fails to return, and `timed` has no deterministic form on an engine whose
clock the host cannot move.

**Fixture.** `tests/integration/external_chain_is_bounded.scxml` holds the
outcomes every engine answers the same way: `spin` (cut at exactly `B`, the rest
left queued), `bounded` (six events: refused nothing at `B = 6`, cut with one
`lap` queued at `B = 5`), `resume` (a chain of thirty finished by the next call
at `B = 20`), `zero` (a chain through a static `delay="0ms"`: the call returns
and the cut is counted), `zero_expr` (the same through `delayexpr="'0ms'"`),
and `timed` (eight pulses at later instants: never refused, however small `B`
is). The two zero outcomes are not the same test: an engine may read a STATIC
zero delay as undelayed (the Rust template does), which sends it straight to the
queue where item 1 bounds it, while an expression has no value to read at
generation time, so an engine whose scheduler delivers a delay that evaluates to
zero reaches its same-instant bound (item 5) only through `zero_expr`. Another
engine hands even a static zero to its scheduler (the Go template reads a delay
as none only when the attribute is empty), and there both outcomes reach item 5.
The
document is an ecmascript one for that reason; `sce-static` refuses `delayexpr`.
It sits beside its drivers rather than under
`integration_resources/`, a stem there being a seven-channel contract, until each
engine has its driver; the Python one is
`backends/python/tests/integration/external_chain_is_bounded/`.

### Event Names at the Door (Single Source of Truth)

An event that arrives BY NAME — a child's `<send target="#_parent">` or autoforward,
a completion `done.invoke.<id>`, a host's `sendEventByName` or its host-served
act's reply, an HTTP response, a Mesh peer — meets a machine whose generated
`Event` type holds only the names its document writes. W3C SCXML
3.12.1 matches a transition's `event` against an event's name by whole tokens, so
`request` matches `request.new` whether or not the document ever writes
`request.new`, and the names an event can arrive under are open. Every engine
delivers an arriving name as:

1. the document's own event of that name, when it writes it;
2. else the event of the longest token prefix of the name that it does write
   (the name cut at its last `.`, repeatedly);
3. else the wildcard event, when the document has a transition that listens
   with `event="*"`;
4. else none: no transition the document has could match the name, so it is
   dropped.

The longest prefix is the whole answer to the descriptors that name something,
because every one that matches an arriving name is a token prefix of it, so is
one of the document's names, so is a prefix of the longest of those, which
therefore matches exactly what the arriving name would. `*` is the one that names
nothing and matches every name, so it is the last resort and not a prefix at all:
`requesty` is not an extension of `request` (the cut is at a `.`, never at a
character) and `req` is a prefix of a descriptor, not an extension of one, so a
document that listens with `*` takes both there and a document that does not drops
both. The wildcard event is the member every engine declares for a document that
has such a transition, and its generated name table keeps it under the name `*`,
which no document can write as the name of an event, so the exact table the
generated code holds is the only thing the rule asks. The generated lookup stays
the exact table; the rule is written once per engine and every by-name door goes
through it:

| Engine | Rule | Held by |
|--------|------|---------|
| C++ AOT and Mesh dispatch | `SCE::Core::resolveArrivingEventName` | `tests/integration/AStaticDatamodelRunsGeneratedCppTest.cpp` |
| Rust | `StatePolicy::resolve_event_by_name` | `backends/rust/tests/tests/static_scenarios.rs` |
| Go | `Engine.ResolveEventByName` | `backends/go/tests/integration/static_datamodel/static_scenarios_test.go` |
| Kotlin | `StateMachineEngine.resolveArrivingEvent` | `backends/kotlin/tests/.../integration/StaticScenarioTest.kt` |
| Python | `StatePolicy.resolve_event_by_name` | `backends/python/tests/integration/static_datamodel/test_static_scenarios.py` |
| C11 | `<machine>_resolve_event_by_name`, emitted | `backends/c/tests/integration/test_static_scalars.c` |

The Interpreter matches descriptors against the arriving name itself
(`matchesEventDescriptor`) and needs no table. Measured 2026-10-04: every AOT
door looked a name up exactly, so a machine that listened for `request` and was
sent `request.new` dropped it in silence on every engine except Python, which
walked the prefixes in one door only (`send_external_by_name`); the Kotlin
generator emitted the lookup only for machines with an invoke, a parent, a script
engine, an HTTP send, a host processor or a typed payload, and now emits it for
every machine.

**The arrival name travels beside the member.** W3C SCXML 5.10 makes
`_event.name` the name the event was sent under, and an event delivered as the
member of a shorter descriptor has lost it. Every door therefore also records the
whole name in the event's metadata (`EventMetadata.name`,
`EventWithMetadata::name`, `event_with_meta_t.name`) when the member's own name
differs from it, and leaves it empty otherwise, so an event the document raises or
sends carries no copy of a name its member already holds. The generated `_event`
binding reads it before the member's name, an autoforward and a `<finalize>` read
it too, and a saved queued event is saved under it and restored through the same
rule. A completion `done.invoke.<id>` is told as the specific name whichever
descriptor (`done.invoke.<id>`, `done.invoke`, `done`) it was matched through. The
doors, with one constructor per engine (Rust `Engine::arriving_event`, Go
`Engine.arrivingEvent`, Kotlin `arrivalNameOf`, C++
`StaticExecutionEngine::arrivalNameOf`, C11 `<machine>_arrive`):

- a host or a parent delivering by name; an autoforwarded copy; a Mesh envelope;
- a child's `<send target="#_parent">`;
- a host-served act's reply, an HTTP response;
- a host-run invocation's completion, failure and deadline; an SCXML child's
  `done.invoke`.

Two documents hold two questions. `static_event_arrival.json` (over
`static_event_arrival.scxml`, both under
`sce-build/tests/fixtures/static_datamodel/`) holds WHICH event a name is delivered
as, and a step that expects the drop says `"dropped": true`; every harness but
Kotlin's asks the engine's own rule whether the name reaches the machine (Kotlin's
lookup is not public, so it holds the variables that did not move).
`static_event_wildcard.json` holds the same question for a document that also
listens with `event="*"`, where no name is dropped. The integration
stem `integration_resources/an_event_keeps_the_name_it_was_sent_under/` holds what
the machine is TOLD it is called, on every channel — a `sce-static` machine has no
`_event`, so a scenario cannot — and
`backends/c/tests/integration_resources/autoforward_keeps_the_arrival_name/` holds
that a parent's autoforwarded copy is told the whole name, whether the parent
delivered the event as the member of a shorter descriptor or as the wildcard member.
Rust, Go, Kotlin and C++ forward the name from the event's metadata; Python's and
C11's forwarded the member's own name (`request` for `request.new`, and now `*` for
the wildcard member) until 2026-10-04, and
`backends/python/tests/integration/static_datamodel/test_an_autoforwarded_copy_keeps_the_arrival_name.py`
holds Python's.

**A name the machine computes meets the same rule.** A `<send eventexpr>` names its
event at run time, so the document cannot have written the name, and every engine
used to look one up exactly in the table of the names its document writes and drop
one that was not in it, in silence, where the Interpreter takes it (measured
2026-10-04: `eventexpr="'request.' + 'new'"` beside a transition on `request`).
The send to this session's own queues now goes through the rule above and tells the
machine the whole name, over the external and the internal queue, now and after a
delay, so a scheduler entry carries the name through the wait (a saved state holds no
such entry: a computed name needs a script engine, and only a `sce-static` machine,
which has none, is saved):

| Engine | Where | Carried through the wait by |
|--------|-------|-----------------------------|
| Rust | `Engine::send_named_external` / `send_named_internal` | `ScheduledAct::{Raise, Routed}.name` |
| Go | `Engine.SendNamedExternal` / `SendNamedInternal` | `scheduledEntry.name`; the internal route's `EventName` |
| Kotlin | the generated send site (`resolveArrivingEvent`, `arrivalNameOf`) | the entry's `EventMetadata.name` |
| Python | the generated send site, `Engine.send_to_self` / `schedule_send` | `ScheduledEvent.name`; the internal route's `event_name` |
| C++ AOT | the generated send site, `engine.resolveEventByName` | `ScheduledRoute::Kind::ExternalQueue` (and the internal route's `eventName`) |
| C11 | the generated send site, `<machine>_arrive` | the scheduled entry's `name[]` |

`a_computed_event_name_is_matched_like_any_other` (`integration_resources/`, every
channel) holds it with six sends — the four of those queues, and two more whose
target is computed too (`targetexpr`), which every engine routes through its table
that classifies a target value. A name sent to `#_parent` or to an invoked child
is the receiver's to resolve, and crosses as the name.

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
| `HierarchicalStateHelper` | 3.13, D | Ancestry and descendancy, the domain-candidate filter |
| `ForeachHelper` | 4.6 | Loop variable declaration and type preservation |
| `InvokeHelper` | 6.4 | Invoke lifecycle (defer/cancel/execute pattern) |
| `TransitionHelper` | 3.13 | Transition selection and execution |
| `ConflictResolutionHelper` | D.2 | Optimal transition set selection |
| `ParallelTransitionHelper` (`ExitSetAlgorithms`) | D.2 | `getTransitionDomain` + `computeExitSet` over the configuration, and the union of a microstep's exit sets in exit order |
| `EntrySetHelper` (`EntrySetAlgorithms`) | D.2, 3.3, 3.6, 3.10 | `computeEntrySet` and its callees: target sets, default entry, history dereference, entry order |
| `MicrostepAlgorithms` | D.2 | The microstep: `selectTransitions`, `removeConflictingTransitions`, `exitStates`, `executeTransitionContent`, `enterStates`, over a Host each engine provides |
| `ParallelCompletionHelper` (`CompletionAlgorithms`) | D.2, 3.4 | `isInFinalState`, recursive over a `<parallel>` nested as a region |
| `ParallelStateHelper` | 3.4 | Parallel region orchestration |
| `HistoryHelper` | 3.11 | History state recording/restoration |
| `EntryExitHelper` | 3.7, 3.8 | State entry/exit action execution |
| `EventMatchingHelper` | 5.9.3 | Event descriptor prefix matching |
| `ExternalEventBudget` | 3.13 | The external-event budget of one main-loop invocation: default, comparison, cut count and head, host-set limit (see "External-Event Budget") |

Appendix D's microstep is written once, in `MicrostepAlgorithms`, over these
helpers. An engine hands it a Host: the document as `EntrySetAlgorithms` reads
it, plus the run — the configuration, which transition of a state an event
enables, and what one state's entry, exit or transition content does. The AOT
engine's Host is `StaticExecutionEngine::MicrostepHost`, over the generated
policy; the Interpreter's is `StateMachine::MicrostepHost`, over
`InterpreterDocument` — the parsed model read the way Appendix D reads a
document. Everything Appendix D does with the Host's answers is the shared
procedure, for a machine with a `<parallel>` and one without alike.

Around the microstep, both engines run `mainEventLoop` in the same shape: a
macrostep completes on eventless transitions and internal events, the invokes
it armed start, and only then is the next external event taken — after a
macrostep stopped at `MAX_MACROSTEP_MICROSTEPS` too. The Interpreter's two
queues are its `IEventRaiser`'s, which keeps them in one structure, so the loop
names the one it takes from (`EventQueue::Internal` / `EventQueue::External`).
An event handed to a machine in the middle of its own macrostep is `enqueue`d
on the queue it belongs to and taken in turn, never processed where it lands.

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

**Template + String Adapter Pattern**: an algorithm is written once over injected accessors — lambdas, or a `Doc`/`Host` object — so the AOT engine (enum states) and the Interpreter (string ids) instantiate the same template; a policy-bound wrapper binds it to the generated policy's static tables:
```cpp
ExitSetAlgorithms::computeExitSet(source, targets, isInternal, isTargetless,
                                  configuration, parentOf, isDomainCandidate);  // lambdas
MicrostepAlgorithms::microstep(host, transitions);                            // a Host
ConflictResolutionHelper<StatePolicy>::removeConflictingTransitions(...);     // AOT wrapper
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
- pthread for `EventDispatcherImpl`

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
- **The microstep**: Appendix D's procedures are `backends/rust/runtime/src/helpers/microstep.rs`, a transcription of `MicrostepAlgorithms` / `EntrySetAlgorithms` / `ExitSetAlgorithms` / `ConflictResolutionAlgorithms` / `CompletionAlgorithms` over a `Document` and a `Run` the caller supplies. `Engine<P>` hands it the generated policy through `EngineHost` and runs `mainEventLoop` in the same shape as the C++ engines. The policy supplies only its document's tables — child states, initial and history targets as written, document order — and one-state hooks (`first_enabled_transition`, `execute_entry_actions`, `execute_exit_actions`, `execute_transition_content`, `execute_history_default_content`), for a machine with a `<parallel>` and one without alike.
- **Template Parity**: mirrors the C++ AOT templates (`process_transition.jinja2`, `state_machine_inl.jinja2`, `entry_exit_actions.jinja2`) — the policy answers the same questions the C++ `MicrostepHost` asks
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

- **The microstep**: `backends/kotlin/runtime/.../Microstep.kt` is the runtime's one transcription of W3C SCXML Appendix D — selection, conflict removal, exit and entry sets with their `<history>` and `<initial>` defaults, `isInFinalState` — function for function with the Go runtime's `microstep.go` and held to the same hand-worked answers (`backends/kotlin/tests/.../runtime/MicrostepTest.kt`). `StateMachineEngine<S, E>` drives it through a `Run` over its own configuration and history store — generated code touches neither — and runs `mainEventLoop` in the same shape as the other engines, in both the synchronous and the coroutine mode. The generated machine supplies its document's tables — child states, initial and history targets as written, document order, each transition as the microstep reads it, built once in a companion object — and one-state hooks (`firstEnabledTransition`, `onEntry(state, isDefaultEntry)`, `onExit`, `executeTransitionContent`, `executeHistoryDefaultContent`), for a machine with a `<parallel>` and one without alike. A `<history>` is named by a `HistoryId` rather than by a third type parameter, as in Go.

### Go Backend

Go backend generates native Go state machines with Go 1.22+ generics:

```
backends/go/runtime      StatePolicy[S,E] interface, Engine[S,E] generic execution engine
backends/go/lua          Lua script engine via Shopify/go-lua (pure Go, no CGo)
backends/go/tests        W3C conformance (202/202)
```

- **Templates**: `tools/codegen/templates/go/*.go.jinja2` — ported from the Rust templates, and answering the same questions (`process_transition` / `entry_exit_actions` / `state_machine`)
- **The microstep**: `backends/go/runtime/microstep.go` is the runtime's one transcription of W3C SCXML Appendix D — selection, conflict removal, exit and entry sets with their `<history>` and `<initial>` defaults, `isInFinalState` — function for function with the Rust runtime's `helpers/microstep.rs` and held to the same hand-worked answers (`microstep_test.go`). `Engine[S, E]` drives it through `engineHost` and runs `mainEventLoop` in the same shape as the C++ and Rust engines. The generated policy supplies its document's tables — child states, initial and history targets as written, document order — and one-state hooks (`FirstEnabledTransition`, `ExecuteEntryActions`, `ExecuteExitActions`, `ExecuteTransitionContent`, `ExecuteHistoryDefaultContent`), for a machine with a `<parallel>` and one without alike
- **Generics**: `Engine[S comparable, E comparable]` with `StatePolicy[S, E]` interface. A `<history>` is named by the policy's `sce.HistoryID` rather than by a third type parameter, so the type a host names does not change for a fact only the policy uses
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
- **The microstep**: `sce_runtime/microstep.py` is the runtime's one transcription of W3C SCXML Appendix D — selection, conflict removal, exit and entry sets with their `<history>` and `<initial>` defaults, `isInFinalState` — function for function with the Rust runtime's `helpers/microstep.rs` and held to the same hand-worked answers (`backends/python/tests/microstep/`). `Engine` drives it through `_MicrostepHost`; the generated policy supplies the document's structure as written (child states, initial targets, `<history>` elements), which of one state's transitions an event enables, and the executable content of states and transitions
- **Template Parity**: ported from the Rust templates (state_machine / entry_exit_actions / process_transition / scriptengine_helpers / invoke_methods). Per-action emission lives in `tools/codegen/templates/python/actions/{assign,cancel,log,raise,script,send}.py.jinja2`; recursive `<if>` / `<foreach>` stay in `_actions.py.jinja2` so nested children can recurse through `emit` without a circular Jinja2 macro import
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
2. **Always Generate Code**: Never refuse generation of W3C SCXML — always produce a working implementation rather than degrading to a runtime fallback. This principle is scoped to the W3C language, and the scoping is load-bearing: an `sce:`-namespace capability that the selected backend has no emission path for is a build-time refusal, not a generation SCE owes the author. Per Principle 8 such a gap is a scope rule rather than unfinished work, so the construct is an error that names the backends which serve it, never a declaration the generator accepts and silently does not service. `<invoke type="sce:mesh-rpc">` was the standing example until §mesh-19's host router gave every backend a route for it (`SCE_MESH.md` §9.5); a gate is retired when that happens, not when this principle is read unscoped — which is why the scope is written here rather than left to be inferred.
3. **Automatic Optimization**: Code generator decides AOT vs Interpreter internally per component
4. **Lazy Initialization**: Pay only for features actually used in SCXML
5. **Zero Duplication**: AOT and Interpreter share core W3C logic through Helper functions
6. **You Don't Pay for What You Don't Use**: 4-tier library structure — link only what you need
7. **Multi-Backend Parity**: Same SCXML sources, same codegen pipeline, same W3C compliance across C++/Rust/Kotlin/Go/Python/C11 — six backends, the set `Language::ALL` carries. Principle 8 scopes what parity does NOT cover.
8. **Parity Scope**: Multi-backend parity covers W3C SCXML codegen. Mesh (the SCE-specific distributed capability) is out of that parity obligation: per-backend mesh expansion is case-by-case, gated on explicit demand, and a backend without a mesh arm refuses the construct at build time rather than emitting a machine that ignores it (Principle 2's scoping). **Which** backends carry one is not stated here. This principle used to name the set by hand — "`tools/codegen/templates/mesh/` contains only `cpp/`" — which is a claim about the tree written somewhere the tree cannot correct, and the same hand-written set had already gone stale one layer down (`SCE_MESH.md` §1 listed five non-mesh backends and omitted C11). The set now lives in exactly one place: `SCE_MESH.md` §9.5's table, derived from `tools/codegen/templates/mesh/<dir>/` and held to both the template tree and the CLI's actual answer by `sce-build/tests/mesh_rpc_backend_contract.rs`. The first expansion made under this rule was not a second generated router but a host one (`SCE_MESH.md` §19): every other backend lowers Mesh sends and requests to host-served types and lets the host's router carry them, so the table now names a route per backend rather than a refusal. The clause that used to read "`SCE_MESH.md` stays agnostic to implementation count" is amended accordingly and deliberately: what it barred was a hand-written count drifting in the contract document, and a table a test regenerates the answer against is not that. The scope RULE still lives here; only the roster moved. The AOT `<parallel>`-final template stays partition-awareness-free for single-process machines; the partition-aware branch lives in `tools/codegen/templates/mesh/cpp/parallel_final.jinja2` (rule 12 designation in SCE_MESH.md §14 — root partition hosts tracker + local `done.state` raise, non-root partition emits wire 21 only) and is the sole coupling point between mesh deploy.yaml and AOT template shape, and lands atomically with the §16.5 `ParallelCompletionTracker` runtime and the rule 12 validator.
9. **One Portable Document, Compiled Per Target** (ADR 0003, `docs/adr/0003-one-portable-document-compiled-per-target.md`): a statechart is authored once and compiled into each backend's own language. A runtime script engine is the **fallback for what cannot be decided at build time**, not a stage of the pipeline — the generate manifest's `script_engine_causes` names every remaining reason with its location, and that population is measured corpus-wide and ratcheted so it cannot grow unremarked. Native prefixes (`cpp:`, `kt:`) stay an **escape hatch**: admitted where the rule around them is about data model languages, because native code is not one (`is_native_script` says so for `<script><cpp>` already), and **counted**, because a document that uses one is no longer portable. This is what protects Principle 7 — a dialect per backend would author the same machine six times and end parity rather than serve it.

## Traceability Ownership Boundary

§5.O traceability (sourcemap JSON + addr2sce + per-symbol SCE-MAP markers) and §6.2.6 generated-source drift detection cover the files SCE emits directly. Files produced by external meta-generators (protoc, bindgen, cbindgen, capnproto), build-system wrappers (CMake / Cargo `build.rs` / Bazel), and hand-authored sources are out-of-scope by design.

**In scope (SCE owns):**

- Every file `sce-codegen` writes: `*_sm.{rs,cpp,h,kt,go,py,c}`, `mod.rs`, per-machine `sce_sourcemap.json` sidecars.
- The §6.2.6 drift header is the canonical SCE-ownership marker. Files carrying a `// SCE-GENERATED — DO NOT EDIT` block with an embedded `source-hash` line are SCE-traced; files without that header are out-of-scope.

**Out of scope (SCE does not trace):**

- Output of external meta-generators (protoc-generated `.pb.go`, bindgen-generated FFI bindings, etc.).
- Hand-authored sources next to generated files (test harnesses, integration drivers).
- Build-system wrappers and shell scripts.

**Why a boundary, not a recursive ownership chain.** A single-vendor traceability map for a multi-tool pipeline would couple SCE's release cadence to every upstream tool's output format. The textbook answer — established by the JavaScript source-maps spec over twenty years — is that each stage in a toolchain emits its own sourcemap, and integration happens via sourcemap chaining at the consumer side. Single-Responsibility Principle and Bounded Context (DDD) both point the same direction: SCE traces what SCE emits, external tools trace what they emit, and the consumer composes chains as needed.

**addr2sce semantics.** When `sce-codegen addr2sce` cannot find a symbol in the SCE sourcemap (or the embedded `// SCE-MAP:` marker), it returns an explicit not-found rather than silently degrading or guessing. The not-found is the correct answer for symbols that originate in code SCE did not emit; pretending to resolve them would be a silently-broken hook.

**PC resolution.** `--pc` / `--hardfault` add one hop in front of that lookup: an address is mapped to the function symbol containing it by reading the ELF **symbol table**, not a DWARF line program. The SCXML coordinates live in the sourcemap, so a line program would only re-derive the generated-language line the sourcemap supersedes — and `.symtab` survives `--strip-debug`, which is the shape an MCU image ships. Containment is `[st_value, st_value + st_size)`: an address in inter-function padding is a not-found for the same reason an unknown symbol is, because attributing a fault to the nearest preceding function names the wrong state. On ARM the Thumb bit is cleared from `st_value` so a function's own first instruction resolves. `--hardfault` applies this per stack frame and exits non-zero when any frame is unattributable — a triage narrative with a silent hole in it is worse than one that says where it stopped.

**Walker contract.** `forge::sourcemap::validate_emitted_files_have_markers` runs after every `cmd_generate` / `cmd_generate_w3c` success path. It walks `out_dir` recursively, identifies SCE-emitted files by the presence of a §6.2.6 drift header, and verifies each one contains at least one `SCE-MAP:` marker line. Files without a drift header are silently skipped (out-of-scope per this section). A drift-headered file with no `SCE-MAP:` marker fires `traceability/meta-generated-source-line-marker-missing` — a codegen-internal invariant violation indicating a template regressed (added a backend, removed a marker macro call) without anyone noticing.

**Future extension — external sourcemap chaining.** When an SCE consumer eventually wraps SCE output behind an external meta-generator that the consumer wants in one resolution chain, the textbook path is a `deploy.yaml` `external_sourcemaps:` field listing each upstream tool's sourcemap path. `addr2sce` chains them at lookup time rather than merging them at emit time, preserving the boundary above. Until a consumer surfaces with that requirement, the field stays unbuilt per `feedback_planned_not_yagni.md`.
