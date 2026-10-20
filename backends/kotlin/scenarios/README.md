# sce-kotlin-scenarios

Replay of an engine-neutral scenario against a generated Kotlin machine
(`datamodel="sce-static"`, docs/SCE_ACCEPTED_SUBSET.md §2.15). The Kotlin twin of
`backends/rust/scenarios` (`sce-rust-scenarios`).

A scenario is data: a list of steps, each an external event with its payload and
what the machine must hold after it runs to quiescence: its current state, the
set of its active states (`configuration`), any of its variables, what it asked
of its host (`host_calls`), what each child session was given as its host and
asked of it (`child_hosts`) and what it sent its Mesh peers (`sent`). What the
machine holds is read from its saved state, the text every backend saves byte for
byte, so one scenario judges every backend by the same answer and needs no
per-type glue. The members are listed in docs/SCE_ACCEPTED_SUBSET.md, "A scenario
of a static machine"; this repository's own scenarios are
`sce-build/tests/fixtures/static_datamodel/scenarios/*.json`.

This module is what `:sce-kotlin-tests` replays those scenarios with, and what a
project outside this repository that generates its own machines replays its own
scenarios with, so it can hold its Kotlin machines to the same scenarios it holds
its Rust machines to.

## Use from a project outside this repository

1. Generate the machine from the project's SCXML:

   ```sh
   sce-codegen generate machine.scxml --language kotlin --output-dir src/main/kotlin
   ```

2. Depend on the runtime and this module. They are not published to a
   repository, so a Gradle build reaches them as a composite build of an SCE
   checkout, by the coordinates the SCE build gives them:

   ```kotlin
   // settings.gradle.kts
   includeBuild("/path/to/scxml-core-engine")

   // build.gradle.kts
   dependencies {
       implementation("com.sce:sce-kotlin-runtime:1.0.0")
       // A datamodel="sce-static" machine checks its integer operations with
       // the forge runtime.
       implementation("com.sce.forge:sce-forge-runtime-kotlin:0.1.0")
       testImplementation("com.sce:sce-kotlin-scenarios:1.0.0")
   }
   ```

   The module is Kotlin Multiplatform with a JVM target, so a JVM build and an
   Android build consume it alike.

3. Read the scenario and replay it with the machine's own functions:

   ```kotlin
   import com.sce.runtime.EventMetadata
   import com.sce.scenarios.parseScenario
   import com.sce.scenarios.replay
   import java.io.File

   val sm = CounterStateMachine()
   sm.initialize()
   try {
       replay(
           parseScenario(File("scenarios/counter.json").readText()),
           send = { name, data -> sm.sendEventByName(name, EventMetadata(data = data)) },
           tick = { sm.tick() },
           save = { sm.save() },
           ended = { sm.isInFinalState },
       )
   } finally {
       sm.cleanup()
   }
   ```

The driver is a set of functions rather than a machine, so one replay holds every
generated machine. The optional ones:

| Parameter | Give it when the machine | Usually |
|---|---|---|
| `donedata` | ends in a top-level `<final>` with `<donedata>` a step states | `{ sm.donedataAtFinal() }` |
| `advance` | runs on a manual clock a step's `advance_ms` moves | `sm.clock = ManualClock(0)` before `initialize`, then `{ ms -> sm.advanceTimeMs(ms) }` |
| `hostCalls` | performs `<sce:action>`s through a host a step states `host_calls` of | the generated `Recording<Machine>Actions`, each call turned into `hostCall(...)` and cleared |
| `childHosts` | answers a host for each invoked child | a `ChildHosts`, each child's host made with `open(...)`; pass `childHosts::take` |
| `sent` | sends to its Mesh peers (`<send target="#name">`) | a `RecordedSends` whose `router` is registered with `registerMeshRouter` before `initialize`; pass `{ sends.take() }` |

A machine with child sessions needs more than one round of `tick` before it is read
back: give it `tick = { repeat(SETTLE_TICKS) { sm.tick() } }`.

## Failures

The first step the machine does not hold throws `ScenarioMismatch`, an
`AssertionError` that names the step (counted from 0), its `note`, what was held
(`what`) and, for a comparison, both sides as JSON (`expected`, `actual`). A test
framework reports it as a failed assertion; a caller outside one catches it and
reads the fields. A step that states what the driver does not record (`host_calls`
with no `hostCalls`, an `advance_ms` with no `advance`) fails too: a step that
judged nothing would pass.

The Rust and Go replays refuse an event name no event of the machine matches
unless the step says `"dropped": true`. A Kotlin machine does not make that lookup
public, so here a step whose event was dropped fails on the state it expects, or,
for a step that expects the drop, on the variables it says did not move.
