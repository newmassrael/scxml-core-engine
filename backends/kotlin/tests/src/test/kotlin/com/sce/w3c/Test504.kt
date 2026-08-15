// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 2d53d2f6482bd48bbe534a774432c7132f924eed253d3c01ee5b53a731642f97
// generated-at: 0
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
