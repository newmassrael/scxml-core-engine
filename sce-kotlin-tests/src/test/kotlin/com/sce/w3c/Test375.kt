// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: d578a9cfec09708cd26393ca0d01ceccd7a2c1ee3a13c2911d4850d61b99f2ce
// generated-at: 1779985213
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test375.scxml:1
package com.sce.w3c

import com.sce.generated.test375.Test375Event
import com.sce.generated.test375.Test375State
import com.sce.generated.test375.Test375StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.8: The SCXML processor MUST execute the onentry handlers of a state in document order when the state is entered.
@DisplayName("Test 375 -- W3C SCXML 3.8")
class Test375 : W3CTestBase<Test375State, Test375Event>() {
    override fun createStateMachine() = Test375StateMachine()
    override val expectedPassState: Test375State = Test375State.Pass
}
