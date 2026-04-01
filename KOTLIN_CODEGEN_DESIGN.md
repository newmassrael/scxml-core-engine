# Kotlin Code Generation Design

Target: Modern Android (Kotlin-first, Jetpack Compose, Coroutines)

Minimum: Kotlin 1.9+, kotlinx-coroutines 1.7+, Kotlin Multiplatform compatible

## Design Principles

1. **Native Kotlin feel** — Generated code should look like hand-written Kotlin, not a C++ port
2. **Compose-ready** — StateFlow-based state observation, direct `collectAsState()` integration
3. **C++ parity** — Same 202 W3C tests, same correctness guarantees, same compile-time engine selection
4. **SCXML is the source of truth** — Generated `.kt` files are build artifacts, regenerated every build

---

## Decision Summary

| Decision | Choice | Rationale |
|----------|--------|-----------|
| State representation | `sealed interface` + `data object` | Hierarchy, data, exhaustive `when`, Compose UiState pattern |
| Event representation | `sealed interface` + `data object` | W3C prefix matching via type hierarchy, future event data |
| Script engine (Phase 1) | Pure static | Pipeline validation first; 63/202 tests |
| Script engine (Phase 2) | Lua 5.4 + QuickJS (both, compile-time select) | C++ parity: both engines via `ScriptEngine` interface |
| Engine selection | Gradle `-PsceScriptEngine=lua\|quickjs` | Mirrors C++ `-DSCE_SCRIPT_ENGINE=lua\|quickjs` |
| Runtime concurrency | `StateFlow` + `Channel` + Coroutines | Android standard async/state management |
| Build / distribution | KMP Gradle, Maven Central | commonMain for multiplatform, androidMain for lifecycle |
| Generated file layout | Single `.kt` per state machine | Kotlin allows multiple public declarations per file |
| Kotlin minimum version | 1.9+ | `data object` support (released July 2023) |
| Microstep threading | `Dispatchers.Default` (forced) | Never block UI thread |
| `send()` API | Non-suspending, `Channel.UNLIMITED` | UI event handlers are not suspend functions |
| Interpreter engine | Not needed | C++ Interpreter+AOT already cross-validates SCXML correctness |

---

## 1. State Representation: `sealed interface`

```kotlin
sealed interface PlayerState : State {
    data object Stopped : PlayerState
    data object Playing : PlayerState
    data object Paused  : PlayerState
}
```

### Why not `enum class`

| Concern | `enum class` | `sealed interface` |
|---------|-------------|-------------------|
| Per-state data | Impossible | `data class Buffering(val progress: Float)` |
| Hierarchy (compound states) | Flat only | Nested sealed interfaces |
| Parallel states | Cannot represent | `data class(region1: R1, region2: R2)` |
| Compose UiState alignment | No | Yes — identical pattern |
| Allocation (data object) | Zero | Zero (singleton) |
| Exhaustive `when` | Yes | Yes |

### Parallel State Representation

W3C SCXML 3.4 parallel regions map to Kotlin `data class` with per-region properties:

```kotlin
sealed interface DashboardState : State {
    data class Active(
        val media: MediaRegion,
        val nav: NavRegion
    ) : DashboardState

    sealed interface MediaRegion {
        data object Playing : MediaRegion
        data object Paused : MediaRegion
    }
    sealed interface NavRegion {
        data object Home : NavRegion
        data object Settings : NavRegion
    }
}
```

Benefits: structural equality via `data class`, `copy()` for partial region updates, destructuring.

### History State Representation

W3C SCXML 3.11 history states use nullable properties:

```kotlin
// Shallow history — remembers direct child
var lastActiveChild: PlayerState? = null

// Deep history — remembers nested descendant
var lastActiveDeep: PlayerState? = null
```

---

## 2. Event Representation: `sealed interface`

```kotlin
sealed interface PlayerEvent : Event {
    data object Play  : PlayerEvent
    data object Pause : PlayerEvent
    data object Stop  : PlayerEvent

    // W3C dot-notation prefix matching via type hierarchy
    sealed interface Error : PlayerEvent {
        data object Execution : Error
        data object Communication : Error
    }
}
```

