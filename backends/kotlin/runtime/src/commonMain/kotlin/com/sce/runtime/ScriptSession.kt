// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.runtime

// Why a machine's script session could not be established, which refuses its
// start (docs/adr/0096).
//
// A machine whose document needs a datamodel runs its conditions, assignments
// and scripts in a script session. Establishing it is three steps in this
// order: create the session, load the shared library into it (the ECMAScript
// semantics, the JSON builtins and the DOM read surface, where the engine
// carries them as chunks of its own), and bind the system variables
// `_sessionid`, `_name` and `_ioprocessors`. A machine started over a session
// one of them failed for would run with no datamodel, every condition an error
// and every assignment lost, and the document could not even handle it: its
// means of handling an error runs in the session that failed. So the start is
// refused, and the host is handed a [ScriptSessionFailure].
//
// Cross-language siblings: `sce_rust_runtime::ScriptSessionFailure` (Rust),
// `SCE::ScriptSessionFailure` (C++) and `sce.ScriptSessionFailure` (Go), whose
// step names and sentence are these.

/** The step of establishing a script session that failed. */
enum class ScriptSessionStage(
    /** The step's name, the same word on every backend. */
    val stepName: String,
) {
    /** The script engine could not create the session. */
    CREATE("create"),

    /**
     * The shared library did not load into the new session, which the engine
     * then did not keep ([ScriptLibraryNotLoadedException]). A library chunk
     * that does not load is a defect of the build that shipped it.
     */
    LIBRARY("library"),

    /**
     * The system variables `_sessionid`, `_name` and `_ioprocessors` could not
     * be bound; the session was destroyed.
     */
    SYSTEM_VARIABLES("system-variables"),
    ;

    override fun toString(): String = stepName
}

/**
 * Why a machine's script session could not be established, so its start was
 * refused: no state was entered, no executable content ran, no event was
 * taken, and what the failed steps made was released.
 *
 * [StateMachineEngine.initialize] and [StateMachineEngine.start] throw it in a
 * [ScriptSessionUnavailableException], [StateMachineEngine.initializeAsRoot]
 * and [StateMachineEngine.startAsRoot] answer it as
 * [RootStartRefusal.ScriptSession], and [StateMachineEngine.enterAt] carries it
 * in its [ConfigurationVerdict].
 */
data class ScriptSessionFailure(
    /** The step that failed. */
    val stage: ScriptSessionStage,
    /** What the script engine said. */
    val message: String,
) {
    /**
     * The failure as a sentence: the step that failed, then the engine's words,
     * the sentence every backend writes.
     */
    override fun toString(): String =
        "the script session could not be established at step '$stage': $message"

    /**
     * The data of the `error.execution` a parent raises for the child of the
     * `<invoke>` [invokeId] whose start this failure refused: the invoke, and
     * the step its child's script session failed at. The parent keeps no child
     * and raises no `done.invoke` for it, the words every backend writes.
     */
    fun childStartRefusal(invokeId: String): String =
        "<invoke id='$invokeId'> did not start: its child's script session failed at step '$stage'"

    companion object {
        /**
         * What [ScxmlScriptEngine.createSession] failed with: a shared library
         * that did not load (a [ScriptLibraryNotLoadedException] anywhere in
         * [failure]'s chain of causes) is the [ScriptSessionStage.LIBRARY]
         * step, any other failure the [ScriptSessionStage.CREATE] step.
         */
        fun ofCreation(failure: Throwable): ScriptSessionFailure {
            val library = generateSequence(failure) { it.cause }.any { it is ScriptLibraryNotLoadedException }
            val stage = if (library) ScriptSessionStage.LIBRARY else ScriptSessionStage.CREATE
            return ScriptSessionFailure(stage, wordsOf(failure))
        }

        /**
         * What [ScxmlScriptEngine.setupSystemVariables] failed with: the
         * [ScriptSessionStage.SYSTEM_VARIABLES] step.
         */
        fun ofSystemVariables(failure: Throwable): ScriptSessionFailure =
            ScriptSessionFailure(ScriptSessionStage.SYSTEM_VARIABLES, wordsOf(failure))

        /** What [failure] says, or the name of its class when it says nothing. */
        private fun wordsOf(failure: Throwable): String =
            failure.message ?: failure::class.simpleName ?: "an unnamed failure"
    }
}

/**
 * A machine's start was refused because its script session could not be
 * established ([failure] names the step), so nothing was entered and nothing
 * ran (docs/adr/0096).
 *
 * Thrown by [StateMachineEngine.initialize] and [StateMachineEngine.start],
 * before the run is marked running or its inbox opened; [StateMachineEngine.start]
 * throws it on the caller's thread, before it launches the coroutine of the
 * run. A machine without a script engine has no session to fail, and its start
 * never throws it.
 */
class ScriptSessionUnavailableException(
    /** Why the session could not be established. */
    val failure: ScriptSessionFailure,
) : RuntimeException(failure.toString())

/**
 * What a [ScxmlScriptEngine.createSession] throws when a chunk of the shared
 * library did not load into the new session. The engine keeps no session
 * then, and the start that asked for it is refused at the
 * [ScriptSessionStage.LIBRARY] step ([ScriptSessionFailure.ofCreation]), which
 * says the build is at fault rather than the machine or the device.
 *
 * The Go sibling is `sce.ScriptLibraryNotLoaded`.
 */
class ScriptLibraryNotLoadedException(
    /** The chunk that did not load, as `ECMAScript semantics`. */
    val library: String,
    /** What loading it failed with. */
    val reason: String,
    cause: Throwable? = null,
) : ScriptEngineException("failed to load $library: $reason", cause)
