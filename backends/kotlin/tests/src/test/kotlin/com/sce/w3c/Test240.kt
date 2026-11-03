// SCE-GENERATED — DO NOT EDIT
// source-hash: 1036b5ee37fb5c89aaa1e8156f295f52aeb22d8997a046fafa16b7778f1ace69
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