### W3C Prefix Matching as Type Hierarchy

SCXML `<transition event="error"/>` matches all `error.*` events.

In Kotlin, this becomes a type check:

```kotlin
// SCXML: event="error" → matches error.execution, error.communication
when (event) {
    is PlayerEvent.Error -> handleAnyError()           // prefix match
    is PlayerEvent.Error.Execution -> handleExecOnly() // exact match
}
```

Compile-time type safety replaces C++'s runtime string prefix comparison.

### Phase 2: Event Data

When script engine support is added (Lua 5.4), events carry W3C metadata:

```kotlin
interface Event {
    val name: String        // W3C _event.name
    val data: Any?          // W3C _event.data
    val type: EventType     // internal, external, platform
    val sendId: String?     // W3C _event.sendid
    val origin: String?     // W3C _event.origin
    val invokeId: String?   // W3C _event.invokeid
}
```

Phase 1 events are pure identifiers (`data object`) with no data.

---

## 3. Runtime Architecture

### Core Engine

```kotlin
abstract class StateMachineEngine<S : State, E : Event> {

    // --- Observable State ---
    val currentState: StateFlow<S>                        // Latest state (Compose UI)
    val transitions: SharedFlow<TransitionRecord<S, E>>   // All transitions (debug/logging)

    // --- Event Submission ---
    fun send(event: E)                        // Non-suspending, fire-and-forget
    suspend fun sendAndAwait(event: E): S     // Suspending, returns new state

    // --- Lifecycle ---
    fun start(scope: CoroutineScope)          // Launches microstep loop
    fun stop()                                // Cancels processing

    // --- Generated Code Overrides ---
    abstract val initialState: S
    abstract fun processEvent(state: S, event: E): TransitionResult<S>  // Pure function
    abstract fun onEntry(state: S)                                       // Entry actions
    abstract fun onExit(state: S)                                        // Exit actions
    abstract fun executeTransitionActions(source: S, event: E)           // Transition actions
}
```

### TransitionResult (No Lambdas)

```kotlin
sealed interface TransitionResult<out S> {
    data class External<S>(val target: S) : TransitionResult<S>   // State change
    data object Internal : TransitionResult<Nothing>               // Actions only, same state
    data object Ignored : TransitionResult<Nothing>                // No matching transition
}
```

Actions are NOT embedded in TransitionResult. The engine calls `onExit()`, `executeTransitionActions()`, `onEntry()` in W3C-specified order.

### TransitionRecord (Debugging)

```kotlin
data class TransitionRecord<S, E>(
    val source: S,
    val event: E,
    val target: S,
    val timestamp: Long
)
```

`SharedFlow` (no conflation) delivers every transition for logging/debugging. `StateFlow` (conflation) delivers only the latest state for UI.

### Internal Event Queue

```kotlin
// Channel.UNLIMITED — never blocks, never drops
private val channel = Channel<E>(Channel.UNLIMITED)
```

- `send(event)` calls `channel.trySend(event)` — always succeeds, non-suspending
- Microstep loop: `for (event in channel) { processMicrostep(event) }`
- Internal events (from `<raise>`) are queued ahead of external events (W3C SCXML 3.12.1)

### Threading Model

```kotlin
fun start(scope: CoroutineScope) {
    scope.launch(Dispatchers.Default) {   // Always background
        for (event in channel) {
            processMicrostep(event)
            // StateFlow.value = newState  (thread-safe, Compose collects on Main)
        }
    }
}
```

- Microstep loop: `Dispatchers.Default` (never blocks UI)
- State observation: `StateFlow` is thread-safe, Compose collects on `Main`
- Delayed sends: `launch { delay(ms); send(event) }` — coroutine-native, `TestScope`-compatible

---

## 4. Android Integration Pattern

### ViewModel + Compose

