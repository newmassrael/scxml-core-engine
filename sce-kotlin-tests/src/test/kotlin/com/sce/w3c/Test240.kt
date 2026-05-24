// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 2e32d261d6350eb3a25f2f20128ae90019b36b8835127308d167f05b44688be3
// generated-at: 1779589482
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
