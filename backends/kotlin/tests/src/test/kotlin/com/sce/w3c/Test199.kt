// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: de19bbe38b591cc01cdfdd92e882d57ccdf3a1af11ac8c67d61cfaa6882bea1d
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test199.scxml:1
package com.sce.w3c

import com.sce.generated.test199.Test199Event
import com.sce.generated.test199.Test199State
import com.sce.generated.test199.Test199StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: If the SCXML Processor does not support the type that is specified, it MUST place the event error.execution on the internal event queue.
@DisplayName("Test 199 -- W3C SCXML 6.2")
class Test199 : W3CTestBase<Test199State, Test199Event>() {
    override fun createStateMachine() = Test199StateMachine()
    override val expectedPassState: Test199State = Test199State.Pass
}
