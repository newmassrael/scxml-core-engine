// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 7aab3b29aa8f5ef17f1c8730c3954aecc89c78aabf4a2226d70ddd8c24038efe
// generated-at: 1785490018
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test577.scxml:1
package com.sce.w3c

import com.sce.generated.test577.Test577Event
import com.sce.generated.test577.Test577State
import com.sce.generated.test577.Test577StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.2: If neither the 'target' nor the 'targetexpr' attribute is specified, the SCXML Processor MUST add the event error.communication to the internal event queue of the sending session.
@DisplayName("Test 577 -- W3C SCXML C.2")
class Test577 : W3CTestBase<Test577State, Test577Event>() {
    override fun createStateMachine() = Test577StateMachine()
    override val expectedPassState: Test577State = Test577State.Pass
}
