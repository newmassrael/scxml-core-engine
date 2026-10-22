// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The machines the step server can host, by the name a world asks for them under. Each is
// compiled from the generated tree of `:sce-kotlin-tests` (see `hostedMachines` in this
// module's build script); a machine is added here and there together.

package com.sce.world

import com.sce.integration.todo_peer.TodoPeerStateMachine

/**
 * The wall clock reading a machine is saved and restored against. A saved delayed send
 * records the moment it comes due on a wall clock, and a restart in a world is a restore
 * at the same instant, so one fixed reading on both sides keeps the time each send still
 * has to wait, and no reading of the host's clock enters a saved state. The Rust engine
 * is saved at the same reading (`save_at(0)`).
 */
private const val AT_WALL_MS = 0L

val hostableMachines: Map<String, (sessionId: String?) -> HostedMachine> =
    mapOf(
        "todo_peer" to
            hosting(
                create = { sessionId -> TodoPeerStateMachine(sessionId = sessionId) },
                save = { it.save(AT_WALL_MS) },
                restore = { machine, saved -> machine.restore(saved, AT_WALL_MS) },
            ),
    )
