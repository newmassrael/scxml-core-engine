// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0096: the values a refused start hands its host, held on their own.
// A refusal names the step that failed in the word every backend writes, the
// sentence is the one every backend writes, and a creation failure is told
// apart from a shared library that did not load wherever in the chain of
// causes the library failure sits. The generated machine that hands these over
// is held in `com.sce.integration.AMachineWhoseScriptSessionFailsDoesNotStartTest`.

package com.sce.runtime

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertSame
import kotlin.test.assertTrue

class ScriptSessionFailureTest {
    @Test
    fun `each step is named by the word every backend writes`() {
        assertEquals("create", ScriptSessionStage.CREATE.toString())
        assertEquals("library", ScriptSessionStage.LIBRARY.toString())
        assertEquals("system-variables", ScriptSessionStage.SYSTEM_VARIABLES.toString())
    }

    @Test
    fun `a failure is the sentence every backend writes`() {
        val failure = ScriptSessionFailure(ScriptSessionStage.CREATE, "out of memory")
        assertEquals(
            "the script session could not be established at step 'create': out of memory",
            failure.toString(),
        )
        assertEquals(failure.toString(), ScriptSessionUnavailableException(failure).message)
    }

    @Test
    fun `a library that did not load is the library step, wherever it sits in the causes`() {
        val library = ScriptLibraryNotLoadedException("JSON builtins", "syntax error")
        assertEquals("failed to load JSON builtins: syntax error", library.message)
        assertEquals(ScriptSessionStage.LIBRARY, ScriptSessionFailure.ofCreation(library).stage)
        assertEquals(library.message, ScriptSessionFailure.ofCreation(library).message)

        val wrapped = IllegalStateException("the engine could not start", library)
        assertEquals(ScriptSessionStage.LIBRARY, ScriptSessionFailure.ofCreation(wrapped).stage)
    }

    @Test
    fun `any other creation failure is the create step`() {
        val failure = ScriptSessionFailure.ofCreation(ScriptEngineException("Failed to create Lua state"))
        assertEquals(ScriptSessionStage.CREATE, failure.stage)
        assertEquals("Failed to create Lua state", failure.message)
    }

    @Test
    fun `a failed bind is the system-variables step`() {
        val failure = ScriptSessionFailure.ofSystemVariables(ScriptEngineException("Session not found: s"))
        assertEquals(ScriptSessionStage.SYSTEM_VARIABLES, failure.stage)
        assertEquals("Session not found: s", failure.message)
    }

    @Test
    fun `a failure that says nothing is named by its class`() {
        assertEquals("IllegalStateException", ScriptSessionFailure.ofCreation(IllegalStateException()).message)
    }

    @Test
    fun `a refused child is reported with its invoke and the step`() {
        val failure = ScriptSessionFailure(ScriptSessionStage.LIBRARY, "failed to load ECMAScript semantics: x")
        assertEquals(
            "<invoke id='chatty'> did not start: its child's script session failed at step 'library'",
            failure.childStartRefusal("chatty"),
        )
    }

    @Test
    fun `a root start is refused for a parent or for the session`() {
        assertSame(RootStartRefusal.NeedsParent, RootStartRefusal.NEEDS_PARENT)
        assertEquals(
            "the machine sends to #_parent and was started with no parent session",
            RootStartRefusal.NeedsParent.reason,
        )
        val failure = ScriptSessionFailure(ScriptSessionStage.SYSTEM_VARIABLES, "no _name")
        val refusal = RootStartRefusal.ScriptSession(failure)
        assertEquals(failure.toString(), refusal.reason)
        assertEquals(refusal.reason, refusal.toString())
    }

    @Test
    fun `an entry refused for the session carries the failure in its verdict`() {
        assertTrue(ConfigurationVerdict.ACCEPTED.accepted)
        assertEquals(ConfigurationRejection.NONE, ConfigurationVerdict.ACCEPTED.rejection)

        val failure = ScriptSessionFailure(ScriptSessionStage.CREATE, "no state")
        val refused = ConfigurationVerdict(ConfigurationRejection.SCRIPT_SESSION_UNAVAILABLE, failure)
        assertFalse(refused.accepted)
        assertEquals(failure.toString(), refused.reason)

        val broken = ConfigurationVerdict(ConfigurationRejection.EMPTY)
        assertEquals(ConfigurationRejection.EMPTY.reason, broken.reason)
    }
}
