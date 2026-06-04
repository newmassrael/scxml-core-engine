// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 7b1a0066fa6a7fefceddfcf4d1e81b9d1fe50e95dd2b02645dfe86a65f3b96fe
// generated-at: 1780606838
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test189.scxml:1
package com.sce.w3c

import com.sce.generated.test189.Test189Event
import com.sce.generated.test189.Test189State
import com.sce.generated.test189.Test189StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.1: When using the scxml event i/o processor] If the target is the special term '#_internal', the Processor MUST add the event to the internal event queue of the sending session
@DisplayName("Test 189 -- W3C SCXML C.1")
class Test189 : W3CTestBase<Test189State, Test189Event>() {
    override fun createStateMachine() = Test189StateMachine()
    override val expectedPassState: Test189State = Test189State.Pass
}
