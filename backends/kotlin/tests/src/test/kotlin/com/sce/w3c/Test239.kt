// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 45fa83625e6b8ed5f1d3803a56ad41a23f2d14f770e66b07d9e986dd8b492ac0
// generated-at: 0
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
