// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 8c07c8492e1d3772f35a6f68b5368478c96a8b79da5206f3742243a191d8e3b5
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test247.scxml:1
package com.sce.w3c

import com.sce.generated.test247.Test247Event
import com.sce.generated.test247.Test247State
import com.sce.generated.test247.Test247StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: If the invoked state machine is of type http://www.w3.org/TR/scxml/ and it reaches a top-level final state, the Processor MUST place the event done.invoke.id on the external event queue of the invoking machine, where id is the invokeid for this invocation
@DisplayName("Test 247 -- W3C SCXML 6.4")
class Test247 : W3CTestBase<Test247State, Test247Event>() {
    override fun createStateMachine() = Test247StateMachine()
    override val expectedPassState: Test247State = Test247State.Pass
}
