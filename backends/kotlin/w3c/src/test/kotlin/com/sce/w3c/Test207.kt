// SCE-GENERATED — DO NOT EDIT
// source-hash: 7f418d70ffbf9e34303ec5b694ace0e8ad8138948007a395fa2c2a2380179efc
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test207.scxml:1
package com.sce.w3c

import com.sce.generated.test207.Test207Event
import com.sce.generated.test207.Test207State
import com.sce.generated.test207.Test207StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.3: The SCXML Processor MUST NOT allow cancel to affect events that were not raised in the same session.
@DisplayName("Test 207 -- W3C SCXML 6.3")
class Test207 : W3CTestBase<Test207State, Test207Event>() {
    override fun createStateMachine() = Test207StateMachine()
    override val expectedPassState: Test207State = Test207State.Pass
    override val timeoutMs: Long = 5000L
}
