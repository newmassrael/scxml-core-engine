// SCE-GENERATED — DO NOT EDIT
// source-hash: 239a1140d1db8dfa52f222a8f17ddb51efb6fc1d0cddb120dc5b4b0ee2173ab0
// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/nested_lifecycle/nested_lifecycle.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: nested_lifecycle.scxml:27 :: _machine

package com.sce.integration.nested_lifecycle_transition_function

/*
 * The transition function of `nested_lifecycle`, generated because its
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
    Working("working"),
    Held("held"),
    Retrying("retrying"),
    Loading("loading"),
    Ready("ready"),
    Closed("closed"),
}

/**
 * An event the machine names: every literal name its transitions write. A
 * descriptor such as `error.*` or `*` names no event of its own. [scxmlName]
 * is the event's name, as the document writes it.
 */
enum class Event(val scxmlName: String) {
    Abort("abort"),
    Pause("pause"),
    Reset("reset"),
    Done("done"),
    Resume("resume"),
    DataReady("data.ready"),
    DataFailed("data.failed"),
    Go("go"),
}

/** Every state, in document order. */
val ALL_STATES: List<State> = listOf(
    State.Working,
    State.Held,
    State.Retrying,
    State.Loading,
    State.Ready,
    State.Closed,
)

/** Every event, in the order the transitions first name them. */
val ALL_EVENTS: List<Event> = listOf(
    Event.Abort,
    Event.Pause,
    Event.Reset,
    Event.Done,
    Event.Resume,
    Event.DataReady,
    Event.DataFailed,
    Event.Go,
)

/** The state the machine starts in. */
val INITIAL: State = State.Loading

/**
 * The digest of the source set this file was generated from: the
 * `source-hash` of its own header, as a value.
 */
const val SOURCE_HASH: String = "239a1140d1db8dfa52f222a8f17ddb51efb6fc1d0cddb120dc5b4b0ee2173ab0"

/**
 * Where [event] takes the machine from [state]: the state it is in
 * afterwards, which is [state] itself when the transition taken has no
 * target, or `null` when no transition takes the event and the machine
 * discards it. A final state takes no event.
 */
fun transition(state: State, event: Event): State? = when (state) {
    State.Working -> when (event) {
        Event.Abort -> State.Closed
        Event.Pause -> State.Held
        Event.Reset -> State.Working
        Event.Done -> State.Closed
        Event.Resume -> State.Working
        else -> null
    }
    State.Held -> when (event) {
        Event.Abort -> State.Held
        Event.Pause -> State.Held
        Event.Reset -> State.Held
        Event.Done -> State.Held
        Event.Resume -> State.Working
        Event.DataReady -> State.Held
        Event.DataFailed -> State.Held
        Event.Go -> State.Held
    }
    State.Retrying -> when (event) {
        Event.Abort -> State.Closed
        Event.DataReady -> State.Ready
        Event.DataFailed -> State.Loading
        else -> null
    }
    State.Loading -> when (event) {
        Event.Abort -> State.Closed
        Event.DataReady -> State.Ready
        Event.DataFailed -> State.Retrying
        else -> null
    }
    State.Ready -> when (event) {
        Event.Abort -> State.Closed
        Event.Go -> State.Working
        else -> null
    }
    State.Closed -> null
}

/**
 * Whether [state] is a `<final>` child of `<scxml>`: reaching it ends the
 * machine, and it takes no event.
 */
fun isFinal(state: State): Boolean = state == State.Closed

/**
 * The ids of every state active while [state] is the current state,
 * outermost first and [state] last.
 */
fun configuration(state: State): List<String> = when (state) {
    State.Working -> listOf("session", "active", "working")
    State.Held -> listOf("session", "active", "held")
    State.Retrying -> listOf("session", "setup", "retrying")
    State.Loading -> listOf("session", "setup", "loading")
    State.Ready -> listOf("session", "setup", "ready")
    State.Closed -> listOf("closed")
}

/** The state the id [name] names, or `null` when it names none of [ALL_STATES]. */
fun stateFromName(name: String): State? = when (name) {
    "working" -> State.Working
    "held" -> State.Held
    "retrying" -> State.Retrying
    "loading" -> State.Loading
    "ready" -> State.Ready
    "closed" -> State.Closed
    else -> null
}

/**
 * The event [name] names, or `null` when it names none of [ALL_EVENTS]. A
 * name is matched whole: the function's events are the names its document
 * writes, and a longer name a descriptor would match is not one of them.
 */
fun eventFromName(name: String): Event? = when (name) {
    "abort" -> Event.Abort
    "pause" -> Event.Pause
    "reset" -> Event.Reset
    "done" -> Event.Done
    "resume" -> Event.Resume
    "data.ready" -> Event.DataReady
    "data.failed" -> Event.DataFailed
    "go" -> Event.Go
    else -> null
}
