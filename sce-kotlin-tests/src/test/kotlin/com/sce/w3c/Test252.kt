// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 00f78dbe00f429352a6571b71d3b75d9ea5e69ddb859956bf6433b48017951ce
// generated-at: 1780031382
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test252.scxml:1
package com.sce.w3c

import com.sce.generated.test252.Test252Event
import com.sce.generated.test252.Test252State
import com.sce.generated.test252.Test252StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: Once it cancels an invoked session, the Processor MUST NOT insert any events it receives from the invoked session into the external event queue of the invoking session.
@DisplayName("Test 252 -- W3C SCXML 6.4")
class Test252 : W3CTestBase<Test252State, Test252Event>() {
    override fun createStateMachine() = Test252StateMachine()
    override val expectedPassState: Test252State = Test252State.Pass
    override val timeoutMs: Long = 5000L
}
