// SCE-GENERATED — DO NOT EDIT
// source-hash: 75f0d3c43c82c6af748dd1f1aba98de1b9c4f7c51e43e8850cdf3b0490c95be4
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