```kotlin
class PlayerViewModel : ViewModel() {
    private val sm = PlayerStateMachine()
    val state: StateFlow<PlayerState> = sm.currentState

    init { sm.start(viewModelScope) }     // Auto-cancelled on ViewModel.onCleared()

    fun onPlay()  = sm.send(PlayerEvent.Play)
    fun onPause() = sm.send(PlayerEvent.Pause)
    fun onStop()  = sm.send(PlayerEvent.Stop)
}

@Composable
fun PlayerScreen(viewModel: PlayerViewModel) {
    val state by viewModel.state.collectAsState()
    when (state) {
        is PlayerState.Stopped -> StoppedView()
        is PlayerState.Playing -> PlayingView()
        is PlayerState.Paused  -> PausedView()
        // Compiler error if a state is missing
    }
}
```

### Dependency Injection (Hilt)

Generated state machines are plain classes — no DI framework dependency:

```kotlin
@Module @InstallIn(ViewModelComponent::class)
object StateMachineModule {
    @Provides fun providePlayerSM(): PlayerStateMachine = PlayerStateMachine()
}
```

### Unit Testing

`processEvent` is a pure function — no runtime, no coroutines needed:

```kotlin
@Test fun `play from stopped transitions to playing`() {
    val sm = PlayerStateMachine()
    val result = sm.processEvent(PlayerState.Stopped, PlayerEvent.Play)
    assertEquals(TransitionResult.External(PlayerState.Playing), result)
}
```

Full integration with Turbine for StateFlow testing:

```kotlin
@Test fun `full lifecycle test`() = runTest {
    val sm = PlayerStateMachine()
    sm.start(this)
    sm.currentState.test {
        assertEquals(PlayerState.Stopped, awaitItem())
        sm.send(PlayerEvent.Play)
        assertEquals(PlayerState.Playing, awaitItem())
    }
}
```

### W3C Compliance Testing

202 W3C SCXML tests — same set as C++ AOT. All registered, phased enablement:

| Category | Count | Phase 1 | Phase 2 |
|----------|-------|---------|---------|
| Pure Static SIMPLE | 43 | RUN | RUN |
| Pure Static SCHEDULED | 20 | RUN | RUN |
| Pure Static HTTP | 7 | SKIP | SKIP (Phase 3) |
| Script Engine SIMPLE | 113 | SKIP | RUN |
| Script Engine SCHEDULED | 14 | SKIP | RUN |
| Script Engine HTTP | 5 | SKIP | SKIP (Phase 3) |
| **Total** | **202** | **63 RUN** | **190 RUN** |

Test harness pattern (mirrors C++ `SimpleAotTest` CRTP):

```kotlin
// Base harness — equivalent to C++ SimpleAotTest<SM, TestId>
abstract class W3CTestBase {
    protected fun <S : State, E : Event> runW3CTest(
        factory: () -> StateMachineEngine<S, E>,
        passState: S,
        timeoutMs: Long = 5000
    ) = runTest {
        val sm = factory()
        sm.start(this)
        withTimeout(timeoutMs) {
            while (!sm.isInFinalState) delay(10)
        }
        assertEquals(passState, sm.currentState.value)
        sm.stop()
    }
}

// Per-test class — auto-generated from SCXML metadata
class Test144 : W3CTestBase() {
    @Test fun `W3C 4-2 raise event ordering`() = runW3CTest(
        ::Test144StateMachine, Test144State.Pass
    )
}
```

Project structure:

```
(project root)
├── settings.gradle.kts              # Multi-module root
├── sce-kotlin-runtime/              # Runtime library (KMP)
├── sce-kotlin-tests/                # W3C test project (JVM)
│   ├── build.gradle.kts
│   └── src/
│       ├── main/kotlin/com/sce/generated/   # 63 SM files (codegen output)
│       └── test/kotlin/com/sce/w3c/
│           ├── W3CTestBase.kt               # Shared harness
│           └── W3CKotlinTests.kt            # 202 test classes
└── tools/codegen/
    └── generate_kotlin_w3c.py       # Generates SM + test code
```

