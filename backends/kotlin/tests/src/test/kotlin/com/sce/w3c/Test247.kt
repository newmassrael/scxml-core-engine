// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 15d6029f4f85b34e5f54293320d31d5c234aec7e25e520553a3723febef59859
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
