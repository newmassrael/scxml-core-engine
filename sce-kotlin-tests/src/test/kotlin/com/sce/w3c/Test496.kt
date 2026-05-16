// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 9faef2370910e1d1b12ff0b00a3d63d3578977b6f3f2045b8b014f47fa072349
// generated-at: 1778932425
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test496.scxml:1
package com.sce.w3c

import com.sce.generated.test496.Test496Event
import com.sce.generated.test496.Test496State
import com.sce.generated.test496.Test496StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML C.1: If the sending SCXML session specifies a session that does not exist or is inaccessible, the SCXML Processor MUST place the error error.communication on the internal event queue of the sending session.
@DisplayName("Test 496 -- W3C SCXML C.1")
class Test496 : W3CTestBase<Test496State, Test496Event>() {
    override fun createStateMachine() = Test496StateMachine(createEngine())
    override val expectedPassState: Test496State = Test496State.Pass
}