No Kotlin Interpreter needed — C++ Interpreter + AOT (404 executions) already
cross-validates SCXML correctness. Kotlin AOT validates codegen equivalence.

---

## 5. Generated Code Shape

### Target Output: `PlayerSm.kt`

```kotlin
// GENERATED CODE — DO NOT EDIT
// Source: player.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.player

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface PlayerState : State {
    data object Stopped : PlayerState
    data object Playing : PlayerState
    data object Paused  : PlayerState
}

// --- Events ---

sealed interface PlayerEvent : Event {
    data object Play  : PlayerEvent
    data object Pause : PlayerEvent
    data object Stop  : PlayerEvent
    sealed interface Error : PlayerEvent {
        data object Execution : Error
    }
}

// --- State Machine ---

class PlayerStateMachine : StateMachineEngine<PlayerState, PlayerEvent>() {

    // Datamodel (W3C SCXML 5.3)
    var trackIndex: Int = 0

    override val initialState: PlayerState = PlayerState.Stopped

    // Pure function: (State, Event) -> TransitionResult
    override fun processEvent(
        state: PlayerState,
        event: PlayerEvent
    ): TransitionResult<PlayerState> = when (state) {
        is PlayerState.Stopped -> processStopped(event)
        is PlayerState.Playing -> processPlaying(event)
        is PlayerState.Paused  -> processPaused(event)
    }

    // --- Per-State Handlers ---

    private fun processStopped(event: PlayerEvent) = when (event) {
        is PlayerEvent.Play -> TransitionResult.External(PlayerState.Playing)
        else -> TransitionResult.Ignored
    }

    private fun processPlaying(event: PlayerEvent) = when (event) {
        is PlayerEvent.Pause -> TransitionResult.External(PlayerState.Paused)
        is PlayerEvent.Stop  -> TransitionResult.External(PlayerState.Stopped)
        else -> TransitionResult.Ignored
    }

    private fun processPaused(event: PlayerEvent) = when (event) {
        is PlayerEvent.Play -> TransitionResult.External(PlayerState.Playing)
        is PlayerEvent.Stop -> TransitionResult.External(PlayerState.Stopped)
        else -> TransitionResult.Ignored
    }

    // --- Entry Actions (W3C SCXML 3.8) ---

    override fun onEntry(state: PlayerState) = when (state) {
        is PlayerState.Playing -> { trackIndex = 0 }
        else -> {}
    }

    // --- Exit Actions (W3C SCXML 3.9) ---

    override fun onExit(state: PlayerState) = when (state) {
        else -> {}
    }

    // --- Transition Actions ---

    override fun executeTransitionActions(
        source: PlayerState,
        event: PlayerEvent
    ) = when (source) {
        else -> {}
    }
}
```

### Properties of Generated Code

- Exhaustive `when` expressions — compiler enforces all states/events handled
- Per-state handler functions — readable at scale (50+ states)
- Pure `processEvent` — no side effects, trivially testable
- Actions separated into `onEntry`/`onExit`/`executeTransitionActions` — engine controls W3C ordering
- `DO NOT EDIT` header — generated from SCXML, regenerated every build

---

## 6. Build Integration

### Gradle (Android / KMP projects)

```kotlin
// build.gradle.kts
plugins {
    id("com.sce.codegen") version "1.0.0"  // Future: Gradle plugin
}

// Or manual task:
tasks.register<Exec>("generateScxml") {
    inputs.files(fileTree("src/main/scxml") { include("*.scxml") })
    inputs.dir("tools/codegen/templates/kotlin/")
    outputs.dir("build/generated/scxml")
    commandLine("python3", "codegen.py",
        "src/main/scxml/player.scxml",
        "-o", "build/generated/scxml",
        "--language", "kotlin")
}

tasks.named("compileKotlin") { dependsOn("generateScxml") }

sourceSets { main { kotlin.srcDir("build/generated/scxml") } }
```

### CMake (Hybrid C++/Kotlin projects)

