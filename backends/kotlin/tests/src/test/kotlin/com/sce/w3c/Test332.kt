// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: b987ea47cf7b98cc29f6a07fbb829bd85b24bd9991a16621d5e7458fb0482788
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test332.scxml:1
package com.sce.w3c

import com.sce.generated.test332.Test332Event
import com.sce.generated.test332.Test332State
import com.sce.generated.test332.Test332StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: If the sending entity has specified a value for this, the Processor MUST set this field to that value.  Otherwise, in the case of error events triggered by a failed attempt to send an event, the Processor MUST set the sendid field to the send id of the triggering send element.
@DisplayName("Test 332 -- W3C SCXML 5.10")
class Test332 : W3CTestBase<Test332State, Test332Event>() {
    override fun createStateMachine() = Test332StateMachine(createEngine())
    override val expectedPassState: Test332State = Test332State.Pass
}
