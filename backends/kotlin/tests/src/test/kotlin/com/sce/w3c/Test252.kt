// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 4b3c3c02df8fbc8c8bdd14a46e1f1d9b76a9416609a553ce18199941c3392f19
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test252.scxml:1
package com.sce.w3c

import com.sce.generated.test252.Test252Event
import com.sce.generated.test252.Test252State
import com.sce.generated.test252.Test252StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: Once it cancels an invoked session, the Processor MUST NOT insert any events it receives from the invoked session into the external event queue of the invoking session.
@DisplayName("Test 252 -- W3C SCXML 6.4")
class Test252 : W3CTestBase<Test252State, Test252Event>() {
    override fun createStateMachine() = Test252StateMachine()
    override val expectedPassState: Test252State = Test252State.Pass
    override val timeoutMs: Long = 5000L
}
