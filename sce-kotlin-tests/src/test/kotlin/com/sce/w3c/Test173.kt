// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 5bcd19449227e607bbf4637f80b3a21d971f8561ecea8393b7bab39ff5ce1cc8
// generated-at: 1779976072
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test173.scxml:1
package com.sce.w3c

import com.sce.generated.test173.Test173Event
import com.sce.generated.test173.Test173State
import com.sce.generated.test173.Test173StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: If 'targetexpr' is present, the SCXML Processor MUST evaluate it when the parent send element is evaluated and treat the result as if it had been entered as the value of 'target'.
@DisplayName("Test 173 -- W3C SCXML 6.2")
class Test173 : W3CTestBase<Test173State, Test173Event>() {
    override fun createStateMachine() = Test173StateMachine(createEngine())
    override val expectedPassState: Test173State = Test173State.Pass
}
