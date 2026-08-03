// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 32616a8c15423facd5b04f320c1acfc24557f2b58b0b9ca0229cb783903eb112
// generated-at: 0
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test530.scxml:1
package com.sce.w3c

import com.sce.generated.test530.Test530Event
import com.sce.generated.test530.Test530State
import com.sce.generated.test530.Test530StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 6.4: The SCXML Processor MUST evaluate a child content element when the parent invoke element is evaluated and pass the resulting data to the invoked service.
@DisplayName("Test 530 -- W3C SCXML 6.4")
class Test530 : W3CTestBase<Test530State, Test530Event>() {
    override fun createStateMachine() = Test530StateMachine(createEngine())
    override val expectedPassState: Test530State = Test530State.Pass
}
