// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: de19bbe38b591cc01cdfdd92e882d57ccdf3a1af11ac8c67d61cfaa6882bea1d
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test208.scxml:1
package com.sce.w3c

import com.sce.generated.test208.Test208Event
import com.sce.generated.test208.Test208State
import com.sce.generated.test208.Test208StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.3: The Processor SHOULD make its best attempt to cancel all delayed events with the specified id.
@DisplayName("Test 208 -- W3C SCXML 6.3")
class Test208 : W3CTestBase<Test208State, Test208Event>() {
    override fun createStateMachine() = Test208StateMachine()
    override val expectedPassState: Test208State = Test208State.Pass
    override val timeoutMs: Long = 5000L
}
