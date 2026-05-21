// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 9c6a4f8dfaed131dd8c4550407375e80f92b4e4373728b55d22f59422722a6ba
// generated-at: 1779372462
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test250.scxml:1
package com.sce.w3c

import com.sce.generated.test250.Test250Event
import com.sce.generated.test250.Test250State
import com.sce.generated.test250.Test250StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: When an invoked process of type http://www.w3.org/TR/scxml/is cancelled by the invoking process, the Processor MUST execute the onexit handlers for all active states in the invoked session
@DisplayName("Test 250 -- W3C SCXML 6.4")
class Test250 : W3CTestBase<Test250State, Test250Event>() {
    override fun createStateMachine() = Test250StateMachine(createEngine())
    override val expectedPassState: Test250State = Test250State.Final
    override val timeoutMs: Long = 5000L
}
