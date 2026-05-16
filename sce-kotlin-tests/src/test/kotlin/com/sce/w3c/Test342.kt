// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 9faef2370910e1d1b12ff0b00a3d63d3578977b6f3f2045b8b014f47fa072349
// generated-at: 1778932425
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test342.scxml:1
package com.sce.w3c

import com.sce.generated.test342.Test342Event
import com.sce.generated.test342.Test342State
import com.sce.generated.test342.Test342StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The SCXML Processor MUST set the name field (of the _event variable) to the name of the event.
@DisplayName("Test 342 -- W3C SCXML 5.10")
class Test342 : W3CTestBase<Test342State, Test342Event>() {
    override fun createStateMachine() = Test342StateMachine(createEngine())
    override val expectedPassState: Test342State = Test342State.Pass
}
