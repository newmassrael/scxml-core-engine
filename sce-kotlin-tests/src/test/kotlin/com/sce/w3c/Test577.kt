// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: d578a9cfec09708cd26393ca0d01ceccd7a2c1ee3a13c2911d4850d61b99f2ce
// generated-at: 1779985213
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test577.scxml:1
package com.sce.w3c

import com.sce.generated.test577.Test577Event
import com.sce.generated.test577.Test577State
import com.sce.generated.test577.Test577StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.2: If neither the 'target' nor the 'targetexpr' attribute is specified, the SCXML Processor MUST add the event error.communication to the internal event queue of the sending session.
@DisplayName("Test 577 -- W3C SCXML C.2")
class Test577 : W3CTestBase<Test577State, Test577Event>() {
    override fun createStateMachine() = Test577StateMachine()
    override val expectedPassState: Test577State = Test577State.Pass
}
