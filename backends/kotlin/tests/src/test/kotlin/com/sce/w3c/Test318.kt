// SCE-GENERATED — DO NOT EDIT
// source-hash: ca3b055deba6b38537c3642140f701b5f98f1b971d59425d3c780765d8a6a19b
// GENERATED -- DO NOT EDIT (sce-codegen)
// SCE-MAP: test318.scxml:1
package com.sce.w3c

import com.sce.generated.test318.Test318Event
import com.sce.generated.test318.Test318State
import com.sce.generated.test318.Test318StateMachine
import org.junit.jupiter.api.DisplayName

// W3C SCXML 5.10: The SCXML Processor MUST bind the _event variable when an event is pulled off the internal or external event queue to be processed, and MUST keep the variable bound to that event until another event is processed.
@DisplayName("Test 318 -- W3C SCXML 5.10")
class Test318 : W3CTestBase<Test318State, Test318Event>() {
    override fun createStateMachine() = Test318StateMachine(createEngine())
    override val expectedPassState: Test318State = Test318State.Pass
}
