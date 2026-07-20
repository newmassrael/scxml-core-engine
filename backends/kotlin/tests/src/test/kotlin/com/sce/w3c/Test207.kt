// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 35c0d03dd34b8d03e7b3891d6751af3cdd0b2bf0e96c5f94ca9790ac72375270
// generated-at: 1784525850
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
