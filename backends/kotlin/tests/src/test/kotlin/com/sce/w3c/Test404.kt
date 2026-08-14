// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 084a969fb5abb3571d5265141500a73eb8505542dc564e6df26ed5160df0909f
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test404.scxml:1
package com.sce.w3c

import com.sce.generated.test404.Test404Event
import com.sce.generated.test404.Test404State
import com.sce.generated.test404.Test404StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: To execute a set of transitions, the SCXML Processor MUST first exit all the states in the transitions' exit set in exit order.
@DisplayName("Test 404 -- W3C SCXML 3.13")
class Test404 : W3CTestBase<Test404State, Test404Event>() {
    override fun createStateMachine() = Test404StateMachine()
    override val expectedPassState: Test404State = Test404State.Pass
}
