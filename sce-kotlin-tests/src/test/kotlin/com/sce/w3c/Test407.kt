// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: d588114b3294b4cb4d7e02d63e6d31a3c0326d3afa0a691deb12b545b5ff5045
// generated-at: 1779460271
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test407.scxml:1
package com.sce.w3c

import com.sce.generated.test407.Test407Event
import com.sce.generated.test407.Test407State
import com.sce.generated.test407.Test407StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 3.13: To exit a state, the SCXML Processor MUST execute the executable content in the state's onexit handler.
@DisplayName("Test 407 -- W3C SCXML 3.13")
class Test407 : W3CTestBase<Test407State, Test407Event>() {
    override fun createStateMachine() = Test407StateMachine(createEngine())
    override val expectedPassState: Test407State = Test407State.Pass
}
