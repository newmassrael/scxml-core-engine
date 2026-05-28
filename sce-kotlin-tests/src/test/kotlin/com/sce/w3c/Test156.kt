// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 5bcd19449227e607bbf4637f80b3a21d971f8561ecea8393b7bab39ff5ce1cc8
// generated-at: 1779976072
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test156.scxml:1
package com.sce.w3c

import com.sce.generated.test156.Test156Event
import com.sce.generated.test156.Test156State
import com.sce.generated.test156.Test156StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 4.6: If the evaluation of any child element of foreach causes an error, the processor MUST cease execution of the foreach element and the block that contains it.
@DisplayName("Test 156 -- W3C SCXML 4.6")
class Test156 : W3CTestBase<Test156State, Test156Event>() {
    override fun createStateMachine() = Test156StateMachine(createEngine())
    override val expectedPassState: Test156State = Test156State.Pass
}
