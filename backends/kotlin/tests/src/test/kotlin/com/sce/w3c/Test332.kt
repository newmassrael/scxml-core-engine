// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: f160b18d725f2c0387242c0463da6808a5b8be392d0dc888f0d564e42c83db17
// generated-at: 1785486331
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
