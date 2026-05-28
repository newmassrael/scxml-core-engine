// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 5bcd19449227e607bbf4637f80b3a21d971f8561ecea8393b7bab39ff5ce1cc8
// generated-at: 1779976072
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test318.scxml:1
package com.sce.w3c

import com.sce.generated.test318.Test318Event
import com.sce.generated.test318.Test318State
import com.sce.generated.test318.Test318StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The SCXML Processor MUST bind the _event variable when an event is pulled off the internal or external event queue to be processed, and MUST keep the variable bound to that event until another event is processed.
@DisplayName("Test 318 -- W3C SCXML 5.10")
class Test318 : W3CTestBase<Test318State, Test318Event>() {
    override fun createStateMachine() = Test318StateMachine(createEngine())
    override val expectedPassState: Test318State = Test318State.Pass
}
