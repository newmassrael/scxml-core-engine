// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 685fb4e0713193a522c8703edbc4c7f9a7c6eb1a29822dc1f9bfa6c38d3bf333
// generated-at: 1780579912
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test186.scxml:1
package com.sce.w3c

import com.sce.generated.test186.Test186Event
import com.sce.generated.test186.Test186State
import com.sce.generated.test186.Test186StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.2: The Processor MUST evaluate all arguments to send when the send element is evaluated, and not when the message is actually dispatched.
@DisplayName("Test 186 -- W3C SCXML 6.2")
class Test186 : W3CTestBase<Test186State, Test186Event>() {
    override fun createStateMachine() = Test186StateMachine(createEngine())
    override val expectedPassState: Test186State = Test186State.Pass
    override val timeoutMs: Long = 5000L
}
