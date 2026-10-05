// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A QuickJS runtime measures its stack limit from the thread that created it, so
// one that is later handed to another thread judges that thread's stack against
// the first one's address and reports a stack overflow wherever the two differ
// enough. The coroutine mode runs a machine on `Dispatchers.Default`, a thread
// other than the one that built its engine, so an expression the engine
// evaluates there can fail for no reason the document gave — and an invocation's
// `_sce_deadline_ms` that fails that way is dropped as an argument that cannot
// be evaluated while the invocation itself starts, which is a deadline that is
// never armed.
//
// The engine is created on this thread and handed, one expression at a time, to
// threads that are all alive together — so their stacks lie at different
// addresses, below this one's — at several depths of their own stack. Each must
// answer as the engine would on the thread it was made on.

package com.sce.integration

import com.sce.scripting.quickjs.QuickJSScriptEngine
import java.util.concurrent.ConcurrentLinkedQueue
import java.util.concurrent.CountDownLatch
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class AQuickJsEngineEvaluatesOnAnyThreadItIsHandedToTest {

    private fun descend(frames: Int, body: () -> Unit) {
        if (frames == 0) body() else descend(frames - 1, body)
    }

    @Test
    fun anExpressionIsEvaluatedAsItIsOnTheThreadTheEngineWasMadeOn() {
        val engine = QuickJSScriptEngine()
        engine.createSession("s")
        try {
            // Where it is made, and first used.
            assertEquals(42.0, (engine.evaluateExpr("s", "40 + 2") as Number).toDouble())

            val threads = 48
            val go = CountDownLatch(1)
            val failures = ConcurrentLinkedQueue<String>()
            val evaluators = List(threads) { n ->
                // Every thread is alive when the others are, so each has a stack of
                // its own; the stack size leaves room for the deepest descent.
                Thread(null, {
                    go.await()
                    descend((n % 8) * 60) {
                        // One thread at a time, as the engine is used by a machine
                        // (under its macrostep lock): a context is not thread-safe,
                        // and what is held apart here is which thread, not whether.
                        synchronized(engine) {
                            try {
                                val answer = (engine.evaluateExpr("s", "40 + 2") as Number).toDouble()
                                if (answer != 42.0) failures.add("evaluator-$n answered $answer")
                            } catch (e: Throwable) {
                                failures.add("evaluator-$n: ${e.message?.lineSequence()?.firstOrNull()}")
                            }
                        }
                    }
                }, "evaluator-$n", 4L shl 20).also { it.start() }
            }
            go.countDown()
            evaluators.forEach { it.join() }
            assertTrue(
                failures.isEmpty(),
                "${failures.size} of $threads threads got a wrong answer from an engine made on another: " +
                    failures.take(6).joinToString("; "),
            )
        } finally {
            engine.destroySession("s")
        }
    }
}
