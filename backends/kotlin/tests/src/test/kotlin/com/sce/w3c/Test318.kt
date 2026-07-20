// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 35c0d03dd34b8d03e7b3891d6751af3cdd0b2bf0e96c5f94ca9790ac72375270
// generated-at: 1784525850
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
