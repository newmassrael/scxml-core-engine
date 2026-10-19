// SCE-GENERATED — DO NOT EDIT
// source-hash: c39067aa383d615204755340a5be95d22b2cb7f22bae673ca1ae1c99211df076
// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/flat_lifecycle/flat_lifecycle.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: flat_lifecycle.scxml:17 :: _machine

package com.sce.integration.flat_lifecycle_transition_function

/*
 * The transition function of `flat_lifecycle`, generated because its
 * document declares `sce:transition-function="emit"`.
 *
 * A pure function of (current state, event): [transition] answers where an
 * event takes the machine from a state, or `null` when no transition takes
 * the event and the machine discards it. Every pair was computed when this
 * file was generated, by the selection rule the machine itself follows, so
 * this file and the machine generated beside it agree on every pair. It
 * needs nothing beyond the Kotlin standard library.
 */

/**
 * A state the machine can be in: an atomic state, or a `<final>` child of
 * `<scxml>`. A compound state is never the current state, so it is none of
 * these; [configuration] names the compound states active with each.
 * [scxmlName] is the state's id, as the document writes it.
 */
enum class State(val scxmlName: String) {
    Idle("idle"),
    Running("running"),
    Paused("paused"),
}

/**
 * An event the machine names: every literal name its transitions write. A
 * descriptor such as `error.*` or `*` names no event of its own. [scxmlName]
 * is the event's name, as the document writes it.
 */
enum class Event(val scxmlName: String) {
    Start("start"),
    Ping("ping"),
    Pause("pause"),
    Tick("tick"),
    Resume("resume"),
    Stop("stop"),
}

/** Every state, in document order. */
val ALL_STATES: List<State> = listOf(
    State.Idle,
    State.Running,
    State.Paused,
)

/** Every event, in the order the transitions first name them. */
val ALL_EVENTS: List<Event> = listOf(
    Event.Start,
    Event.Ping,
    Event.Pause,
    Event.Tick,
    Event.Resume,
    Event.Stop,
)

/** The state the machine starts in. */
val INITIAL: State = State.Idle

/**
 * The digest of the source set this file was generated from: the
 * `source-hash` of its own header, as a value.
 */
const val SOURCE_HASH: String = "c39067aa383d615204755340a5be95d22b2cb7f22bae673ca1ae1c99211df076"

/**
 * Where [event] takes the machine from [state]: the state it is in
 * afterwards, which is [state] itself when the transition taken has no
 * target, or `null` when no transition takes the event and the machine
 * discards it. A final state takes no event.
 */
fun transition(state: State, event: Event): State? = when (state) {
    State.Idle -> when (event) {
        Event.Start -> State.Running
        Event.Ping -> State.Idle
        else -> null
    }
    State.Running -> when (event) {
        Event.Ping -> State.Running
        Event.Pause -> State.Paused
        Event.Tick -> State.Running
        else -> null
    }
    State.Paused -> when (event) {
        Event.Resume -> State.Running
        Event.Stop -> State.Idle
        else -> null
    }
}

/**
 * Whether [state] is a `<final>` child of `<scxml>`: reaching it ends the
 * machine, and it takes no event.
 */
@Suppress("UNUSED_PARAMETER")
fun isFinal(state: State): Boolean = false

/**
 * The ids of every state active while [state] is the current state,
 * outermost first and [state] last.
 */
fun configuration(state: State): List<String> = when (state) {
    State.Idle -> listOf("idle")
    State.Running -> listOf("running")
    State.Paused -> listOf("paused")
}

/** The state the id [name] names, or `null` when it names none of [ALL_STATES]. */
fun stateFromName(name: String): State? = when (name) {
    "idle" -> State.Idle
    "running" -> State.Running
    "paused" -> State.Paused
    else -> null
}

/**
 * The event [name] names, or `null` when it names none of [ALL_EVENTS]. A
 * name is matched whole: the function's events are the names its document
 * writes, and a longer name a descriptor would match is not one of them.
 */
fun eventFromName(name: String): Event? = when (name) {
    "start" -> Event.Start
    "ping" -> Event.Ping
    "pause" -> Event.Pause
    "tick" -> Event.Tick
    "resume" -> Event.Resume
    "stop" -> Event.Stop
    else -> null
}
