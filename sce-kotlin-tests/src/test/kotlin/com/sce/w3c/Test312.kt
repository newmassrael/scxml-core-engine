// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 7b1a0066fa6a7fefceddfcf4d1e81b9d1fe50e95dd2b02645dfe86a65f3b96fe
// generated-at: 1780606838
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test312.scxml:1
package com.sce.w3c

import com.sce.generated.test312.Test312Event
import com.sce.generated.test312.Test312State
import com.sce.generated.test312.Test312StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.9: If a value expression does not return a legal data value, the SCXML processor MUST place the error error.execution in the internal event queue.
@DisplayName("Test 312 -- W3C SCXML 5.9")
class Test312 : W3CTestBase<Test312State, Test312Event>() {
    override fun createStateMachine() = Test312StateMachine(createEngine())
    override val expectedPassState: Test312State = Test312State.Pass
}
