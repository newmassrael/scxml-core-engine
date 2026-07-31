// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 7aab3b29aa8f5ef17f1c8730c3954aecc89c78aabf4a2226d70ddd8c24038efe
// generated-at: 1785490018
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test189.scxml:1
package com.sce.w3c

import com.sce.generated.test189.Test189Event
import com.sce.generated.test189.Test189State
import com.sce.generated.test189.Test189StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.1: When using the scxml event i/o processor] If the target is the special term '#_internal', the Processor MUST add the event to the internal event queue of the sending session
@DisplayName("Test 189 -- W3C SCXML C.1")
class Test189 : W3CTestBase<Test189State, Test189Event>() {
    override fun createStateMachine() = Test189StateMachine()
    override val expectedPassState: Test189State = Test189State.Pass
}
