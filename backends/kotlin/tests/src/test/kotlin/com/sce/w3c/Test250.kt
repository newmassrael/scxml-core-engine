// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 082e347ab97b9b491598f98d263b24d185e7e030b1c1600c8a0939850d86f8db
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
