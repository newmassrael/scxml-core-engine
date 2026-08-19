// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 60da764009afb96185d876c542254f2e8363dba627394829757a2a8f121eddd1
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
