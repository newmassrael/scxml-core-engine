// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 00f78dbe00f429352a6571b71d3b75d9ea5e69ddb859956bf6433b48017951ce
// generated-at: 1780031382
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test192.scxml:1
package com.sce.w3c

import com.sce.generated.test192.Test192Event
import com.sce.generated.test192.Test192State
import com.sce.generated.test192.Test192StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.1: [When using the scxml event i/o processor] If the target is the special term '#_invokeid', where invokeid is the invokeid of an SCXML session that the sending session has created by invoke, the Processor MUST must add the event to the external queue of that session.
@DisplayName("Test 192 -- W3C SCXML C.1")
class Test192 : W3CTestBase<Test192State, Test192Event>() {
    override fun createStateMachine() = Test192StateMachine()
    override val expectedPassState: Test192State = Test192State.Pass
    override val timeoutMs: Long = 5000L
}
