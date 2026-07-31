// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 7aab3b29aa8f5ef17f1c8730c3954aecc89c78aabf4a2226d70ddd8c24038efe
// generated-at: 1785490018
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
