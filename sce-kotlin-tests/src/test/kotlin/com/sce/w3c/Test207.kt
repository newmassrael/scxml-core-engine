// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 00f78dbe00f429352a6571b71d3b75d9ea5e69ddb859956bf6433b48017951ce
// generated-at: 1780031382
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test207.scxml:1
package com.sce.w3c

import com.sce.generated.test207.Test207Event
import com.sce.generated.test207.Test207State
import com.sce.generated.test207.Test207StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.3: The SCXML Processor MUST NOT allow cancel to affect events that were not raised in the same session.
@DisplayName("Test 207 -- W3C SCXML 6.3")
class Test207 : W3CTestBase<Test207State, Test207Event>() {
    override fun createStateMachine() = Test207StateMachine()
    override val expectedPassState: Test207State = Test207State.Pass
    override val timeoutMs: Long = 5000L
}
