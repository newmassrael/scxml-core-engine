// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.hostfeatures

import com.sce.runtime.Event
import com.sce.runtime.State
import com.sce.runtime.StateMachineEngine

/**
 * Register [host] as the invoker of every standard type on [engine], before it starts.
 *
 * A start is answered with its completion (a failure is a completion whose `error` is a code)
 * or its refusal, or not at all when the host holds it. The cancel the engine delivers when a
 * state exits or a deadline passes reaches the host too.
 */
fun <S : State, E : Event> serve(engine: StateMachineEngine<S, E>, host: ReferenceHost) {
    for (operation in TYPES) {
        engine.registerInvoker(operation) { event ->
            val cancel = event.cancel
            val start = event.start
            when {
                cancel != null -> {
                    host.cancel(cancel.invokeId)
                    null
                }
                start != null -> {
                    val params = start.params.mapNotNull { (name, values) -> values.firstOrNull()?.let { name to it } }.toMap()
                    when (val outcome = host.start(operation, start.invokeId, params)) {
                        is Outcome.Done -> StateMachineEngine.HostInvokeResponse(doneData = outcome.data)
                        is Outcome.Refuse -> StateMachineEngine.HostInvokeResponse(refusal = outcome.data)
                        else -> null
                    }
                }
                else -> null
            }
        }
    }
}
