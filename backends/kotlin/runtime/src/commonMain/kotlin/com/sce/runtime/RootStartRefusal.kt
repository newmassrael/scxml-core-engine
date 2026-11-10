// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.runtime

/**
 * Why [StateMachineEngine.initializeAsRoot] or [StateMachineEngine.startAsRoot]
 * refused to start a machine.
 *
 * The default is to run a machine that needs a parent — its `#_parent` sends
 * then raise `error.communication` (§scxml-C-1). A host that would rather not
 * start such a machine, when it has none to give, asks for the refusal by
 * starting it with one of the `AsRoot` entry points.
 *
 * Those entry points refuse for either of two reasons, the two every engine
 * spells `NeedsParent` and `ScriptSession`: [NeedsParent] before anything is
 * established, and [ScriptSession] when the machine's script session cannot
 * be established (docs/adr/0096). A refusal starts nothing.
 */
sealed class RootStartRefusal {
    /** The refusal as a sentence, for a host to report — the same words on every engine. */
    abstract val reason: String

    /** The [reason], which is what a host prints. */
    final override fun toString(): String = reason

    /**
     * §scxml-6.2.4: the document sends to `#_parent`, and a session its host
     * started has no parent to reach.
     */
    data object NeedsParent : RootStartRefusal() {
        override val reason: String = "the machine sends to #_parent and was started with no parent session"
    }

    /**
     * The machine's script session could not be established, so nothing was
     * entered and nothing ran; [failure] names the step and carries the
     * engine's words, which are the [reason].
     */
    data class ScriptSession(
        /** Why the session could not be established. */
        val failure: ScriptSessionFailure,
    ) : RootStartRefusal() {
        override val reason: String get() = failure.toString()
    }

    companion object {
        /**
         * [NeedsParent] under the name it had while this was an enumeration of
         * one value, so a host that names the refusal that way still compiles.
         */
        val NEEDS_PARENT: NeedsParent = NeedsParent
    }
}
