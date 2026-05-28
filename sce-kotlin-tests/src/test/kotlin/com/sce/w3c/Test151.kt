// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: d578a9cfec09708cd26393ca0d01ceccd7a2c1ee3a13c2911d4850d61b99f2ce
// generated-at: 1779985213
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test151.scxml:1
package com.sce.w3c

import com.sce.generated.test151.Test151Event
import com.sce.generated.test151.Test151State
import com.sce.generated.test151.Test151StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 4.6: In the foreach element, if 'index' is present, the SCXML processor MUST declare a new variable if the one specified by 'index' is not already defined.
@DisplayName("Test 151 -- W3C SCXML 4.6")
class Test151 : W3CTestBase<Test151State, Test151Event>() {
    override fun createStateMachine() = Test151StateMachine(createEngine())
    override val expectedPassState: Test151State = Test151State.Pass
}
