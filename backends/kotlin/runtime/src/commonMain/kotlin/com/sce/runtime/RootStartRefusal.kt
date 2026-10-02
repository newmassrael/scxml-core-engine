// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.runtime

/**
 * Why [StateMachineEngine.initializeAsRoot] or [StateMachineEngine.startAsRoot]
 * refused to start a machine.
 *
 * The default is to run such a machine — its `#_parent` sends then raise
 * `error.communication` (§scxml-C-1). A host that would rather not start a
 * machine that needs a parent, when it has none to give, asks for the refusal
 * by starting it with one of the `AsRoot` entry points.
 */
enum class RootStartRefusal(
    /** The refusal as a sentence, for a host to report — the same words on every engine. */
    val reason: String,
) {
    /**
     * §scxml-6.2.4: the document sends to `#_parent`, and a session its host
     * started has no parent to reach.
     */
    NEEDS_PARENT("the machine sends to #_parent and was started with no parent session"),
}
