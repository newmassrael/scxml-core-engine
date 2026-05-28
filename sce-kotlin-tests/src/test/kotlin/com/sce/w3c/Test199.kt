// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: d578a9cfec09708cd26393ca0d01ceccd7a2c1ee3a13c2911d4850d61b99f2ce
// generated-at: 1779985213
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test199.scxml:1
package com.sce.w3c

import com.sce.generated.test199.Test199Event
import com.sce.generated.test199.Test199State
import com.sce.generated.test199.Test199StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: If the SCXML Processor does not support the type that is specified, it MUST place the event error.execution on the internal event queue.
@DisplayName("Test 199 -- W3C SCXML 6.2")
class Test199 : W3CTestBase<Test199State, Test199Event>() {
    override fun createStateMachine() = Test199StateMachine()
    override val expectedPassState: Test199State = Test199State.Pass
}
