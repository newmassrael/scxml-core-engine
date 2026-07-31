// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 7aab3b29aa8f5ef17f1c8730c3954aecc89c78aabf4a2226d70ddd8c24038efe
// generated-at: 1785490018
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
