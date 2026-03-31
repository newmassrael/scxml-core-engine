# Kotlin Code Generation Design

Target: Modern Android (Kotlin-first, Jetpack Compose, Coroutines)

Minimum: Kotlin 1.9+, kotlinx-coroutines 1.7+, Kotlin Multiplatform compatible

## Design Principles

1. **Native Kotlin feel** — Generated code should look like hand-written Kotlin, not a C++ port
2. **Compose-ready** — StateFlow-based state observation, direct `collectAsState()` integration
3. **Zero-overhead Phase 1** — Pure static only, no script engine, no reflection, no runtime parsing
4. **SCXML is the source of truth** — Generated `.kt` files are build artifacts, regenerated every build

---

## Decision Summary

| Decision | Choice | Rationale |
|----------|--------|-----------|
| State representation | `sealed interface` + `data object` | Hierarchy, data, exhaustive `when`, Compose UiState pattern |
| Event representation | `sealed interface` + `data object` | W3C prefix matching via type hierarchy, future event data |
| Script engine | Pure static (Phase 1) | Pipeline validation first; QuickJS JNI in Phase 2 |
| Runtime concurrency | `StateFlow` + `Channel` + Coroutines | Android standard async/state management |
| Build / distribution | KMP Gradle, Maven Central | commonMain for multiplatform, androidMain for lifecycle |
| Generated file layout | Single `.kt` per state machine | Kotlin allows multiple public declarations per file |
| Kotlin minimum version | 1.9+ | `data object` support (released July 2023) |
| Microstep threading | `Dispatchers.Default` (forced) | Never block UI thread |
| `send()` API | Non-suspending, `Channel.UNLIMITED` | UI event handlers are not suspend functions |

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

When script engine support is added, events carry W3C metadata:

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

### Testing

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
        ├── TransitionRecord.kt         # Transition history record
        └── helpers/                    # W3C algorithm helpers
            ├── ConflictResolver.kt     # W3C D.2
            ├── StateHierarchy.kt       # W3C 3.7, 3.8
            └── EventQueue.kt          # W3C 3.12.1 (internal FIFO)
```

### Generated Code

```
build/generated/scxml/
└── com/sce/generated/{machineName}/
    └── {MachineName}Sm.kt             # Single file: State + Event + SM
```

### Maven Coordinates

```
com.sce:sce-kotlin-runtime:1.0.0      # Runtime library (commonMain)
com.sce:sce-kotlin-android:1.0.0      # Android extensions (future)
com.sce:sce-gradle-plugin:1.0.0       # Gradle plugin (future)
```

---

## 8. Phased Implementation

### Phase 1: Pure Static Pipeline (Current Target)

Scope: SCXML without script engine (`needs_script_engine = false`)

| Component | Deliverable |
|-----------|-------------|
| `KotlinCodeGenerator` | `generators/kotlin_generator.py` — skeleton with filters |
| Jinja2 templates | `templates/kotlin/` — state_machine, process_transition, actions, entry_exit |
| Runtime library | `StateMachineEngine`, `TransitionResult`, `StateFlow` integration |
| CMake extension | `LANGUAGE` parameter in `sce_add_state_machine()` |
| Validation | Generate + compile + run a simple 3-state SCXML |

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
- Parallel states (`data class` with region properties)
- Final states (`done.state.*` event emission)

Not supported in Phase 1:
- ECMAScript expressions (`cond="x > 5"`)
- `<script>` blocks
- Dynamic invoke (`srcexpr`)
- `_event.data` access

### Phase 2: Script Engine (Future)

- QuickJS via JNI for Android ECMAScript evaluation
- `EcmaScriptToKotlinTransformer` (or reuse QuickJS directly)
- Static Hybrid generation (static structure + runtime expressions)

### Phase 3: Advanced Features (Future)

- `<invoke>` with static child state machines
- HTTP BasicEventProcessor (`<send type="BasicHTTPEventProcessor">`)
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