```cmake
sce_add_state_machine(
    TARGET my_app
    SCXML_FILE player.scxml
    LANGUAGE kotlin                    # New parameter
    OUTPUT_DIR ${CMAKE_BINARY_DIR}/generated/kotlin
)
```

- `LANGUAGE kotlin` passes `--language kotlin` to codegen.py
- Output: `.kt` files instead of `.h`/`.inl`
- No `target_sources` or `target_include_directories` for Kotlin (Gradle handles compilation)

### Generated File Protection

- Output directory: `build/generated/scxml/` (build artifact, not source tree)
- `.gitignore`: `build/`, `generated/` already excluded
- Gradle `inputs`/`outputs`: skip regeneration if SCXML unchanged (`UP-TO-DATE`)
- `gradle clean`: removes all generated files

---

## 7. Package Structure

### Runtime Library

```
sce-kotlin-runtime/
├── build.gradle.kts
└── src/
    └── commonMain/kotlin/com/sce/runtime/
        ├── State.kt                    # State marker interface
        ├── Event.kt                    # Event marker interface
        ├── StateMachineEngine.kt       # Abstract engine base class
        ├── TransitionResult.kt         # Sealed transition result
        └── TransitionRecord.kt         # Transition history record
```

### Script Engine Libraries (Phase 2)

```
sce-kotlin-quickjs/                     # QuickJS JNI — ECMAScript 직접 평가
├── build.gradle.kts
└── src/
    ├── jvmMain/kotlin/                 # JNI bridge → quickjs.h
    ├── nativeMain/kotlin/              # Kotlin/Native cinterop
    └── commonMain/kotlin/com/sce/scripting/quickjs/
        └── QuickJSScriptEngine.kt

sce-kotlin-lua/                         # Lua 5.4 JNI + ECMAScript→Lua transformer
├── build.gradle.kts
└── src/
    ├── jvmMain/kotlin/                 # JNI bridge → lua.h
    ├── nativeMain/kotlin/              # Kotlin/Native cinterop
    └── commonMain/kotlin/com/sce/scripting/lua/
        └── LuaScriptEngine.kt
```

`ScriptEngine` interface lives in `sce-kotlin-runtime` (commonMain) — zero engine dependency for Phase 1.

### Generated Code

```
build/generated/scxml/
└── com/sce/generated/{machineName}/
    └── {MachineName}Sm.kt             # Single file: State + Event + SM
```

### Maven Coordinates

```
com.sce:sce-kotlin-runtime:1.0.0      # Runtime library + ScriptEngine interface
com.sce:sce-kotlin-quickjs:1.0.0      # QuickJS engine (Phase 2, Android default)
com.sce:sce-kotlin-lua:1.0.0          # Lua 5.4 engine (Phase 2, C++ parity)
com.sce:sce-kotlin-android:1.0.0      # Android extensions (Phase 3)
com.sce:sce-gradle-plugin:1.0.0       # Gradle plugin (Phase 3)
```

---

## 8. Phased Implementation

### Phase 1: Pure Static Pipeline (Current)

Scope: SCXML without script engine (`needs_script_engine = false`) — 63/202 W3C tests

| Component | Status | Deliverable |
|-----------|--------|-------------|
| `KotlinCodeGenerator` | Done | `generators/kotlin_generator.py` — 456 lines, 7 Jinja2 filters |
| Jinja2 templates | Done | `templates/kotlin/` — 15 files (states, events, process_event, actions, entry_exit) |
| Runtime library | Done | `StateMachineEngine`, `TransitionResult`, `StateFlow`, coroutines |
| CMake extension | Done | `LANGUAGE` parameter in `sce_add_state_machine()` |
| W3C test infrastructure | **Pending** | 202 tests registered, 63 RUN / 139 SKIP |
| Parallel exit fix | **Pending** | isDescendantOf, documentOrder, recursive exit |

