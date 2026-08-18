// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 6316b28d6d1ad6f020128cbcac8380bcbacc703c59c413c6fbd546c800047e63
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test533.scxml:1
package com.sce.w3c

import com.sce.generated.test533.Test533Event
import com.sce.generated.test533.Test533State
import com.sce.generated.test533.Test533StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: If a transition has 'type' of "internal", but its source state is not a compound state, its exit set is defined as if it had 'type' of "external".
@DisplayName("Test 533 -- W3C SCXML 3.13")
class Test533 : W3CTestBase<Test533State, Test533Event>() {
    override fun createStateMachine() = Test533StateMachine(createEngine())
    override val expectedPassState: Test533State = Test533State.Pass
}
