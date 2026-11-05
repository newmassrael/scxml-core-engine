// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The sessions of one Kotlin session tree, as the inboxes of its sessions are
// reached by (docs/adr/0084, ARCHITECTURE.md "Engine Threading").

@file:OptIn(ExperimentalAtomicApi::class)

package com.sce.runtime

import kotlin.concurrent.atomics.AtomicReference
import kotlin.concurrent.atomics.ExperimentalAtomicApi

/**
 * One session tree: the machine a host built ([root]), the one waker its host
 * installed ([waker]), and the directory from the id of each child session the
 * tree holds to that session, whose inbox an input naming it is posted to.
 *
 * A parent hands its own tree to each child it starts or resumes, as it hands
 * the tree's invokers, so every session of the tree reads the same one. The
 * directory is read from any thread. It is written by the owners of the tree's
 * sessions, each for the children it starts and ends: in the coroutine mode a
 * child runs a loop of its own and starts its own children, so more than one
 * owner writes it, and each write replaces the whole map at once.
 */
internal class SessionTree(val root: StateMachineEngine<*, *>) {
    /** What the host installed through an inbox of the tree ([EngineInbox.wakeWith]). */
    @Volatile
    var waker: (() -> Unit)? = null

    private val children = AtomicReference<Map<String, StateMachineEngine<*, *>>>(emptyMap())

    /** The child session the tree holds under [id], or `null` when it holds none. */
    fun childNamed(id: String): StateMachineEngine<*, *>? = children.load()[id]

    /** Hold [child] under [id], the id its parent gave it. */
    fun enter(id: String, child: StateMachineEngine<*, *>) {
        change { held -> held + (id to child) }
    }

    /** Hold [child] no more; a later session held under the same [id] is kept. */
    fun leave(id: String, child: StateMachineEngine<*, *>) {
        change { held -> if (held[id] === child) held - id else held }
    }

    private inline fun change(next: (Map<String, StateMachineEngine<*, *>>) -> Map<String, StateMachineEngine<*, *>>) {
        while (true) {
            val held = children.load()
            if (children.compareAndSet(held, next(held))) return
        }
    }
}
