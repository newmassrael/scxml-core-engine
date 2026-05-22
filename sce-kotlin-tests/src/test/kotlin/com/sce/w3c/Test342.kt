// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: d588114b3294b4cb4d7e02d63e6d31a3c0326d3afa0a691deb12b545b5ff5045
// generated-at: 1779460271
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
