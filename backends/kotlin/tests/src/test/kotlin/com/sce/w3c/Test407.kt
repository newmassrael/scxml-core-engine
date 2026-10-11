// SCE-GENERATED — DO NOT EDIT
// source-hash: ca3b055deba6b38537c3642140f701b5f98f1b971d59425d3c780765d8a6a19b
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
