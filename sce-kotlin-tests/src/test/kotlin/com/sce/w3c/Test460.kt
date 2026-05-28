// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 5bcd19449227e607bbf4637f80b3a21d971f8561ecea8393b7bab39ff5ce1cc8
// generated-at: 1779976072
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test460.scxml:1
package com.sce.w3c

import com.sce.generated.test460.Test460Event
import com.sce.generated.test460.Test460State
import com.sce.generated.test460.Test460StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript data model, since shallow copy is required for the foreach element, foreach assignment is equivalent to item = array_name[index] in ECMAScript.
@DisplayName("Test 460 -- W3C SCXML B.2")
class Test460 : W3CTestBase<Test460State, Test460Event>() {
    override fun createStateMachine() = Test460StateMachine(createEngine())
    override val expectedPassState: Test460State = Test460State.Pass
}
