// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 5bcd19449227e607bbf4637f80b3a21d971f8561ecea8393b7bab39ff5ce1cc8
// generated-at: 1779976072
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test576.scxml:1
package com.sce.w3c

import com.sce.generated.test576.Test576Event
import com.sce.generated.test576.Test576State
import com.sce.generated.test576.Test576StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.2: At system initialization time, the SCXML Processor MUST enter the states specified by the 'initial' attribute, if it is present.
@DisplayName("Test 576 -- W3C SCXML 3.2")
class Test576 : W3CTestBase<Test576State, Test576Event>() {
    override fun createStateMachine() = Test576StateMachine()
    override val expectedPassState: Test576State = Test576State.Pass
}
