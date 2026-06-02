// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: bc7b5b1dd90f65e6c3a4df2e3c4223cf8922d7e6b2d5d124b66683d16074cb6e
// generated-at: 1780362263
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
