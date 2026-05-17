// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: c1736039ea6628ae1068e428522a9d89bbe2ccef2705503db256c49ec169955e
// generated-at: 1778992486
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test239.scxml:1
package com.sce.w3c

import com.sce.generated.test239.Test239Event
import com.sce.generated.test239.Test239State
import com.sce.generated.test239.Test239StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: Invoked services of type http://www.w3.org/TR/scxml/, http://www.w3.org/TR/ccxml/, http://www.w3.org/TR/voicexml30/, or http://www.w3.org/TR/voicexml21 MUST interpret values specified by the content element or 'src' attribute as markup to be executed
@DisplayName("Test 239 -- W3C SCXML 6.4")
class Test239 : W3CTestBase<Test239State, Test239Event>() {
    override fun createStateMachine() = Test239StateMachine()
    override val expectedPassState: Test239State = Test239State.Pass
    override val timeoutMs: Long = 5000L
}
