// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 7b98e70bedf81ba26d5ed411dff10dd848488064e2842a1bb6ffe1d783a88c2a
// generated-at: 0
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