Supported W3C features:
- States, transitions, events (exhaustive `when`)
- Entry/exit actions (`onEntry`/`onExit`)
- Datamodel variables (Kotlin properties — `Int`, `String`, `Boolean`)
- `<raise>` (internal event queue)
- `<if>`/`<elseif>`/`<else>` (Kotlin `when`/`if`)
- `<foreach>` (Kotlin `forEach`)
- `<assign>` (property assignment)
- `<send>` without delay (event queue)
- `<send>` with delay (coroutine `delay()` + `Job`)
- `<cancel>` (`Job.cancel()`)
- History states (nullable properties)
- Parallel states (flat `activeStateIds` tracking)
- Final states (`done.state.*` event emission)

Known gaps (vs C++ AOT):

| Gap | Severity | W3C Section |
|-----|----------|-------------|
| Parallel exit: no recursive descendant exit | Critical | 3.4, 3.13 |
| Missing `isDescendantOf()` / `getDocumentOrder()` | Critical | 3.4, 3.13 |
| Missing `ParallelCompletionHelper` (done.state for parallel) | High | 3.7.1 |
| Exhaustive `when`: not all states get branch (relies on `else`) | Medium | — |
| Parallel state own transitions skipped in `processEvent` | Medium | 3.4 |

Not supported in Phase 1:
- ECMAScript expressions (`cond="x > 5"`)
- `<script>` blocks
- Dynamic invoke (`srcexpr`)
- `_event.data` access

### Phase 2: Script Engine (Future)

Target: 190/202 W3C tests (all except HTTP)

Both engines supported — compile-time selection, mirrors C++ `ScriptEngineProvider`:

| Engine | Kotlin Module | ECMAScript 처리 | C++ 대응 |
|--------|--------------|----------------|----------|
| **QuickJS** | `sce-kotlin-quickjs` | 직접 평가 (변환 불필요) | `SCE_SCRIPT_ENGINE=quickjs` |
| **Lua 5.4** | `sce-kotlin-lua` | ECMAScript→Lua 트랜스포머 필요 | `SCE_SCRIPT_ENGINE=lua` |

Architecture:

```
C++:
  CMake: -DSCE_SCRIPT_ENGINE=lua|quickjs
  C++:   #if defined(SCE_SCRIPT_ENGINE_LUA) → LuaEngine::instance()
         #elif defined(SCE_SCRIPT_ENGINE_QUICKJS) → JSEngine::instance()
  Interface: IScriptEngine

Kotlin:
  Gradle: -PsceScriptEngine=lua|quickjs
  Kotlin: ServiceLoader / dependency injection → LuaScriptEngine | QuickJSScriptEngine
  Interface: ScriptEngine
```

```kotlin
// Interface (commonMain — sce-kotlin-runtime)
interface ScriptEngine {
    fun evaluate(session: String, expr: String): Any?
    fun setVariable(session: String, name: String, value: Any?)
    fun getVariable(session: String, name: String): Any?
    fun createSession(sessionId: String)
    fun destroySession(sessionId: String)
}

// QuickJS implementation (sce-kotlin-quickjs) — JNI binding
// ECMAScript 직접 평가, 트랜스포머 불필요
class QuickJSScriptEngine : ScriptEngine { /* JNI → quickjs.h */ }

// Lua implementation (sce-kotlin-lua) — JNI binding
// ECMAScript→Lua 트랜스포머 + Lua 5.4 평가
class LuaScriptEngine : ScriptEngine { /* JNI → lua.h + transformer */ }

// build.gradle.kts — compile-time selection
val sceScriptEngine: String by project  // -PsceScriptEngine=quickjs (default for Android)
dependencies {
    when (sceScriptEngine) {
        "quickjs" -> implementation(project(":sce-kotlin-quickjs"))
        "lua" -> implementation(project(":sce-kotlin-lua"))
    }
}
```

Key deliverables:
- `ScriptEngine` interface in `sce-kotlin-runtime` (commonMain)
- `sce-kotlin-quickjs/` — QuickJS JNI, ECMAScript 직접 평가
- `sce-kotlin-lua/` — Lua 5.4 JNI, C++ transformer pipeline 공유
- Static Hybrid codegen (static structure + runtime expression eval)
- `_event.data`, `_event.name` via script engine bindings
- `<script>` blocks

