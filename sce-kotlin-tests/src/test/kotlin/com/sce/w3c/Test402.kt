// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 5bcd19449227e607bbf4637f80b3a21d971f8561ecea8393b7bab39ff5ce1cc8
// generated-at: 1779976072
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test402.scxml:1
package com.sce.w3c

import com.sce.generated.test402.Test402Event
import com.sce.generated.test402.Test402State
import com.sce.generated.test402.Test402StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.12: The processor MUST process them [error events] like any other event.
@DisplayName("Test 402 -- W3C SCXML 3.12")
class Test402 : W3CTestBase<Test402State, Test402Event>() {
    override fun createStateMachine() = Test402StateMachine(createEngine())
    override val expectedPassState: Test402State = Test402State.Pass
}
