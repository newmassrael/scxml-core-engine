// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 615c09cf1e666fafc78d1f8f6d6f319491336c3f372af9d38785e88a213f5256
// generated-at: 1785425248
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
