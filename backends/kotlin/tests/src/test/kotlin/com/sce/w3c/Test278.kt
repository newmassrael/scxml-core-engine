// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 7aab3b29aa8f5ef17f1c8730c3954aecc89c78aabf4a2226d70ddd8c24038efe
// generated-at: 1785490018
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test278.scxml:1
package com.sce.w3c

import com.sce.generated.test278.Test278Event
import com.sce.generated.test278.Test278State
import com.sce.generated.test278.Test278StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML B.2: In the ECMAScript data model, the SCXML processor MUST allow any data element to be accessed from any state.
@DisplayName("Test 278 -- W3C SCXML B.2")
class Test278 : W3CTestBase<Test278State, Test278Event>() {
    override fun createStateMachine() = Test278StateMachine(createEngine())
    override val expectedPassState: Test278State = Test278State.Pass
}
