// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 3acf03cd1e197da0d6a3e7ecc2541747678939372fbe1d99b37c7415a38be32a
// generated-at: 1780830703
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
