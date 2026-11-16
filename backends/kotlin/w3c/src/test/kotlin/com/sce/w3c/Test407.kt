// SCE-GENERATED — DO NOT EDIT
// source-hash: 7f418d70ffbf9e34303ec5b694ace0e8ad8138948007a395fa2c2a2380179efc
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
