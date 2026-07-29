// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: aa58405544015ba4d1b8207b13e783fe4f4b991c1d05b4cc1602d85ec7348310
// generated-at: 1785339169
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
