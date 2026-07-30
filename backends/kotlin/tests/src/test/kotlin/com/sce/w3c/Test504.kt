// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 615c09cf1e666fafc78d1f8f6d6f319491336c3f372af9d38785e88a213f5256
// generated-at: 1785425248
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test504.scxml:1
package com.sce.w3c

import com.sce.generated.test504.Test504Event
import com.sce.generated.test504.Test504State
import com.sce.generated.test504.Test504StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: if [a transition's] 'type' is "external", its exit set consists of all active states that are proper descendents of the Least Common Compound Ancestor (LCCA) of the source and target states.
@DisplayName("Test 504 -- W3C SCXML 3.13")
class Test504 : W3CTestBase<Test504State, Test504Event>() {
    override fun createStateMachine() = Test504StateMachine(createEngine())
    override val expectedPassState: Test504State = Test504State.Pass
}
