// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 4b3c3c02df8fbc8c8bdd14a46e1f1d9b76a9416609a553ce18199941c3392f19
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test240.scxml:1
package com.sce.w3c

import com.sce.generated.test240.Test240Event
import com.sce.generated.test240.Test240State
import com.sce.generated.test240.Test240StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: Invoked services of type http://www.w3.org/TR/scxml/, http://www.w3.org/TR/ccxml/, http://www.w3.org/TR/voicexml30/, or http://www.w3.org/TR/voicexml21 MUST interpret values specified by param element or 'namelist' attribute as values that are to be injected into their data models
@DisplayName("Test 240 -- W3C SCXML 6.4")
class Test240 : W3CTestBase<Test240State, Test240Event>() {
    override fun createStateMachine() = Test240StateMachine(createEngine())
    override val expectedPassState: Test240State = Test240State.Pass
    override val timeoutMs: Long = 5000L
}
