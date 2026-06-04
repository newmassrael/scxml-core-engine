// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 7b1a0066fa6a7fefceddfcf4d1e81b9d1fe50e95dd2b02645dfe86a65f3b96fe
// generated-at: 1780606838
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