### Phase 3: Advanced Features (Future)

Target: 202/202 W3C tests

- `<invoke>` with static child state machines
- HTTP BasicEventProcessor (`<send type="BasicHTTPEventProcessor">`) — 12 tests
- `sce-kotlin-android` library (Lifecycle, SavedStateHandle integration)
- Gradle plugin for seamless SCXML-to-Kotlin pipeline

---

## 9. C++ vs Kotlin Mapping Reference

| C++ Concept | Kotlin Equivalent |
|-------------|------------------|
| `enum class State` | `sealed interface State` + `data object` |
| `enum class Event` | `sealed interface Event` + `data object` |
| `StaticExecutionEngine<Policy>` (CRTP) | `StateMachineEngine<S, E>` (abstract class + generics) |
| `std::optional<State>` | `State?` (nullable) |
| `switch(state)` | `when(state)` (exhaustive expression) |
| `EventQueueManager` (FIFO) | `Channel<E>(UNLIMITED)` |
| `std::thread` + mutex | `CoroutineScope` + `Dispatchers.Default` |
| `SendSchedulingHelper` (timer) | `launch { delay(ms); send(event) }` |
| `Job.cancel()` for `<cancel>` | `Job.cancel()` (identical concept) |
| `processTransition()` per state | `processXxx(event)` per state |
| `.h` + `.inl` files | Single `.kt` file |
| CMake `add_custom_command` | Gradle `inputs`/`outputs` task |
| `DEPFILE` tracking | Gradle incremental build |
| `sce_core` (header-only) | `commonMain` (multiplatform) |
| `#if defined(SCE_SCRIPT_ENGINE_LUA)` | Gradle `-PsceScriptEngine=lua` dependency |
| `ScriptEngineProvider::getScriptEngine()` | `ScriptEngineProvider.engine` (DI / ServiceLoader) |
| `IScriptEngine` interface | `ScriptEngine` interface |
| `LuaEngine::instance()` (singleton) | `LuaScriptEngine` (Lua 5.4 JNI) |
| `JSEngine::instance()` (singleton) | `QuickJSScriptEngine` (QuickJS JNI) |
| `isDescendantOf()` (HierarchicalStateHelper) | Generated `parentOf` map + walk (Phase 1 gap) |
| `getDocumentOrder()` (generated switch) | Generated `documentOrder` map (Phase 1 gap) |
| `isParallelState()` (generated switch) | Generated set or `when` check (Phase 1 gap) |
| `ParallelCompletionHelper` (shared) | Generated inline check (Phase 1 gap) |
| `SimpleAotTest<SM, ID>` (CRTP) | `W3CTestBase.runW3CTest()` (generics) |
| `w3c_test_cli` (202 AOT + 202 Interp) | `gradle test` (202 Kotlin AOT only) |

---

## 10. Architecture Decisions Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2025-03 | Phase 1: Pure static only | Validate codegen pipeline before script engine complexity |
| 2025-03 | sealed interface over enum class | Future parallel `data class`, compound nesting, per-state data |
| 2025-03 | Single .kt per SM | Kotlin allows multiple public declarations; simplifies build |
| 2025-03 | No Kotlin Interpreter | C++ Interp+AOT cross-validates; Kotlin AOT validates codegen only |
| 2026-04 | Phase 2 engines: QuickJS + Lua 5.4 (both) | C++ parity — both engines, compile-time selection via Gradle |
| 2026-04 | QuickJS default for Android | ECMAScript 직접 평가 (트랜스포머 불필요), Android 검증됨 |
| 2026-04 | Compile-time engine selection via Gradle | Mirrors C++ `-DSCE_SCRIPT_ENGINE=lua\|quickjs` |
| 2026-04 | 202 W3C test registration | Full C++ AOT parity; Phase 1 runs 63, Phase 2 runs 190, Phase 3 runs 202 |
