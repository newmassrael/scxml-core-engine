// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 7aab3b29aa8f5ef17f1c8730c3954aecc89c78aabf4a2226d70ddd8c24038efe
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test506.scxml:1
package com.sce.w3c

import com.sce.generated.test506.Test506Event
import com.sce.generated.test506.Test506State
import com.sce.generated.test506.Test506StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: If a transition has 'type' of "internal", but its source state is not a compound state or its target states are not all proper descendents of its source state, its exit set is defined as if it had 'type' of "external".
@DisplayName("Test 506 -- W3C SCXML 3.13")
class Test506 : W3CTestBase<Test506State, Test506Event>() {
    override fun createStateMachine() = Test506StateMachine(createEngine())
    override val expectedPassState: Test506State = Test506State.Pass
}
