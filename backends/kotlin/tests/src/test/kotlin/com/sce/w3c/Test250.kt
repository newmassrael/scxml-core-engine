// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 42298195b20865d87e273e6a89fd9b7e20af26d02f54273007f21322d047b5d4
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test250.scxml:1
package com.sce.w3c

import com.sce.generated.test250.Test250Event
import com.sce.generated.test250.Test250State
import com.sce.generated.test250.Test250StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: When an invoked process of type http://www.w3.org/TR/scxml/is cancelled by the invoking process, the Processor MUST execute the onexit handlers for all active states in the invoked session
@DisplayName("Test 250 -- W3C SCXML 6.4")
class Test250 : W3CTestBase<Test250State, Test250Event>() {
    override fun createStateMachine() = Test250StateMachine(createEngine())
    override val expectedPassState: Test250State = Test250State.Final
    override val timeoutMs: Long = 5000L
}
